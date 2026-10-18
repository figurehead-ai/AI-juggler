//! AJ-E304 — the recommendation engine (pass2/03 §5): turns the
//! inferred inventory into ONE explained set-level proposal the user
//! accepts, customises, or ignores per item. Pure and UI-free (E4/E5
//! surfaces reuse it); the engine STAGES only — no account creation,
//! login, or file copy ever happens here (SPEC §26 automation
//! invariant: execution rides the accept click).

use std::collections::BTreeMap;

/// One scanned candidate (already inferred, AJ-E303).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecCandidate {
    /// Provider id.
    pub provider: String,
    /// Inferred purpose (the label BEFORE collision suffixing).
    pub purpose: String,
    /// Login observed.
    pub signed_in: bool,
    /// The S8 activity tier was dormant/archive.
    pub dormant: bool,
    /// Already managed by AI Juggler — NEVER recommended (property-pinned).
    pub managed: bool,
    /// The scanned config dir (rides through to the item).
    pub source_path: Option<String>,
}

/// One installed-but-unconfigured Provider (S6 positive, S1 empty).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecProviderGap {
    /// Provider id.
    pub provider: String,
    /// Display name ("Codex").
    pub display_name: String,
}

/// One recommended item: label + its ONE-LINE rationale (an item
/// without a rationale is unrepresentable — Req.8).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecommendedItem {
    /// Provider id.
    pub provider: String,
    /// Collision-suffixed label ("Work", "Work 2").
    pub label: String,
    /// `import` (a scanned dir) | `add` (a fresh account).
    pub action: String,
    /// The one-line rationale, shown verbatim.
    pub rationale: String,
    /// Default on in the accepted set (quiet items default off).
    pub default_on: bool,
    /// The scanned dir to import (None for fresh adds).
    pub path: Option<String>,
}

/// The set-level proposal.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Recommendation {
    /// The recommended items (signed-in/active first, quiet after).
    pub items: Vec<RecommendedItem>,
    /// The one-sentence summary ("Your juggling set: Claude ×3 (…) +
    /// Codex ×1 (…).").
    pub summary: String,
    /// The most-recent project suggestion ("Start here").
    pub start_here: Option<String>,
}

/// The engine (§5 rules 1–5, deterministic):
/// 1. active candidates default ON ("Signed in — import keeps history"
///    / "Active — you'll sign in after import");
/// 2. dormant candidates default OFF ("Dormant — include when you
///    want it");
/// 3. installed-but-unconfigured Providers → "Add {P} (installed,
///    not set up)" default ON (P = the Provider's display name);
/// 4. labels = inferred purpose, collisions suffixed in the ENGINE
///    ("Work", "Work 2") — never a post-typing validation error;
/// 5. the most-recent project rides along as `start_here`.
///
/// Managed candidates never appear (rule from §4.1).
#[must_use]
pub fn recommend(
    candidates: &[RecCandidate],
    gaps: &[RecProviderGap],
    start_here: Option<String>,
) -> Recommendation {
    let mut items: Vec<RecommendedItem> = Vec::new();
    let mut labels: BTreeMap<String, u32> = BTreeMap::new();
    let claim = |label: &str, labels: &mut BTreeMap<String, u32>| -> String {
        let seen = labels.entry(label.to_owned()).or_insert(0);
        *seen += 1;
        if *seen == 1 {
            label.to_owned()
        } else {
            format!("{label} {seen}")
        }
    };
    // Input order is deterministic (the scan's own order); no re-sort.
    let active: Vec<&RecCandidate> = candidates
        .iter()
        .filter(|candidate| !candidate.managed && !candidate.dormant)
        .collect();
    let quiet: Vec<&RecCandidate> = candidates
        .iter()
        .filter(|candidate| !candidate.managed && candidate.dormant)
        .collect();
    for candidate in active {
        items.push(RecommendedItem {
            provider: candidate.provider.clone(),
            label: claim(&candidate.purpose, &mut labels),
            action: "import".to_owned(),
            rationale: if candidate.signed_in {
                "Signed in — import keeps history".to_owned()
            } else {
                "Active — you'll sign in after import".to_owned()
            },
            default_on: true,
            path: candidate.source_path.clone(),
        });
    }
    for candidate in quiet {
        items.push(RecommendedItem {
            provider: candidate.provider.clone(),
            label: claim(&candidate.purpose, &mut labels),
            action: "import".to_owned(),
            rationale: "Dormant — include when you want it".to_owned(),
            default_on: false,
            path: candidate.source_path.clone(),
        });
    }
    for gap in gaps {
        items.push(RecommendedItem {
            provider: gap.provider.clone(),
            label: claim(&gap.display_name, &mut labels),
            action: "add".to_owned(),
            rationale: format!("Add {} (installed, not set up)", gap.display_name),
            default_on: true,
            path: None,
        });
    }
    Recommendation {
        summary: summary_sentence(&items),
        start_here,
        items,
    }
}

