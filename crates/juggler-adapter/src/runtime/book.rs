//! AJ-E2302: what every codec keeps the same way: the asks that are open
//! (so an answer can only be to an open ask, with an option it offered)
//! and letting go at a turn boundary. Shared by the adapters' codecs.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use super::events::{Approval, AskAnswer, AskId, LiveStatusWord, Question, RuntimeEvent};
use super::{DrivenSession, Release, ReleaseOutcome};
use crate::error::AdapterError;

/// One open ask.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenAsk {
    /// An approval, with the options it offered.
    Approval(Approval),
    /// A question.
    Question(Question),
}

/// The asks open now, by id.
#[derive(Clone, Debug, Default)]
pub struct AskBook {
    open: BTreeMap<String, OpenAsk>,
}

impl AskBook {
    /// Notes what an event opens or closes.
    pub fn note(&mut self, event: &RuntimeEvent) {
        match event {
            RuntimeEvent::ApprovalRequested { approval } => {
                self.open
                    .insert(approval.id.0.clone(), OpenAsk::Approval(approval.clone()));
            }
            RuntimeEvent::QuestionAsked { question } => {
                self.open
                    .insert(question.id.0.clone(), OpenAsk::Question(question.clone()));
            }
            RuntimeEvent::AskResolved { id } => {
                self.open.remove(&id.0);
            }
            RuntimeEvent::TurnEnded { .. } | RuntimeEvent::Exited => self.open.clear(),
            _ => {}
        }
    }

    /// The open ask `id` names, if `answer` fits it: a choice only for an
    /// approval and only of an option it offered; a reply only for a
    /// question and only to its parts.
    ///
    /// # Errors
    ///
    /// [`AdapterError::InvalidLaunch`] in words when it doesn't fit.
    pub fn check(&self, id: &AskId, answer: &AskAnswer) -> Result<&OpenAsk, AdapterError> {
        let open = self.open.get(&id.0).ok_or_else(|| {
            AdapterError::InvalidLaunch("that question isn't open any more".into())
        })?;
        match (open, answer) {
            (OpenAsk::Approval(approval), AskAnswer::Choose { option }) => {
                super::offered(approval, option)?;
            }
            (OpenAsk::Question(question), AskAnswer::Reply { parts }) => {
                let known = |part: &str| question.parts.iter().any(|known| known.id == part);
                if parts.is_empty() || !parts.iter().all(|part| known(&part.part)) {
                    return Err(AdapterError::InvalidLaunch(
                        "that reply doesn't answer this question".into(),
                    ));
                }
            }
            _ => {
                return Err(AdapterError::InvalidLaunch(
                    "that answer doesn't fit this ask".into(),
                ));
            }
        }
        Ok(open)
    }

    /// The events the user's answer to `id` makes (the ask is resolved and
    /// the turn works again), already noted here so it is never answered
    /// twice.
    pub fn answered(&mut self, id: &AskId) -> Vec<RuntimeEvent> {
        let events = vec![
            RuntimeEvent::AskResolved { id: id.clone() },
            RuntimeEvent::State {
                status: LiveStatusWord::Busy,
            },
        ];
        for event in &events {
            self.note(event);
        }
        events
    }

    /// Whether any ask is open.
    #[must_use]
    pub fn any_open(&self) -> bool {
        !self.open.is_empty()
    }
}

/// Reads events until the running turn ends or `wait` passes, keeping
/// them: `(events, ended at a boundary)`.
pub fn drain_turn(
    next: &mut dyn FnMut(Duration) -> Result<Option<RuntimeEvent>, AdapterError>,
    wait: Duration,
) -> (Vec<RuntimeEvent>, bool) {
    let deadline = Instant::now() + wait;
    let mut tail = Vec::new();
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return (tail, false);
        }
        match next(left.min(Duration::from_millis(200))) {
            Ok(Some(event)) => {
                let ended = matches!(event, RuntimeEvent::TurnEnded { .. });
                let gone = matches!(event, RuntimeEvent::Exited);
                tail.push(event);
                if ended {
                    return (tail, true);
                }
                if gone {
                    return (tail, false);
                }
            }
            Ok(None) => {}
            Err(_) => return (tail, false),
        }
    }
}

/// How long a [`Release::Now`] waits for the interrupted turn's end.
pub const INTERRUPT_WAIT: Duration = Duration::from_secs(5);

