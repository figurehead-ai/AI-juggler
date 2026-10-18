//! Stuck/waiting/quota heuristics (SPEC §5.1, AJ-E405).
//!
//! The engine is deliberately **pure over injected inputs and clocks**:
//! evaluations take `now_ms` from the caller so tests and the fixture
//! corpus can time-travel without sleeping. Rules:
//!
//! - **Stuck?** — no output ≥ `stuck_no_output_secs` AND CPU below
//!   `stuck_cpu_max_pct` AND transcript mtime stale ≥ the same window.
//!   `?` is permanent honesty: this is a heuristic.
//! - **Needs you** — an unterminated permission/wait prompt in the output
//!   tail, unchanged for ≥ `waiting_prompt_secs`.
//! - **Quota blocked** — non-zero exit whose output tail carries a
//!   rate-limit signature. Emits immediately (no debounce): an exit is
//!   a fact, not a trend.
//!
//! Debounce (Req.4): a heuristic state must hold for **2 consecutive
//! evaluations ≥ 30 s apart** before it is emitted; recovery on resumed
//! output is immediate (Req.5). No automatic action ever follows an
//! emission (SPEC §7) — the caller only records and shows.

use std::collections::HashMap;

use juggler_core::model::{AttentionReason, SessionId, SessionStatus};
use serde::{Deserialize, Serialize};

/// User-tunable thresholds (SPEC §5.1 "thresholds user-tunable").
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeuristicThresholds {
    /// No-output window that feeds the Stuck rule, seconds.
    pub stuck_no_output_secs: u64,
    /// CPU % below which the Stuck rule can fire.
    pub stuck_cpu_max_pct: f64,
    /// Transcript-mtime staleness that feeds the Stuck rule, seconds.
    pub stuck_transcript_stale_secs: u64,
    /// How long an unchanged prompt must sit before Needs-you fires.
    pub waiting_prompt_secs: u64,
}

impl Default for HeuristicThresholds {
    fn default() -> Self {
        Self {
            stuck_no_output_secs: 300,
            stuck_cpu_max_pct: 5.0,
            stuck_transcript_stale_secs: 300,
            waiting_prompt_secs: 120,
        }
    }
}

/// Debounce contract: consecutive evaluations at least this far apart.
pub const DEBOUNCE_GAP_SECS: u64 = 30;

/// Measured inputs for one Instance at evaluation time.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HeuristicInputs {
    /// When output last arrived (epoch ms); `None` when never.
    pub last_output_ms: Option<i64>,
    /// Current CPU % summed over the process tree; `None` unsampled.
    pub cpu_pct: Option<f64>,
    /// Transcript file mtime (epoch ms); `None` when there is no transcript.
    pub transcript_mtime_ms: Option<i64>,
    /// The output tail ends with an unterminated permission/wait prompt.
    pub prompt_alive: bool,
    /// Evaluation clock (epoch ms) — injected, never wall-read.
    pub now_ms: i64,
}

/// An emitted attention state plus its structured reason.
#[derive(Clone, Debug, PartialEq)]
pub struct AttentionTransition {
    /// The lifecycle status to record (`stuck`/`waiting_user`/`quota_blocked`).
    pub status: SessionStatus,
    /// Structured reason: rule id, measured inputs, since timestamp.
    pub reason: AttentionReason,
}

/// True when the output tail ends with an unterminated permission/wait
/// prompt. Prompt patterns are deliberately conservative substrings —
/// a false "healthy" is recoverable, a false accusation is not.
#[must_use]
pub fn prompt_alive(tail: &str) -> bool {
    let last = tail
        .lines()
        .rev()
        .map(str::trim_end)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    if last.is_empty() {
        return false;
    }
    let prompts: [&str; 8] = [
        "(y/n)",
        "[y/n]",
        "(yes/no)",
        "do you want to",
        "allow",
        "permission",
        "press enter",
        "waiting for input",
    ];
    let lower = last.to_lowercase();
    // A prompt that has already been answered ends with a choice marker.
    let answered =
        lower.ends_with("... yes") || lower.ends_with("... no") || lower.ends_with("(accepted)");
    !answered && prompts.iter().any(|pattern| lower.contains(pattern))
}

/// True when an output tail carries a vendor rate-limit signature.
/// Negative-evidence wording: these are *observed signatures*, not a
/// claim about any vendor's behaviour.
#[must_use]
pub fn quota_signature(tail: &str) -> bool {
    let signatures: [&str; 8] = [
        "rate limit",
        "rate_limit",
        "usage limit reached",
        "usage_limit_reached",
        "quota exceeded",
        "quota_exceeded",
        "you've reached your limit",
        "plan limit",
    ];
    let lower = tail.to_lowercase();
    signatures.iter().any(|signature| lower.contains(signature))
}

