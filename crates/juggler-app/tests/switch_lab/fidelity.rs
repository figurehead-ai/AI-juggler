//! The lab's side of the measured switch fidelity
//! (`juggler_app::features::switch_fidelity`): reads each hop's source at
//! switch time and its target right after the switch through AI
//! Juggler's own readers, probes the planted facts, and keeps every
//! hop's table for the report.

use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use juggler_app::features::switch_fidelity::facts;
use juggler_app::features::switch_fidelity::facts::{Fact, Kind, Probed};
use juggler_app::features::switch_fidelity::{self as fidelity, Conversation, Hop, Received, Side};
use juggler_app::state::AppState;
use serde_json::{Value, json};

/// Where the source stands when a switch starts: its conversation as its
/// tool's reader sees it, and the work around it.
#[derive(Clone)]
pub struct Source {
    pub side: Side<Conversation>,
}

/// The source conversation `id` of `account` (found in the conversations
/// index), read now.
pub fn source(
    state: &Arc<AppState>,
    account: &str,
    id: &str,
    folder: &Path,
    label: &str,
) -> Option<Source> {
    let path = fidelity::conversation_path(state, account, id)?;
    source_at(state, account, id, &path, folder, label)
}

/// [`source`] when the transcript's path is already known.
pub fn source_at(
    state: &Arc<AppState>,
    account: &str,
    id: &str,
    path: &Path,
    folder: &Path,
    label: &str,
) -> Option<Source> {
    let history = Conversation::read(path).ok()?;
    let around = fidelity::surroundings(state, account, Some((path, id)), folder, true);
    Some(Source {
        side: Side {
            label: label.to_owned(),
            history,
            around,
        },
    })
}

/// A chain of hops as one source (switching back: every hop's
/// conversation since the work left the original).
pub fn chain(parts: Vec<Source>) -> Option<Source> {
    let mut parts = parts.into_iter();
    let first = parts.next()?;
    let mut side = first.side;
    for part in parts {
        side.history = side.history.then(part.side.history);
        side.label = part.side.label;
        side.around = part.side.around;
    }
    Some(Source { side })
}

/// The target of a transplant: its own copy of the conversation.
pub fn native(
    state: &Arc<AppState>,
    account: &str,
    copy: &Path,
    id: &str,
    folder: &Path,
    label: &str,
) -> Side<Received> {
    Side {
        label: label.to_owned(),
        history: Received::Native(Conversation::read(copy).unwrap_or_default()),
        around: fidelity::surroundings(state, account, Some((copy, id)), folder, false),
    }
}

/// The target of a replay: the handover file it is pointed at, and what
/// its writer says it holds (`state.json` beside it).
pub fn handover(
    state: &Arc<AppState>,
    account: &str,
    file: &Path,
    folder: &Path,
    label: &str,
) -> Side<Received> {
    let written: Value = file
        .parent()
        .and_then(|dir| std::fs::read_to_string(dir.join("state.json")).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    let count = |key: &str| {
        written[key]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .unwrap_or_default()
    };
    Side {
        label: label.to_owned(),
        history: Received::Handover {
            text: std::fs::read_to_string(file).unwrap_or_default(),
            included: count("turns_included"),
            left_out: count("turns_left_out"),
        },
        around: fidelity::surroundings(state, account, None, folder, false),
    }
}

/// The target of a switch back: the original conversation as it is
/// before the update, and the update turn.
pub fn back(
    state: &Arc<AppState>,
    account: &str,
    id: &str,
    folder: &Path,
    update: &str,
    label: &str,
) -> Side<Received> {
    let path = fidelity::conversation_path(state, account, id);
    let original = path
        .as_deref()
        .and_then(|path| Conversation::read(path).ok())
        .unwrap_or_default();
    Side {
        label: label.to_owned(),
        history: Received::Return {
            original,
            update: update.to_owned(),
        },
        around: fidelity::surroundings(
            state,
            account,
            path.as_deref().map(|path| (path, id)),
            folder,
            false,
        ),
    }
}

/// One planted fact.
pub fn fact(key: &str, ask: &str, expect: &[&str], planted: &str, kind: Kind) -> Fact {
    Fact {
        key: key.to_owned(),
        ask: ask.to_owned(),
        expect: expect.iter().map(|word| (*word).to_owned()).collect(),
        planted: planted.to_owned(),
        kind,
    }
}

/// Scores the target's `answer` for every fact, noting whether any file
/// in `project` holds it (any case).
pub fn probe(facts: &[Fact], answer: &str, project: &Path) -> Vec<Probed> {
    facts
        .iter()
        .map(|fact| {
            let in_a_file = !fact.expect.is_empty()
                && fact
                    .expect
                    .iter()
                    .all(|word| written_anywhere_any_case(project, word));
            facts::score(answer, fact, in_a_file)
        })
        .collect()
}

/// Measures one hop and keeps its table: the report's JSON and Markdown.
pub struct Book {
    pub hops: Vec<Value>,
    pub markdown: String,
    pub openings: Vec<String>,
}

impl Book {
    pub fn new(state: &Arc<AppState>) -> Self {
        Self {
            hops: Vec::new(),
            markdown: String::new(),
            openings: fidelity::replay_openings(state),
        }
    }

    /// Measures `source` → `target` and records it; returns the grade.
    pub fn hop(
        &mut self,
        name: &str,
        mechanism: &str,
        source: Source,
        target: Side<Received>,
        probed: Vec<Probed>,
    ) -> &'static str {
        let hop = Hop {
            name: name.to_owned(),
            mechanism: mechanism.to_owned(),
            source: source.side,
            target,
            facts: probed,
            openings: self.openings.clone(),
        };
        let report = fidelity::measure(&hop);
        self.markdown.push_str(&report.markdown());
        self.markdown.push('\n');
        self.hops.push(json!({
            "name": report.name, "route": report.route, "form": report.form,
            "grade": report.grade, "verdict": report.verdict, "rows": report.rows,
            "facts": report.facts,
            "source_folder": hop.source.around.folder, "target_folder": hop.target.around.folder,
        }));
        report.grade
    }

    /// A hop that couldn't be measured, and why.
    pub fn missing(&mut self, name: &str, why: &str) {
        self.markdown
            .push_str(&format!("### {name}\n\nNot measured: {why}.\n\n"));
        self.hops
            .push(json!({"name": name, "grade": "NOT MEASURED", "why": why}));
    }

    /// Writes the Markdown beside the JSON report (`<out>.fidelity.md`).
    pub fn write(&self, out: &Path) -> PathBuf {
        let path = out.with_extension("fidelity.md");
        std::fs::write(&path, &self.markdown).unwrap();
        path
    }
}

fn written_anywhere_any_case(project: &Path, word: &str) -> bool {
    let word = word.to_lowercase();
    let mut stack = vec![project.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.file_name().is_some_and(|n| n == ".git") {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if std::fs::read_to_string(&path)
                .is_ok_and(|text| text.to_lowercase().contains(&word))
            {
                return true;
            }
        }
    }
    false
}
