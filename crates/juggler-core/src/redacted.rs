//! A secret that never prints (AJ-E2010).
//!
//! `Redacted<T>` carries a key or token through code that logs, formats
//! errors or derives `Debug`: both `Debug` and `Display` print
//! `[redacted]`, and the value comes out only through [`Redacted::expose`],
//! so a reader of the code sees where the secret is used. The bytes are
//! wiped when it's dropped.

use zeroize::Zeroize;

/// A secret value that formats as `[redacted]`.
#[derive(Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(transparent)]
pub struct Redacted<T: Zeroize>(T);

impl<T: Zeroize> Redacted<T> {
    /// Wraps `value`.
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    /// The secret itself, for the one place that must use it.
    pub const fn expose(&self) -> &T {
        &self.0
    }
}

impl<T: Zeroize> Drop for Redacted<T> {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl<T: Zeroize> std::fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[redacted]")
    }
}

impl<T: Zeroize> std::fmt::Display for Redacted<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[redacted]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_never_prints() {
        let secret = Redacted::new("sk-live-0123456789abcdef".to_owned());
        assert_eq!(format!("{secret}"), "[redacted]");
        assert_eq!(format!("{secret:?}"), "[redacted]");
        assert_eq!(format!("{:?}", Some(&secret)), "Some([redacted])");
        assert_eq!(secret.expose(), "sk-live-0123456789abcdef");
        let parsed: Redacted<String> = serde_json::from_str("\"tok\"").unwrap();
        assert_eq!(parsed.expose(), "tok");
    }
}
