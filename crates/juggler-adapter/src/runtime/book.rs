//! AJ-E2302: what every codec keeps the same way: the asks that are open
//! (so an answer can only be to an open ask, with an option it offered)
//! and letting go at a turn boundary. Shared by the adapters' codecs.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use super::events::{Approval, AskAnswer, AskId, Question, RuntimeEvent};
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
}
