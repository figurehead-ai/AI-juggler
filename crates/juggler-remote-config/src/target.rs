//! Targeting, evaluated on this computer (§5).
//!
//! A rule in the file says *who* a value is for; the app checks the rule
//! against its own [`Facts`], so the anonymous fetch sends nothing about
//! the user. The rollout bucket comes from a random per-install id that is
//! made here, never sent anywhere, and can be reset.

use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest as _, Sha256};

use crate::envelope::Channel;
use crate::version::{Version, VersionRange};

/// How many rollout buckets there are: a 10 % rollout is buckets 0–9.
pub const BUCKETS: u8 = 100;

/// The domain-separation prefix of the bucket hash.
const BUCKET_CONTEXT: &[u8] = b"AIJuggler rollout bucket v1\0";

/// An operating system family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Os {
    /// Apple's desktop OS.
    Macos,
    /// Windows.
    Windows,
    /// Linux.
    Linux,
}

impl Os {
    /// The name rules use.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Macos => "macos",
            Self::Windows => "windows",
            Self::Linux => "linux",
        }
    }

    /// The OS this build runs on.
    #[must_use]
    pub const fn this() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else {
            Self::Macos
        }
    }
}

/// A random per-install id: only ever hashed into a bucket, never sent.
///
/// Its `Debug` hides the bytes, so it can't reach a log by accident.
#[derive(Clone, PartialEq, Eq)]
pub struct InstallId([u8; 16]);

impl fmt::Debug for InstallId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("InstallId(..)")
    }
}

impl InstallId {
    /// A fresh random id (first run, or "reset" in Settings).
    ///
    /// # Errors
    ///
    /// The OS random source failed.
    pub fn generate() -> Result<Self, getrandom::Error> {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes)?;
        Ok(Self(bytes))
    }

    /// An id from its saved bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// The bytes, for saving locally.
    #[must_use]
    pub const fn to_bytes(&self) -> [u8; 16] {
        self.0
    }

    /// This install's bucket, `0..BUCKETS`, for the rollout named `salt`.
    ///
    /// Each rollout salts the hash with its own name, so the same installs
    /// aren't first in line for every rollout; one rollout's bucket is
    /// stable across launches and versions.
    #[must_use]
    pub fn bucket(&self, salt: &str) -> u8 {
        let digest = Sha256::new()
            .chain_update(BUCKET_CONTEXT)
            .chain_update(self.0)
            .chain_update(salt.as_bytes())
            .finalize();
        let mut head = [0u8; 8];
        head.copy_from_slice(&digest[..8]);
        // 2⁶⁴ mod 100 = 16: the bias toward buckets 0–15 is 2⁻⁶⁰.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the remainder is below 100"
        )]
        let bucket = (u64::from_be_bytes(head) % u64::from(BUCKETS)) as u8;
        bucket
    }
}

/// What this app knows about itself, for matching rules.
#[derive(Debug, Clone)]
pub struct Facts {
    /// The app's version.
    pub app_version: Version,
    /// The app's channel.
    pub channel: Channel,
    /// The OS family.
    pub os: Os,
    /// The OS version, when known.
    pub os_version: Option<Version>,
    /// Installed tools' versions, by tool id (from the adapters).
    pub tools: BTreeMap<String, Version>,
    /// The per-install id the rollout bucket comes from.
    pub install: InstallId,
}

/// The keys a rule's `when` may use. A rule using any other key never
/// matches: a newer file's narrower targeting must not widen to everyone
/// on an older app that can't read it.
const KNOWN_KEYS: [&str; 6] = ["app", "channels", "os", "os_version", "tools", "buckets"];

/// A rule's `when`: every present clause must match.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    /// App versions.
    #[serde(default)]
    pub app: Option<VersionRange>,
    /// Channel names.
    #[serde(default)]
    pub channels: Option<Vec<String>>,
    /// OS names ([`Os::as_str`]).
    #[serde(default)]
    pub os: Option<Vec<String>>,
    /// OS versions.
    #[serde(default)]
    pub os_version: Option<VersionRange>,
    /// Installed tool versions; a tool that isn't installed doesn't match.
    #[serde(default)]
    pub tools: Option<BTreeMap<String, VersionRange>>,
    /// Inclusive rollout buckets `[first, last]`, within `0..BUCKETS`.
    #[serde(default)]
    pub buckets: Option<[u8; 2]>,
}

/// A `when` that can't be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("bad targeting: {0}")]
pub struct TargetError(pub String);

/// A parsed `when`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum When {
    /// No `when`: everyone.
    Always,
    /// A target this app can evaluate.
    Target(Box<Target>),
    /// A target naming a key this app doesn't know: never matches.
    Unknown,
}

impl When {
    /// Parses a rule's `when` object.
    ///
    /// # Errors
    ///
    /// [`TargetError`] for a known key with a bad value, or a bucket range
    /// outside `0..BUCKETS` or reversed.
    pub fn parse(value: &Value) -> Result<Self, TargetError> {
        let object = value
            .as_object()
            .ok_or_else(|| TargetError("`when` is not an object".to_owned()))?;
        if object.keys().any(|key| !KNOWN_KEYS.contains(&key.as_str())) {
            return Ok(Self::Unknown);
        }
        let target: Target =
            serde_json::from_value(value.clone()).map_err(|e| TargetError(e.to_string()))?;
        if let Some([first, last]) = target.buckets
            && (first > last || last >= BUCKETS)
        {
            return Err(TargetError("bucket range".to_owned()));
        }
        Ok(Self::Target(Box::new(target)))
    }

