//! AJ-E808 — the provider-pair fidelity register: per ordered pair, one
//! cell per carried area, each `UNVALIDATED` (the default), `PASS` or
//! `FAIL`. The register is the truth behind "Validated": a pair's badge
//! is promoted only when every one of its cells is a PASS from a live
//! run on real binaries (`validation::PairCell::validated`).
//!
//! - A PASS records the run: both binaries' versions, the date, the
//!   fixture ID and the evidence path. Fixture and fake-harness runs
//!   never promote a cell (Req.3).
//! - A FAIL carries the user-facing drop note (Req.5).
//! - A Provider upgrade resets its PASS cells to UNVALIDATED by itself
//!   (Req.4): the effective verdict compares the cell's recorded
//!   versions with the installed ones.
//!
//! The data lives in `data/fidelity-register.json`;
//! `docs/qa/evidence/AJ-E808/register.md` is rendered from it.

use std::sync::OnceLock;

/// The carried areas every pair is validated on (Req.1).
pub const AREAS: [&str; 8] = [
    "transcript-resume",
    "rules-carry",
    "todos",
    "subagents",
    "file-history",
    "mcp-references",
    "cwd-worktree-edges",
    "switch-back",
];

/// A cell's verdict.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Verdict {
    /// Not run on real binaries for the installed versions (default).
    Unvalidated,
    /// Passed a live run.
    Pass,
    /// Failed a live run; the drop note says what doesn't carry.
    Fail,
}

impl Verdict {
    /// The register's word for it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unvalidated => "UNVALIDATED",
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
        }
    }
}

/// How a cell's result was produced.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunKind {
    /// Real installed Providers, a sacrificial project.
    Live,
    /// The version-tagged fixture corpus.
    Fixture,
    /// The fake harness.
    FakeHarness,
}

/// One area of one pair.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cell {
    /// One of [`AREAS`].
    pub area: String,
    /// The recorded verdict.
    pub verdict: Verdict,
    /// How the result was produced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<RunKind>,
    /// The source binary's version during the run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_version: Option<String>,
    /// The target binary's version during the run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_version: Option<String>,
    /// ISO date of the run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// The fixture or project-shape ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixture: Option<String>,
    /// Repository path of the evidence (log, recording).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
    /// What doesn't carry, in the words the product shows (FAIL).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_note: Option<String>,
}

/// One ordered pair.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pair {
    /// Source Provider.
    pub from: String,
    /// Target Provider.
    pub to: String,
    /// One cell per area.
    pub cells: Vec<Cell>,
}

/// The register.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Register {
    /// Format version.
    pub schema: u32,
    /// What the file is (kept with the data).
    pub note: String,
    /// Every ordered pair.
    pub pairs: Vec<Pair>,
}

/// The installed version of a Provider, when known.
pub type Versions<'a> = &'a dyn Fn(&str) -> Option<String>;

/// No version information (docs, tests, the static matrix).
#[must_use]
pub fn unknown_versions(_: &str) -> Option<String> {
    None
}

/// The register compiled into this build. A file that doesn't parse
/// reads as an empty register — nothing validates, the safe direction;
/// `the_committed_register_is_well_formed_and_covers_the_matrix` keeps
/// the committed file valid.
#[must_use]
pub fn embedded() -> &'static Register {
    static REGISTER: OnceLock<Register> = OnceLock::new();
    REGISTER.get_or_init(|| {
        Register::parse(include_str!("../data/fidelity-register.json")).unwrap_or_else(|_| {
            Register {
                schema: 1,
                note: String::new(),
                pairs: Vec::new(),
            }
        })
    })
}

impl Register {
    /// Parses a register.
    ///
    /// # Errors
    ///
    /// Returns the JSON error.
    pub fn parse(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }

    /// One pair's row.
    #[must_use]
    pub fn pair(&self, from: &str, to: &str) -> Option<&Pair> {
        self.pairs
            .iter()
            .find(|pair| pair.from == from && pair.to == to)
    }