#[derive(Debug, Default, Clone)]
struct DebounceState {
    candidate: Option<(AttentionTransition, i64)>,
    confirmed_at: Option<i64>,
}

/// The heuristic engine: one instance per app, debounce state per session.
pub struct HeuristicEngine {
    thresholds: HeuristicThresholds,
    states: std::sync::Mutex<HashMap<SessionId, DebounceState>>,
}

impl HeuristicEngine {
    /// Creates an engine with the given (user-tunable) thresholds.
    #[must_use]
    pub fn new(thresholds: HeuristicThresholds) -> Self {
        Self {
            thresholds,
            states: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// The active thresholds (the caller persists/edits these).
    #[must_use]
    pub fn thresholds(&self) -> HeuristicThresholds {
        self.thresholds
    }

    fn stuck_rule(&self, inputs: &HeuristicInputs) -> bool {
        if inputs.prompt_alive {
            return false; // an alive prompt is WaitingUser territory —
            // accusing it of Stuck would erode badge trust
        }
        let Some(last_output) = inputs.last_output_ms else {
            return false; // never produced output — Starting, not Stuck
        };
        let no_output_secs = (inputs.now_ms - last_output) / 1000;
        if no_output_secs < self.thresholds.stuck_no_output_secs.cast_signed() {
            return false;
        }
        // CPU above the threshold means it is working, not stuck; an
        // unsampled CPU (None) does NOT block the rule — output silence
        // plus a stale transcript is the primary evidence.
        if let Some(cpu) = inputs.cpu_pct
            && cpu > self.thresholds.stuck_cpu_max_pct
        {
            return false; // busy CPU means working, not stuck
        }
        if let Some(mtime) = inputs.transcript_mtime_ms {
            let stale_secs = (inputs.now_ms - mtime) / 1000;
            if stale_secs < self.thresholds.stuck_transcript_stale_secs.cast_signed() {
                return false; // transcript keeps growing — it is working
            }
        }
        true
    }

    fn waiting_rule(&self, inputs: &HeuristicInputs) -> bool {
        if !inputs.prompt_alive {
            return false;
        }
        match inputs.last_output_ms {
            // The prompt itself was output; it must have sat unanswered.
            Some(last_output) => {
                (inputs.now_ms - last_output) / 1000
                    >= self.thresholds.waiting_prompt_secs.cast_signed()
            }
            None => false,
        }
    }

    /// One evaluation. Returns the transition to emit (if any): a newly
    /// confirmed heuristic state, or `Running` recovery when a formerly
    /// attentioned Instance is healthy again. Debounced per Req.4.
    pub fn evaluate(
        &self,
        session: &SessionId,
        inputs: &HeuristicInputs,
    ) -> Option<AttentionTransition> {
        use std::sync::MutexGuard;
        let mut states: MutexGuard<'_, HashMap<SessionId, DebounceState>> =
            self.states.lock().ok()?;
        let state = states.entry(session.clone()).or_default();

        let candidate: Option<AttentionTransition> = if self.stuck_rule(inputs) {
            Some(AttentionTransition {
                status: SessionStatus::Stuck,
                reason: AttentionReason {
                    rule: "stuck_no_output".into(),
                    inputs: vec![
                        format!(
                            "no output {}m",
                            (inputs.now_ms - inputs.last_output_ms.unwrap_or_default()) / 60_000
                        ),
                        format!(
                            "cpu {}%",
                            inputs
                                .cpu_pct
                                .map_or("unsampled".to_string(), |cpu| format!("{cpu:.1}"))
                        ),
                        format!(
                            "transcript stale {}",
                            inputs
                                .transcript_mtime_ms
                                .map_or("no transcript".into(), |mtime| {
                                    format!("{}m", (inputs.now_ms - mtime) / 60_000)
                                })
                        ),
                    ],
                    since_ms: inputs.last_output_ms.unwrap_or(inputs.now_ms),
                },
            })
        } else if self.waiting_rule(inputs) {
            Some(AttentionTransition {
                status: SessionStatus::WaitingUser,
                reason: AttentionReason {
                    rule: "waiting_prompt".into(),
                    inputs: vec![
                        "prompt alive".into(),
                        format!(
                            "no output {:.1}m",
                            (inputs.now_ms - inputs.last_output_ms.unwrap_or_default()) / 60_000
                        ),
                    ],
                    since_ms: inputs.last_output_ms.unwrap_or(inputs.now_ms),
                },
            })
        } else {
            None
        };

        let Some(transition) = candidate else {
            // Healthy inputs: clear the debounce ladder. (Recovery for a
            // previously confirmed state is event-driven via `recover`.)
            state.candidate = None;
            state.confirmed_at = None;
            return None;
        };
        let same_rule = state
            .candidate
            .as_ref()
            .is_some_and(|(prior, _)| prior.reason.rule == transition.reason.rule);
        let first_seen_at = if same_rule {
            state
                .candidate
                .as_ref()
                .map_or(inputs.now_ms, |(_, at)| *at)
        } else {
            inputs.now_ms
        };
        let gap_ok = inputs.now_ms - first_seen_at >= DEBOUNCE_GAP_SECS.cast_signed() * 1000;
        state.candidate = Some((transition.clone(), first_seen_at));
        if gap_ok && state.confirmed_at.is_none() {
            state.confirmed_at = Some(inputs.now_ms);
            Some(transition)
        } else {
            None
        }
    }

    /// Immediate recovery (Req.5): output resumed on a session currently
    /// in a heuristic state. The caller maps this to `Running` (or
    /// `Finished` when the process already exited) and emits.
    pub fn recover(&self, session: &SessionId) -> Option<SessionStatus> {
        let mut states = self.states.lock().ok()?;
        let state = states.get_mut(session)?;
        if state.confirmed_at.is_some() {
            state.candidate = None;
            state.confirmed_at = None;
            Some(SessionStatus::Running)
        } else {
            state.candidate = None;
            state.confirmed_at = None;
            None
        }
    }

    /// Immediate exit classification (Req.4 exception): non-zero exit +
    /// rate-limit signature → [`SessionStatus::QuotaBlocked`] with evidence.
    /// `resets_at_ms` rides along when a quota window is known.
    pub fn classify_exit(
        &self,
        exit_code: i32,
        tail: &str,
        now_ms: i64,
        resets_at_ms: Option<i64>,
    ) -> Option<AttentionTransition> {
        if exit_code == 0 || !quota_signature(tail) {
            return None;
        }
        let mut inputs = vec![
            format!("exit code {exit_code}"),
            "rate-limit signature in output".into(),
        ];
        if let Some(resets) = resets_at_ms {
            inputs.push(format!("resets in {}m", (resets - now_ms) / 60_000));
        }
        Some(AttentionTransition {
            status: SessionStatus::QuotaBlocked,
            reason: AttentionReason {
                rule: "quota_exit".into(),
                inputs,
                since_ms: now_ms,
            },
        })
    }

    /// Drops debounce state (session ended / re-adopted).
    pub fn forget(&self, session: &SessionId) {
        if let Ok(mut states) = self.states.lock() {
            states.remove(session);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_728_800_000_000;

    fn engine() -> HeuristicEngine {
        HeuristicEngine::new(HeuristicThresholds {
            stuck_no_output_secs: 300,
            stuck_cpu_max_pct: 5.0,
            stuck_transcript_stale_secs: 300,
            waiting_prompt_secs: 120,
        })
    }

    fn stuck_inputs(
        secs_since_output: i64,
        cpu: Option<f64>,
        transcript_stale: i64,
    ) -> HeuristicInputs {
        HeuristicInputs {
            last_output_ms: Some(T0 - secs_since_output * 1000),
            cpu_pct: cpu,
            transcript_mtime_ms: Some(T0 - transcript_stale * 1000),
            prompt_alive: false,
            now_ms: T0,
        }
    }

    #[test]
    fn stuck_requires_two_evals_30s_apart() {
        let engine = engine();
        let session = SessionId::new("s-stuck");
        let inputs = stuck_inputs(400, Some(0.3), 400);
        assert!(engine.evaluate(&session, &inputs).is_none());
        // Same rule, 25 s later: still debounced.
        assert!(
            engine
                .evaluate(
                    &session,
                    &HeuristicInputs {
                        now_ms: T0 + 25_000,
                        ..inputs
                    }
                )
                .is_none()
        );
        // 31 s later: confirmed.
        let transition = engine
            .evaluate(
                &session,
                &HeuristicInputs {
                    now_ms: T0 + 31_000,
                    ..inputs
                },
            )
            .expect("confirmed after gap");
        assert_eq!(transition.status, SessionStatus::Stuck);
        assert_eq!(transition.reason.rule, "stuck_no_output");
        assert!(
            transition
                .reason
                .inputs
                .iter()
                .any(|input| input.contains("no output"))
        );
        assert!(
            transition
                .reason
                .inputs
                .iter()
                .any(|input| input.contains("cpu 0.3%"))
        );
    }

    #[test]
    fn an_alive_prompt_is_waiting_never_stuck() {
        let engine = engine();
        let session = SessionId::new("s-prompt");
        let inputs = HeuristicInputs {
            prompt_alive: true,
            ..stuck_inputs(400, Some(0.1), 400)
        };
        // Silence that would trip Stuck, but the prompt is alive — the
        // Waiting rule owns it (checked in waiting_rule).
        assert!(engine.evaluate(&session, &inputs).is_none());
        assert!(
            engine
                .evaluate(
                    &session,
                    &HeuristicInputs {
                        now_ms: T0 + 31_000,
                        ..inputs
                    }
                )
                .is_some_and(|transition| transition.status == SessionStatus::WaitingUser)
        );
    }

    #[test]
    fn busy_cpu_never_stuck() {
        let engine = engine();
        let session = SessionId::new("s-busy");
        assert!(
            engine
                .evaluate(&session, &stuck_inputs(400, Some(87.0), 400))
                .is_none()
        );
    }

    #[test]
    fn fresh_transcript_blocks_stuck() {
        let engine = engine();
        let session = SessionId::new("s-writing");
        // Silent on stdout but the transcript keeps growing: working.
        assert!(
            engine
                .evaluate(&session, &stuck_inputs(400, Some(0.3), 20))
                .is_none()
        );
    }

    #[test]
    fn waiting_prompt_fires_after_window_and_debounce() {
        let engine = engine();
        let session = SessionId::new("s-wait");
        let inputs = HeuristicInputs {
            last_output_ms: Some(T0 - 150_000), // prompt printed 2.5m ago
            cpu_pct: Some(0.1),
            transcript_mtime_ms: Some(T0 - 150_000),
            prompt_alive: true,
            now_ms: T0,
        };
        assert!(engine.evaluate(&session, &inputs).is_none());
        let transition = engine
            .evaluate(
                &session,
                &HeuristicInputs {
                    now_ms: T0 + 31_000,
                    ..inputs
                },
            )
            .expect("waiting confirmed");
        assert_eq!(transition.status, SessionStatus::WaitingUser);
        assert_eq!(transition.reason.rule, "waiting_prompt");
    }

    #[test]
    fn output_resume_recovers_immediately() {
        let engine = engine();
        let session = SessionId::new("s-recover");
        let inputs = stuck_inputs(400, Some(0.3), 400);
        engine.evaluate(&session, &inputs);
        engine
            .evaluate(
                &session,
                &HeuristicInputs {
                    now_ms: T0 + 31_000,
                    ..inputs
                },
            )
            .expect("stuck confirmed");
        assert_eq!(engine.recover(&session), Some(SessionStatus::Running));
        // Idempotent: recovering a healthy session is a no-op.
        assert_eq!(engine.recover(&session), None);
    }

    #[test]
    fn quota_exit_classifies_immediately() {
        let engine = engine();
        let transition = engine
            .classify_exit(
                1,
                "Error: usage limit reached until 5pm",
                T0,
                Some(T0 + 3_600_000),
            )
            .expect("quota signature");
        assert_eq!(transition.status, SessionStatus::QuotaBlocked);
        assert_eq!(transition.reason.rule, "quota_exit");
        assert!(
            transition
                .reason
                .inputs
                .iter()
                .any(|input| input.contains("exit code 1"))
        );
        // Plain crash: not quota.
        assert!(engine.classify_exit(139, "segfault", T0, None).is_none());
        // Success exit: never attention.
        assert!(
            engine
                .classify_exit(0, "usage limit reached", T0, None)
                .is_none()
        );
    }

    #[test]
    fn prompt_patterns_answer_vs_alive() {
        assert!(prompt_alive("Do you want to proceed? (y/n)"));
        assert!(prompt_alive("Allow file write? [y/n]"));
        assert!(!prompt_alive("turn 3/10"));
        assert!(!prompt_alive("... yes"));
        assert!(!prompt_alive(""));
    }

    #[test]
    fn thresholds_are_tunable() {
        let mut thresholds = HeuristicThresholds::default();
        assert_eq!(thresholds.stuck_no_output_secs, 300);
        thresholds.stuck_no_output_secs = 30;
        thresholds.stuck_transcript_stale_secs = 30;
        let engine = HeuristicEngine::new(thresholds);
        let session = SessionId::new("s-fast");
        // 60 s of silence trips a 30 s window (still debounced once).
        let inputs = stuck_inputs(60, Some(0.2), 60);
        assert!(engine.evaluate(&session, &inputs).is_none());
        assert!(
            engine
                .evaluate(
                    &session,
                    &HeuristicInputs {
                        now_ms: T0 + 31_000,
                        ..inputs
                    }
                )
                .is_some()
        );
    }
}
