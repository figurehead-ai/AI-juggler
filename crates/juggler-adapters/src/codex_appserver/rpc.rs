//! app-server's wire: JSON-RPC 2.0 messages without the `jsonrpc` field,
//! one per line (research §3.2). Requests carry an `id` and a `method`;
//! responses an `id` and a `result` or an `error`; notifications a
//! `method` only.

use serde_json::{Value, json};

/// One message from the server.
#[derive(Clone, Debug, PartialEq)]
pub enum Incoming {
    /// The answer to one of AI Juggler's requests.
    Response {
        /// The request's id.
        id: i64,
        /// Its result, or the server's error message.
        result: Result<Value, String>,
    },
    /// The server asks AI Juggler something (an approval, a question).
    Request {
        /// Its id, exactly as sent (a number or a string): the answer
        /// names it.
        id: Value,
        /// The method.
        method: String,
        /// Its parameters.
        params: Value,
    },
    /// Something happened.
    Notification {
        /// The method.
        method: String,
        /// Its parameters.
        params: Value,
    },
}

/// What a line from the server is; `None` for one that isn't a message.
#[must_use]
pub fn classify(line: &str) -> Option<Incoming> {
    let mut value: Value = serde_json::from_str(line).ok()?;
    let object = value.as_object_mut()?;
    let method = object
        .get("method")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let params = object.remove("params").unwrap_or(Value::Null);
    match (object.remove("id"), method) {
        (Some(id), Some(method)) => Some(Incoming::Request { id, method, params }),
        (None, Some(method)) => Some(Incoming::Notification { method, params }),
        (Some(id), None) => {
            let id = id.as_i64()?;
            let result = match object.remove("error") {
                Some(error) => Err(error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("the server refused")
                    .to_owned()),
                None => Ok(object.remove("result").unwrap_or(Value::Null)),
            };
            Some(Incoming::Response { id, result })
        }
        (None, None) => None,
    }
}

/// A request line (a method without parameters sends none).
#[must_use]
pub fn request(id: i64, method: &str, params: Value) -> Value {
    let mut line = json!({"id": id, "method": method});
    if !params.is_null() {
        line["params"] = params;
    }
    line
}

/// A notification line.
#[must_use]
pub fn notification(method: &str) -> Value {
    json!({"method": method})
}

/// The answer to a server request.
#[must_use]
pub fn answer(id: &Value, result: Value) -> Value {
    let mut line = json!({ "id": id });
    line["result"] = result;
    line
}

/// A refusal of a server request AI Juggler doesn't handle (never an
/// answer to a permission prompt: those always wait for the user).
#[must_use]
pub fn refusal(id: &Value, message: &str) -> Value {
    json!({"id": id, "error": {"code": -32601, "message": message}})
}

#[cfg(test)]
mod tests {
    use super::{Incoming, classify};
    use serde_json::json;

    #[test]
    fn messages_are_told_apart() {
        assert_eq!(
            classify(r#"{"id":3,"result":{"ok":true}}"#),
            Some(Incoming::Response {
                id: 3,
                result: Ok(json!({"ok": true}))
            })
        );
        assert_eq!(
            classify(r#"{"id":3,"error":{"code":-1,"message":"no"}}"#),
            Some(Incoming::Response {
                id: 3,
                result: Err("no".into())
            })
        );
        assert!(matches!(
            classify(r#"{"id":"r-1","method":"item/tool/requestUserInput","params":{}}"#),
            Some(Incoming::Request { id, .. }) if id == json!("r-1")
        ));
        assert!(matches!(
            classify(r#"{"method":"turn/started","params":{"threadId":"t"}}"#),
            Some(Incoming::Notification { method, .. }) if method == "turn/started"
        ));
        assert_eq!(classify("not json"), None);
        assert_eq!(classify(r#"{"x":1}"#), None);
    }
}
