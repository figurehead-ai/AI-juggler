//! Smart destination ranking (AJ-E611, deep/09 §Q6): the explainable
//! account ranking used at `NewSession` / handover / reset-return —
//! SUGGEST-ONLY, never auto-switching. Hard filters eliminate (shown
//! greyed with their reason, never silently dropped); survivors score
//! `0.45·headroom + 0.20·affinity + 0.15·fidelity + 0.10·costFit +
//! 0.10·calm` with the weights as CONSTANTS in this one function.
//! Every ranked row carries its top-2 contributing factors in words —
//! no score without an explanation ships. Unknown headroom (an
//! unverified Provider, AJ-E605) ranks by the remaining inputs and
//! SAYS so.

use crate::model::Confidence;

/// The five weights (deep/09 line 454) — constants, unit-tested, never
/// tuned per call site.
pub const W_HEADROOM: f64 = 0.45;
/// Project-history affinity weight.
pub const W_AFFINITY: f64 = 0.20;
/// Switch-fidelity weight.
pub const W_FIDELITY: f64 = 0.15;
/// Projected-USD fit weight.
pub const W_COST_FIT: f64 = 0.10;
/// Active-load calm weight.
pub const W_CALM: f64 = 0.10;
/// Confidence discount for Estimated headroom (Measured truth wins
/// ties, deep/09 lines 456-457).
pub const ESTIMATED_HEADROOM_DISCOUNT: f64 = 0.85;

/// One ranking input row (the caller assembles it from storage +
/// adapters; the maths here stays pure).
#[derive(Clone, PartialEq, Debug, Default)]
#[allow(clippy::struct_excessive_bools)] // the bool set IS §Q6's literal hard filters + extension
pub struct Candidate {
    /// Account id.
    pub account: String,
    /// Hard filter: the login terminal is logged out
    /// (`claude_code.rs` login-state) — cannot receive work.
    pub logged_out: bool,
    /// Hard filter: wrong Provider for a restricted request.
    pub wrong_provider: bool,
    /// Hard filter: quota exhausted with no reset before need-by.
    pub exhausted_no_reset: bool,
    /// min(5h, weekly) headroom percent (0–100); `None` = unknown —
    /// ranked by the remaining inputs, and the row says so.
    pub headroom_pct: Option<f64>,
    /// The headroom reading's confidence (Estimated ⇒ discounted).
    pub headroom_confidence: Confidence,
    /// This project's historical account share (0–1) — the
    /// wrong-billing guard (deep/09 lines 458-461).
    pub affinity: f64,
    /// Switch-fidelity input (0–1); launch-neutral surfaces pass 0.5.
    pub fidelity: f64,
    /// Projected session USD (AJ-E609); `None` = unpriced ⇒ neutral.
    pub projected_usd: Option<f64>,
    /// Active sessions currently on the account.
    pub active_sessions: u32,
    /// The soft cap spreading load (`calm = 1 − active/soft_cap`).
    pub soft_cap: u32,
    /// AJ-E209 §Q6 extension: this target is a RETURNABLE original
    /// (an open excursion leads back to it) — earns the return-
    /// likelihood bonus when the source resets soon.
    pub returnable: bool,
    /// AJ-E209 §Q6 extension: the SOURCE account's minutes-to-
    /// exhaustion (`None` = unknown ⇒ no bonus).
    pub source_resets_min: Option<f64>,
}

/// The return-likelihood bonus (§Q6 extension, pass2/02 §7): a
/// small documented addition on top of the published formula —
/// never enough to outweigh a real factor gap, and hard-filtered
/// rows never reach scoring.
pub const RETURN_BONUS: f64 = 0.05;

/// One ranked (or filtered) output row.
#[derive(Clone, PartialEq, Debug)]
pub struct RankedRow {
    /// Account id.
    pub account: String,
    /// The score; `None` for hard-filtered rows (greyed, with reason).
    pub score: Option<f64>,
    /// The hard filter's reason, when filtered.
    pub filtered_reason: Option<String>,
    /// The top-2 contributing factors in words (every survivor has ≥2).
    pub top_factors: Vec<String>,
    /// All five named inputs for the "Why this order?" expander.
    pub factors: Vec<(String, f64)>,
    /// True when headroom was unknown (the copy says so).
    pub headroom_unknown: bool,
    /// The headroom confidence badge the row renders.
    pub confidence: Confidence,
    /// Whether this headroom is Measured (tie-break winner).
    pub measured_headroom: bool,
}

