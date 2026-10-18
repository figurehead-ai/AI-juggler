//! Pairing (AJ-E704; ADR-0013 architecture contract).
//!
//! The **host** (the machine the user opened "Add a machine" on) shows a
//! random, single-use 6-digit code; the **joiner** enters it. The code is
//! the password of a SPAKE2 exchange that runs inside a TLS 1.3 session
//! whose keys are not yet pinned. Both sides then prove they derived the
//! same key with a confirmation MAC over a transcript binding both
//! identity keys, both `MachineId`s, the protocol version, and the TLS
//! exporter of THIS session — so the key each side saw in TLS is the key
//! that gets pinned, and a relaying attacker's confirmation fails. A
//! wrong code is one online guess; nothing about the code leaks offline.
//!
//! [`PairingGuard`] enforces the rest of the contract: pairing only
//! inside a user-opened 2-minute window, one session at a time, a fresh
//! code after every failure, and persisted lockouts (5 failures → 60 s;
//! 20 failures in 24 h → disabled until the user re-enables pairing).

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use spake2::{Ed25519Group, Identity, Password, Spake2};
use subtle::ConstantTimeEq;

/// How long a pairing window stays open.
pub const PAIRING_WINDOW_MS: i64 = 2 * 60_000;
/// Failures before a short lockout.
pub const SHORT_LOCKOUT_AFTER: u32 = 5;
/// The short lockout.
pub const SHORT_LOCKOUT_MS: i64 = 60_000;
/// Failures in 24 h before pairing is disabled.
pub const DAILY_DISABLE_AFTER: u32 = 20;
const DAY_MS: i64 = 24 * 60 * 60_000;

/// Pairing failures, in plain words.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PairingError {
    /// No window is open on the host.
    #[error("Pairing isn't open on that Mac — click Add a machine there first.")]
    NotOpen,
    /// The window closed (2 minutes).
    #[error("The pairing window closed — start again.")]
    Expired,
    /// Another pairing is already in progress.
    #[error("Another Mac is pairing right now — try again in a moment.")]
    Busy,
    /// Temporarily locked after repeated failures.
    #[error("Too many wrong codes — pairing is paused for a minute.")]
    LockedOut,
    /// Disabled after many failures in a day.
    #[error("Pairing is turned off after too many wrong codes today — turn it back on in Fleet.")]
    Disabled,
    /// The code did not match (or someone interfered).
    #[error("The code didn't match. A new code is shown on the other Mac.")]
    Mismatch,
    /// A protocol step was malformed.
    #[error("The pairing exchange was malformed.")]
    Malformed,
    /// The random source failed.
    #[error("Couldn't generate a pairing code.")]
    Random,
}

/// A single-use 6-digit code.
#[derive(Clone, PartialEq, Eq)]
pub struct PairingCode(String);

impl std::fmt::Debug for PairingCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never logged.
        f.write_str("PairingCode(******)")
    }
}

impl PairingCode {
    /// A uniformly random code (rejection sampling — no modulo bias).
    ///
    /// # Errors
    ///
    /// Returns [`PairingError::Random`] if the OS random source fails.
    pub fn generate() -> Result<Self, PairingError> {
        loop {
            let mut bytes = [0_u8; 4];
            getrandom::fill(&mut bytes).map_err(|_| PairingError::Random)?;
            let value = u32::from_le_bytes(bytes);
            // 4_294_000_000 is the largest multiple of 1_000_000 ≤ u32::MAX.
            if value < 4_294_000_000 {
                return Ok(Self(format!("{:06}", value % 1_000_000)));
            }
        }
    }

    /// Parses what a user typed: six digits, spaces and dashes ignored.
    ///
    /// # Errors
    ///
    /// Returns [`PairingError::Malformed`] for anything else.
    pub fn parse(typed: &str) -> Result<Self, PairingError> {
        let digits: String = typed.chars().filter(|c| !matches!(c, ' ' | '-')).collect();
        if digits.len() == 6 && digits.chars().all(|c| c.is_ascii_digit()) {
            Ok(Self(digits))
        } else {
            Err(PairingError::Malformed)
        }
    }

