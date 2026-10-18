//! AJ-E210: prints the E2 validation gate report (markdown) from the
//! ordered-pair matrix — the SAME table the UI badges derive from
//! (one source, no drift). Regenerate with:
//!
//! ```text
//! just e2-gate   # scripts/e2_gate_report.sh > docs/qa/gate-E2-validation.md
//! ```
//!
//! The report lists GREEN cells and remaining UNVALIDATED pairs as
//! open beta debt — never as passed (Req.6). Live dogfood cells are
//! recorded per pair by the maintainer (before/after sha256 +
//! continuity verdict); rows without one read PENDING.
use juggler_handover::validation::{DogfoodVerdict, validation_matrix};

fn main() {
    let rows = validation_matrix();
    let validated = rows.iter().filter(|row| row.validated()).count();
    println!("# E2 validation gate (D-04) — beta-exit per pair");
    println!();
    println!(
        "Generated from `juggler_handover::validation::validation_matrix()` — regenerate with \
         `just e2-gate`. {} of {} pairs validated; {} open beta debt.",
        validated,
        rows.len(),
        rows.len() - validated
    );
    println!();
    println!("A pair graduates out of \"Beta\" ONLY when its real-adapter CI row is green AND one");
    println!("live dogfood cell passed on real binaries. Fake-harness mechanism fixtures never");
    println!("promote a badge. A FAILED live cell lowers that pair's advertised ceiling one level");
    println!("until fixed and re-run.");
    println!();
    println!("| Pair | Mechanism | Researched | Advertised | CI row | Live cell | Status |");
    println!("|---|---|---|---|---|---|---|");
    for row in &rows {
        let mechanism = match row.mechanism {
            juggler_handover::validation::PairMechanism::Transplant => "transplant",
            juggler_handover::validation::PairMechanism::Replay => "replay",
            juggler_handover::validation::PairMechanism::PackOnly => "pack-only",
        };
        let live = match &row.dogfood {
            None => "PENDING".to_owned(),
            Some(cell) => match &cell.verdict {
                DogfoodVerdict::Pass { .. } => {
                    format!("pass {} ({})", cell.at, cell.project_shape)
                }
                DogfoodVerdict::Fail { reason } => {
                    format!("FAIL {} — {reason}", cell.at)
                }
            },
        };
        let notes = if row.notes.is_empty() {
            String::new()
        } else {
            format!(" {}", row.notes.join("; "))
        };
        println!(
            "| {}→{} | {mechanism} | {} | {} | {} | {live} | {}{notes} |",
            row.from,
            row.to,
            row.researched_ceiling.as_str(),
            row.advertised_ceiling().as_str(),
            if row.ci_green { "green" } else { "not landed" },
            row.status_words(),
        );
    }
    println!();
    println!("## Open beta debt (never listed as passed)");
    println!();
    for row in &rows {
        if !row.validated() {
            println!(
                "- {}→{} — {}{}",
                row.from,
                row.to,
                row.status_words(),
                if row.notes.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", row.notes.join("; "))
                }
            );
        }
    }
    println!();
    println!("## Failure-taxonomy evidence (S-F1..S-F9)");
    println!();
    println!(
        "S-F1..S-F9 copy + handling are exercised by the E205 pipeline, E206 door, E207 \
         switch-back, and E208 redaction suites; per-pair taxonomy counts accrue as live cells \
         run (recorded per cell, never aggregated)."
    );
}
