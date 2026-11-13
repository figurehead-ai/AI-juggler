//! The driven session every line-protocol codec shares: a [`LineSession`]
//! reads the tool's lines through its [`LineCodec`], sends the lines the
//! codec makes for each call, queues the events, and lets go at a turn
//! boundary. A codec keeps only what the lines mean (Claude Code's
//! stream-json, Codex's and `ZCode`'s app servers).

use std::time::{Duration, Instant};

use serde_json::Value;

use super::book::release_turn;
use super::pipe::Pipe;
use super::{
    AskAnswer, AskId, Connection, DrivenSession, LimitsSnapshot, Release, ReleaseOutcome,
    RuntimeEvent, TurnInput, cannot,
};
use crate::error::AdapterError;

/// A line and the events it makes, as a codec's call gives them.
pub type LineAndEvents = (Value, Vec<RuntimeEvent>);

/// What one tool's protocol means: the events and replies each line it
/// writes makes, and the line each of the session's calls sends. A call
/// the tool lacks keeps its default, [`DrivenSession`]'s own words.
pub trait LineCodec: Send {
    /// One line the tool wrote: the events it makes, and any lines to send
    /// back at once.
    fn take_line(&mut self, line: &str) -> (Vec<RuntimeEvent>, Vec<Value>);
    /// The conversation's id, once known.
    fn native_id(&self) -> Option<String>;
    /// Whether a turn is running.
    fn turn_running(&self) -> bool;
    /// The line starting a turn, and the events it makes at once.
    ///
    /// # Errors
    ///
    /// While a turn runs, or before the conversation is open.
    fn turn(&mut self, input: TurnInput) -> Result<LineAndEvents, AdapterError>;
    /// The line stopping the running turn; `None` between turns.
    fn stop(&mut self) -> Option<Value>;
    /// The line carrying the user's answer to an open ask, and the events
    /// it makes.
    ///
    /// # Errors
    ///
    /// For an ask that isn't open, or an answer it doesn't take.
    fn answer(&mut self, ask: &AskId, answer: &AskAnswer) -> Result<LineAndEvents, AdapterError>;
    /// The line changing the model (where the tool takes one; else the
    /// next turn carries it), and the event saying so.
    ///
    /// # Errors
    ///
    /// Where the tool keeps the model to itself.
    fn set_model(&mut self, model: &str) -> Result<(Option<Value>, RuntimeEvent), AdapterError>;
    /// The line adding to the running turn.
    ///
    /// # Errors
    ///
    /// Where the tool can't ([`cannot::steer`] by default).
    fn steer(&mut self, input: &TurnInput) -> Result<Value, AdapterError> {
        let _ = input;
        Err(cannot::steer())
    }
    /// The line asking for the models.
    ///
    /// # Errors
    ///
    /// Where the tool can't ([`cannot::list_models`] by default).
    fn models(&mut self) -> Result<Value, AdapterError> {
        Err(cannot::list_models())
    }
    /// The line compacting the conversation, and the events it makes.
    ///
    /// # Errors
    ///
    /// Where the tool can't ([`cannot::compact`] by default).
    fn compact(&mut self) -> Result<LineAndEvents, AdapterError> {
        Err(cannot::compact())
    }
    /// The line asking for the plan's limits; `None` where the tool
    /// reports them by itself or not at all.
    fn limits_request(&mut self) -> Option<Value> {
        None
    }
    /// The limits the tool last reported.
    fn limits(&self) -> Option<LimitsSnapshot> {
        None
    }
}

/// One driven session: the connection and its codec.
pub struct LineSession<C> {
    /// The lines, the writer and the events not yet handed out.
    pub pipe: Pipe,
    /// What the lines mean.
    pub codec: C,
    /// How errors name the tool's side ("Codex's app-server").
    name: &'static str,
}

impl<C> std::fmt::Debug for LineSession<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LineSession")
            .field("name", &self.name)
            .field("pipe", &self.pipe)
            .finish_non_exhaustive()
    }
}

