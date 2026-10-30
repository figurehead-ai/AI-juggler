//! ADR-0023: how big the local cache may be, from the machine it runs
//! on. The numbers, and why:
//!
//! - **2 % of the free disk, between 64 MB and 2 GB** ([`SHARE_PERCENT`],
//!   [`FLOOR`], [`CEILING`]). A Mac with 100 GB free keeps up to 2 GB; one
//!   with 10 GB free keeps 200 MB. 64 MB still holds every app icon and
//!   the indexes of a few dozen busy sessions, so a full disk keeps the
//!   app quick where it matters most. Above 2 GB nothing gets faster: the
//!   caches registered today come to a few hundred MB on a heavy setup.
//! - **Low disk: under 5 GB free, or under 5 % of a small disk**
//!   ([`low_disk_line`]), whichever is lower. macOS starts purging its own
//!   caches near there. In low-disk mode the cache trims to the floor and
//!   writes nothing new; it leaves the mode only once there is a fifth
//!   more room than the line ([`LEAVE_EXTRA_FIFTHS`]), so it doesn't flap
//!   around it.
//! - **Unknown free space** (a system without the reading yet): 256 MB
//!   ([`UNKNOWN`]), what the 2 % share gives at 12.8 GB free.
//! - **Memory tiers**: 8 GB or less is small, up to 16 GB medium, more is
//!   large ([`Tier`]). What a cache keeps in memory scales by half, one
//!   and two times its base (an 8 GB Mac keeps half the subagent readers
//!   a 16 GB one does).

use juggler_platform::room::DiskSpace;

// Decimal units, as Finder and Settings count them.
const MB: u64 = 1_000_000;
const GB: u64 = 1_000 * MB;
/// Memory is counted in binary units: an "8 GB" Mac has 8 GiB.
const GIB: u64 = 1 << 30;

/// The least the cache may keep, and what low-disk mode trims to.
pub const FLOOR: u64 = 64 * MB;
/// The most the automatic limit gives.
pub const CEILING: u64 = 2 * GB;
/// The automatic limit's share of the free disk, in percent.
pub const SHARE_PERCENT: u64 = 2;
/// The limit when the free disk can't be read.
pub const UNKNOWN: u64 = 256 * MB;
/// Low disk below this much free space…
pub const LOW_DISK_BYTES: u64 = 5 * GB;
/// …or below this share of a small disk, in percent.
pub const LOW_DISK_PERCENT: u64 = 5;
/// Leaving low-disk mode takes this many fifths of the line more room.
pub const LEAVE_EXTRA_FIFTHS: u64 = 1;
/// The largest fixed limit a user may pick.
pub const MOST: u64 = 64 * GB;

/// The limit the user chose.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Limit {
    /// From the machine (the default).
    #[default]
    Automatic,
    /// A size the user picked, in bytes.
    Fixed(u64),
}

impl Limit {
    /// The stored value: empty for automatic, else megabytes.
    #[must_use]
    pub fn stored(self) -> String {
        match self {
            Self::Automatic => String::new(),
            Self::Fixed(bytes) => (bytes / MB).to_string(),
        }
    }

    /// From its stored value (anything unreadable is automatic).
    #[must_use]
    pub fn parse(raw: Option<&str>) -> Self {
        raw.and_then(|raw| raw.trim().parse::<u64>().ok())
            .filter(|mb| *mb > 0)
            .map_or(Self::Automatic, Self::from_mb)
    }

    /// A fixed limit of `mb` megabytes, within the floor and [`MOST`].
    #[must_use]
    pub fn from_mb(mb: u64) -> Self {
        Self::Fixed(mb.saturating_mul(MB).clamp(FLOOR, MOST))
    }
}

/// What the cache may use now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    /// Bytes on disk, every cache together.
    pub bytes: u64,
    /// The disk is nearly full: trim to the floor and write nothing new.
    pub low_disk: bool,
}

/// Where low disk starts on `disk`.
#[must_use]
pub fn low_disk_line(disk: DiskSpace) -> u64 {
    LOW_DISK_BYTES.min(disk.total / 100 * LOW_DISK_PERCENT)
}

/// The budget for `limit` on `disk` (`None`: unknown), given whether the
/// last one was in low-disk mode.
#[must_use]
pub fn budget(limit: Limit, disk: Option<DiskSpace>, was_low: bool) -> Budget {
    let low_disk = disk.is_some_and(|disk| {
        let line = low_disk_line(disk);
        let line = if was_low {
            line + line / 5 * LEAVE_EXTRA_FIFTHS
        } else {
            line
        };
        disk.free < line
    });
    if low_disk {
        return Budget {
            bytes: FLOOR,
            low_disk,
        };
    }
    let bytes = match (limit, disk) {
        (Limit::Fixed(bytes), _) => bytes,
        (Limit::Automatic, Some(disk)) => (disk.free / 100 * SHARE_PERCENT).clamp(FLOOR, CEILING),
        (Limit::Automatic, None) => UNKNOWN,
    };
    Budget { bytes, low_disk }
}

