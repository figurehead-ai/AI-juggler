//! The kill types on the wire (exported to TypeScript).

/// One PGID member in the verify sheet's collateral list (Req.5).
#[derive(Clone, Debug, serde::Serialize, PartialEq, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct KillMemberInfo {
    /// OS process id.
    pub pid: u32,
    /// The member's live PGID (`None` when unreadable).
    pub pgid: Option<u32>,
    /// Executable basename (display).
    pub exe: String,
    /// Truncated command line (display).
    pub cmdline: String,
    /// pass2 §5 role guess in plain words ([`role_words`](crate::features::kill::role_words); empty when
    /// unknown) — always labelled a guess in the UI.
    pub role_guess: &'static str,
    /// The exact Req.5 warning when the role may hold unsaved state.
    pub unsaved_state_warning: Option<String>,
    /// True when this pid has a Juggler ledger row (attributed).
    pub from_ledger: bool,
}

/// The verify sheet's data (step 1): click-time identity verdict +
/// the re-read-now facts + the PGID collateral list.
#[derive(Clone, Debug, serde::Serialize, PartialEq, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct KillVerifyInfo {
    /// The machine that owns the target (AJ-E511: every kill-sheet
    /// payload names its machine — `This Mac` locally).
    pub machine_id: String,
    /// OS process id.
    pub pid: u32,
    /// Owning Instance.
    pub session: String,
    /// True only when click-time E503 verification SUCCEEDED.
    pub verified: bool,
    /// The exact refusal copy when `!verified`.
    pub refusal: Option<String>,
    /// Kernel-reported exe NOW.
    pub exe_path: String,
    /// Full command line NOW (recorded value when unreadable).
    pub cmdline: String,
    /// Working directory NOW (recorded value when unreadable).
    pub cwd: String,
    /// Ports the pid holds NOW.
    pub ports: Vec<u16>,
    /// The live PGID.
    pub pgid: Option<u32>,
    /// True when the row left the root's group (E502).
    pub escaped_group: bool,
    /// Every live PGID member (self included) with role guesses.
    pub members: Vec<KillMemberInfo>,
    /// The whole-Instance tree's live pids (scope 3's enumeration).
    pub tree_pids: Vec<u32>,
    /// `TERM → grace → KILL` | `TerminateProcess only` (verbatim).
    pub platform: String,
    /// The ladder grace, seconds.
    #[ts(type = "number")]
    pub grace_secs: u64,
    /// Attribution.
    pub provider: String,
    /// Attribution.
    pub account_id: Option<String>,
    /// Attribution.
    pub project_id: Option<String>,
}

/// One still-held port after the ladder (the honest post-kill report,
/// FMEA C2 — never a success claim).
#[derive(Clone, Debug, serde::Serialize, PartialEq, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct StillHoldingInfo {
    /// The port still held.
    pub port: u16,
    /// The pid holding it NOW (may differ from the killed pid).
    pub pid: u32,
    /// The holder's exe (basename).
    pub exe: String,
}

/// The ladder's per-process report (Req.6's vocabulary, one label per
/// member).
#[derive(Clone, Debug, serde::Serialize, PartialEq, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct KillMemberOutcome {
    /// OS process id.
    pub pid: u32,
    /// `exited on TERM` | `needed KILL` | `already gone` | `refused` |
    /// `TERM'd — outcome unknown` | [`LEFT_NOT_OURS`](crate::features::kill::LEFT_NOT_OURS) |
    /// [`LEFT_UNVERIFIED`](crate::features::kill::LEFT_UNVERIFIED) (the last two: never signalled).
    pub outcome: String,
}

/// The kill outcome (step 3's report): per-process outcomes, the
/// honest still-holding list, and — when an escapee holds the port —
/// the pid-only re-offer data.
#[derive(Clone, Debug, serde::Serialize, PartialEq, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct KillExecuteInfo {
    /// The exact refusal copy when the same-tick verification failed
    /// (zero signals were sent).
    pub refusal: Option<String>,
    /// Per-process outcomes.
    pub members: Vec<KillMemberOutcome>,
    /// True when the user cancelled mid-grace (audit left pending).
    pub cancelled: bool,
    /// True when every member succeeded (`already gone` included).
    pub all_succeeded: bool,
    /// Ports still held after the ladder (by anyone).
    pub still_holding: Vec<StillHoldingInfo>,
    /// The pid-only re-offer when an ESCAPED member still holds a
    /// killed target's port (fresh verification is the UI's next
    /// step, never an automatic signal).
    pub reoffer_pid: Option<u32>,
}
