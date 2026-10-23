//! AJ-E210 — the ordered-pair validation matrix (D-04): the single
//! source of truth that graduates a pair out of "Beta" labelling.
//!
//! Three layers, never conflated:
//! 1. **`mechanism_green`** — the pair's SHAPE (its E203 disposition
//!    table) is proven end-to-end over the fake harness in CI
//!    (bundle → pack IR → install → rebuild). Green fixtures prove
//!    OUR mechanism; they NEVER promote a fidelity badge (the
//!    handover rule: stub/fake results never validate a pair).
//! 2. **`ci_green`** — both sides' REAL adapters have green fixture
//!    rows (version-tagged corpus). Today that is `claude-code` and
//!    `codex` (AJ-E1304: synthetic 0.163.0 rollouts through the real
//!    adapter and the app's switch); `ZCode` rows record
//!    `ci_green: false` — unproven, never silently skipped.
//! 3. **`dogfood`** — one live cell on real binaries, recorded by a
//!    human (before/after sha256 + continuity verdict). A FAILING
//!    cell lowers the advertised ceiling one level until fixed
//!    (Req.4); a PASSING cell validates the pair only when
//!    `ci_green` is also true.
//!
//! A pair advertises "F-label · Proven" only when `validated()`
//! is true; everything else reads "Beta — not proven yet"
//! (D-11: never "lossless").

use crate::fidelity::FidelityLabel;

/// The live-dogfood verdict for one pair cell (human-recorded).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum DogfoodVerdict {
    /// Target resumed and its next turn answered a question only
    /// answerable from pre-switch context (the §H8 bar).
    Pass {
        /// What the continuity check asked and got back.
        continuity_note: String,
    },
    /// The cell failed — the pair's advertised ceiling drops one
    /// level until fixed and re-run.
    Fail {
        /// The §9/S-F failure taxonomy id or free-form reason.
        reason: String,
    },
}

/// One recorded live dogfood cell (the matrix's third layer).
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct DogfoodCell {
    /// ISO date the cell ran.
    pub at: String,
    /// Project shape (tiny / monorepo / worktree / compacted-long /
    /// subagent-heavy).
    pub project_shape: String,
    /// sha256 of the source transcript BEFORE the switch.
    pub source_sha256: String,
    /// sha256 of the target transcript AFTER the resume.
    pub target_sha256: String,
    /// The verdict.
    pub verdict: DogfoodVerdict,
}

/// The switch mechanism a pair supports at its researched ceiling.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum PairMechanism {
    /// T2 transplant (bundle → install).
    Transplant,
    /// T3 replay (pack → re-feed).
    Replay,
    /// Pack-only (Gemini CLI / `OpenCode` / future — F0, SPEC:140).
    PackOnly,
}

/// One row of the ordered-pair validation matrix.
#[derive(Clone, PartialEq, Debug, serde::Serialize)]
pub struct PairCell {
    /// Source provider id.
    pub from: String,
    /// Target provider id.
    pub to: String,
    /// The researched ceiling (E203 label) BEFORE validation gating.
    pub researched_ceiling: FidelityLabel,
    /// The pair's switch mechanism at the ceiling.
    pub mechanism: PairMechanism,
    /// The pair's shape is proven over the fake harness in CI.
    pub mechanism_green: bool,
    /// Both sides' REAL adapters have green fixture rows.
    pub ci_green: bool,
    /// The recorded live dogfood cell, when one exists.
    pub dogfood: Option<DogfoodCell>,
    /// Honest notes (blocks, pending adapters) — surfaced verbatim.
    pub notes: Vec<String>,
}

impl PairCell {
    /// The advertised ceiling: the researched ceiling, lowered ONE
    /// level per FAILED dogfood cell until fixed (Req.4). A pending
    /// cell lowers nothing — absence of evidence is not failure.
    /// (A red CI row blocks the release at the gate itself: the
    /// `validation_matrix` test is that gate; it cannot ship.)
    #[must_use]
    pub fn advertised_ceiling(&self) -> FidelityLabel {
        let failed = self
            .dogfood
            .as_ref()
            .is_some_and(|cell| matches!(cell.verdict, DogfoodVerdict::Fail { .. }));
        if failed {
            self.researched_ceiling.lowered()
        } else {
            self.researched_ceiling
        }
    }

