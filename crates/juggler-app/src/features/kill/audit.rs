//! The cleanup audit's writer: intent before the kill, outcome after.

use crate::commands::now_millis;

/// The audit-writer seam (AJ-E509 Req.3's fault-injection gate):
/// production is the repo; tests inject failures to prove NO AUDIT,
/// NO KILL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The audit-writer failure (there is nothing to distinguish — any
/// failure refuses the kill).
pub struct AuditWriteError;

impl std::fmt::Display for AuditWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cleanup audit write failed")
    }
}

/// The audit-writer seam (production = the repo; tests inject
/// failures for the NO-AUDIT-NO-KILL gate).
pub trait AuditWriter {
    /// Writes the intent row; `Err` refuses the kill.
    ///
    /// # Errors
    ///
    /// [`AuditWriteError`] when the store fails — NO AUDIT, NO KILL.
    fn intent(
        &self,
        record: &juggler_core::storage::CleanupAuditRecord,
    ) -> Result<i64, AuditWriteError>;

    /// Completes the record with the ladder outcome (best-effort).
    ///
    /// # Errors
    ///
    /// [`AuditWriteError`] when the store fails.
    fn complete(
        &self,
        id: i64,
        outcome: &str,
        ports_after: Option<&str>,
    ) -> Result<(), AuditWriteError>;
}

impl AuditWriter for juggler_core::storage::CleanupAuditRepo {
    fn intent(
        &self,
        record: &juggler_core::storage::CleanupAuditRecord,
    ) -> Result<i64, AuditWriteError> {
        juggler_core::storage::CleanupAuditRepo::intent(self, record).map_err(|_| AuditWriteError)
    }

    fn complete(
        &self,
        id: i64,
        outcome: &str,
        ports_after: Option<&str>,
    ) -> Result<(), AuditWriteError> {
        juggler_core::storage::CleanupAuditRepo::complete(
            self,
            id,
            outcome,
            ports_after,
            now_millis(),
        )
        .map_err(|_| AuditWriteError)
    }
}
