//! Switch-aware ranking extensions (AJ-E209; deep/09 §Q6 wholesale +
//! pass2/02 §7): the §Q6 base lives in `juggler_core::ranking`
//! (hard filters eliminative + logged, then the five fixed weights);
//! THIS module supplies the switch-side inputs — the E203 ordered-
//! pair fidelity number, the priced switch cost (−replay tokens at
//! target rates), and the return-likelihood bonus (prefer returnable
//! targets when the source resets soon) — plus the S-F9 hop-cost
//! DISPLAY line (inform, never block, SPEC:237). Suggests only;
//! execution is always the user's click (D-07: no composite quality
//! score beyond the published formula).

use crate::fidelity::{self, FidelityLabel};

/// The numeric fidelity input for §Q6's 0.15 slot (F-level numeric,
/// F3 > F1): F0 = 0.0 … F4 = 1.0.
#[must_use]
pub fn fidelity_number(label: FidelityLabel) -> f64 {
    match label {
        FidelityLabel::F0 => 0.0,
        FidelityLabel::F1 => 0.25,
        FidelityLabel::F2 => 0.5,
        FidelityLabel::F3 => 0.75,
        FidelityLabel::F4 => 1.0,
    }
}

/// The ordered pair's fidelity number for one source→target
/// Provider combination (E203's matrix, honestly capped).
#[must_use]
pub fn pair_fidelity_number(from_provider: &str, to_provider: &str) -> f64 {
    fidelity_number(
        fidelity::score_pair(&fidelity::base_dispositions(from_provider, to_provider)).label,
    )
}

/// The pair's badge wire name for UI surfaces (never "lossless"
/// below F4 — the badge text is the label + Beta + validated state).
#[must_use]
pub fn pair_badge(from_provider: &str, to_provider: &str) -> String {
    format!(
        "{} · Beta",
        fidelity::score_pair(&fidelity::base_dispositions(from_provider, to_provider))
            .label
            .as_str()
    )
}

/// The priced switch cost for §Q6's costFit slot (pass2/02 §7: −
/// replay tokens priced at TARGET rates). A replay pays the full
/// measured replay floor; a transplant/native resume pays a nominal
/// hop (still honest — counted, never hidden). `None` = unpriced ⇒
/// neutral, exactly like an unpriced launch.
#[must_use]
pub fn switch_cost_usd(
    mechanism_replay: bool,
    source_replay_tokens: Option<u64>,
    target_rate_per_mtok: Option<f64>,
) -> Option<f64> {
    match (mechanism_replay, source_replay_tokens, target_rate_per_mtok) {
        (true, Some(tokens), Some(rate)) => Some(tokens_as_mtok(tokens, rate)),
        _ => None,
    }
}

fn tokens_as_mtok(tokens: u64, rate_per_mtok: f64) -> f64 {
    // Token counts are pricing inputs (cents of precision); the
    // u64→f64 rounding at 2^52 tokens is irrelevant and allowed.
    #[allow(clippy::cast_precision_loss)]
    {
        (tokens as f64 / 1_000_000.0) * rate_per_mtok
    }
}

/// The return-likelihood bonus (§Q6 extension): prefer returnable
/// targets when the SOURCE account resets soon (< 60 min) — a small
/// documented bonus on top of the published formula, never enough
/// to outweigh a hard filter (those never reach scoring).
pub const RETURN_BONUS: f64 = 0.05;

/// Whether the source resets "soon" for the bonus.
#[must_use]
pub fn source_resets_soon(source_tte_min: Option<f64>) -> bool {
    source_tte_min.is_some_and(|tte| tte < 60.0)
}

/// The S-F9 hop-cost display line ("3 switches so far ≈ N tokens") —
/// amber, inform never block; `None` under 2 hops (nothing to say).
#[must_use]
pub fn hop_note(hops: u32, cumulative_tokens: u64) -> Option<String> {
    if hops < 2 {
        return None;
    }
    let tokens = if cumulative_tokens >= 1_000_000 {
        format!("≈{}M", cumulative_tokens / 1_000_000)
    } else if cumulative_tokens >= 1_000 {
        format!("≈{}k", cumulative_tokens / 1_000)
    } else {
        format!("{cumulative_tokens}")
    };
    Some(format!("{hops} switches so far {tokens} tokens"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fidelity_numbers_order_the_labels() {
        // E203's honest cap: unvalidated pairs sit at/below F1 today
        // (D-04) — same-Provider never scores BELOW cross-Provider,
        // and a pack-only target floors at F0.
        assert!(
            pair_fidelity_number("claude-code", "claude-code")
                >= pair_fidelity_number("claude-code", "codex")
        );
        assert_eq!(pair_fidelity_number("claude-code", "unknown-cli"), 0.0);
        assert_eq!(fidelity_number(FidelityLabel::F4), 1.0);
        assert_eq!(fidelity_number(FidelityLabel::F0), 0.0);
    }

    #[test]
    fn badges_carry_beta_and_never_claim_lossless() {
        for pair in [
            ("claude-code", "claude-code"),
            ("claude-code", "codex"),
            ("zcode", "codex"),
        ] {
            let badge = pair_badge(pair.0, pair.1);
            assert!(badge.contains("Beta"), "{badge}");
            assert!(!badge.to_lowercase().contains("lossless"), "{badge}");
            assert!(!badge.contains("exactly"), "{badge}");
        }
    }

    #[test]
    fn replay_pays_the_measured_floor_transplants_stay_neutral() {
        // 2M tokens at $3/M ⇒ $6.
        let cost = switch_cost_usd(true, Some(2_000_000), Some(3.0));
        assert!((cost.unwrap() - 6.0).abs() < 1e-9);
        // Unpriced or non-replay ⇒ neutral.
        assert!(switch_cost_usd(false, Some(2_000_000), Some(3.0)).is_none());
        assert!(switch_cost_usd(true, Some(2_000_000), None).is_none());
    }

    #[test]
    fn hop_note_informs_only_and_stays_quiet_early() {
        assert_eq!(hop_note(1, 50_000), None);
        assert_eq!(
            hop_note(3, 41_000).as_deref(),
            Some("3 switches so far ≈41k tokens")
        );
        assert_eq!(
            hop_note(5, 2_500_000).as_deref(),
            Some("5 switches so far ≈2M tokens")
        );
    }

    #[test]
    fn return_bonus_applies_only_when_source_resets_soon() {
        assert!(source_resets_soon(Some(45.0)));
        assert!(!source_resets_soon(Some(90.0)));
        assert!(!source_resets_soon(None));
        assert!((RETURN_BONUS - 0.05).abs() < 1e-9);
    }
}