    /// The digits, for display as `123 456` on the host only.
    #[must_use]
    pub fn display(&self) -> String {
        format!("{} {}", &self.0[..3], &self.0[3..])
    }

    fn password(&self) -> Password {
        Password::new(self.0.as_bytes())
    }
}

/// What both sides bind into the confirmation MAC.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transcript {
    /// The host's identity key, as seen in TLS by the joiner.
    pub host_key: [u8; 32],
    /// The joiner's identity key, as seen in TLS by the host.
    pub joiner_key: [u8; 32],
    /// The host's `MachineId`.
    pub host_machine_id: String,
    /// The joiner's `MachineId`.
    pub joiner_machine_id: String,
    /// `major.minor` of the protocol both speak.
    pub protocol: (u32, u32),
    /// The TLS 1.3 exporter value of THIS connection (32 bytes, label
    /// [`EXPORTER_LABEL`]).
    pub tls_exporter: [u8; 32],
}

/// The TLS keying-material exporter label for pairing.
pub const EXPORTER_LABEL: &[u8] = b"EXPORTER-aijuggler-pairing-v1";

impl Transcript {
    fn bytes(&self) -> Vec<u8> {
        let mut out = b"aijuggler-pair-v1".to_vec();
        out.extend_from_slice(&self.host_key);
        out.extend_from_slice(&self.joiner_key);
        for id in [&self.host_machine_id, &self.joiner_machine_id] {
            out.extend_from_slice(&u32::try_from(id.len()).unwrap_or(u32::MAX).to_be_bytes());
            out.extend_from_slice(id.as_bytes());
        }
        out.extend_from_slice(&self.protocol.0.to_be_bytes());
        out.extend_from_slice(&self.protocol.1.to_be_bytes());
        out.extend_from_slice(&self.tls_exporter);
        out
    }
}

/// Which side computes a confirmation (domain-separated so one side's
/// MAC can never be reflected back as the other's).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// The machine showing the code.
    Host,
    /// The machine entering the code.
    Joiner,
}

fn confirmation(key: &[u8], role: Role, transcript: &Transcript) -> Result<[u8; 32], PairingError> {
    let mut mac =
        <Hmac<Sha256> as KeyInit>::new_from_slice(key).map_err(|_| PairingError::Malformed)?;
    mac.update(match role {
        Role::Host => b"host",
        Role::Joiner => b"joiner",
    });
    mac.update(&transcript.bytes());
    Ok(mac.finalize().into_bytes().into())
}

/// One side of a SPAKE2 pairing exchange.
pub struct PairingSession {
    role: Role,
    state: Option<Spake2<Ed25519Group>>,
    key: Option<Vec<u8>>,
}

impl std::fmt::Debug for PairingSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PairingSession")
            .field("role", &self.role)
            .finish_non_exhaustive()
    }
}

impl PairingSession {
    /// Starts one side; returns the session and the message to send.
    /// The SPAKE2 identities are the fixed role names — the joiner can't
    /// know the host's `MachineId` before the exchange — and the real
    /// ids are bound by the confirmation MAC's transcript instead.
    #[must_use]
    pub fn start(role: Role, code: &PairingCode) -> (Self, Vec<u8>) {
        let host = Identity::new(b"aijuggler-pair-host");
        let joiner = Identity::new(b"aijuggler-pair-joiner");
        let (state, message) = match role {
            Role::Host => Spake2::<Ed25519Group>::start_a(&code.password(), &host, &joiner),
            Role::Joiner => Spake2::<Ed25519Group>::start_b(&code.password(), &host, &joiner),
        };
        (
            Self {
                role,
                state: Some(state),
                key: None,
            },
            message,
        )
    }

    /// Consumes the other side's message and returns OUR confirmation to
    /// send.
    ///
    /// # Errors
    ///
    /// Returns [`PairingError::Malformed`] for a bad message.
    pub fn finish(
        &mut self,
        theirs: &[u8],
        transcript: &Transcript,
    ) -> Result<[u8; 32], PairingError> {
        let state = self.state.take().ok_or(PairingError::Malformed)?;
        let key = state.finish(theirs).map_err(|_| PairingError::Malformed)?;
        let ours = confirmation(&key, self.role, transcript)?;
        self.key = Some(key);
        Ok(ours)
    }

