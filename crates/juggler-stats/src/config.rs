//! The service's config file (spec §5.4): fetched at most once a day when
//! a report is due, reused for up to 7 days, unknown fields ignored. It
//! can switch collection off, narrow versions and schemas, set the
//! sampling rate and ask for a puzzle — but never change where reports go.

/// The config, with the service's defaults for missing fields.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Config {
    /// Collection is on.
    pub enabled: bool,
    /// The fraction of weeks to send, in (0, 1].
    pub sample_rate: f64,
    /// Accepted schema versions.
    pub schemas: Vec<u32>,
    /// The oldest app version accepted.
    pub min_app_version: Option<String>,
    /// The newest app version accepted.
    pub max_app_version: Option<String>,
    /// The size limit, uncompressed.
    pub max_report_bytes: usize,
    /// Puzzle difficulty (0: none).
    pub pow_bits: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            sample_rate: 1.0,
            schemas: vec![1],
            min_app_version: None,
            max_app_version: None,
            max_report_bytes: crate::report::MAX_REPORT_BYTES,
            pow_bits: 0,
        }
    }
}

/// Refresh after a day.
pub const FRESH_MS: i64 = 24 * 3_600_000;
/// Reuse a stale copy for up to 7 days when a refresh fails.
pub const USABLE_MS: i64 = 7 * 24 * 3_600_000;

impl Config {
    /// Parses the file (lenient: unknown fields ignored; a nonsensical
    /// rate makes the config unusable).
    #[must_use]
    pub fn parse(body: &[u8]) -> Option<Self> {
        let config: Self = serde_json::from_slice(body).ok()?;
        (config.sample_rate > 0.0 && config.sample_rate <= 1.0).then_some(config)
    }

    /// `version` is within `min_app_version..=max_app_version`.
    #[must_use]
    pub fn accepts_version(&self, version: &str) -> bool {
        let v = triple(version);
        self.min_app_version
            .as_deref()
            .is_none_or(|min| v >= triple(min))
            && self
                .max_app_version
                .as_deref()
                .is_none_or(|max| v <= triple(max))
    }
}

/// `major.minor.patch` as numbers (pre-release and build tags ignored).
fn triple(version: &str) -> (u64, u64, u64) {
    let core = version.split(['-', '+']).next().unwrap_or_default();
    let mut parts = core.split('.').map(|p| p.parse::<u64>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_leniently_and_checks_versions() {
        let config = Config::parse(
            br#"{"enabled":true,"sample_rate":0.5,"schemas":[1],"min_app_version":"0.1.0","max_app_version":null,"max_report_bytes":8192,"pow_bits":0,"known_versions":[],"future":1}"#,
        )
        .unwrap();
        assert!(config.enabled);
        assert!((config.sample_rate - 0.5).abs() < f64::EPSILON);
        assert!(config.accepts_version("0.1.0"));
        assert!(config.accepts_version("0.3.1-beta.2"));
        assert!(!config.accepts_version("0.0.9"));
        let capped = Config {
            max_app_version: Some("0.2.0".into()),
            ..config
        };
        assert!(!capped.accepts_version("0.2.1"));
        assert!(Config::parse(br#"{"sample_rate":0}"#).is_none());
        assert!(Config::parse(b"not json").is_none());
        assert!(
            !Config::parse(b"{}").unwrap().enabled,
            "missing enabled means off"
        );
    }
}