    /// Whether the rule applies to `facts`; `salt` names the rollout.
    #[must_use]
    pub fn matches(&self, facts: &Facts, salt: &str) -> bool {
        let target = match self {
            Self::Always => return true,
            Self::Unknown => return false,
            Self::Target(target) => target,
        };
        let app = target
            .app
            .as_ref()
            .is_none_or(|range| range.contains(&facts.app_version));
        let channel = target
            .channels
            .as_ref()
            .is_none_or(|names| names.iter().any(|name| name == facts.channel.as_str()));
        let os = target
            .os
            .as_ref()
            .is_none_or(|names| names.iter().any(|name| name == facts.os.as_str()));
        let os_version = target.os_version.as_ref().is_none_or(|range| {
            facts
                .os_version
                .as_ref()
                .is_some_and(|version| range.contains(version))
        });
        let tools = target.tools.as_ref().is_none_or(|wanted| {
            wanted.iter().all(|(tool, range)| {
                facts
                    .tools
                    .get(tool)
                    .is_some_and(|version| range.contains(version))
            })
        });
        let bucket = target
            .buckets
            .is_none_or(|[first, last]| (first..=last).contains(&facts.install.bucket(salt)));
        app && channel && os && os_version && tools && bucket
    }

    /// Whether the rule is limited to some rollout buckets.
    #[must_use]
    pub fn is_bucketed(&self) -> bool {
        matches!(self, Self::Target(target) if target.buckets.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn facts(install: [u8; 16]) -> Facts {
        Facts {
            app_version: "0.5.2".parse().unwrap(),
            channel: Channel::Beta,
            os: Os::Macos,
            os_version: Some("26.1.0".parse().unwrap()),
            tools: BTreeMap::from([("tool-a".to_owned(), "2.3.0".parse().unwrap())]),
            install: InstallId::from_bytes(install),
        }
    }

    #[test]
    fn bucket_is_stable_and_salted() {
        let id = InstallId::from_bytes([42; 16]);
        assert_eq!(id.bucket("flags.x"), id.bucket("flags.x"));
        assert_eq!(
            id.bucket("flags.x"),
            InstallId::from_bytes([42; 16]).bucket("flags.x")
        );
        // A golden, so a change to the hash (which would reshuffle every
        // rollout) is a deliberate one.
        assert_eq!(id.bucket("flags.x"), GOLDEN_BUCKET);
        let salts: std::collections::BTreeSet<u8> = (0..50)
            .map(|i| id.bucket(&format!("rollout-{i}")))
            .collect();
        assert!(salts.len() > 20, "salts should spread one install around");
    }

    const GOLDEN_BUCKET: u8 = 37;

    #[test]
    fn buckets_spread_evenly() {
        let mut counts = [0u32; BUCKETS as usize];
        let samples = 100_000u32;
        for i in 0..samples {
            let mut bytes = [0u8; 16];
            bytes[..4].copy_from_slice(&i.to_be_bytes());
            counts[usize::from(InstallId::from_bytes(bytes).bucket("flags.x"))] += 1;
        }
        // Expected 1,000 per bucket; σ ≈ 31.5. A chi-square over 99 degrees
        // of freedom stays well under 160 for a uniform hash.
        let expected = f64::from(samples) / f64::from(BUCKETS);
        let chi: f64 = counts
            .iter()
            .map(|&c| (f64::from(c) - expected).powi(2) / expected)
            .sum();
        assert!(chi < 160.0, "chi-square {chi}");
        // A 10 % rollout holds about 10 % of installs.
        let ten: u32 = counts[..10].iter().sum();
        assert!((9_400..=10_600).contains(&ten), "{ten}");
    }

    #[test]
    fn generated_ids_differ() {
        assert_ne!(
            InstallId::generate().unwrap(),
            InstallId::generate().unwrap()
        );
        assert_eq!(
            format!("{:?}", InstallId::from_bytes([1; 16])),
            "InstallId(..)"
        );
    }

    #[test]
    fn every_clause_must_match() {
        let f = facts([0; 16]);
        let when = |v: Value| When::parse(&v).unwrap();
        assert!(when(json!({})).matches(&f, "s"));
        assert!(when(json!({"app": {"min": "0.5.0", "max": "0.6.0"}})).matches(&f, "s"));
        assert!(!when(json!({"app": {"min": "0.5.3"}})).matches(&f, "s"));
        assert!(when(json!({"channels": ["beta", "nightly"]})).matches(&f, "s"));
        assert!(!when(json!({"channels": ["stable"]})).matches(&f, "s"));
        assert!(when(json!({"os": ["macos"], "os_version": {"min": "26.0.0"}})).matches(&f, "s"));
        assert!(!when(json!({"os": ["windows", "freebsd"]})).matches(&f, "s"));
        assert!(when(json!({"tools": {"tool-a": {"min": "2.0.0"}}})).matches(&f, "s"));
        assert!(!when(json!({"tools": {"tool-b": {}}})).matches(&f, "s"));
        let bucket = f.install.bucket("s");
        assert!(when(json!({"buckets": [bucket, bucket]})).matches(&f, "s"));
        assert!(!when(json!({"channels": ["beta"], "app": {"max": "0.5.0"}})).matches(&f, "s"));
    }

    #[test]
    fn unknown_keys_never_match_and_bad_values_are_errors() {
        let f = facts([0; 16]);
        assert_eq!(When::parse(&json!({"region": "x"})).unwrap(), When::Unknown);
        assert!(!When::Unknown.matches(&f, "s"));
        assert!(When::parse(&json!({"buckets": [5, 4]})).is_err());
        assert!(When::parse(&json!({"buckets": [0, 100]})).is_err());
        assert!(When::parse(&json!({"app": {"min": "x"}})).is_err());
        assert!(When::parse(&json!([1])).is_err());
    }
}
