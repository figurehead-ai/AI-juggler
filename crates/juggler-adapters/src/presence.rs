//! Reading a file that may hold a secret for its SHAPE only (ADR-0003):
//! which fields are filled, never what they hold. Each value is consumed
//! by the parser and dropped on the spot; nothing of it is stored, logged
//! or returned.

/// Whether a field holds something — never what. A string counts when
/// non-empty, an object when it has entries; the value is consumed by
/// the parser and dropped on the spot (ADR-0003).
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Present(pub(crate) bool);

impl<'de> serde::Deserialize<'de> for Present {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Shape;
        impl<'de> serde::de::Visitor<'de> for Shape {
            type Value = Present;
            fn expecting(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                out.write_str("any JSON value")
            }
            fn visit_str<E>(self, value: &str) -> Result<Present, E> {
                Ok(Present(!value.is_empty()))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Present, A::Error> {
                let mut any = false;
                while map
                    .next_entry::<serde::de::IgnoredAny, serde::de::IgnoredAny>()?
                    .is_some()
                {
                    any = true;
                }
                Ok(Present(any))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Present, A::Error> {
                while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
                Ok(Present(false))
            }
            fn visit_unit<E>(self) -> Result<Present, E> {
                Ok(Present(false))
            }
            fn visit_bool<E>(self, _: bool) -> Result<Present, E> {
                Ok(Present(false))
            }
            fn visit_i64<E>(self, _: i64) -> Result<Present, E> {
                Ok(Present(false))
            }
            fn visit_u64<E>(self, _: u64) -> Result<Present, E> {
                Ok(Present(false))
            }
            fn visit_f64<E>(self, _: f64) -> Result<Present, E> {
                Ok(Present(false))
            }
        }
        deserializer.deserialize_any(Shape)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::Present;

    /// A secret's field reads as filled; the parsed shape holds no part of
    /// the value.
    #[test]
    fn a_filled_field_is_seen_and_its_value_kept_nowhere() {
        #[derive(Debug, Default, serde::Deserialize)]
        #[serde(default)]
        struct Shape {
            token: Present,
            nested: Present,
            empty: Present,
        }
        let shape: Shape = serde_json::from_str(
            r#"{"token":"sk-very-secret","nested":{"refresh":"rt-secret"},"empty":"","other":"x"}"#,
        )
        .unwrap();
        assert!(shape.token.0 && shape.nested.0 && !shape.empty.0);
        let shown = format!("{shape:?}");
        assert!(!shown.contains("secret"), "{shown}");
    }
}