    /// Validated = real CI row green AND a PASSING live cell (Req.4)
    /// AND every area of the pair PASS in the AJ-E808 register. Fake-
    /// harness mechanism green never counts (handover rule). Without
    /// installed versions; the app uses [`Self::validated_for`].
    #[must_use]
    pub fn validated(&self) -> bool {
        self.validated_for(&crate::register::unknown_versions, false)
    }

    /// [`Self::validated`] for the installed Provider versions: a
    /// register PASS recorded against other versions reads UNVALIDATED
    /// (AJ-E808 Req.4); with `strict`, an unknown version can't confirm
    /// one either.
    #[must_use]
    pub fn validated_for(&self, installed: crate::register::Versions<'_>, strict: bool) -> bool {
        self.validated_against(crate::register::embedded(), installed, strict)
    }

    /// [`Self::validated_for`] against a given register.
    #[must_use]
    pub fn validated_against(
        &self,
        register: &crate::register::Register,
        installed: crate::register::Versions<'_>,
        strict: bool,
    ) -> bool {
        self.ci_green
            && self
                .dogfood
                .as_ref()
                .is_some_and(|cell| matches!(cell.verdict, DogfoodVerdict::Pass { .. }))
            && register.pair_validated(&self.from, &self.to, installed, strict)
    }

    /// The wire badge the UI renders ("Full story, fresh start · Beta" / "Same conversation, new account · Proven")
    /// — never "lossless" below F4 (D-11).
    #[must_use]
    pub fn badge(&self) -> String {
        self.badge_for(&crate::register::unknown_versions, false)
    }

    /// [`Self::badge`] for the installed Provider versions.
    #[must_use]
    pub fn badge_for(&self, installed: crate::register::Versions<'_>, strict: bool) -> String {
        format!(
            "{} · {}",
            self.advertised_ceiling().user_label(),
            if self.validated_for(installed, strict) {
                "Proven"
            } else {
                "Beta"
            }
        )
    }

    /// The one-line status for the Settings → Beta table.
    #[must_use]
    pub fn status_words(&self) -> String {
        match (&self.dogfood, self.ci_green) {
            (Some(cell), _) => match &cell.verdict {
                DogfoodVerdict::Pass { .. } if self.validated() => format!(
                    "Proven: tried by hand {} on a {} project",
                    cell.at, cell.project_shape
                ),
                DogfoodVerdict::Pass { .. } if self.ci_green => format!(
                    "Tried by hand {}, but not every part of the switch is proven yet, so it stays in beta",
                    cell.at
                ),
                DogfoodVerdict::Pass { .. } => format!(
                    "Tried by hand {}, but the automatic tests with the real tools don't pass yet, so it stays in beta",
                    cell.at
                ),
                DogfoodVerdict::Fail { reason } => {
                    format!(
                        "Failed when tried by hand {}: {reason}. It promises one level less until fixed",
                        cell.at
                    )
                }
            },
            (None, true) => {
                "Automatic tests pass with the real tools; not tried by hand yet".to_owned()
            }
            (None, false) => {
                "Automatic tests with the real tools aren't in place; not tried by hand yet"
                    .to_owned()
            }
        }
    }
}

/// Providers whose real adapters carry green fixture rows in CI.
const REAL_ADAPTERS: [&str; 2] = ["claude-code", "codex"];

