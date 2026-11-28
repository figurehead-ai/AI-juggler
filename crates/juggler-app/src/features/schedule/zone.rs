//! The user's local time zone (ADR-0039 §10, D-151), read fresh each time
//! it is asked: `jiff`'s own system zone is cached for minutes, and a
//! zone change must apply at the next recompute. On Unix the zone is the
//! one `/etc/localtime` names; elsewhere (and when that can't be read),
//! `jiff`'s own reading of the system's zone.

use jiff::tz::TimeZone;

/// The zone a `/etc/localtime` link names (`…/zoneinfo/Europe/Berlin`).
#[must_use]
pub fn named_by_link(target: &str) -> Option<&str> {
    let (_, name) = target.split_once("zoneinfo/")?;
    (!name.is_empty()).then_some(name)
}

/// The user's local time zone now.
#[must_use]
pub fn local() -> TimeZone {
    #[cfg(unix)]
    if let Ok(target) = std::fs::read_link("/etc/localtime")
        && let Some(name) = target.to_str().and_then(named_by_link)
        && let Ok(zone) = TimeZone::get(name)
    {
        return zone;
    }
    TimeZone::system()
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_link_names_the_zone() {
        assert_eq!(
            super::named_by_link("/var/db/timezone/zoneinfo/Europe/Berlin"),
            Some("Europe/Berlin")
        );
        assert_eq!(
            super::named_by_link("/usr/share/zoneinfo/America/New_York"),
            Some("America/New_York")
        );
        assert_eq!(super::named_by_link("/etc/zoneinfo/"), None);
        assert_eq!(super::named_by_link("/etc/localtime"), None);
        // Whatever the machine, a zone comes back.
        let _ = super::local();
    }
}
