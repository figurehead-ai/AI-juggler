//! What the JSON-RPC driven codecs (`ZCode`'s app-server, Muse Code's
//! session server) read and keep alike: the requests sent and what each
//! was for, a field as text, an error event, a question's choices, an ask
//! that leaves the turn waiting, and whether a server runs on the folder
//! it should.

use std::collections::BTreeMap;
use std::path::Path;

use juggler_adapter::runtime::events::bounded;
use juggler_adapter::runtime::{Choice, ErrorCategory, LiveStatusWord, RuntimeEvent};
use serde_json::Value;

/// The requests a codec sent and not yet answered, each with what it was
/// for (`T`), numbered from 1.
#[derive(Debug)]
pub(crate) struct Outbox<T> {
    next: i64,
    asked: BTreeMap<i64, T>,
}

impl<T> Default for Outbox<T> {
    fn default() -> Self {
        Self {
            next: 0,
            asked: BTreeMap::new(),
        }
    }
}

impl<T> Outbox<T> {
    /// The next request's id, noting what it is for.
    pub(crate) fn note(&mut self, what: T) -> i64 {
        self.next += 1;
        self.asked.insert(self.next, what);
        self.next
    }

    /// What answered request `id` was for, once.
    pub(crate) fn answered(&mut self, id: i64) -> Option<T> {
        self.asked.remove(&id)
    }
}

/// `value[key]` as text; empty when it isn't a string.
pub(crate) fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// An error the user sees, bounded.
pub(crate) fn error(category: ErrorCategory, message: &str) -> RuntimeEvent {
    RuntimeEvent::Error {
        category,
        message: bounded(message),
        will_retry: false,
    }
}

/// A question's offered choices (`options`, each a `label` and maybe a
/// `description`).
pub(crate) fn choices(part: &Value) -> Vec<Choice> {
    part.get("options")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|choice| Choice {
            label: bounded(&text(choice, "label")),
            description: choice
                .get("description")
                .and_then(Value::as_str)
                .map(bounded),
        })
        .collect()
}

/// An ask, and the turn waiting on it.
pub(crate) fn waiting(ask: RuntimeEvent) -> Vec<RuntimeEvent> {
    vec![
        ask,
        RuntimeEvent::State {
            status: LiveStatusWord::Waiting,
        },
    ]
}

/// Whether two paths name one folder (each as it resolves, else as given).
pub(crate) fn same_folder(a: &Path, b: &Path) -> bool {
    let canonical = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    canonical(a) == canonical(b)
}