/// How much memory this Mac has, in three steps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tier {
    /// 8 GB or less.
    Small,
    /// Up to 16 GB (and when it can't be read).
    #[default]
    Medium,
    /// More than 16 GB.
    Large,
}

impl Tier {
    /// The tier of `memory` bytes.
    #[must_use]
    pub fn of(memory: Option<u64>) -> Self {
        match memory {
            Some(bytes) if bytes <= 8 * GIB => Self::Small,
            Some(bytes) if bytes > 16 * GIB => Self::Large,
            _ => Self::Medium,
        }
    }

    /// `base` (what a medium Mac keeps in memory) for this tier.
    #[must_use]
    pub fn scale(self, base: usize) -> usize {
        match self {
            Self::Small => (base / 2).max(1),
            Self::Medium => base,
            Self::Large => base.saturating_mul(2),
        }
    }

    pub(super) fn to_u8(self) -> u8 {
        match self {
            Self::Small => 0,
            Self::Medium => 1,
            Self::Large => 2,
        }
    }

    pub(super) fn from_u8(raw: u8) -> Self {
        match raw {
            0 => Self::Small,
            2 => Self::Large,
            _ => Self::Medium,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn disk(free_gb: u64, total_gb: u64) -> Option<DiskSpace> {
        Some(DiskSpace {
            free: free_gb * GB,
            total: total_gb * GB,
        })
    }

    #[test]
    fn the_automatic_budget_is_a_share_of_free_disk_clamped() {
        let auto = |free, total| budget(Limit::Automatic, disk(free, total), false).bytes;
        // 2 % of 50 GB is 1 GB.
        assert_eq!(auto(50, 500), GB);
        // Never above the ceiling…
        assert_eq!(auto(900, 1000), CEILING);
        // …and never under the floor (2 % of 6 GB is 120 MB).
        assert_eq!(auto(6, 1000), 6 * GB / 100 * 2);
        assert_eq!(
            budget(Limit::Automatic, disk(6, 2000), false).bytes,
            6 * GB / 100 * 2
        );
        assert_eq!(
            budget(Limit::Automatic, None, false),
            Budget {
                bytes: UNKNOWN,
                low_disk: false
            }
        );
        // A tiny disk with room: at the floor, not in low-disk mode.
        assert_eq!(
            budget(Limit::Automatic, disk(2, 16), false),
            Budget {
                bytes: FLOOR,
                low_disk: false
            }
        );
        // A fixed limit is the user's.
        assert_eq!(
            budget(Limit::from_mb(512), disk(900, 1000), false).bytes,
            512 * MB
        );
        assert_eq!(Limit::from_mb(1), Limit::Fixed(FLOOR));
        assert_eq!(Limit::from_mb(u64::MAX), Limit::Fixed(MOST));
    }

    #[test]
    fn low_disk_trims_to_the_floor_and_leaves_only_with_room_to_spare() {
        // 4 GB free of 500 GB: under 5 GB.
        let low = budget(Limit::Automatic, disk(4, 500), false);
        assert_eq!(
            low,
            Budget {
                bytes: FLOOR,
                low_disk: true
            }
        );
        // A fixed limit too.
        assert_eq!(budget(Limit::from_mb(4096), disk(4, 500), false), low);
        // A small disk: 5 % of 64 GB is 3.2 GB, lower than 5 GB.
        assert!(!budget(Limit::Automatic, disk(4, 64), false).low_disk);
        assert!(budget(Limit::Automatic, disk(3, 64), false).low_disk);
        // Just over the line after being low: still low until a fifth more.
        let just_over = Some(DiskSpace {
            free: 5 * GB + 100 * MB,
            total: 500 * GB,
        });
        assert!(!budget(Limit::Automatic, just_over, false).low_disk);
        assert!(budget(Limit::Automatic, just_over, true).low_disk);
        assert!(!budget(Limit::Automatic, disk(7, 500), true).low_disk);
    }

    #[test]
    fn memory_tiers_scale_what_is_kept_in_memory() {
        assert_eq!(Tier::of(Some(8 * GIB)), Tier::Small);
        assert_eq!(Tier::of(Some(16 * GIB)), Tier::Medium);
        assert_eq!(Tier::of(Some(64 * GIB)), Tier::Large);
        assert_eq!(Tier::of(None), Tier::Medium);
        assert_eq!(Tier::Small.scale(8), 4);
        assert_eq!(Tier::Small.scale(1), 1);
        assert_eq!(Tier::Large.scale(8), 16);
        for tier in [Tier::Small, Tier::Medium, Tier::Large] {
            assert_eq!(Tier::from_u8(tier.to_u8()), tier);
        }
        assert_eq!(Limit::parse(Some("")), Limit::Automatic);
        assert_eq!(Limit::parse(Some("junk")), Limit::Automatic);
        assert_eq!(Limit::parse(Some("1000")), Limit::Fixed(GB));
        assert_eq!(
            Limit::parse(Some(&Limit::Fixed(GB).stored())),
            Limit::Fixed(GB)
        );
        assert_eq!(Limit::Automatic.stored(), "");
    }
}
