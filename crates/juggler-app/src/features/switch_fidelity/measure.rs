//! The pure half: one hop's source against what its target received.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use super::facts::{FactRow, Score};
use super::{Carry, Conversation, Hop, HopReport, Received, Row, Turn};

/// How much of a turn's opening words, at least, shows that a shortened
/// copy of it arrived (a one-line summary, or a topic named in a list of
/// turns left out).
const OPENING_CHARS: usize = 40;

/// How one source turn arrived.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Arrived {
    Whole,
    Shortened,
    Missing,
}

/// Measures `hop`: every row of its table, its facts and its verdict.
#[must_use]
pub fn measure(hop: &Hop) -> HopReport {
    let source = &hop.source.history;
    let (turns, form) = match &hop.target.history {
        Received::Native(target) => native_turns(source, target),
        Received::Handover {
            text,
            included,
            left_out,
        } => carried_turns(
            source.turns.iter().collect(),
            text,
            &format!(
                "a handover file the target reads first: its writer quoted {included} turns and \
                 left out {left_out} (named by topic)"
            ),
        ),
        Received::Return { original, update } => {
            let new = new_turns(source, original, &hop.openings);
            carried_turns(
                new,
                update,
                &format!(
                    "the original conversation resumed as it was ({} turns of its own), plus one \
                     update turn",
                    original.turns.len()
                ),
            )
        }
    };
    let mut rows = vec![
        turn_row("User turns", &turns, true),
        turn_row("Assistant messages", &turns, false),
        tool_calls_row(&turns, &hop.target.history),
        tool_results_row(source, &hop.target.history),
        images_row(source, &hop.target.history),
        tasks_row(source, &hop.target.history),
        subagents_row(hop),
        rules_row(hop),
        mcp_row(hop),
        folder_row(hop),
        changes_row(hop),
    ];
    let facts = fact_rows(hop);
    let answered = facts
        .iter()
        .filter(|row| row.probed.score == Score::Answered)
        .count();
    if !facts.is_empty() {
        let wrong = facts
            .iter()
            .filter(|row| row.probed.score == Score::Wrong)
            .count();
        rows.push(Row {
            measure: "Planted facts answered",
            source: facts.len(),
            carried: answered,
            carry: if answered == facts.len() {
                Carry::Whole
            } else {
                Carry::Lost
            },
            notes: format!("{wrong} wrong, {} unknown", facts.len() - answered - wrong),
        });
    }
    let (grade, verdict) = verdict(&rows, &facts);
    HopReport {
        name: hop.name.clone(),
        route: format!(
            "{} → {} ({})",
            hop.source.label, hop.target.label, hop.mechanism
        ),
        form: form_line(&form, &turns),
        rows,
        facts,
        grade,
        verdict,
    }
}

/// A source turn and how it arrived.
type Measured<'a> = Vec<(&'a Turn, Arrived)>;

/// Whitespace collapsed, quoting (`> `) removed: how a turn is found in a
/// handover or an update, whatever their layout.
fn flat(text: &str) -> String {
    text.lines()
        .map(|line| line.trim_start().strip_prefix('>').unwrap_or(line))
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ")
}

/// A transplant: each source turn is in the target's own transcript, by
/// its id or, without one, by its words.
fn native_turns<'a>(source: &'a Conversation, target: &Conversation) -> (Measured<'a>, String) {
    let mut by_id: BTreeMap<&str, &Turn> = BTreeMap::new();
    for turn in &target.turns {
        if let Some(id) = &turn.uuid {
            by_id.insert(id, turn);
        }
    }
    let mut unclaimed: Vec<&Turn> = target.turns.iter().collect();
    let mut changed = 0usize;
    let measured = source
        .turns
        .iter()
        .map(|turn| {
            let same = |other: &Turn| {
                other.user == turn.user && other.text == turn.text && other.tools == turn.tools
            };
            if let Some(found) = turn.uuid.as_deref().and_then(|id| by_id.get(id)) {
                unclaimed.retain(|other| !std::ptr::eq(*other, *found));
                if same(found) {
                    return (turn, Arrived::Whole);
                }
                changed += 1;
                return (turn, Arrived::Shortened);
            }
            match unclaimed.iter().position(|other| same(other)) {
                Some(at) => {
                    unclaimed.remove(at);
                    (turn, Arrived::Whole)
                }
                None => (turn, Arrived::Missing),
            }
        })
        .collect();
    let mut form = format!(
        "the conversation itself, in the target tool's own transcript ({} turns there)",
        target.turns.len()
    );
    if changed > 0 {
        let _ = write!(form, "; {changed} turns changed on the way");
    }
    (measured, form)
}