/// Ranks the candidates: hard filters first (greyed + reason), then
/// the weighted score, Estimated headroom discounted so Measured truth
/// wins ties. Output order: ranked descending, then filtered rows.
#[must_use]
#[allow(clippy::too_many_lines)] // one function, one set of weights — the law
pub fn rank(candidates: &[Candidate]) -> Vec<RankedRow> {
    // costFit normalisation needs the max priced candidate.
    let max_usd = candidates
        .iter()
        .filter_map(|candidate| candidate.projected_usd)
        .fold(0.0_f64, f64::max);
    let mut survivors: Vec<RankedRow> = Vec::new();
    let mut filtered: Vec<RankedRow> = Vec::new();

    for candidate in candidates {
        let reason = if candidate.wrong_provider {
            Some("wrong Provider for this request".to_owned())
        } else if candidate.logged_out {
            Some("logged out".to_owned())
        } else if candidate.exhausted_no_reset {
            Some("quota exhausted — no reset before you need it".to_owned())
        } else {
            None
        };
        if let Some(reason) = reason {
            filtered.push(RankedRow {
                account: candidate.account.clone(),
                score: None,
                filtered_reason: Some(reason),
                top_factors: Vec::new(),
                factors: Vec::new(),
                headroom_unknown: candidate.headroom_pct.is_none(),
                confidence: candidate.headroom_confidence,
                measured_headroom: candidate.headroom_confidence == Confidence::Measured,
            });
            continue;
        }

        let calm = if candidate.soft_cap == 0 {
            1.0
        } else {
            (1.0 - f64::from(candidate.active_sessions) / f64::from(candidate.soft_cap))
                .clamp(0.0, 1.0)
        };
        let cost_fit = match candidate.projected_usd {
            Some(usd) if max_usd > 0.0 => 1.0 - (usd / max_usd),
            _ => 0.5, // unpriced or all-equal ⇒ neutral
        };
        let headroom_unknown = candidate.headroom_pct.is_none();
        let (headroom_input, headroom_words) = match candidate.headroom_pct {
            Some(pct) => {
                let discounted = if candidate.headroom_confidence == Confidence::Estimated {
                    pct * ESTIMATED_HEADROOM_DISCOUNT
                } else {
                    pct
                };
                let words = format!(
                    "{}% headroom · {}",
                    pct.round(),
                    match candidate.headroom_confidence {
                        Confidence::Measured => "Measured",
                        Confidence::Computed => "Computed",
                        Confidence::Estimated => "Estimate",
                    }
                );
                (discounted.clamp(0.0, 100.0) / 100.0, words)
            }
            None => (0.0, "headroom unknown — ranked without quota".to_owned()),
        };
        let affinity = candidate.affinity.clamp(0.0, 1.0);
        let fidelity = candidate.fidelity.clamp(0.0, 1.0);

        let mut score = W_HEADROOM * headroom_input
            + W_AFFINITY * affinity
            + W_FIDELITY * fidelity
            + W_COST_FIT * cost_fit
            + W_CALM * calm;
        // AJ-E209 §Q6 extension: the return-likelihood bonus rides
        // AFTER the published formula (documented, capped, and only
        // for survivors — hard filters never reach scoring).
        let returnable_now =
            candidate.returnable && candidate.source_resets_min.is_some_and(|tte| tte < 60.0);
        if returnable_now {
            score += RETURN_BONUS;
        }

        let mut factors = vec![
            (headroom_words, W_HEADROOM * headroom_input),
            (
                if affinity >= 0.5 {
                    "usual account for this repo".to_owned()
                } else {
                    format!("{}% history here", (affinity * 100.0).round())
                },
                W_AFFINITY * affinity,
            ),
            (
                format!("fidelity {}", fidelity.round()),
                W_FIDELITY * fidelity,
            ),
            (
                match candidate.projected_usd {
                    Some(usd) => format!("≈${usd:.2} projected"),
                    None => "cost unpriced".to_owned(),
                },
                W_COST_FIT * cost_fit,
            ),
            (
                if candidate.active_sessions == 0 {
                    "quiet".to_owned()
                } else {
                    format!("{} active", candidate.active_sessions)
                },
                W_CALM * calm,
            ),
            (
                if returnable_now {
                    "returnable — your source resets soon".to_owned()
                } else {
                    "not a return target".to_owned()
                },
                if returnable_now { RETURN_BONUS } else { 0.0 },
            ),
        ];
        factors.sort_by(|(_, a), (_, b)| b.total_cmp(a));
        // Headroom-unknown rows keep their honest note in the top-2
        // words even at weight 0 (the copy is the contract, Req.3).
        let mut top_factors: Vec<String> = factors
            .iter()
            .take(2)
            .map(|(words, _)| words.clone())
            .collect();
        if headroom_unknown && !top_factors.iter().any(|words| words.contains("unknown")) {
            top_factors.push("headroom unknown — ranked without quota".to_owned());
        }
        survivors.push(RankedRow {
            account: candidate.account.clone(),
            score: Some(score),
            filtered_reason: None,
            top_factors,
            factors,
            headroom_unknown,
            confidence: candidate.headroom_confidence,
            measured_headroom: candidate.headroom_confidence == Confidence::Measured,
        });
    }

    // Measured truth wins ties: equal scores rank Measured-headroom
    // rows first (stable for the rest).
    survivors.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.measured_headroom.cmp(&a.measured_headroom))
    });
    survivors.extend(filtered);
    survivors
}
