//! Fixtures the crate's unit tests share (D-94: test setup in one place,
//! not copied per module).

use juggler_core::model::{
    InstanceKind, InstanceRecord, MachineId, ProfileId, SessionId, SessionStatus, Timestamp,
    UsageSnapshot,
};

/// A terminal session on "this-mac", started at a fixed time and active a
/// minute later, with nothing else known.
pub(crate) fn instance(
    id: &str,
    profile: &str,
    status: SessionStatus,
    cwd: std::path::PathBuf,
) -> InstanceRecord {
    InstanceRecord {
        id: SessionId::new(id),
        conversation: None,
        profile: ProfileId::new(profile),
        machine: MachineId::new("this-mac"),
        kind: InstanceKind::Terminal,
        project: None,
        cwd,
        status,
        started_at: Timestamp::from_millis(1_700_000_000_000),
        last_activity: Some(Timestamp::from_millis(1_700_000_060_000)),
        usage: UsageSnapshot::default(),
        attention: None,
        running: None,
        window: None,
        transcript_ref: None,
    }
}