/// A handover or an update: each turn is there whole (its words and
/// every tool line), shortened (its opening words), or missing.
fn carried_turns<'a>(turns: Vec<&'a Turn>, carrier: &str, form: &str) -> (Measured<'a>, String) {
    let carrier = flat(carrier);
    let measured = turns
        .into_iter()
        .map(|turn| {
            let words = flat(&turn.text);
            let whole = (words.is_empty() || carrier.contains(&words))
                && turn.tools.iter().all(|tool| carrier.contains(&flat(tool)));
            let opening: String = words.chars().take(OPENING_CHARS).collect();
            let arrived = if whole {
                Arrived::Whole
            } else if opening.chars().count() >= OPENING_CHARS.min(words.chars().count())
                && !opening.is_empty()
                && carrier.contains(&opening)
            {
                Arrived::Shortened
            } else {
                Arrived::Missing
            };
            (turn, arrived)
        })
        .collect();
    (measured, form.to_owned())
}

/// Switching back: the turns new on the away side (not in the original,
/// by id or words), without a replay's opening (the handover itself).
fn new_turns<'a>(
    away: &'a Conversation,
    original: &Conversation,
    openings: &[String],
) -> Vec<&'a Turn> {
    let ids: std::collections::BTreeSet<&str> = original
        .turns
        .iter()
        .filter_map(|turn| turn.uuid.as_deref())
        .collect();
    // By id when the turn has one: identical words (a second "ran the
    // tests") are new work all the same.
    away.turns
        .iter()
        .filter(|turn| match turn.uuid.as_deref() {
            Some(id) => !ids.contains(id),
            None => !original.turns.iter().any(|own| {
                own.user == turn.user && own.text == turn.text && own.tools == turn.tools
            }),
        })
        .filter(|turn| {
            !(turn.user
                && openings
                    .iter()
                    .any(|opening| turn.text.trim_start().starts_with(opening.trim())))
        })
        .collect()
}

fn count(turns: &Measured<'_>, user: bool, arrived: Arrived) -> usize {
    turns
        .iter()
        .filter(|(turn, how)| turn.user == user && *how == arrived)
        .count()
}

fn turn_row(measure: &'static str, turns: &Measured<'_>, user: bool) -> Row {
    let source = turns.iter().filter(|(turn, _)| turn.user == user).count();
    let whole = count(turns, user, Arrived::Whole);
    let shortened = count(turns, user, Arrived::Shortened);
    let missing = count(turns, user, Arrived::Missing);
    let carry = if whole == source {
        Carry::Whole
    } else if missing == 0 {
        Carry::ByDesign
    } else {
        Carry::Lost
    };
    Row {
        measure,
        source,
        carried: whole,
        carry,
        notes: format!("{whole} whole, {shortened} shortened, {missing} missing"),
    }
}

fn tool_calls_row(turns: &Measured<'_>, received: &Received) -> Row {
    let source: usize = turns.iter().map(|(turn, _)| turn.tools.len()).sum();
    match received {
        Received::Native(_) => {
            let carried: usize = turns
                .iter()
                .filter(|(_, how)| *how == Arrived::Whole)
                .map(|(turn, _)| turn.tools.len())
                .sum();
            Row {
                measure: "Tool calls",
                source,
                carried,
                carry: whole_or_lost(source, carried),
                notes: "as the calls themselves".to_owned(),
            }
        }
        Received::Handover { text, .. } | Received::Return { update: text, .. } => {
            let carrier = flat(text);
            let mut wanted: BTreeMap<String, usize> = BTreeMap::new();
            for (turn, _) in turns {
                for tool in &turn.tools {
                    *wanted.entry(flat(tool)).or_default() += 1;
                }
            }
            let found: usize = wanted
                .iter()
                .map(|(tool, times)| (*times).min(carrier.matches(tool.as_str()).count()))
                .sum();
            Row {
                measure: "Tool calls",
                source,
                carried: found,
                carry: match (source, found == source) {
                    (0, _) => Carry::Whole,
                    (_, true) => Carry::ByDesign,
                    (_, false) => Carry::Lost,
                },
                notes: "as one line each saying what it did".to_owned(),
            }
        }
    }
}

