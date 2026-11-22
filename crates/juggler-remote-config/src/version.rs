//! Versions and version ranges, for targeting (§5) and for range controls
//! (§2): `MAJOR.MINOR.PATCH[-PRE][+BUILD]`, ordered by `SemVer` 2.0.0
//! precedence (a pre-release sorts before its release; build metadata is
//! ignored).

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The longest version string accepted.
pub const MAX_VERSION_LEN: usize = 64;

/// A version string that isn't `MAJOR.MINOR.PATCH[-PRE][+BUILD]`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("not a version: {0:?}")]
pub struct VersionError(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum PreId {
    Num(u64),
    Alpha(String),
}

/// A version, compared by `SemVer` precedence.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
    pre: Vec<PreId>,
}

impl Version {
    /// A release version with no pre-release part.
    #[must_use]
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
            pre: Vec::new(),
        }
    }
}

fn number(part: &str, text: &str) -> Result<u64, VersionError> {
    let bad = || VersionError(text.to_owned());
    if part.is_empty() || (part.len() > 1 && part.starts_with('0')) {
        return Err(bad());
    }
    if !part.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad());
    }
    part.parse().map_err(|_| bad())
}

impl FromStr for Version {
    type Err = VersionError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let bad = || VersionError(text.to_owned());
        if text.len() > MAX_VERSION_LEN {
            return Err(bad());
        }
        let core_and_pre = text.split_once('+').map_or(text, |(head, _)| head);
        let (core, pre) = match core_and_pre.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (core_and_pre, None),
        };
        let mut parts = core.split('.');
        let (Some(major), Some(minor), Some(patch), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(bad());
        };
        let mut ids = Vec::new();
        if let Some(pre) = pre {
            for id in pre.split('.') {
                if id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
                    return Err(bad());
                }
                if id.bytes().all(|b| b.is_ascii_digit()) {
                    ids.push(PreId::Num(number(id, text)?));
                } else {
                    ids.push(PreId::Alpha(id.to_owned()));
                }
            }
        }
        Ok(Self {
            major: number(major, text)?,
            minor: number(minor, text)?,
            patch: number(patch, text)?,
            pre: ids,
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        for (i, id) in self.pre.iter().enumerate() {
            f.write_str(if i == 0 { "-" } else { "." })?;
            match id {
                PreId::Num(n) => write!(f, "{n}")?,
                PreId::Alpha(s) => f.write_str(s)?,
            }
        }
        Ok(())
    }
}

impl Ord for PreId {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Num(a), Self::Num(b)) => a.cmp(b),
            (Self::Num(_), Self::Alpha(_)) => Ordering::Less,
            (Self::Alpha(_), Self::Num(_)) => Ordering::Greater,
            (Self::Alpha(a), Self::Alpha(b)) => a.cmp(b),
        }
    }
}

impl PartialOrd for PreId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => self.pre.cmp(&other.pre),
            })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Serialize for Version {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// A half-open range of versions: `min` included, `max` excluded; either
/// end may be open.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VersionRange {
    /// The lowest version in the range, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<Version>,
    /// The first version past the range, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<Version>,
}

impl VersionRange {
    /// `[min, max)`.
    #[must_use]
    pub const fn new(min: Option<Version>, max: Option<Version>) -> Self {
        Self { min, max }
    }

    /// Whether `version` is in the range.
    #[must_use]
    pub fn contains(&self, version: &Version) -> bool {
        self.min.as_ref().is_none_or(|min| version >= min)
            && self.max.as_ref().is_none_or(|max| version < max)
    }

    /// Whether the range holds no version.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        matches!((&self.min, &self.max), (Some(min), Some(max)) if min >= max)
    }

    /// Whether every version in `self` is also in `outer`.
    #[must_use]
    pub fn within(&self, outer: &Self) -> bool {
        let low = match (&outer.min, &self.min) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(o), Some(s)) => s >= o,
        };
        let high = match (&outer.max, &self.max) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(o), Some(s)) => s <= o,
        };
        low && high
    }

    /// The versions in both ranges.
    #[must_use]
    pub fn intersect(&self, other: &Self) -> Self {
        let min = match (&self.min, &other.min) {
            (Some(a), Some(b)) => Some(a.max(b).clone()),
            (a, b) => a.clone().or_else(|| b.clone()),
        };
        let max = match (&self.max, &other.max) {
            (Some(a), Some(b)) => Some(a.min(b).clone()),
            (a, b) => a.clone().or_else(|| b.clone()),
        };
        Self { min, max }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        text.parse().unwrap()
    }

    #[test]
    fn parses_and_orders_by_semver_precedence() {
        let ordered = [
            "0.4.9",
            "0.5.0-alpha",
            "0.5.0-alpha.1",
            "0.5.0-alpha.beta",
            "0.5.0-beta.2",
            "0.5.0-beta.11",
            "0.5.0-nightly.20261118",
            "0.5.0",
            "0.5.1",
            "1.0.0",
        ];
        for pair in ordered.windows(2) {
            assert!(v(pair[0]) < v(pair[1]), "{} < {}", pair[0], pair[1]);
        }
        assert_eq!(v("1.2.3+build.5"), v("1.2.3"));
        assert_eq!(v("0.5.0-beta.2").to_string(), "0.5.0-beta.2");
    }

    #[test]
    fn rejects_what_is_not_a_version() {
        for bad in [
            "",
            "1",
            "1.2",
            "1.2.3.4",
            "01.2.3",
            "1.2.x",
            "1.2.3-",
            "1.2.3-a..b",
            "1.2.3-é",
            " 1.2.3",
        ] {
            assert!(bad.parse::<Version>().is_err(), "{bad:?} parsed");
        }
        assert!("1.2.3".repeat(20).parse::<Version>().is_err());
    }

    #[test]
    fn ranges_contain_nest_and_intersect() {
        let range = VersionRange::new(Some(v("1.0.0")), Some(v("2.0.0")));
        assert!(range.contains(&v("1.0.0")) && range.contains(&v("1.9.9")));
        assert!(!range.contains(&v("2.0.0")) && !range.contains(&v("0.9.0")));
        assert!(
            !range.contains(&v("1.0.0-beta.1")),
            "a pre-release sorts before its release"
        );
        let open = VersionRange::default();
        assert!(open.contains(&v("9.9.9")));
        assert!(range.within(&open) && !open.within(&range));
        let narrow = VersionRange::new(Some(v("1.2.0")), Some(v("1.5.0")));
        assert!(narrow.within(&range));
        assert_eq!(range.intersect(&narrow), narrow);
        assert!(VersionRange::new(Some(v("2.0.0")), Some(v("1.0.0"))).is_empty());
    }
}
