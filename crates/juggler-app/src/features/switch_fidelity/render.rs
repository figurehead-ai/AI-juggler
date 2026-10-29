//! A hop's report as Markdown: the table, the facts and the verdict.

use std::fmt::Write as _;

use super::{Carry, HopReport};

impl HopReport {
    /// The report as Markdown: a heading, the history's form, one row per
    /// measure (source count, carried count, %, notes), one row per
    /// planted fact, and the one-line verdict.
    #[must_use]
    pub fn markdown(&self) -> String {
        let mut out = format!("### {}: {}\n\n", self.name, self.route);
        let _ = writeln!(out, "History: {}\n", self.form);
        out.push_str("| Measure | Source | Carried | % | Notes |\n|---|---|---|---|---|\n");
        for row in &self.rows {
            let mark = match row.carry {
                Carry::Whole => "",
                Carry::ByDesign => " (by design)",
                Carry::Lost => " (lost)",
            };
            let _ = writeln!(
                out,
                "| {} | {} | {} | {}{mark} | {} |",
                row.measure,
                row.source,
                row.carried,
                percent(row.carried, row.source),
                cell(&row.notes)
            );
        }
        if !self.facts.is_empty() {
            out.push_str(
                "\n| Fact | Planted | How | In a file | In what the target got | Probe |\n\
                 |---|---|---|---|---|---|\n",
            );
            for row in &self.facts {
                let fact = &row.probed.fact;
                let _ = writeln!(
                    out,
                    "| {} | {} | {} | {} | {} | {}{} |",
                    fact.key,
                    cell(&fact.planted),
                    fact.kind.words(),
                    yes_no(row.probed.in_a_file),
                    yes_no(row.received),
                    row.probed.score.word(),
                    if row.in_source {
                        ""
                    } else {
                        " (not in the source)"
                    }
                );
            }
        }
        let _ = writeln!(out, "\n**Verdict.** {}", self.verdict);
        out
    }
}

fn percent(carried: usize, source: usize) -> String {
    (carried.min(source) * 100)
        .checked_div(source)
        .map_or_else(|| "—".to_owned(), |share| format!("{share}%"))
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

/// A table cell: no pipe or line break breaks the row.
fn cell(text: &str) -> String {
    text.replace('|', "\\|").replace(['\n', '\r'], " ")
}