    /// Every rule a well-formed register keeps; empty means valid.
    #[must_use]
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        for pair in &self.pairs {
            let id = format!("{}→{}", pair.from, pair.to);
            for area in AREAS {
                let count = pair.cells.iter().filter(|cell| cell.area == area).count();
                if count != 1 {
                    out.push(format!(
                        "{id}: area {area} appears {count} times (needs exactly 1)"
                    ));
                }
            }
            for cell in &pair.cells {
                let at = format!("{id} {}", cell.area);
                if !AREAS.contains(&cell.area.as_str()) {
                    out.push(format!("{at}: unknown area"));
                }
                match cell.verdict {
                    Verdict::Unvalidated => {}
                    Verdict::Pass | Verdict::Fail => {
                        if cell.run != Some(RunKind::Live) {
                            out.push(format!(
                                "{at}: a {} needs a live run on real binaries (fixture and fake-harness runs never decide a cell)",
                                cell.verdict.as_str()
                            ));
                        }
                        for (field, value) in [
                            ("source_version", &cell.source_version),
                            ("target_version", &cell.target_version),
                            ("date", &cell.date),
                            ("fixture", &cell.fixture),
                            ("evidence", &cell.evidence),
                        ] {
                            if value.as_deref().is_none_or(|v| v.trim().is_empty()) {
                                out.push(format!(
                                    "{at}: a {} needs {field}",
                                    cell.verdict.as_str()
                                ));
                            }
                        }
                        if cell.verdict == Verdict::Fail
                            && cell
                                .drop_note
                                .as_deref()
                                .is_none_or(|v| v.trim().is_empty())
                        {
                            out.push(format!("{at}: a FAIL needs the user-facing drop_note"));
                        }
                    }
                }
            }
        }
        out
    }

    /// A cell's verdict for the installed versions: a PASS or FAIL
    /// recorded against other versions than the installed ones reads
    /// UNVALIDATED (Req.4 — the upgrade resets it, nobody has to
    /// remember to). With `strict`, an unknown installed version can't
    /// confirm a PASS either.
    #[must_use]
    pub fn effective(
        cell: &Cell,
        from: &str,
        to: &str,
        installed: Versions<'_>,
        strict: bool,
    ) -> (Verdict, Option<String>) {
        if cell.verdict == Verdict::Unvalidated {
            return (Verdict::Unvalidated, None);
        }
        for (provider, recorded) in [(from, &cell.source_version), (to, &cell.target_version)] {
            let Some(recorded) = recorded else {
                return (Verdict::Unvalidated, Some("no recorded version".to_owned()));
            };
            match installed(provider) {
                Some(now) if &now != recorded => {
                    return (
                        Verdict::Unvalidated,
                        Some(format!(
                            "reset: {provider} is {now}, the run used {recorded}"
                        )),
                    );
                }
                None if strict => {
                    return (
                        Verdict::Unvalidated,
                        Some(format!("{provider}'s installed version is unknown")),
                    );
                }
                _ => {}
            }
        }
        (cell.verdict, None)
    }

    /// Every area of the pair passed for the installed versions — the
    /// register's half of "Validated".
    #[must_use]
    pub fn pair_validated(
        &self,
        from: &str,
        to: &str,
        installed: Versions<'_>,
        strict: bool,
    ) -> bool {
        self.pair(from, to).is_some_and(|pair| {
            AREAS.iter().all(|area| {
                pair.cells.iter().any(|cell| {
                    cell.area == *area
                        && Self::effective(cell, from, to, installed, strict).0 == Verdict::Pass
                })
            })
        })
    }

    /// Beta-exit (Req.5) for the pairs that ship a switch (pack-only
    /// pairs are F0 by definition): same-Provider pairs PASS on every
    /// cell; cross-Provider pairs PASS or FAIL with a drop note. Returns
    /// what is still open; empty means met.
    #[must_use]
    pub fn beta_exit_open(
        &self,
        installed: Versions<'_>,
        pack_only: &dyn Fn(&str, &str) -> bool,
    ) -> Vec<String> {
        let mut open = Vec::new();
        for pair in self
            .pairs
            .iter()
            .filter(|pair| !pack_only(&pair.from, &pair.to))
        {
            let same = pair.from == pair.to;
            for cell in &pair.cells {
                let (verdict, _) = Self::effective(cell, &pair.from, &pair.to, installed, false);
                let ok = match verdict {
                    Verdict::Pass => true,
                    Verdict::Fail => !same && cell.drop_note.is_some(),
                    Verdict::Unvalidated => false,
                };
                if !ok {
                    open.push(format!(
                        "{}→{} {}: {}",
                        pair.from,
                        pair.to,
                        cell.area,
                        verdict.as_str()
                    ));
                }
            }
        }
        open
    }
}