    /// Checks the other side's confirmation (constant time).
    ///
    /// # Errors
    ///
    /// Returns [`PairingError::Mismatch`] when the codes differed or the
    /// transcript (keys, ids, version, TLS session) is not the same on
    /// both ends.
    pub fn verify(&self, theirs: &[u8; 32], transcript: &Transcript) -> Result<(), PairingError> {
        let key = self.key.as_ref().ok_or(PairingError::Malformed)?;
        let other = match self.role {
            Role::Host => Role::Joiner,
            Role::Joiner => Role::Host,
        };
        let expected = confirmation(key, other, transcript)?;
        if bool::from(expected.ct_eq(theirs)) {
            Ok(())
        } else {
            Err(PairingError::Mismatch)
        }
    }
}

/// The persisted lockout state.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GuardState {
    /// Failures since the last short lockout ended.
    pub recent_failures: u32,
    /// Epoch ms until which pairing is locked.
    pub locked_until_ms: i64,
    /// Failure timestamps in the last 24 h.
    pub daily_failures: Vec<i64>,
    /// Pairing disabled until the user re-enables it.
    pub disabled: bool,
}

/// Where the guard keeps its state (the app persists it; lockouts must
/// survive restarts).
pub trait GuardStore: Send + Sync {
    /// Loads the state.
    fn load(&self) -> GuardState;
    /// Saves the state.
    fn save(&self, state: &GuardState);
}

/// An in-memory [`GuardStore`] (tests).
#[derive(Default, Debug)]
pub struct MemoryGuardStore(std::sync::Mutex<GuardState>);

impl GuardStore for MemoryGuardStore {
    fn load(&self) -> GuardState {
        self.0.lock().map(|state| state.clone()).unwrap_or_default()
    }
    fn save(&self, state: &GuardState) {
        if let Ok(mut current) = self.0.lock() {
            *current = state.clone();
        }
    }
}

/// The host's pairing window.
#[derive(Debug)]
struct Window {
    code: PairingCode,
    opened_at_ms: i64,
    in_session: bool,
}

/// The host-side guard: window, single session, rotation, lockouts.
pub struct PairingGuard<S: GuardStore> {
    store: S,
    window: Option<Window>,
}

impl<S: GuardStore> std::fmt::Debug for PairingGuard<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PairingGuard")
            .field("open", &self.window.is_some())
            .finish_non_exhaustive()
    }
}

impl<S: GuardStore> PairingGuard<S> {
    /// A guard over `store` (no window open).
    #[must_use]
    pub fn new(store: S) -> Self {
        Self {
            store,
            window: None,
        }
    }

    fn check_locks(&self, now_ms: i64) -> Result<(), PairingError> {
        let state = self.store.load();
        if state.disabled {
            return Err(PairingError::Disabled);
        }
        if now_ms < state.locked_until_ms {
            return Err(PairingError::LockedOut);
        }
        Ok(())
    }

    /// Opens a window ("Add a machine") and returns the code to show.
    ///
    /// # Errors
    ///
    /// Returns [`PairingError::LockedOut`] / [`PairingError::Disabled`]
    /// while locked, or [`PairingError::Random`].
    pub fn open(&mut self, now_ms: i64) -> Result<PairingCode, PairingError> {
        self.check_locks(now_ms)?;
        let code = PairingCode::generate()?;
        self.window = Some(Window {
            code: code.clone(),
            opened_at_ms: now_ms,
            in_session: false,
        });
        Ok(code)
    }

    /// Closes the window (Cancel, success, or the user left Fleet).
    pub fn close(&mut self) {
        self.window = None;
    }

    /// The current code, if a window is open and unexpired.
    #[must_use]
    pub fn current_code(&self, now_ms: i64) -> Option<PairingCode> {
        self.window
            .as_ref()
            .filter(|window| now_ms - window.opened_at_ms <= PAIRING_WINDOW_MS)
            .map(|window| window.code.clone())
    }