fn tool_results_row(source: &Conversation, received: &Received) -> Row {
    // Switching back, the away side began as the original's own turns:
    // only the results after them are new.
    let had = match received {
        Received::Return { original, .. } => source
            .content
            .tool_results
            .saturating_sub(original.content.tool_results),
        _ => source.content.tool_results,
    };
    match received {
        Received::Native(target) => {
            let carried = had.min(target.content.tool_results);
            Row {
                measure: "Tool results",
                source: had,
                carried,
                carry: whole_or_lost(had, carried),
                notes: format!(
                    "verbatim; {} were errors at the source",
                    source.content.tool_errors
                ),
            }
        }
        Received::Handover { .. } | Received::Return { .. } => Row {
            measure: "Tool results",
            source: had,
            carried: 0,
            carry: if had == 0 {
                Carry::Whole
            } else {
                Carry::ByDesign
            },
            notes: "summarised: each call's one line stands for its result".to_owned(),
        },
    }
}

fn images_row(source: &Conversation, received: &Received) -> Row {
    let had = source.content.images;
    let carried = match received {
        Received::Native(target) => had.min(target.content.images),
        _ => 0,
    };
    Row {
        measure: "Images and attachments",
        source: had,
        carried,
        carry: whole_or_lost(had, carried),
        notes: String::new(),
    }
}

fn tasks_row(source: &Conversation, received: &Received) -> Row {
    let items = source.tasks.clone().unwrap_or_default();
    let carried = items
        .iter()
        .filter(|item| match received {
            Received::Native(target) => target
                .tasks
                .as_ref()
                .is_some_and(|tasks| tasks.contains(item)),
            Received::Handover { text, .. } => text.contains(&item.rendered()),
            Received::Return { original, update } => {
                update.contains(&item.rendered())
                    || original
                        .tasks
                        .as_ref()
                        .is_some_and(|tasks| tasks.contains(item))
            }
        })
        .count();
    let states: Vec<String> = items.iter().map(super::Task::rendered).collect();
    Row {
        measure: "Task list items (with state)",
        source: items.len(),
        carried,
        carry: whole_or_lost(items.len(), carried),
        notes: if states.is_empty() {
            "no task list".to_owned()
        } else {
            states.join("; ")
        },
    }
}

fn subagents_row(hop: &Hop) -> Row {
    let had = &hop.source.around.subagents;
    let got = &hop.target.around.subagents;
    let carried = had.iter().filter(|id| got.contains(id)).count();
    let native = matches!(hop.target.history, Received::Native(_));
    Row {
        measure: "Subagents",
        source: had.len(),
        carried,
        carry: if carried == had.len() {
            Carry::Whole
        } else if native {
            Carry::Lost
        } else {
            Carry::ByDesign
        },
        notes: format!(
            "{} folder(s) at the source, {} at the target{}",
            hop.source.around.subagent_folders.len(),
            hop.target.around.subagent_folders.len(),
            if native || had.is_empty() {
                ""
            } else {
                "; another tool can't take them over (the switch lists them as not carried)"
            }
        ),
    }
}

fn rules_row(hop: &Hop) -> Row {
    let had = &hop.source.around.rules;
    let carried: Vec<&String> = had
        .iter()
        .filter(|name| {
            hop.target.around.rules.contains(name)
                || match &hop.target.history {
                    Received::Handover { text, .. } | Received::Return { update: text, .. } => {
                        text.contains(name.as_str())
                    }
                    Received::Native(_) => false,
                }
        })
        .collect();
    Row {
        measure: "Rules files in force",
        source: had.len(),
        carried: carried.len(),
        carry: whole_or_lost(had.len(), carried.len()),
        notes: format!(
            "source reads {}; target reads {} itself{}",
            list(had),
            list(&hop.target.around.rules),
            match (
                &hop.target.history,
                had.iter()
                    .all(|name| hop.target.around.rules.contains(name))
            ) {
                (_, true) | (Received::Native(_), _) => "",
                (Received::Handover { .. }, false) => ", the handover names the rest",
                (Received::Return { .. }, false) => ", the update names the rest",
            }
        ),
    }
}

fn mcp_row(hop: &Hop) -> Row {
    let had = &hop.source.around.mcp;
    let carried = had
        .iter()
        .filter(|name| hop.target.around.mcp.contains(name))
        .count();
    Row {
        measure: "MCP servers the conversation used",
        source: had.len(),
        carried,
        carry: whole_or_lost(had.len(), carried),
        notes: format!(
            "used: {}; set up on the target: {}",
            list(had),
            list(&hop.target.around.mcp)
        ),
    }
}