/// The register as the evidence document (`just fidelity-register`):
/// the matrix at a glance, each pair's product wording (the badge and
/// status the app shows, from the same code), every cell's provenance,
/// and what beta-exit still needs.
#[must_use]
pub fn render_markdown(register: &Register) -> String {
    use std::fmt::Write as _;
    let matrix = crate::validation::validation_matrix();
    let pack_only = |from: &str, to: &str| {
        matrix.iter().any(|row| {
            row.from == from
                && row.to == to
                && row.mechanism == crate::validation::PairMechanism::PackOnly
        })
    };
    let mut out = String::new();
    let _ = writeln!(out, "# AJ-E808 — provider-pair fidelity register\n");
    let _ = writeln!(
        out,
        "Rendered from `crates/juggler-handover/data/fidelity-register.json` by `just fidelity-register`;\n\
         a test fails when this file is out of date. UNVALIDATED is the default. A cell turns PASS\n\
         or FAIL only from a live run on real binaries with a sacrificial project: fixture and\n\
         fake-harness runs never decide a cell. A Provider upgrade resets its cells by itself: the\n\
         app compares each cell's recorded versions with the installed ones. A pair shows\n\
         \"Validated\" only when its CI row is green, a live cell passed, and every area below is PASS.\n"
    );
    let open = register.beta_exit_open(&unknown_versions, &pack_only);
    let _ = writeln!(
        out,
        "**Beta-exit (Req.5):** {} — same-Provider pairs PASS on every area; cross-Provider pairs PASS or FAIL with a drop note (pack-only pairs excluded: F0 by definition).\n",
        if open.is_empty() {
            "met".to_owned()
        } else {
            format!("{} cells open", open.len())
        }
    );
    let _ = writeln!(out, "## Matrix\n");
    render_matrix(&mut out, register, &matrix);
    render_decided(&mut out, register);
    out
}

/// The pairs × areas table, with the product's exact words per pair.
fn render_matrix(out: &mut String, register: &Register, matrix: &[crate::validation::PairCell]) {
    use std::fmt::Write as _;
    let _ = writeln!(
        out,
        "| Pair | Switch rows show | Settings › Beta shows | {} |",
        AREAS.join(" | ")
    );
    let _ = writeln!(
        out,
        "|---|---|---|{}|",
        AREAS.iter().map(|_| "---").collect::<Vec<_>>().join("|")
    );
    for pair in &register.pairs {
        // The exact words the UI renders (SwitchTargetRows, BetaMatrix).
        let row = matrix
            .iter()
            .find(|row| row.from == pair.from && row.to == pair.to);
        let validated = row.is_some_and(crate::validation::PairCell::validated);
        let badge = row.map_or_else(
            || "F0 · Beta".to_owned(),
            crate::validation::PairCell::badge,
        );
        let (switch_rows, settings) = if validated {
            (badge, "Validated ✓")
        } else {
            (
                format!("{badge} · Not yet validated"),
                "Beta — not yet validated",
            )
        };
        let cells: Vec<&str> = AREAS
            .iter()
            .map(|area| {
                pair.cells
                    .iter()
                    .find(|cell| cell.area == *area)
                    .map_or("—", |cell| cell.verdict.as_str())
            })
            .collect();
        let _ = writeln!(
            out,
            "| {}→{} | {switch_rows} | {settings} | {} |",
            pair.from,
            pair.to,
            cells.join(" | ")
        );
    }
}

