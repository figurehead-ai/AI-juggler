//! A test model for D-149's jobs: answers in order (the last repeated),
//! keeps every request; `FAIL` answers with a timed-out engine.

use std::cell::RefCell;

use super::engine::{Draft, Finish, Request, Stop, Usage};
use super::run::{AiFailure, JobModel};

/// See the module.
pub struct Scripted {
    answers: Vec<&'static str>,
    /// Every request, in order.
    pub seen: RefCell<Vec<Request>>,
}

impl Scripted {
    /// A model answering `answers`.
    #[must_use]
    pub fn new(answers: &[&'static str]) -> Self {
        Self {
            answers: answers.to_vec(),
            seen: RefCell::new(Vec::new()),
        }
    }

    /// The calls made.
    #[must_use]
    pub fn calls(&self) -> usize {
        self.seen.borrow().len()
    }
}

impl JobModel for Scripted {
    fn generate(&self, request: &Request, _stop: &Stop) -> Result<Draft, AiFailure> {
        let n = self.seen.borrow().len();
        self.seen.borrow_mut().push(request.clone());
        let text = self.answers[n.min(self.answers.len().saturating_sub(1))];
        if text == "FAIL" {
            return Err(AiFailure::TimedOut);
        }
        Ok(Draft {
            text: text.to_owned(),
            finish: Finish::Stop,
            usage: Usage::default(),
        })
    }
}