fn folder_row(hop: &Hop) -> Row {
    let same = hop.source.around.folder == hop.target.around.folder;
    Row {
        measure: "Working folder (worktree)",
        source: 1,
        carried: usize::from(same),
        carry: whole_or_lost(1, usize::from(same)),
        notes: if same {
            "the same folder".to_owned()
        } else {
            "a different folder".to_owned()
        },
    }
}

fn changes_row(hop: &Hop) -> Row {
    let (a, b) = (&hop.source.around, &hop.target.around);
    let same = a.diff_stat == b.diff_stat && a.changed == b.changed;
    Row {
        measure: "Uncommitted changes (git diff stat)",
        source: a.changed.len(),
        carried: if same { b.changed.len() } else { 0 },
        carry: if same { Carry::Whole } else { Carry::Lost },
        notes: match (&a.diff_stat, same) {
            (Some(stat), true) => format!("identical: {stat}"),
            (None, true) => "identical: nothing tracked changed".to_owned(),
            (_, false) => format!(
                "differs: {} vs {}",
                a.diff_stat.as_deref().unwrap_or("none"),
                b.diff_stat.as_deref().unwrap_or("none")
            ),
        },
    }
}

fn fact_rows(hop: &Hop) -> Vec<FactRow> {
    let source = hop.source.history.all_text().to_lowercase();
    let received = match &hop.target.history {
        Received::Native(target) => target.all_text(),
        Received::Handover { text, .. } => text.clone(),
        Received::Return { original, update } => format!("{}\n{update}", original.all_text()),
    }
    .to_lowercase();
    let holds = |text: &str, words: &[String]| {
        !words.is_empty() && words.iter().all(|word| text.contains(&word.to_lowercase()))
    };
    hop.facts
        .iter()
        .map(|probed| FactRow {
            in_source: holds(&source, &probed.fact.expect),
            received: holds(&received, &probed.fact.expect),
            probed: probed.clone(),
        })
        .collect()
}

fn whole_or_lost(source: usize, carried: usize) -> Carry {
    if carried >= source {
        Carry::Whole
    } else {
        Carry::Lost
    }
}

fn list(names: &[String]) -> String {
    if names.is_empty() {
        "none".to_owned()
    } else {
        names.join(", ")
    }
}

/// The history's form, with how many turns of each.
fn form_line(form: &str, turns: &Measured<'_>) -> String {
    let of = |how| turns.iter().filter(|(_, arrived)| *arrived == how).count();
    format!(
        "{form}. Of the source's {} turns: {} whole, {} shortened, {} missing.",
        turns.len(),
        of(Arrived::Whole),
        of(Arrived::Shortened),
        of(Arrived::Missing)
    )
}

/// PASS: every fact answered and nothing lost (a reduction the switch
/// names is not a loss); FAIL: a fact answered wrong, or the folder or
/// the uncommitted changes differ; PARTIAL otherwise.
fn verdict(rows: &[Row], facts: &[FactRow]) -> (&'static str, String) {
    let lost: Vec<&str> = rows
        .iter()
        .filter(|row| row.carry == Carry::Lost && row.measure != "Planted facts answered")
        .map(|row| row.measure)
        .collect();
    let reduced: Vec<&str> = rows
        .iter()
        .filter(|row| row.carry == Carry::ByDesign)
        .map(|row| row.measure)
        .collect();
    let answered = facts
        .iter()
        .filter(|row| row.probed.score == Score::Answered)
        .count();
    let wrong = facts.iter().any(|row| row.probed.score == Score::Wrong);
    let moved = lost
        .iter()
        .any(|measure| measure.starts_with("Working folder") || measure.starts_with("Uncommitted"));
    let grade = if wrong || moved {
        "FAIL"
    } else if lost.is_empty() && answered == facts.len() {
        "PASS"
    } else {
        "PARTIAL"
    };
    let mut parts = Vec::new();
    if lost.is_empty() {
        parts.push("nothing lost".to_owned());
    } else {
        parts.push(format!("lost: {}", lost.join(", ").to_lowercase()));
    }
    if !reduced.is_empty() {
        parts.push(format!(
            "reduced by design: {}",
            reduced.join(", ").to_lowercase()
        ));
    }
    if !facts.is_empty() {
        parts.push(format!("{answered} of {} facts answered", facts.len()));
    }
    (grade, format!("{grade}: {}.", parts.join("; ")))
}