impl<C: LineCodec> LineSession<C> {
    /// Starts reading `io` for `codec`; `name` names the tool's side in
    /// errors.
    #[must_use]
    pub fn new(io: Connection, codec: C, name: &'static str) -> Self {
        Self {
            pipe: Pipe::new(io),
            codec,
            name,
        }
    }

    /// Writes one line.
    ///
    /// # Errors
    ///
    /// The write's error.
    pub fn send(&mut self, value: &Value) -> Result<(), AdapterError> {
        self.pipe.send(value)
    }

    /// Reads one line (at most `timeout`) into the queue.
    ///
    /// # Errors
    ///
    /// A reply's write error.
    pub fn pump(&mut self, timeout: Duration) -> Result<(), AdapterError> {
        let Some(line) = self.pipe.read(timeout) else {
            return Ok(());
        };
        let (events, replies) = self.codec.take_line(&line);
        self.pipe.take(events, &replies)
    }

    /// Reads until `done` holds, an error is queued, the tool ends, or
    /// `wait` passes; `what` says what it was waiting for.
    ///
    /// # Errors
    ///
    /// [`AdapterError::InvalidLaunch`] with the tool's own error, or that
    /// it didn't in time.
    pub fn wait_for(
        &mut self,
        wait: Duration,
        what: &str,
        done: impl Fn(&C) -> bool,
    ) -> Result<(), AdapterError> {
        let deadline = Instant::now() + wait;
        while !done(&self.codec) {
            if let Some(why) = self.pipe.queued_error() {
                return Err(AdapterError::InvalidLaunch(format!(
                    "{} didn't {what}: {why}",
                    self.name
                )));
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() || self.pipe.exited() {
                return Err(AdapterError::InvalidLaunch(format!(
                    "{} didn't {what} in time",
                    self.name
                )));
            }
            self.pump(left.min(Duration::from_millis(200)))?;
        }
        Ok(())
    }

    fn send_and_queue(&mut self, (line, events): LineAndEvents) -> Result<(), AdapterError> {
        self.pipe.take(events, &[line])
    }
}

impl<C: LineCodec> DrivenSession for LineSession<C> {
    fn native_id(&self) -> Option<String> {
        self.codec.native_id()
    }

    fn next_event(&mut self, timeout: Duration) -> Result<Option<RuntimeEvent>, AdapterError> {
        if self.pipe.queue.is_empty() {
            self.pump(timeout)?;
        }
        Ok(self.pipe.queue.pop_front())
    }

    fn send_turn(&mut self, input: TurnInput) -> Result<(), AdapterError> {
        let turn = self.codec.turn(input)?;
        self.send_and_queue(turn)
    }

    fn steer(&mut self, input: TurnInput) -> Result<(), AdapterError> {
        let line = self.codec.steer(&input)?;
        self.send(&line)
    }

    fn interrupt(&mut self) -> Result<(), AdapterError> {
        match self.codec.stop() {
            Some(line) => self.send(&line),
            None => Ok(()),
        }
    }

    fn answer(&mut self, ask: &AskId, answer: AskAnswer) -> Result<(), AdapterError> {
        let answered = self.codec.answer(ask, &answer)?;
        self.send_and_queue(answered)
    }

    fn set_model(&mut self, model: &str) -> Result<(), AdapterError> {
        let (line, event) = self.codec.set_model(model)?;
        if let Some(line) = line {
            self.send(&line)?;
        }
        self.pipe.queue.push_back(event);
        Ok(())
    }

    fn list_models(&mut self) -> Result<(), AdapterError> {
        let line = self.codec.models()?;
        self.send(&line)
    }

    fn compact(&mut self) -> Result<(), AdapterError> {
        let compact = self.codec.compact()?;
        self.send_and_queue(compact)
    }