/// Every decided cell with its provenance.
fn render_decided(out: &mut String, register: &Register) {
    use std::fmt::Write as _;
    let decided: Vec<(&Pair, &Cell)> = register
        .pairs
        .iter()
        .flat_map(|pair| pair.cells.iter().map(move |cell| (pair, cell)))
        .filter(|(_, cell)| cell.verdict != Verdict::Unvalidated)
        .collect();
    let _ = writeln!(out, "\n## Decided cells\n");
    if decided.is_empty() {
        let _ = writeln!(
            out,
            "None yet — no cell has run live. Every pair reads Beta in the product."
        );
    } else {
        let _ = writeln!(
            out,
            "| Pair | Area | Verdict | Versions (source → target) | Date | Fixture | Evidence | Drop note |"
        );
        let _ = writeln!(out, "|---|---|---|---|---|---|---|---|");
        for (pair, cell) in decided {
            let _ = writeln!(
                out,
                "| {}→{} | {} | {} | {} → {} | {} | {} | [{}]({}) | {} |",
                pair.from,
                pair.to,
                cell.area,
                cell.verdict.as_str(),
                cell.source_version.as_deref().unwrap_or("?"),
                cell.target_version.as_deref().unwrap_or("?"),
                cell.date.as_deref().unwrap_or("?"),
                cell.fixture.as_deref().unwrap_or("?"),
                cell.evidence.as_deref().unwrap_or("?"),
                cell.evidence
                    .as_deref()
                    .map_or_else(String::new, |path| format!("../../../../{path}")),
                cell.drop_note.as_deref().unwrap_or(""),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    fn live_pass(area: &str) -> Cell {
        Cell {
            area: area.to_owned(),
            verdict: Verdict::Pass,
            run: Some(RunKind::Live),
            source_version: Some("2.1.296".to_owned()),
            target_version: Some("2.1.296".to_owned()),
            date: Some("2026-10-19".to_owned()),
            fixture: Some("tiny-repo".to_owned()),
            evidence: Some("docs/qa/evidence/AJ-E808/cells/x.txt".to_owned()),
            drop_note: None,
        }
    }

    fn all_pass(from: &str, to: &str) -> Register {
        Register {
            schema: 1,
            note: String::new(),
            pairs: vec![Pair {
                from: from.to_owned(),
                to: to.to_owned(),
                cells: AREAS.iter().map(|area| live_pass(area)).collect(),
            }],
        }
    }

    #[test]
    fn the_committed_register_is_well_formed_and_covers_the_matrix() {
        let register = embedded();
        assert!(register.problems().is_empty(), "{:#?}", register.problems());
        for row in crate::validation::validation_matrix() {
            assert!(
                register.pair(&row.from, &row.to).is_some(),
                "{}→{} missing",
                row.from,
                row.to
            );
        }
        assert_eq!(
            register.pairs.len(),
            crate::validation::validation_matrix().len()
        );
    }

    #[test]
    fn unvalidated_is_the_default_and_nothing_validates_yet() {
        for pair in &embedded().pairs {
            assert!(!embedded().pair_validated(&pair.from, &pair.to, &unknown_versions, false));
        }
    }

    #[test]
    fn only_live_runs_with_full_provenance_decide_a_cell() {
        let mut register = all_pass("claude-code", "claude-code");
        assert!(register.problems().is_empty());
        register.pairs[0].cells[0].run = Some(RunKind::FakeHarness);
        register.pairs[0].cells[1].run = Some(RunKind::Fixture);
        register.pairs[0].cells[2].evidence = None;
        register.pairs[0].cells[3].verdict = Verdict::Fail; // no drop note
        let problems = register.problems().join("\n");
        assert!(
            problems.contains("transcript-resume: a PASS needs a live run"),
            "{problems}"
        );
        assert!(
            problems.contains("rules-carry: a PASS needs a live run"),
            "{problems}"
        );
        assert!(
            problems.contains("todos: a PASS needs evidence"),
            "{problems}"
        );
        assert!(
            problems.contains("subagents: a FAIL needs the user-facing drop_note"),
            "{problems}"
        );
    }

    #[test]
    fn a_provider_upgrade_resets_its_cells_by_itself() {
        let register = all_pass("claude-code", "claude-code");
        let same = |_: &str| Some("2.1.296".to_owned());
        let upgraded = |_: &str| Some("2.2.0".to_owned());
        assert!(register.pair_validated("claude-code", "claude-code", &same, true));
        assert!(!register.pair_validated("claude-code", "claude-code", &upgraded, true));
        let (verdict, why) = Register::effective(
            &register.pairs[0].cells[0],
            "claude-code",
            "claude-code",
            &upgraded,
            true,
        );
        assert_eq!(verdict, Verdict::Unvalidated);
        assert!(
            why.unwrap()
                .contains("reset: claude-code is 2.2.0, the run used 2.1.296")
        );
        // Strict: an unknown installed version can't confirm a PASS.
        assert!(!register.pair_validated("claude-code", "claude-code", &unknown_versions, true));
        assert!(register.pair_validated("claude-code", "claude-code", &unknown_versions, false));
    }

    #[test]
    fn the_register_document_is_current() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/qa/evidence/AJ-E808/register.md");
        let committed = std::fs::read_to_string(&path).unwrap_or_default();
        assert_eq!(
            committed,
            render_markdown(embedded()),
            "docs/qa/evidence/AJ-E808/register.md is out of date — run `just fidelity-register`"
        );
    }

    #[test]
    fn beta_exit_needs_same_provider_pass_and_cross_provider_drop_notes() {
        let not_pack_only = |_: &str, _: &str| false;
        let mut register = all_pass("claude-code", "codex");
        assert!(
            register
                .beta_exit_open(&unknown_versions, &not_pack_only)
                .is_empty()
        );
        register.pairs[0].cells[2].verdict = Verdict::Fail;
        register.pairs[0].cells[2].drop_note =
            Some("Your todo list stays on Claude Code.".to_owned());
        assert!(
            register
                .beta_exit_open(&unknown_versions, &not_pack_only)
                .is_empty(),
            "cross-provider FAIL with drop note is allowed"
        );
        let mut same = all_pass("claude-code", "claude-code");
        same.pairs[0].cells[2].verdict = Verdict::Fail;
        same.pairs[0].cells[2].drop_note = Some("x".to_owned());
        assert_eq!(
            same.beta_exit_open(&unknown_versions, &not_pack_only).len(),
            1,
            "same-provider must PASS"
        );
        let open = embedded().beta_exit_open(&unknown_versions, &|_, to| {
            to == "gemini-cli" || to == "opencode"
        });
        assert_eq!(
            open.len(),
            9 * AREAS.len(),
            "today every shipped cell is open"
        );
    }
}
