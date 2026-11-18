//! Canonical JSON: the exact bytes a signature covers.
//!
//! The publisher signs the canonical bytes of the payload; the app checks
//! the signature over the bytes it received, and only then parses them,
//! and parses them only if they are canonical (re-encoding the parsed value
//! gives the same bytes back). So two different byte strings can never
//! carry the same signed meaning, and no parser leniency (duplicate keys,
//! odd number spellings, needless escapes) can be used to smuggle a second
//! reading past the signature.
//!
//! # The rules
//!
//! 1. **Encoding:** UTF-8, no byte-order mark.
//! 2. **Whitespace:** none outside strings.
//! 3. **Objects:** members sorted by key, comparing the keys' UTF-8 bytes
//!    (which is Unicode code-point order; it differs from RFC 8785's UTF-16
//!    order only for keys mixing astral and U+E000–U+FFFF characters).
//!    A duplicate key is an error: the parser keeps one of them, so the
//!    re-encoding is shorter than the input and the bytes don't match.
//! 4. **Numbers:** integers only, within ±(2⁵³ − 1) (so every value means
//!    the same in JavaScript), written in shortest decimal: no `+`, no
//!    leading zeros, no fraction, no exponent, no `-0`. A fraction or an
//!    exponent is an error, even `1.0` or `1e3`.
//! 5. **Strings:** `"` and `\` are escaped as `\"` and `\\`; U+0008,
//!    U+0009, U+000A, U+000C and U+000D as `\b`, `\t`, `\n`, `\f`, `\r`;
//!    the other controls U+0000–U+001F as `\u00xx` with lower-case hex.
//!    Everything else is written as itself in UTF-8, including `/`,
//!    U+007F, U+2028, U+2029 and every non-ASCII character. So an input
//!    `é` or a surrogate-pair escape is *not* canonical: the
//!    canonical form is the character itself. A lone surrogate escape is
//!    an error.
//! 6. **Literals:** `true`, `false`, `null`.
//! 7. **Depth:** at most [`MAX_DEPTH`] nested arrays and objects.

use serde_json::Value;

/// The largest integer a canonical number may hold (2⁵³ − 1).
pub const MAX_SAFE_INTEGER: i64 = (1 << 53) - 1;

/// The deepest nesting of arrays and objects allowed.
pub const MAX_DEPTH: usize = 32;

/// Why a value or a byte string isn't canonical.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CanonicalError {
    /// The bytes are not JSON (or not UTF-8).
    #[error("not JSON: {0}")]
    NotJson(String),
    /// A number with a fraction or an exponent.
    #[error("a number is not an integer")]
    NotInteger,
    /// An integer beyond ±(2⁵³ − 1).
    #[error("an integer is out of the safe range")]
    OutOfRange,
    /// Arrays and objects nested deeper than [`MAX_DEPTH`].
    #[error("nested too deeply")]
    TooDeep,
    /// Valid JSON whose bytes differ from its canonical form.
    #[error("the bytes are not in canonical form")]
    NotCanonical,
}

/// The canonical bytes of `value`.
///
/// # Errors
///
/// [`CanonicalError::NotInteger`], [`CanonicalError::OutOfRange`] or
/// [`CanonicalError::TooDeep`] when `value` has no canonical form.
pub fn to_canonical_bytes(value: &Value) -> Result<Vec<u8>, CanonicalError> {
    let mut out = Vec::new();
    write_value(value, 0, &mut out)?;
    Ok(out)
}

/// Parses `bytes`, and accepts them only if they are canonical.
///
/// # Errors
///
/// [`CanonicalError::NotJson`] for bytes that aren't JSON, the
/// [`to_canonical_bytes`] errors, and [`CanonicalError::NotCanonical`]
/// when the bytes are JSON but not in canonical form.
pub fn parse_canonical(bytes: &[u8]) -> Result<Value, CanonicalError> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|e| CanonicalError::NotJson(e.to_string()))?;
    if to_canonical_bytes(&value)? == bytes {
        Ok(value)
    } else {
        Err(CanonicalError::NotCanonical)
    }
}