/// The researched matrix (pass2/02 §10 pairs). `mechanism_green` and
/// `ci_green` are release facts asserted by the
/// `validation_matrix` test — this table and that test move together.
#[must_use]
pub fn validation_matrix() -> Vec<PairCell> {
    let mut rows = Vec::new();
    let same = |from: &str, to: &str, ceiling, notes| PairCell {
        from: from.to_owned(),
        to: to.to_owned(),
        researched_ceiling: ceiling,
        mechanism: PairMechanism::Transplant,
        mechanism_green: true, // fake-shape fixtures run for every row (the matrix test)
        ci_green: REAL_ADAPTERS.contains(&from) && REAL_ADAPTERS.contains(&to),
        dogfood: None, // live cells are RECORDED, never asserted here
        notes,
    };
    // A same-tool transplant resumes the same conversation on the new
    // account (AJ-E1540): F3, "Same conversation, new account", as the
    // switch screen says (F2 there when the session was compacted). The
    // beta words carry how sure that is (AJ-E1543).
    rows.push(same(
        "claude-code",
        "claude-code",
        FidelityLabel::F3,
        vec![],
    ));
    rows.push(same(
        "codex",
        "codex",
        FidelityLabel::F3,
        vec!["Tested on sample Codex 0.163.0 sessions only; resuming a copied long session is unproven until it's tried by hand".to_owned()],
    ));
    // Z Code's transplant is off (below): the row never promises a move
    // that can't run, so it stays at the replay's F1.
    rows.push(same(
        "zcode",
        "zcode",
        FidelityLabel::F1,
        vec![
            "Not available: Z Code keeps the chat in a database whose layout changes between versions, so moving it stays off until it works on two versions".to_owned(),
        ],
    ));
    for (from, to, ceiling, note) in [
        (
            "claude-code",
            "codex",
            FidelityLabel::F1,
            "Replays the story into a new Codex session; tested with stand-in tools",
        ),
        (
            "codex",
            "claude-code",
            FidelityLabel::F1,
            "Replays a Codex session's story; tested with stand-in tools",
        ),
        (
            "claude-code",
            "zcode",
            FidelityLabel::F0,
            "Z Code takes a briefing pack only, until starting it from a story is proven",
        ),
        (
            "zcode",
            "claude-code",
            FidelityLabel::F1,
            "Reading Z Code sessions isn't built yet",
        ),
        (
            "codex",
            "zcode",
            FidelityLabel::F0,
            "Z Code support isn't built yet; it takes a briefing pack only",
        ),
        (
            "zcode",
            "codex",
            FidelityLabel::F1,
            "Reading Z Code sessions isn't built yet",
        ),
    ] {
        rows.push(PairCell {
            from: from.to_owned(),
            to: to.to_owned(),
            researched_ceiling: ceiling,
            mechanism: PairMechanism::Replay,
            mechanism_green: true,
            ci_green: REAL_ADAPTERS.contains(&from) && REAL_ADAPTERS.contains(&to),
            dogfood: None,
            notes: vec![note.to_owned()],
        });
    }
    for target in ["gemini-cli", "opencode"] {
        rows.push(PairCell {
            from: "claude-code".to_owned(),
            to: target.to_owned(),
            researched_ceiling: FidelityLabel::F0,
            mechanism: PairMechanism::PackOnly,
            mechanism_green: true,
            ci_green: false,
            dogfood: None,
            notes: vec![
                "Briefing pack only: starting this tool from a story isn't planned for the beta"
                    .to_owned(),
            ],
        });
    }
    rows
}

/// One pair's cell from the researched matrix.
#[must_use]
pub fn pair_cell(from: &str, to: &str) -> Option<PairCell> {
    validation_matrix()
        .into_iter()
        .find(|row| row.from == from && row.to == to)
}

/// Merges a RECORDED dogfood cell into the researched row (the app
/// read side: static row + persisted cell). A recorded cell for an
/// unknown pair is refused (never silently invented).
#[must_use]
pub fn with_dogfood(mut row: PairCell, cell: DogfoodCell) -> PairCell {
    row.dogfood = Some(cell);
    row
}