    /// Admits one joiner session; returns the code to run SPAKE2 with.
    ///
    /// # Errors
    ///
    /// Returns why the session can't start.
    pub fn begin_session(&mut self, now_ms: i64) -> Result<PairingCode, PairingError> {
        self.check_locks(now_ms)?;
        let window = self.window.as_mut().ok_or(PairingError::NotOpen)?;
        if now_ms - window.opened_at_ms > PAIRING_WINDOW_MS {
            self.window = None;
            return Err(PairingError::Expired);
        }
        if window.in_session {
            return Err(PairingError::Busy);
        }
        window.in_session = true;
        Ok(window.code.clone())
    }

    /// Records a failed session: rotates the code (the window stays
    /// open for the user to retry) and applies the lockouts.
    ///
    /// # Errors
    ///
    /// Returns [`PairingError::Random`] if a new code can't be generated.
    pub fn fail(&mut self, now_ms: i64) -> Result<(), PairingError> {
        let mut state = self.store.load();
        state.daily_failures.retain(|at| now_ms - at < DAY_MS);
        state.daily_failures.push(now_ms);
        state.recent_failures += 1;
        if u32::try_from(state.daily_failures.len()).unwrap_or(u32::MAX) >= DAILY_DISABLE_AFTER {
            state.disabled = true;
        }
        if state.recent_failures >= SHORT_LOCKOUT_AFTER {
            state.locked_until_ms = now_ms + SHORT_LOCKOUT_MS;
            state.recent_failures = 0;
        }
        self.store.save(&state);
        if let Some(window) = self.window.as_mut() {
            window.code = PairingCode::generate()?;
            window.in_session = false;
        }
        Ok(())
    }

    /// Ends a session that neither failed nor succeeded (cancel,
    /// timeout): the slot frees, the code and counters are unchanged.
    pub fn release(&mut self) {
        if let Some(window) = self.window.as_mut() {
            window.in_session = false;
        }
    }

    /// Records success: the window closes and the short counter resets.
    pub fn succeed(&mut self) {
        let mut state = self.store.load();
        state.recent_failures = 0;
        self.store.save(&state);
        self.window = None;
    }