/// The §5 worked-example sentence: "Your juggling set: Claude ×3
/// (Work — signed in; Personal — signed in; Side — needs sign-in) +
/// Codex ×1 (signed in)." Built from the DEFAULT-ON items.
#[must_use]
pub fn summary_sentence(items: &[RecommendedItem]) -> String {
    let on: Vec<&RecommendedItem> = items.iter().filter(|item| item.default_on).collect();
    if on.is_empty() {
        return "Your juggling set: nothing to import yet — add an account to start.".to_owned();
    }
    let mut by_provider: BTreeMap<&str, Vec<&RecommendedItem>> = BTreeMap::new();
    for item in &on {
        by_provider
            .entry(item.provider.as_str())
            .or_default()
            .push(item);
    }
    let groups: Vec<String> = by_provider
        .iter()
        .map(|(provider, rows)| {
            let names = rows
                .iter()
                .map(|item| {
                    let state = if item.rationale.contains("keeps history") {
                        "signed in"
                    } else if item.rationale.contains("sign in after") {
                        "needs sign-in"
                    } else {
                        "signed in"
                    };
                    if item.action == "add" {
                        state.to_owned()
                    } else {
                        format!("{} — {}", item.label, state)
                    }
                })
                .collect::<Vec<_>>()
                .join("; ");
            format!("{} ×{} ({})", display_name(provider), rows.len(), names)
        })
        .collect();
    format!("Your juggling set: {}.", groups.join(" + "))
}