    fn limits(&mut self) -> Result<Option<LimitsSnapshot>, AdapterError> {
        if let Some(line) = self.codec.limits_request() {
            self.send(&line)?;
        }
        Ok(self.codec.limits())
    }

    /// The connection closes with `self` (the tool's stdin with it): one
    /// writer at a time.
    fn release(mut self: Box<Self>, how: Release) -> Result<ReleaseOutcome, AdapterError> {
        let queued = self.pipe.drain();
        let running = self.codec.turn_running();
        release_turn(self.as_mut(), queued, running, how)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::ErrorCategory;
    use std::io::Cursor;

    /// Answers once a line reads `answer`; `oops` is an error; anything
    /// else not a line it knows is counted.
    #[derive(Default)]
    struct Probe {
        answered: bool,
        skipped: usize,
    }

    impl LineCodec for Probe {
        fn take_line(&mut self, line: &str) -> (Vec<RuntimeEvent>, Vec<Value>) {
            match line {
                "answer" => self.answered = true,
                "oops" => {
                    let error = RuntimeEvent::Error {
                        category: ErrorCategory::Other,
                        message: "refused".into(),
                        will_retry: false,
                    };
                    return (vec![error], Vec::new());
                }
                _ => self.skipped += 1,
            }
            (Vec::new(), Vec::new())
        }
        fn native_id(&self) -> Option<String> {
            None
        }
        fn turn_running(&self) -> bool {
            false
        }
        fn turn(&mut self, _: TurnInput) -> Result<LineAndEvents, AdapterError> {
            Ok((Value::Null, Vec::new()))
        }
        fn stop(&mut self) -> Option<Value> {
            None
        }
        fn answer(&mut self, _: &AskId, _: &AskAnswer) -> Result<LineAndEvents, AdapterError> {
            Ok((Value::Null, Vec::new()))
        }
        fn set_model(
            &mut self,
            model: &str,
        ) -> Result<(Option<Value>, RuntimeEvent), AdapterError> {
            let event = RuntimeEvent::ModelChanged {
                model: model.to_owned(),
            };
            Ok((None, event))
        }
    }

    fn session(text: &str) -> LineSession<Probe> {
        let io = Connection {
            reader: Box::new(Cursor::new(text.as_bytes().to_vec())),
            writer: Box::new(std::io::sink()),
        };
        LineSession::new(io, Probe::default(), "The tool")
    }

    #[test]
    fn waiting_ends_at_done_an_error_or_the_end() {
        let wait = Duration::from_secs(5);
        let mut ok = session("stray\nanswer\n");
        ok.wait_for(wait, "answer", |probe| probe.answered)
            .expect("answered");
        assert_eq!(ok.codec.skipped, 1, "the stray line is counted");

        let mut refused = session("oops\n");
        let error = refused
            .wait_for(wait, "answer", |probe| probe.answered)
            .expect_err("refused");
        assert!(
            error
                .to_string()
                .contains("The tool didn't answer: refused")
        );

        let mut gone = session("");
        let error = gone
            .wait_for(wait, "answer", |probe| probe.answered)
            .expect_err("gone");
        assert!(error.to_string().contains("The tool didn't answer in time"));
        assert_eq!(
            gone.next_event(Duration::from_millis(10)).expect("event"),
            Some(RuntimeEvent::Exited)
        );
    }

    #[test]
    fn calls_a_codec_lacks_answer_as_the_session_default_does() {
        let mut session = session("");
        assert_eq!(
            session.compact().expect_err("no compact").to_string(),
            cannot::compact().to_string()
        );
        assert!(session.steer(TurnInput::from_user("x")).is_err());
        assert!(session.list_models().is_err());
        assert_eq!(session.limits().expect("limits"), None);
        session.set_model("m").expect("model");
        assert_eq!(
            session.pipe.drain(),
            vec![RuntimeEvent::ModelChanged { model: "m".into() }]
        );
        let released = Box::new(session).release(Release::Now).expect("release");
        assert!(released.at_boundary && released.tail.is_empty());
    }
}