/// The advertised badge for an ordered pair as the switch surfaces
/// render it (E209's `pair_badge`, gated by the matrix). Unknown
/// pairs degrade to the honest F0 Beta floor.
#[must_use]
pub fn advertised_badge(from: &str, to: &str, dogfood: Option<&DogfoodCell>) -> String {
    match pair_cell(from, to) {
        Some(row) => {
            let row = match dogfood {
                Some(cell) => with_dogfood(row, cell.clone()),
                None => row,
            };
            row.badge()
        }
        None => "Briefing pack · Beta".to_owned(),
    }
}

/// The advertised badge derived straight from the researched E203
/// table (no recorded cells) — the pure-computation path tests pin.
#[must_use]
pub fn researched_badge(from: &str, to: &str) -> String {
    match pair_cell(from, to) {
        Some(row) => row.badge(),
        None => "Briefing pack · Beta".to_owned(),
    }
}

/// The ceiling for an ordered pair after validation gating — used by
/// the ranking fidelity slot so a failed cell's lowered ceiling also
/// drops the score input (the label IS the score). Unknown pairs
/// floor at 0.0 (F0).
#[must_use]
pub fn advertised_ceiling_number(from: &str, to: &str, dogfood: Option<&DogfoodCell>) -> f64 {
    match pair_cell(from, to) {
        Some(row) => {
            let row = match dogfood {
                Some(cell) => with_dogfood(row, cell.clone()),
                None => row,
            };
            f64::from(row.advertised_ceiling().wire_index()) / 4.0
        }
        None => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fidelity::FidelityLabel as L;

    fn passing_cell() -> DogfoodCell {
        DogfoodCell {
            at: "2026-10-16".to_owned(),
            project_shape: "tiny".to_owned(),
            source_sha256: "aa".to_owned(),
            target_sha256: "bb".to_owned(),
            verdict: DogfoodVerdict::Pass {
                continuity_note: "asked for the pre-switch file name; answered".to_owned(),
            },
        }
    }

    #[test]
    fn matrix_enumerates_every_researched_pair_exactly_once() {
        let rows = validation_matrix();
        // 3 same-provider + 6 cross + 2 pack-only.
        assert_eq!(rows.len(), 11);
        let mut seen = std::collections::BTreeSet::new();
        for row in &rows {
            assert!(
                seen.insert((row.from.clone(), row.to.clone())),
                "no dup rows"
            );
        }
        for pair in [
            ("claude-code", "claude-code"),
            ("codex", "codex"),
            ("zcode", "zcode"),
            ("claude-code", "codex"),
            ("codex", "claude-code"),
            ("claude-code", "zcode"),
            ("zcode", "claude-code"),
            ("codex", "zcode"),
            ("zcode", "codex"),
            ("claude-code", "gemini-cli"),
            ("claude-code", "opencode"),
        ] {
            assert!(pair_cell(pair.0, pair.1).is_some(), "{pair:?} present");
        }
    }

    #[test]
    fn nothing_validates_without_a_live_cell_and_real_ci() {
        for row in validation_matrix() {
            assert!(!row.validated(), "{}→{} ships Beta", row.from, row.to);
            assert!(row.badge().contains("Beta"));
            assert!(!row.badge().to_lowercase().contains("lossless"));
        }
    }

    #[test]
    fn a_passing_cell_validates_only_real_ci_rows() {
        let claude = with_dogfood(
            pair_cell("claude-code", "claude-code").unwrap(),
            passing_cell(),
        );
        // AJ-E808: a passing live cell alone no longer promotes — every
        // area must PASS in the register, and today none is validated.
        assert!(!claude.validated());
        assert_eq!(claude.badge(), "Same conversation, new account · Beta");
        assert!(
            claude
                .status_words()
                .contains("not every part of the switch is proven"),
            "{}",
            claude.status_words()
        );
        // With every area PASS on the installed versions, it validates.
        let register = crate::register::Register {
            schema: 1,
            note: String::new(),
            pairs: vec![crate::register::Pair {
                from: "claude-code".to_owned(),
                to: "claude-code".to_owned(),
                cells: crate::register::AREAS
                    .iter()
                    .map(|area| crate::register::Cell {
                        area: (*area).to_owned(),
                        verdict: crate::register::Verdict::Pass,
                        run: Some(crate::register::RunKind::Live),
                        source_version: Some("2.1.296".to_owned()),
                        target_version: Some("2.1.296".to_owned()),
                        date: Some("2026-10-19".to_owned()),
                        fixture: Some("tiny-repo".to_owned()),
                        evidence: Some("docs/qa/evidence/AJ-E808/cells/x.txt".to_owned()),
                        drop_note: None,
                    })
                    .collect(),
            }],
        };
        let installed = |_: &str| Some("2.1.296".to_owned());
        assert!(claude.validated_against(&register, &installed, true));
        let upgraded = |_: &str| Some("2.2.0".to_owned());
        assert!(
            !claude.validated_against(&register, &upgraded, true),
            "an upgrade resets the badge"
        );

        let codex = with_dogfood(pair_cell("codex", "codex").unwrap(), passing_cell());
        assert!(
            !codex.validated(),
            "no pair validates while its register cells are unvalidated"
        );
        assert_eq!(codex.badge(), "Same conversation, new account · Beta");
        assert!(
            codex.status_words().contains("stays in beta"),
            "{}",
            codex.status_words()
        );
    }

    #[test]
    fn a_failing_cell_lowers_the_ceiling_one_level_until_fixed() {
        let mut failed = passing_cell();
        failed.verdict = DogfoodVerdict::Fail {
            reason: "S-F4: rules ring lost in translation".to_owned(),
        };
        let row = with_dogfood(pair_cell("claude-code", "claude-code").unwrap(), failed);
        assert_eq!(row.advertised_ceiling(), L::F2, "F3 lowered one level");
        assert!(!row.validated());
        assert_eq!(
            row.badge(),
            "Same conversation, new account (compacted) · Beta"
        );
        assert!(row.status_words().contains("promises one level less"));
    }

    #[test]
    fn pending_cells_and_zcode_blocks_read_honestly() {
        let rows = validation_matrix();
        let z2z = rows
            .iter()
            .find(|r| r.from == "zcode" && r.to == "zcode")
            .unwrap();
        assert!(z2z.notes.iter().any(|n| n.starts_with("Not available")));
        assert_eq!(z2z.advertised_ceiling(), L::F1, "blocked ≠ lowered");
        let claude_zcode = rows
            .iter()
            .find(|r| r.from == "claude-code" && r.to == "zcode")
            .unwrap();
        assert_eq!(claude_zcode.researched_ceiling, L::F0, "prime stays F0");
        let pending = rows.iter().find(|r| r.ci_green).unwrap();
        assert!(pending.status_words().contains("not tried by hand yet"));
    }

    #[test]
    fn advertised_badge_degrades_unknown_pairs_to_f0_beta() {
        assert_eq!(
            advertised_badge("claude-code", "unknown-cli", None),
            "Briefing pack · Beta"
        );
        assert_eq!(
            researched_badge("claude-code", "claude-code"),
            "Same conversation, new account · Beta"
        );
    }

    #[test]
    fn advertised_ceiling_number_never_exceeds_the_researched_score() {
        // claude→claude researches F3 (0.75, AJ-E1543); the number
        // matches and a failed cell drops it a level.
        let n = advertised_ceiling_number("claude-code", "claude-code", None);
        assert!((n - 0.75).abs() < 1e-9);
        let mut failed = passing_cell();
        failed.verdict = DogfoodVerdict::Fail {
            reason: "S-F6".to_owned(),
        };
        let lowered = advertised_ceiling_number("claude-code", "claude-code", Some(&failed));
        assert!((lowered - 0.5).abs() < 1e-9, "F3 → F2");
        assert_eq!(
            advertised_ceiling_number("claude-code", "unknown-cli", None),
            0.0
        );
    }
}
