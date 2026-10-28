//! Protocol versioning (AJ-E703; ADR-0013 architecture contract).
//!
//! Same major and minor: full service. One minor apart: core telemetry
//! only, with a labelled reason. Any other gap — or a version below this
//! node's minimum — is refused with a named reason. Degraded service
//! never relaxes permission, replay, or size checks.

/// This build's protocol major version.
pub const PROTOCOL_MAJOR: u32 = 1;
/// This build's protocol minor version.
pub const PROTOCOL_MINOR: u32 = 0;
/// The oldest minor (of this major) this node will talk to at all.
pub const MIN_SUPPORTED_MINOR: u32 = 0;

/// What a node serves a peer at a given version.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ServiceLevel {
    /// Everything this version defines.
    Full,
    /// Core telemetry only (heartbeat, resources, session deltas).
    Degraded {
        /// Plain words for the banner.
        reason: String,
    },
    /// Not served.
    Refused {
        /// Plain words for the error.
        reason: String,
    },
}

impl ServiceLevel {
    /// The wire word (`full` / `degraded`); refused never reaches the wire
    /// as a level — it is an error.
    #[must_use]
    pub fn wire(&self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Degraded { .. } => "degraded",
            Self::Refused { .. } => "refused",
        }
    }
}

/// Negotiates the service level for a peer speaking `major.minor`.
#[must_use]
#[allow(
    clippy::absurd_extreme_comparisons,
    reason = "the supported floor is 0 today and rises with future minors"
)]
pub fn negotiate(major: u32, minor: u32) -> ServiceLevel {
    if major != PROTOCOL_MAJOR {
        return ServiceLevel::Refused {
            reason: format!(
                "That Mac speaks Fleet protocol v{major}; this one speaks v{PROTOCOL_MAJOR}. Update both to the same AI Juggler release."
            ),
        };
    }
    if minor < MIN_SUPPORTED_MINOR {
        return ServiceLevel::Refused {
            reason: format!(
                "That Mac's AI Juggler is too old (protocol v{major}.{minor}). Update it to connect."
            ),
        };
    }
    match minor.abs_diff(PROTOCOL_MINOR) {
        0 => ServiceLevel::Full,
        1 => ServiceLevel::Degraded {
            reason: format!(
                "Peer v{major}.{minor}, this computer v{PROTOCOL_MAJOR}.{PROTOCOL_MINOR} — telemetry only until both are updated"
            ),
        },
        _ => ServiceLevel::Refused {
            reason: format!(
                "Versions too far apart (v{major}.{minor} vs v{PROTOCOL_MAJOR}.{PROTOCOL_MINOR}). Update both to the same release."
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_downgrade_matrix() {
        assert_eq!(
            negotiate(PROTOCOL_MAJOR, PROTOCOL_MINOR),
            ServiceLevel::Full
        );
        assert!(matches!(
            negotiate(PROTOCOL_MAJOR, PROTOCOL_MINOR + 1),
            ServiceLevel::Degraded { .. }
        ));
        assert!(matches!(
            negotiate(PROTOCOL_MAJOR, PROTOCOL_MINOR + 2),
            ServiceLevel::Refused { .. }
        ));
        assert!(matches!(
            negotiate(PROTOCOL_MAJOR + 1, PROTOCOL_MINOR),
            ServiceLevel::Refused { .. }
        ));
        assert!(matches!(negotiate(0, 9), ServiceLevel::Refused { .. }));
        assert_eq!(ServiceLevel::Full.wire(), "full");
    }
}