fn write_value(value: &Value, depth: usize, out: &mut Vec<u8>) -> Result<(), CanonicalError> {
    match value {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(true) => out.extend_from_slice(b"true"),
        Value::Bool(false) => out.extend_from_slice(b"false"),
        Value::Number(number) => write_number(number, out)?,
        Value::String(text) => write_string(text, out),
        Value::Array(items) => {
            let depth = deeper(depth)?;
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_value(item, depth, out)?;
            }
            out.push(b']');
        }
        Value::Object(map) => {
            let depth = deeper(depth)?;
            // Sorted here, whatever order the map keeps (serde_json's
            // `preserve_order` may be on elsewhere in the workspace).
            let mut members: Vec<(&String, &Value)> = map.iter().collect();
            members.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
            out.push(b'{');
            for (i, (key, item)) in members.into_iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_string(key, out);
                out.push(b':');
                write_value(item, depth, out)?;
            }
            out.push(b'}');
        }
    }
    Ok(())
}

fn deeper(depth: usize) -> Result<usize, CanonicalError> {
    if depth >= MAX_DEPTH {
        Err(CanonicalError::TooDeep)
    } else {
        Ok(depth + 1)
    }
}

fn write_number(number: &serde_json::Number, out: &mut Vec<u8>) -> Result<(), CanonicalError> {
    let Some(value) = number.as_i64() else {
        return Err(if number.as_u64().is_some() {
            CanonicalError::OutOfRange
        } else {
            CanonicalError::NotInteger
        });
    };
    if !(-MAX_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&value) {
        return Err(CanonicalError::OutOfRange);
    }
    out.extend_from_slice(value.to_string().as_bytes());
    Ok(())
}

