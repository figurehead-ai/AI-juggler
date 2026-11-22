//! A job's output shape ([`crate::features::ai::engine::Shape`], the
//! helpers' small form) as a JSON schema, which the local helper turns
//! into a grammar with llama.cpp's own converter. The same forms the
//! Apple helper reads, with the same defaults, so one shape constrains
//! both engines alike:
//!
//! | Small form | JSON schema |
//! |---|---|
//! | `"string"` or `{"type":"string"}` (or any unknown type) | `{"type":"string"}` |
//! | `"bool"` | `{"type":"boolean"}` |
//! | `{"type":"integer","min","max"}` (0 and 1000 when absent) | `integer` with `minimum`, `maximum` |
//! | `{"type":"enum","values":[…]}` | `string` with `enum` |
//! | `{"type":"array","items","min"?,"max"?}` | `array` with `minItems`, `maxItems` |
//! | `{"type":"object","properties":[{"name","schema"}]}` | `object`, every property required, no others |
//!
//! The schema is written by hand, not through a JSON map, so the
//! properties keep the shape's order (the order the model writes them in)
//! whatever map order the build's `serde_json` has.

use std::fmt::Write as _;

use serde_json::Value;

/// How deep a shape may nest.
const MAX_DEPTH: usize = 8;

/// The integer bounds the Apple helper assumes when a shape gives none.
const DEFAULT_MIN: i64 = 0;
const DEFAULT_MAX: i64 = 1_000;

/// `shape` as a JSON schema's text; `None` when it nests too deep or a
/// property has no name.
#[must_use]
pub fn json_schema(shape: &Value) -> Option<String> {
    let mut out = String::new();
    write(shape, 0, &mut out)?;
    Some(out)
}

fn quoted(text: &str) -> Option<String> {
    serde_json::to_string(text).ok()
}

fn write(shape: &Value, depth: usize, out: &mut String) -> Option<()> {
    if depth > MAX_DEPTH {
        return None;
    }
    let kind = match shape {
        Value::String(kind) => kind.as_str(),
        Value::Object(map) => map.get("type").and_then(Value::as_str).unwrap_or("string"),
        _ => "string",
    };
    match kind {
        "bool" => out.push_str(r#"{"type":"boolean"}"#),
        "integer" => {
            let min = shape
                .get("min")
                .and_then(Value::as_i64)
                .unwrap_or(DEFAULT_MIN);
            let max = shape
                .get("max")
                .and_then(Value::as_i64)
                .unwrap_or(DEFAULT_MAX)
                .max(min);
            let _ = write!(
                out,
                r#"{{"type":"integer","minimum":{min},"maximum":{max}}}"#
            );
        }
        "enum" => {
            let values: Vec<String> = shape
                .get("values")
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .filter_map(quoted)
                        .collect()
                })
                .unwrap_or_default();
            let _ = write!(out, r#"{{"type":"string","enum":[{}]}}"#, values.join(","));
        }
        "array" => {
            out.push_str(r#"{"type":"array","items":"#);
            let items = shape
                .get("items")
                .cloned()
                .unwrap_or_else(|| Value::from("string"));
            write(&items, depth + 1, out)?;
            if let Some(min) = shape.get("min").and_then(Value::as_u64) {
                let _ = write!(out, r#","minItems":{min}"#);
            }
            if let Some(max) = shape.get("max").and_then(Value::as_u64) {
                let _ = write!(out, r#","maxItems":{max}"#);
            }
            out.push('}');
        }
        "object" => {
            let properties = shape
                .get("properties")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let mut names = Vec::new();
            out.push_str(r#"{"type":"object","properties":{"#);
            for (i, property) in properties.iter().enumerate() {
                let name = quoted(property.get("name").and_then(Value::as_str)?)?;
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&name);
                out.push(':');
                let schema = property
                    .get("schema")
                    .cloned()
                    .unwrap_or_else(|| Value::from("string"));
                write(&schema, depth + 1, out)?;
                names.push(name);
            }
            let _ = write!(
                out,
                r#"}},"required":[{}],"additionalProperties":false}}"#,
                names.join(",")
            );
        }
        _ => out.push_str(r#"{"type":"string"}"#),
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn each_small_form_becomes_its_schema_in_order() {
        let shape = json!({
            "type": "object",
            "name": "Review",
            "properties": [
                { "name": "verdict", "schema": { "type": "enum", "name": "V", "values": ["ok", "say \"no\""] } },
                { "name": "done", "schema": "bool" },
                { "name": "count", "schema": { "type": "integer", "min": 1, "max": 5 } },
                { "name": "notes", "description": "why", "schema": { "type": "array", "items": "string", "max": 3 } },
                { "name": "about" }
            ]
        });
        let schema = json_schema(&shape).unwrap();
        assert_eq!(
            schema,
            concat!(
                r#"{"type":"object","properties":{"verdict":{"type":"string","enum":["ok","say \"no\""]},"#,
                r#""done":{"type":"boolean"},"count":{"type":"integer","minimum":1,"maximum":5},"#,
                r#""notes":{"type":"array","items":{"type":"string"},"maxItems":3},"#,
                r#""about":{"type":"string"}},"required":["verdict","done","count","notes","about"],"#,
                r#""additionalProperties":false}"#
            )
        );
        let parsed: Value = serde_json::from_str(&schema).unwrap();
        assert_eq!(parsed["required"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn defaults_follow_the_apple_helper() {
        assert_eq!(
            json_schema(&json!({"type": "integer"})).unwrap(),
            r#"{"type":"integer","minimum":0,"maximum":1000}"#
        );
        assert_eq!(
            json_schema(&json!({"type": "integer", "min": 9, "max": 2})).unwrap(),
            r#"{"type":"integer","minimum":9,"maximum":9}"#
        );
        assert_eq!(
            json_schema(&json!({"type": "something"})).unwrap(),
            r#"{"type":"string"}"#
        );
        assert_eq!(json_schema(&json!(7)).unwrap(), r#"{"type":"string"}"#);
    }

    #[test]
    fn too_deep_or_nameless_is_refused() {
        let mut shape = json!("string");
        for _ in 0..=MAX_DEPTH {
            shape = json!({"type": "array", "items": shape});
        }
        assert_eq!(json_schema(&shape), None);
        assert_eq!(
            json_schema(&json!({"type": "object", "properties": [{"schema": "bool"}]})),
            None
        );
    }
}