fn display_name(provider: &str) -> &str {
    match provider {
        "claude-code" => "Claude",
        "codex" => "Codex",
        "zcode" => "ZCode",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(provider: &str, purpose: &str, signed_in: bool, dormant: bool) -> RecCandidate {
        RecCandidate {
            provider: provider.to_owned(),
            purpose: purpose.to_owned(),
            signed_in,
            dormant,
            managed: false,
            source_path: Some(format!("/x/{purpose}")),
        }
    }

    /// The §5 worked example, verbatim parts.
    #[test]
    fn worked_example_sentence() {
        let candidates = vec![
            candidate("claude-code", "Work", true, false),
            candidate("claude-code", "Personal", true, false),
            candidate("claude-code", "Side", false, false),
        ];
        let gaps = vec![RecProviderGap {
            provider: "codex".to_owned(),
            display_name: "Codex".to_owned(),
        }];
        let recommendation = recommend(&candidates, &gaps, Some("~/Dev/api".into()));
        assert_eq!(
            recommendation.summary,
            "Your juggling set: Claude ×3 (Work — signed in; Personal — signed in; Side — needs sign-in) + Codex ×1 (signed in)."
        );
        assert_eq!(recommendation.start_here.as_deref(), Some("~/Dev/api"));
        assert_eq!(recommendation.items.len(), 4);
        assert!(
            recommendation
                .items
                .iter()
                .take(3)
                .all(|item| item.default_on)
        );
        assert_eq!(
            recommendation.items[0].path.as_deref(),
            Some("/x/Work"),
            "the source dir rides through for execution"
        );
        assert!(recommendation.items[3].path.is_none(), "adds have no dir");
    }

    /// Rule 2: dormant candidates default OFF under the quiet group.
    #[test]
    fn dormant_defaults_off() {
        let candidates = vec![
            candidate("claude-code", "Work", true, false),
            candidate("claude-code", "Old", false, true),
        ];
        let recommendation = recommend(&candidates, &[], None);
        let old = recommendation
            .items
            .iter()
            .find(|item| item.label == "Old")
            .unwrap();
        assert!(!old.default_on);
        assert_eq!(old.rationale, "Dormant — include when you want it");
        assert!(recommendation.summary.contains("Claude ×1"));
    }

    /// Rule 4: collision suffixing happens in the engine.
    #[test]
    fn collisions_suffixed_in_engine() {
        let candidates = vec![
            candidate("claude-code", "Work", true, false),
            candidate("codex", "Work", true, false),
            candidate("claude-code", "Work", true, false),
        ];
        let labels: Vec<String> = recommend(&candidates, &[], None)
            .items
            .iter()
            .map(|item| item.label.clone())
            .collect();
        assert_eq!(
            labels,
            vec!["Work".to_owned(), "Work 2".to_owned(), "Work 3".to_owned()]
        );
    }

    /// Property: no proposal ever contains a duplicate label or a
    /// managed candidate — over adversarial inputs.
    #[test]
    fn no_duplicates_no_managed_ever() {
        for round in 0..64u32 {
            let mut candidates = Vec::new();
            for index in 0..(round % 7) {
                candidates.push(candidate(
                    if index % 2 == 0 {
                        "claude-code"
                    } else {
                        "codex"
                    },
                    ["Work", "Personal", "Work"][(index % 3) as usize],
                    index % 2 == 0,
                    index % 3 == 0,
                ));
            }
            candidates.push(RecCandidate {
                provider: "claude-code".into(),
                purpose: "Managed".into(),
                signed_in: true,
                dormant: false,
                managed: true,
                source_path: None,
            });
            let recommendation = recommend(&candidates, &[], None);
            let mut labels: Vec<&str> = recommendation
                .items
                .iter()
                .map(|item| item.label.as_str())
                .collect();
            let total = labels.len();
            labels.sort_unstable();
            labels.dedup();
            assert_eq!(labels.len(), total, "round {round}: labels unique");
            assert!(
                recommendation
                    .items
                    .iter()
                    .all(|item| item.label != "Managed"),
                "round {round}: managed candidate never recommended"
            );
        }
    }

    /// Determinism: the same inventory yields a byte-identical
    /// proposal struct.
    #[test]
    fn deterministic_byte_identical() {
        let candidates = vec![
            candidate("claude-code", "Work", true, false),
            candidate("claude-code", "Personal", true, false),
        ];
        let one = serde_json::to_vec(&recommend(&candidates, &[], Some("p".into()))).unwrap();
        let two = serde_json::to_vec(&recommend(&candidates, &[], Some("p".into()))).unwrap();
        assert_eq!(one, two);
    }

    /// Budget: ≤200 candidates propose well inside the 100 ms cap
    /// (SPEC §26 staging; measured, not assumed).
    #[test]
    fn two_hundred_candidates_within_budget() {
        let purposes = ["Work", "Personal", "Side", "Lab", "Api"];
        let candidates: Vec<RecCandidate> = (0..200)
            .map(|index| {
                let purpose = purposes[index % purposes.len()].to_owned();
                RecCandidate {
                    provider: if index % 4 == 3 {
                        "codex"
                    } else {
                        "claude-code"
                    }
                    .to_owned(),
                    signed_in: index % 3 != 0,
                    dormant: index % 10 == 0,
                    managed: false,
                    source_path: Some(format!("/x/{purpose}/{index}")),
                    purpose,
                }
            })
            .collect();
        let gaps = vec![
            RecProviderGap {
                provider: "codex".to_owned(),
                display_name: "Codex".to_owned(),
            },
            RecProviderGap {
                provider: "gemini".to_owned(),
                display_name: "Gemini CLI".to_owned(),
            },
        ];
        let start = std::time::Instant::now();
        let recommendation = recommend(&candidates, &gaps, Some("~/Dev/api".into()));
        let elapsed = start.elapsed();
        assert_eq!(recommendation.items.len(), 202);
        assert!(
            elapsed.as_millis() < 100,
            "202 items proposed in {elapsed:?} — the 100 ms budget blew"
        );
    }
}
