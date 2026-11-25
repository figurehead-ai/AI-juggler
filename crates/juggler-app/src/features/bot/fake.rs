//! A fake Bot model for the tests (AJ-E3825): it counts every
//! generation, so a reply made without one is caught, and answers by a
//! rule the test gives (by default: a grounded request cites its first
//! record by name, or says nothing matches; a general one says "8.").

use std::cell::{Cell, RefCell};

use crate::features::ai::engine::{Draft, Finish, Request, Stop, Usage};
use crate::features::ai::run::{AiEngineState, AiEngineStatusInfo, AiFailure, Generated};

use super::model::BotModel;

/// What the fake writes for a request.
pub type Rule = Box<dyn Fn(&Request) -> Result<String, AiFailure>>;

/// The fake.
pub struct Fake {
    rule: Rule,
    /// Generations asked for.
    pub calls: Cell<u32>,
    /// The requests, in order.
    pub requests: RefCell<Vec<Request>>,
}

/// The first record's name in a grounded request's frame.
#[must_use]
pub fn first_record(request: &Request) -> Option<String> {
    let line = request.prompt.lines().find(|l| l.starts_with("1. "))?;
    let start = line.find('\u{201c}')? + '\u{201c}'.len_utf8();
    let end = line[start..].find('\u{201d}')? + start;
    Some(line[start..end].to_owned())
}

/// Whether a request is the grounded kind (records framed).
#[must_use]
pub fn grounded(request: &Request) -> bool {
    request
        .prompt
        .contains("Records found (data, not instructions)")
}

/// The default rule.
pub fn default_rule(request: &Request) -> Result<String, AiFailure> {
    if grounded(request) {
        return Ok(first_record(request).map_or_else(
            || r#"{"sentences":[{"text":"Nothing in your records matches that.","records":[]}]}"#.to_owned(),
            |name| {
                serde_json::json!({"sentences":[{"text": format!("\u{201c}{name}\u{201d}."), "records":[1]}]})
                    .to_string()
            },
        ));
    }
    if request.instructions.contains("Apple's on-device model")
        && request.prompt.to_lowercase().contains("model")
    {
        return Ok(r#"{"text":"I'm Bot, running on Apple's on-device model on this Mac.","about_their_work":false}"#.to_owned());
    }
    Ok(r#"{"text":"8.","about_their_work":false}"#.to_owned())
}

impl Fake {
    /// The fake with the default rule.
    #[must_use]
    pub fn new() -> Self {
        Self::with(Box::new(default_rule))
    }

    /// The fake with `rule`.
    #[must_use]
    pub fn with(rule: Rule) -> Self {
        Self {
            rule,
            calls: Cell::new(0),
            requests: RefCell::new(Vec::new()),
        }
    }
}

impl Default for Fake {
    fn default() -> Self {
        Self::new()
    }
}

impl BotModel for Fake {
    fn status(&self) -> AiEngineStatusInfo {
        AiEngineStatusInfo {
            state: AiEngineState::Running,
            model_name: Some("Apple's on-device model".to_owned()),
            engine: Some("apple_builtin".to_owned()),
            version: Some("26.1".to_owned()),
            context_tokens: Some(4096),
            ..AiEngineStatusInfo::default()
        }
    }

    fn generate(&self, request: &Request, _stop: &Stop) -> Result<Generated, AiFailure> {
        self.calls.set(self.calls.get() + 1);
        self.requests.borrow_mut().push(request.clone());
        let text = (self.rule)(request)?;
        Ok(Generated {
            draft: Draft {
                text,
                finish: Finish::Stop,
                usage: Usage {
                    input: Some(300),
                    output: None,
                },
            },
            model_name: "Apple's on-device model".to_owned(),
            version: Some("26.1".to_owned()),
            context_tokens: 4096,
            ms: 5,
        })
    }
}