fn write_string(text: &str, out: &mut Vec<u8>) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    out.push(b'"');
    for ch in text.chars() {
        match ch {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\u{8}' => out.extend_from_slice(b"\\b"),
            '\t' => out.extend_from_slice(b"\\t"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\u{c}' => out.extend_from_slice(b"\\f"),
            '\r' => out.extend_from_slice(b"\\r"),
            c if u32::from(c) < 0x20 => {
                let code = u32::from(c);
                out.extend_from_slice(b"\\u00");
                out.push(HEX[(code >> 4) as usize]);
                out.push(HEX[(code & 0xf) as usize]);
            }
            c => {
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
    out.push(b'"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn canon(value: &Value) -> String {
        String::from_utf8(to_canonical_bytes(value).unwrap()).unwrap()
    }

    #[test]
    fn keys_sort_by_code_point_and_whitespace_goes() {
        let value: Value = serde_json::from_str(
            r#"{ "b": 1, "a": [ true, null ], "B": {"z": "", "é": 0, "y": -3} }"#,
        )
        .unwrap();
        assert_eq!(
            canon(&value),
            r#"{"B":{"y":-3,"z":"","é":0},"a":[true,null],"b":1}"#
        );
    }

    #[test]
    fn strings_escape_only_what_they_must() {
        let value = json!("q\"b\\/\u{8}\t\n\u{c}\r\u{1}\u{1f}\u{7f}é\u{2028}😀");
        assert_eq!(
            canon(&value),
            "\"q\\\"b\\\\/\\b\\t\\n\\f\\r\\u0001\\u001f\u{7f}é\u{2028}😀\""
        );
    }

    #[test]
    fn unicode_escapes_are_not_canonical_their_characters_are() {
        // The same text, escaped and literal: only the literal is canonical.
        assert_eq!(
            parse_canonical(br#""\u00e9""#),
            Err(CanonicalError::NotCanonical)
        );
        assert_eq!(parse_canonical(r#""é""#.as_bytes()), Ok(json!("é")));
        assert_eq!(
            parse_canonical(br#""\ud83d\ude00""#),
            Err(CanonicalError::NotCanonical)
        );
        assert_eq!(parse_canonical("\"😀\"".as_bytes()), Ok(json!("😀")));
        assert_eq!(
            parse_canonical(br#""\/""#),
            Err(CanonicalError::NotCanonical)
        );
        assert_eq!(parse_canonical(br#""\u0001""#), Ok(json!("\u{1}")));
        assert_eq!(
            parse_canonical(br#""\u0041""#),
            Err(CanonicalError::NotCanonical)
        );
        assert!(matches!(
            parse_canonical(br#""\ud800""#),
            Err(CanonicalError::NotJson(_))
        ));
    }

    #[test]
    fn numbers_are_safe_integers_in_shortest_form() {
        assert_eq!(canon(&json!(0)), "0");
        assert_eq!(canon(&json!(-42)), "-42");
        assert_eq!(canon(&json!(MAX_SAFE_INTEGER)), "9007199254740991");
        assert_eq!(
            to_canonical_bytes(&json!(MAX_SAFE_INTEGER + 1)),
            Err(CanonicalError::OutOfRange)
        );
        assert_eq!(
            to_canonical_bytes(&json!(-MAX_SAFE_INTEGER - 1)),
            Err(CanonicalError::OutOfRange)
        );
        assert_eq!(
            to_canonical_bytes(&json!(u64::MAX)),
            Err(CanonicalError::OutOfRange)
        );
        assert_eq!(
            to_canonical_bytes(&json!(1.5)),
            Err(CanonicalError::NotInteger)
        );
        for spelled in ["1.0", "1e3", "1E3", "-0", "01", "+1", "0.0"] {
            assert!(
                parse_canonical(spelled.as_bytes()).is_err(),
                "{spelled} accepted"
            );
        }
        assert_eq!(parse_canonical(b"7"), Ok(json!(7)));
    }

    #[test]
    fn duplicate_keys_and_whitespace_are_rejected() {
        assert_eq!(
            parse_canonical(br#"{"a":1,"a":2}"#),
            Err(CanonicalError::NotCanonical)
        );
        assert_eq!(
            parse_canonical(br#"{"a": 1}"#),
            Err(CanonicalError::NotCanonical)
        );
        assert_eq!(
            parse_canonical(br#"{"b":1,"a":2}"#),
            Err(CanonicalError::NotCanonical)
        );
        assert_eq!(
            parse_canonical(b"{\"a\":1}\n"),
            Err(CanonicalError::NotCanonical)
        );
        assert_eq!(
            parse_canonical(br#"{"a":1,"b":2}"#),
            Ok(json!({"a": 1, "b": 2}))
        );
    }

    #[test]
    fn depth_is_bounded() {
        let mut deep = json!(1);
        for _ in 0..MAX_DEPTH {
            deep = json!([deep]);
        }
        assert!(to_canonical_bytes(&deep).is_ok());
        assert_eq!(
            to_canonical_bytes(&json!([deep])),
            Err(CanonicalError::TooDeep)
        );
    }

    #[test]
    fn canonical_bytes_are_stable() {
        // A golden: if this changes, every signed file in the wild breaks.
        let value = json!({
            "serial": 418, "channel": "stable", "sections": {"x": {"schema": 1, "values": {"k": false}}},
            "note": "Größe ✓", "list": [3, -1, "a\nb"]
        });
        let expected = "{\"channel\":\"stable\",\"list\":[3,-1,\"a\\nb\"],\"note\":\"Größe ✓\",\
                        \"sections\":{\"x\":{\"schema\":1,\"values\":{\"k\":false}}},\"serial\":418}";
        assert_eq!(canon(&value), expected);
        assert_eq!(parse_canonical(expected.as_bytes()), Ok(value));
    }

    proptest::proptest! {
        #[test]
        fn canonical_form_is_a_fixed_point(value in arb_json()) {
            let bytes = to_canonical_bytes(&value).unwrap();
            proptest::prop_assert_eq!(parse_canonical(&bytes).unwrap(), value.clone());
            let again = to_canonical_bytes(&parse_canonical(&bytes).unwrap()).unwrap();
            proptest::prop_assert_eq!(again, bytes);
        }
    }

    fn arb_json() -> impl proptest::strategy::Strategy<Value = Value> {
        use proptest::prelude::*;
        let leaf = prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            (-MAX_SAFE_INTEGER..=MAX_SAFE_INTEGER).prop_map(|n| json!(n)),
            any::<String>().prop_map(Value::String),
        ];
        leaf.prop_recursive(4, 32, 6, |inner| {
            prop_oneof![
                proptest::collection::vec(inner.clone(), 0..6).prop_map(Value::Array),
                proptest::collection::btree_map(any::<String>(), inner, 0..6)
                    .prop_map(|m| Value::Object(m.into_iter().collect())),
            ]
        })
    }
}
