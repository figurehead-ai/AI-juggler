//! Rounding and bucketing (the schema's rules): counts become buckets,
//! large totals keep two significant figures, versions keep only
//! `major.minor`, histograms count observations per labelled bucket.

pub use crate::generated::CountBucket;

impl CountBucket {
    /// The bucket a count falls in: `0`, `1`, `2`, `3`, `4-5`, `6-9`,
    /// `10+`.
    #[must_use]
    pub const fn of(count: u64) -> Self {
        match count {
            0 => Self::Zero,
            1 => Self::One,
            2 => Self::Two,
            3 => Self::Three,
            4..=5 => Self::FourToFive,
            6..=9 => Self::SixToNine,
            _ => Self::TenOrMore,
        }
    }
}

/// A Provider CLI version as the report may carry it: `major.minor` with
/// up to four digits each, or `other`. It holds numbers only, so no text
/// from the CLI reaches a report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliVersion {
    /// `major.minor`.
    Known {
        /// Major version.
        major: u16,
        /// Minor version.
        minor: u16,
    },
    /// Anything that isn't a plain `major.minor` (or wasn't detected).
    Other,
}

impl CliVersion {
    /// Reads a version string such as `2.1.37` or `v0.46`; keeps only the
    /// first two numeric parts, each at most four digits.
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        let raw = raw.trim().trim_start_matches('v');
        let mut parts = raw.split('.');
        let (Some(major), Some(minor)) = (parts.next(), parts.next()) else {
            return Self::Other;
        };
        let minor = minor
            .split(|c: char| !c.is_ascii_digit())
            .next()
            .unwrap_or_default();
        let digits = |part: &str| {
            (!part.is_empty() && part.len() <= 4 && part.bytes().all(|b| b.is_ascii_digit()))
                .then(|| part.parse::<u16>().ok())
                .flatten()
        };
        match (digits(major), digits(minor)) {
            (Some(major), Some(minor)) => Self::Known { major, minor },
            _ => Self::Other,
        }
    }

    /// The value the report carries.
    #[must_use]
    pub fn wire(self) -> String {
        match self {
            Self::Known { major, minor } => format!("{major}.{minor}"),
            Self::Other => "other".to_owned(),
        }
    }
}

/// Rounds down to two significant figures (123,456 → 120,000).
#[must_use]
pub fn two_significant(value: u64) -> u64 {
    if value < 100 {
        return value;
    }
    let digits = value.ilog10();
    let scale = 10_u64.pow(digits - 1);
    value / scale * scale
}

/// The lower edge of a bucket label (`<5` → −∞, `5-15` → 5, `60+` → 60,
/// `0` → 0, `0.5-1` → 0.5).
fn lower_edge(label: &str) -> f64 {
    if label.starts_with('<') {
        return f64::NEG_INFINITY;
    }
    let number: String = label
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    number.parse().unwrap_or(f64::NEG_INFINITY)
}

/// The index of the bucket `value` falls in, given the schema's labels in
/// order (each bucket runs from its lower edge up to the next one's).
#[must_use]
pub fn histogram_index(buckets: &[&str], value: f64) -> usize {
    let mut index = 0;
    for (i, label) in buckets.iter().enumerate() {
        if value >= lower_edge(label) {
            index = i;
        }
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_bucket_as_the_schema_says() {
        let got: Vec<&str> = [0, 1, 2, 3, 4, 5, 6, 9, 10, 400]
            .iter()
            .map(|n| CountBucket::of(*n).wire())
            .collect();
        assert_eq!(
            got,
            ["0", "1", "2", "3", "4-5", "4-5", "6-9", "6-9", "10+", "10+"]
        );
    }

    #[test]
    fn versions_keep_major_minor_only() {
        assert_eq!(CliVersion::parse("2.1.37").wire(), "2.1");
        assert_eq!(CliVersion::parse("v0.46.0-beta").wire(), "0.46");
        assert_eq!(CliVersion::parse("1.2rc3").wire(), "1.2");
        assert_eq!(CliVersion::parse("12345.1").wire(), "other");
        assert_eq!(CliVersion::parse("claude 2.1 (my-build)").wire(), "other");
        assert_eq!(CliVersion::parse("").wire(), "other");
    }

    #[test]
    fn totals_keep_two_significant_figures() {
        assert_eq!(two_significant(123_456), 120_000);
        assert_eq!(two_significant(99), 99);
        assert_eq!(two_significant(1_999), 1_900);
        assert_eq!(two_significant(10_000_000_000), 10_000_000_000);
    }

    #[test]
    fn histogram_buckets_follow_their_lower_edges() {
        let switch = ["<5", "5-15", "15-60", "60+"];
        assert_eq!(histogram_index(&switch, 0.4), 0);
        assert_eq!(histogram_index(&switch, 5.0), 1);
        assert_eq!(histogram_index(&switch, 59.9), 2);
        assert_eq!(histogram_index(&switch, 3_600.0), 3);
        let children = ["0", "1-2", "3-5", "6+"];
        assert_eq!(histogram_index(&children, 0.0), 0);
        assert_eq!(histogram_index(&children, 2.0), 1);
        assert_eq!(histogram_index(&children, 3.0), 2);
        assert_eq!(histogram_index(&children, 9.0), 3);
        let cpu = ["<0.5", "0.5-1", "1-3", "3+"];
        assert_eq!(histogram_index(&cpu, 0.49), 0);
        assert_eq!(histogram_index(&cpu, 0.75), 1);
    }
}