    /// Re-enables pairing after the daily disable (a user click).
    pub fn re_enable(&mut self) {
        let mut state = self.store.load();
        state.disabled = false;
        state.daily_failures.clear();
        state.recent_failures = 0;
        self.store.save(&state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transcript() -> Transcript {
        Transcript {
            host_key: [1; 32],
            joiner_key: [2; 32],
            host_machine_id: "host".into(),
            joiner_machine_id: "joiner".into(),
            protocol: (1, 0),
            tls_exporter: [9; 32],
        }
    }

    fn exchange(
        host_code: &str,
        joiner_code: &str,
        host_t: &Transcript,
        joiner_t: &Transcript,
    ) -> (Result<(), PairingError>, Result<(), PairingError>) {
        let (mut host, to_joiner) =
            PairingSession::start(Role::Host, &PairingCode::parse(host_code).unwrap());
        let (mut joiner, to_host) =
            PairingSession::start(Role::Joiner, &PairingCode::parse(joiner_code).unwrap());
        let host_confirm = host.finish(&to_host, host_t).unwrap();
        let joiner_confirm = joiner.finish(&to_joiner, joiner_t).unwrap();
        (
            host.verify(&joiner_confirm, host_t),
            joiner.verify(&host_confirm, joiner_t),
        )
    }

    #[test]
    fn matching_codes_and_transcripts_confirm_on_both_sides() {
        let t = transcript();
        let (host, joiner) = exchange("123456", "123456", &t, &t);
        assert_eq!(host, Ok(()));
        assert_eq!(joiner, Ok(()));
    }

    #[test]
    fn a_wrong_code_fails_both_confirmations() {
        let t = transcript();
        let (host, joiner) = exchange("123456", "123457", &t, &t);
        assert_eq!(host, Err(PairingError::Mismatch));
        assert_eq!(joiner, Err(PairingError::Mismatch));
    }

    #[test]
    fn any_transcript_difference_fails_confirmation() {
        // A relaying attacker terminates TLS twice: each side sees a
        // different peer key and a different exporter.
        let base = transcript();
        let variants = [
            Transcript {
                joiner_key: [3; 32],
                ..base.clone()
            },
            Transcript {
                host_key: [4; 32],
                ..base.clone()
            },
            Transcript {
                tls_exporter: [8; 32],
                ..base.clone()
            },
            Transcript {
                joiner_machine_id: "other".into(),
                ..base.clone()
            },
            Transcript {
                protocol: (1, 1),
                ..base.clone()
            },
        ];
        for variant in variants {
            let (host, joiner) = exchange("123456", "123456", &base, &variant);
            assert!(host.is_err() && joiner.is_err(), "{variant:?}");
        }
    }

    #[test]
    fn a_confirmation_cannot_be_reflected() {
        let t = transcript();
        let code = PairingCode::parse("111111").unwrap();
        let (mut host, to_joiner) = PairingSession::start(Role::Host, &code);
        let (mut joiner, to_host) = PairingSession::start(Role::Joiner, &code);
        let host_confirm = host.finish(&to_host, &t).unwrap();
        joiner.finish(&to_joiner, &t).unwrap();
        // The host's own MAC echoed back must not verify as the joiner's.
        assert_eq!(host.verify(&host_confirm, &t), Err(PairingError::Mismatch));
    }

    #[test]
    fn codes_are_six_digits_and_parse_leniently() {
        for _ in 0..200 {
            let code = PairingCode::generate().unwrap();
            assert_eq!(code.0.len(), 6);
            assert!(code.0.chars().all(|c| c.is_ascii_digit()));
        }
        assert!(PairingCode::parse("123-456").is_ok());
        assert!(PairingCode::parse(" 123 456 ").is_ok());
        assert!(PairingCode::parse("12345").is_err());
        assert!(PairingCode::parse("abcdef").is_err());
        assert_eq!(
            format!("{:?}", PairingCode::parse("123456").unwrap()),
            "PairingCode(******)"
        );
        assert_eq!(PairingCode::parse("123456").unwrap().display(), "123 456");
    }

    #[test]
    fn the_guard_enforces_window_single_session_rotation_and_lockouts() {
        let mut guard = PairingGuard::new(MemoryGuardStore::default());
        assert_eq!(guard.begin_session(0), Err(PairingError::NotOpen));
        let first = guard.open(0).unwrap();
        assert_eq!(guard.begin_session(1).unwrap(), first);
        assert_eq!(
            guard.begin_session(2),
            Err(PairingError::Busy),
            "one session at a time"
        );
        guard.fail(3).unwrap();
        let rotated = guard.current_code(4).unwrap();
        assert_ne!(rotated, first, "a failure rotates the code");
        // Five failures → 60 s lockout.
        for at in 5..9 {
            guard.begin_session(at).unwrap();
            guard.fail(at).unwrap();
        }
        assert_eq!(guard.begin_session(10), Err(PairingError::LockedOut));
        assert_eq!(guard.open(10), Err(PairingError::LockedOut));
        // The window expires after 2 minutes.
        let later = SHORT_LOCKOUT_MS + 10;
        guard.open(later).unwrap();
        assert_eq!(
            guard.begin_session(later + PAIRING_WINDOW_MS + 1),
            Err(PairingError::Expired)
        );
        guard.succeed();
        assert!(guard.current_code(later).is_none());
    }

    #[test]
    fn lockouts_persist_across_a_restart_and_twenty_failures_disable_pairing() {
        let store = std::sync::Arc::new(MemoryGuardStore::default());
        struct Shared(std::sync::Arc<MemoryGuardStore>);
        impl GuardStore for Shared {
            fn load(&self) -> GuardState {
                self.0.load()
            }
            fn save(&self, state: &GuardState) {
                self.0.save(state);
            }
        }
        let mut now = 0_i64;
        for _ in 0..DAILY_DISABLE_AFTER {
            // A fresh guard each round = an app restart.
            let mut guard = PairingGuard::new(Shared(store.clone()));
            now += SHORT_LOCKOUT_MS + 1;
            if guard.open(now).is_err() {
                break;
            }
            guard.begin_session(now).unwrap();
            guard.fail(now).unwrap();
        }
        let mut guard = PairingGuard::new(Shared(store.clone()));
        assert_eq!(
            guard.open(now + SHORT_LOCKOUT_MS * 10),
            Err(PairingError::Disabled)
        );
        guard.re_enable();
        assert!(guard.open(now + SHORT_LOCKOUT_MS * 10).is_ok());
    }
}