/// The release every driven session shares. With a turn `running`, it
/// waits for the turn's end ([`Release::AfterTurn`]) or interrupts it and
/// waits [`INTERRUPT_WAIT`] ([`Release::Now`]). The outcome's tail is
/// `queued`, then what arrived meanwhile; it is at a boundary when no
/// turn was cut short; its id is the session's own.
///
/// # Errors
///
/// The interrupt's error.
pub fn release_turn(
    session: &mut dyn DrivenSession,
    queued: Vec<RuntimeEvent>,
    running: bool,
    how: Release,
) -> Result<ReleaseOutcome, AdapterError> {
    let mut tail = queued;
    let mut at_boundary = true;
    if running {
        let wait = match how {
            Release::AfterTurn(wait) => wait,
            Release::Now => {
                session.interrupt()?;
                INTERRUPT_WAIT
            }
        };
        let (more, ended) = drain_turn(&mut |timeout| session.next_event(timeout), wait);
        tail.extend(more);
        at_boundary = ended && matches!(how, Release::AfterTurn(_));
    }
    Ok(ReleaseOutcome {
        native_id: session.native_id(),
        at_boundary,
        tail,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::events::*;

    fn approval() -> RuntimeEvent {
        RuntimeEvent::ApprovalRequested {
            approval: Approval {
                id: AskId("a".into()),
                kind: ApprovalKind::Command,
                summary: "ls".into(),
                reason: None,
                options: vec![AskOption {
                    id: "once".into(),
                    label: "Allow once".into(),
                    effect: OptionEffect::AllowOnce,
                }],
            },
        }
    }

    #[test]
    fn only_open_asks_with_offered_options_are_answered() {
        let mut book = AskBook::default();
        let id = AskId("a".into());
        let choose = |option: &str| AskAnswer::Choose {
            option: option.into(),
        };
        assert!(book.check(&id, &choose("once")).is_err(), "not open yet");
        book.note(&approval());
        assert!(book.check(&id, &choose("once")).is_ok());
        assert!(book.check(&id, &choose("always")).is_err(), "not offered");
        assert!(
            book.check(&id, &AskAnswer::Reply { parts: vec![] })
                .is_err(),
            "a reply to an approval"
        );
        book.note(&RuntimeEvent::AskResolved { id: id.clone() });
        assert!(
            book.check(&id, &choose("once")).is_err(),
            "answered elsewhere"
        );
        book.note(&approval());
        let events = book.answered(&id);
        assert_eq!(events.len(), 2, "resolved, then busy");
        assert!(book.check(&id, &choose("once")).is_err(), "answered here");
        book.note(&approval());
        book.note(&RuntimeEvent::TurnEnded {
            outcome: TurnEnd::Interrupted,
            usage: None,
        });
        assert!(!book.any_open(), "a turn's end closes its asks");
    }

    #[test]
    fn draining_stops_at_the_turn_end() {
        let mut events = vec![
            RuntimeEvent::TextDelta { text: "x".into() },
            RuntimeEvent::TurnEnded {
                outcome: TurnEnd::Completed,
                usage: None,
            },
            RuntimeEvent::TextDelta {
                text: "after".into(),
            },
        ]
        .into_iter();
        let (tail, clean) = drain_turn(&mut |_| Ok(events.next()), Duration::from_secs(1));
        assert!(clean);
        assert_eq!(tail.len(), 2);
        let (tail, clean) = drain_turn(&mut |_| Ok(None), Duration::from_millis(50));
        assert!(!clean && tail.is_empty(), "no end within the wait");
    }

    /// A session whose turn ends after `left` more events (or never).
    struct Turn {
        left: Option<usize>,
        interrupted: bool,
    }

    impl crate::runtime::DrivenSession for Turn {
        fn native_id(&self) -> Option<String> {
            Some("t".into())
        }
        fn next_event(&mut self, _: Duration) -> Result<Option<RuntimeEvent>, AdapterError> {
            Ok(match &mut self.left {
                Some(0) => Some(RuntimeEvent::TurnEnded {
                    outcome: if self.interrupted {
                        TurnEnd::Interrupted
                    } else {
                        TurnEnd::Completed
                    },
                    usage: None,
                }),
                Some(left) => {
                    *left -= 1;
                    Some(RuntimeEvent::TextDelta { text: "x".into() })
                }
                None => None,
            })
        }
        fn send_turn(&mut self, _: TurnInput) -> Result<(), AdapterError> {
            Ok(())
        }
        fn interrupt(&mut self) -> Result<(), AdapterError> {
            self.interrupted = true;
            self.left = Some(0);
            Ok(())
        }
        fn answer(&mut self, _: &AskId, _: AskAnswer) -> Result<(), AdapterError> {
            Ok(())
        }
        fn set_model(&mut self, _: &str) -> Result<(), AdapterError> {
            Ok(())
        }
        fn limits(&mut self) -> Result<Option<LimitsSnapshot>, AdapterError> {
            Ok(None)
        }
        fn release(
            self: Box<Self>,
            _: Release,
        ) -> Result<crate::runtime::ReleaseOutcome, AdapterError> {
            unreachable!("not released here")
        }
    }

    #[test]
    fn a_release_waits_for_the_turn_or_interrupts_it() {
        let queued = || vec![RuntimeEvent::Exited];
        let mut idle = Turn {
            left: None,
            interrupted: false,
        };
        let out = release_turn(&mut idle, queued(), false, Release::Now).expect("idle");
        assert!(
            out.at_boundary && out.tail == queued() && !idle.interrupted,
            "no turn runs"
        );
        assert_eq!(out.native_id.as_deref(), Some("t"));

        let mut running = Turn {
            left: Some(2),
            interrupted: false,
        };
        let wait = Release::AfterTurn(Duration::from_secs(5));
        let out = release_turn(&mut running, queued(), true, wait).expect("after");
        assert!(out.at_boundary && !running.interrupted);
        assert_eq!(out.tail.len(), 4, "queued, two deltas, the end");

        let mut cut = Turn {
            left: Some(9),
            interrupted: false,
        };
        let out = release_turn(&mut cut, Vec::new(), true, Release::Now).expect("now");
        assert!(
            !out.at_boundary && cut.interrupted,
            "interrupted: no boundary"
        );
        assert_eq!(out.tail.len(), 1);

        let mut stuck = Turn {
            left: None,
            interrupted: false,
        };
        let wait = Release::AfterTurn(Duration::from_millis(50));
        let out = release_turn(&mut stuck, Vec::new(), true, wait).expect("stuck");
        assert!(!out.at_boundary, "the turn didn't end in time");
    }
}
