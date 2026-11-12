//! ADR-0028 (`automation_handover`): a switch hands over what wakes the
//! conversation by itself, so the original never works on beside its copy
//! and nobody tidies up by hand.
//!
//! The adapter says what that is and how it changes
//! ([`juggler_adapter::automation::HostAutomation`]); this module decides
//! when, for every tool alike:
//! - **Before the switch** [`preview`] names it (the confirm screen), and
//!   what stays with the account, and whether it needs the switch to land
//!   in the desktop app (a terminal has no Auto-fix to take it).
//! - **At the switch** [`hand_over`] queues two changes, the copy taking
//!   it over and the original's turned off, and records the pair for
//!   switching back.
//! - **Queued changes** wait in AI Juggler's data folder
//!   (`automation/waiting.json`) and are applied by [`apply`]: one kept in
//!   an account's tool folder at once, through the tool's API; one kept in
//!   a desktop app profile's records only while that app is closed (it
//!   reads them when it starts and rewrites them while it runs). Closing
//!   is noticed by the watch every few seconds, by AI Juggler's own
//!   restart or Open of it, so a profile the user quits and starts again
//!   by hand gets them too.
//! - **Checked**: an app profile's change stays queued until the app has
//!   run with it for a while and it's still there; one the app wrote over
//!   (started in the moment it was applied) is applied again at its next
//!   close.
//! - **Switching back** ([`switch_back`]) queues the reverse: the original
//!   takes over the copy's automation as it is now and comes back out of
//!   the archive; the copy's is turned off.
//!
//! A change the tool refuses (a record of a shape it doesn't know) is
//! dropped and said once; nothing is guessed.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use juggler_adapter::automation::{Applied, AutomationHome, HostAutomation};
use juggler_core::model::{Account, HarnessId, TranscriptRef};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::state::AppState;

/// How long an app profile runs with a change before it counts as kept.
const SETTLED_MS: i64 = 30_000;
/// A change that can't be applied (its record never appears) is given up
/// after this long.
const GIVE_UP_MS: i64 = 7 * 24 * 3_600_000;

/// Where a tool keeps the automation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Home {
    /// A desktop app profile (a place id), changed while it's closed.
    Place {
        /// The place id.
        place: String,
    },
    /// An account's tool folder, changed at once.
    Account {
        /// The account's tool folder.
        dir: PathBuf,
    },
}

/// One change.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    /// The copies take over `carried` (`renamed`: original id → copy's).
    Adopt {
        /// What the originals had ([`HostAutomation::read`]).
        carried: Value,
        /// Original id → copy's, where they differ.
        renamed: Vec<(String, String)>,
        /// The switch's edge id (empty for a switch back's).
        #[serde(default)]
        edge: String,
    },
    /// The originals' automation is turned off; what undoes it is kept
    /// with the switch `edge`.
    Quiet {
        /// The originals' ids.
        conversations: Vec<String>,
        /// The switch's edge id.
        edge: String,
    },
    /// The originals come back as they were before `Quiet`.
    Wake {
        /// What [`HostAutomation::quiet`] returned.
        restore: Value,
    },
}

/// A queued change.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Edit {
    /// The tool, whose adapter makes the change.
    pub tool: String,
    /// Where.
    pub home: Home,
    /// What.
    pub op: Op,
    /// When it was queued.
    pub queued_ms: i64,
    /// When it was applied (an app profile's waits to be seen kept).
    #[serde(default)]
    pub applied_ms: Option<i64>,
}

/// A switch whose automation was handed over, for switching back.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Switched {
    /// The switch's edge id.
    pub edge: String,
    /// The tool.
    pub tool: String,
    /// The original's home.
    pub source: Home,
    /// The copy's home.
    pub target: Home,
    /// The originals' conversation ids.
    pub originals: Vec<String>,
    /// The copies' ids, in the same order.
    pub copies: Vec<String>,
    /// What undoes the original's quieting, once it's applied.
    #[serde(default)]
    pub restore: Option<Value>,
}

/// What the confirm screen says about it.
// Flattened into `Moving`'s TypeScript (AJ-E2019), so not exported itself.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct Preview {
    /// What moves to the copy and is turned off on the original.
    pub automation: Vec<String>,
    /// What stays with the account whatever the switch does.
    pub automation_stays: Vec<String>,
    /// Whether taking it along needs the switch to continue in the
    /// desktop app (the app keeps it; a terminal session can't).
    pub automation_needs_desktop: bool,
}

/// One lock for both files: the watch, a switch and a restart can come
/// together.
fn lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn folder(state: &AppState) -> PathBuf {
    state.data_dir.join("automation")
}

fn load<T: for<'de> Deserialize<'de>>(state: &AppState, name: &str) -> Vec<T> {
    std::fs::read(folder(state).join(name))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save<T: Serialize>(state: &AppState, name: &str, items: &[T]) {
    let dir = folder(state);
    let path = dir.join(name);
    if items.is_empty() {
        let _ = std::fs::remove_file(path);
        return;
    }
    let write = || -> std::io::Result<()> {
        std::fs::create_dir_all(&dir)?;
        let staged = path.with_extension("json.new");
        std::fs::write(
            &staged,
            serde_json::to_vec_pretty(items).map_err(std::io::Error::other)?,
        )?;
        std::fs::rename(staged, &path)
    };
    if let Err(error) = write() {
        tracing::warn!(target: "aij::flow", flow = "automation", step = "save", %error);
    }
}

/// The tool's adapter, when it hands automation over.
fn adapter(state: &AppState, tool: &str) -> Option<juggler_adapter::SharedAdapter> {
    state
        .registry
        .get(&HarnessId::new(tool.to_owned()))
        .filter(|adapter| adapter.host_automation().is_some())
}

/// The app profiles AI Juggler knows that work on `account_id`.
fn places_of(state: &Arc<AppState>, account_id: &str) -> Vec<(String, PathBuf)> {
    crate::features::places::with_book(state, |book| {
        book.places
            .iter()
            .filter(|(_, place)| place.account_id.as_deref() == Some(account_id))
            .filter_map(|(id, place)| place.instance_dir.clone().map(|dir| (id.clone(), dir)))
            .collect()
    })
    .unwrap_or_default()
}

/// Where the original's automation is, and what it is: each home that
/// keeps any for `conversations`.
fn sources(
    state: &Arc<AppState>,
    from: &Account,
    conversations: &[String],
) -> Vec<(Home, juggler_adapter::automation::Automation)> {
    let Some(adapter) = adapter(state, from.harness.as_str()) else {
        return Vec::new();
    };
    let Some(host) = adapter.host_automation() else {
        return Vec::new();
    };
    let homes: Vec<(Home, PathBuf)> = match host.home() {
        AutomationHome::Account => vec![(
            Home::Account {
                dir: from.state_dir.clone(),
            },
            from.state_dir.clone(),
        )],
        AutomationHome::AppProfile => places_of(state, from.id.as_str())
            .into_iter()
            .map(|(place, dir)| (Home::Place { place }, dir))
            .collect(),
    };
    homes
        .into_iter()
        .filter_map(|(home, dir)| {
            let found = host.read(&dir, conversations).ok()?;
            (found.any() || !found.stays.is_empty()).then_some((home, found))
        })
        .collect()
}

/// The conversation `reference` and the family that moves with it.
fn conversations(reference: &TranscriptRef, family: &[TranscriptRef]) -> Vec<String> {
    std::iter::once(reference)
        .chain(family)
        .filter_map(|r| {
            r.native_id.clone().or_else(|| {
                r.path
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
            })
        })
        .collect()
}

/// What the confirm screen says: what moves with the switch, what stays.
#[must_use]
pub fn preview(
    state: &Arc<AppState>,
    from: &Account,
    to: &Account,
    reference: &TranscriptRef,
    family: &[TranscriptRef],
) -> Preview {
    let found = sources(state, from, &conversations(reference, family));
    let mut preview = Preview::default();
    for (_, automation) in &found {
        if from.harness == to.harness {
            preview.automation.extend(automation.what.iter().cloned());
        } else {
            preview
                .automation_stays
                .extend(automation.what.iter().map(|what| {
                    format!(
                        "{what} stays with the original: {} doesn't take it",
                        crate::features::tools::display_name(&state.registry, to.harness.as_str())
                    )
                }));
        }
        preview
            .automation_stays
            .extend(automation.stays.iter().cloned());
    }
    preview.automation_needs_desktop = !preview.automation.is_empty()
        && adapter(state, from.harness.as_str()).is_some_and(|adapter| {
            adapter
                .host_automation()
                .is_some_and(|host| host.home() == AutomationHome::AppProfile)
        });
    preview
}

/// What [`hand_over`] did, for the done step.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HandedOver {
    /// What the copy takes over.
    pub moved: Vec<String>,
    /// The copy's app profile, when the copy takes it over only once that
    /// app restarts (the done step offers the restart).
    pub copy_waits: Option<String>,
    /// Whether the original's is turned off only once its app closes.
    pub original_waits: bool,
    /// What stays with the original, and why, in words.
    pub kept: Vec<String>,
}

/// After a switch from `from` to `to`: queues the copy taking over the
/// original's automation and the original's being turned off, and applies
/// what can be now. `target_place` is the app profile the switch
/// continued in (`None` in a terminal); `renamed` maps an original's id
/// to its copy's where they differ.
#[allow(clippy::too_many_arguments)]
pub fn hand_over(
    state: &Arc<AppState>,
    from: &Account,
    to: &Account,
    reference: &TranscriptRef,
    family: &[TranscriptRef],
    renamed: &[(String, String)],
    target_place: Option<&str>,
    edge: &str,
) -> HandedOver {
    let originals = conversations(reference, family);
    let found = sources(state, from, &originals);
    let mut done = HandedOver::default();
    let Some(adapter) = adapter(state, from.harness.as_str()) else {
        return done;
    };
    let Some(host) = adapter.host_automation() else {
        return done;
    };
    for (source, automation) in found {
        if !automation.any() {
            continue;
        }
        let target = match (host.home(), target_place, from.harness == to.harness) {
            (AutomationHome::Account, _, true) => Some(Home::Account {
                dir: to.state_dir.clone(),
            }),
            (AutomationHome::AppProfile, Some(place), true) => Some(Home::Place {
                place: place.to_owned(),
            }),
            // Another tool, or a terminal for what only the app runs.
            (_, _, false) | (AutomationHome::AppProfile, None, true) => None,
        };
        let Some(target) = target else {
            done.kept.extend(
                automation
                    .what
                    .iter()
                    .map(|what| format!("{what} stays with the original, which keeps it running")),
            );
            continue;
        };
        let copies: Vec<String> = originals
            .iter()
            .map(|id| juggler_adapter::automation::renamed_to(renamed, id).to_owned())
            .collect();
        let now = crate::commands::now_millis();
        {
            let Ok(_guard) = lock().lock() else {
                continue;
            };
            let mut edits: Vec<Edit> = load(state, "waiting.json");
            edits.push(Edit {
                tool: from.harness.to_string(),
                home: target.clone(),
                op: Op::Adopt {
                    carried: automation.carried.clone(),
                    renamed: renamed.to_vec(),
                    edge: edge.to_owned(),
                },
                queued_ms: now,
                applied_ms: None,
            });
            edits.push(Edit {
                tool: from.harness.to_string(),
                home: source.clone(),
                op: Op::Quiet {
                    conversations: originals.clone(),
                    edge: edge.to_owned(),
                },
                queued_ms: now,
                applied_ms: None,
            });
            save(state, "waiting.json", &edits);
            let mut switched: Vec<Switched> = load(state, "switched.json");
            switched.push(Switched {
                edge: edge.to_owned(),
                tool: from.harness.to_string(),
                source,
                target,
                originals: originals.clone(),
                copies,
                restore: None,
            });
            save(state, "switched.json", &switched);
        }
        done.moved.extend(automation.what);
    }
    if !done.moved.is_empty() {
        apply(state);
        let edits: Vec<Edit> = load(state, "waiting.json");
        done.original_waits = edits.iter().any(|edit| {
            matches!(&edit.op, Op::Quiet { edge: e, .. } if e == edge) && edit.applied_ms.is_none()
        });
        done.copy_waits = target_place
            .filter(|place| {
                edits.iter().any(|edit| {
                    matches!(&edit.op, Op::Adopt { .. })
                        && edit.applied_ms.is_none()
                        && edit.home
                            == Home::Place {
                                place: (*place).to_owned(),
                            }
                })
            })
            .map(str::to_owned);
    }
    tracing::info!(target: "aij::flow", flow = "automation", step = "handed_over", moved = done.moved.len(), kept = done.kept.len(), copy_waits = done.copy_waits.is_some(), original_waits = done.original_waits);
    done
}

/// After switching back over `edge`: the original takes over the copy's
/// automation as it is now and comes back out of the archive; the copy's
/// is turned off. Returns what moved back, in words.
pub fn switch_back(state: &Arc<AppState>, edge: &str) -> Vec<String> {
    let Ok(guard) = lock().lock() else {
        return Vec::new();
    };
    let mut switched: Vec<Switched> = load(state, "switched.json");
    let Some(at) = switched.iter().position(|s| s.edge == edge) else {
        return Vec::new();
    };
    let pair = switched.remove(at);
    save(state, "switched.json", &switched);
    let Some(adapter) = adapter(state, &pair.tool) else {
        return Vec::new();
    };
    let Some(host) = adapter.host_automation() else {
        return Vec::new();
    };
    let now = crate::commands::now_millis();
    let mut edits: Vec<Edit> = load(state, "waiting.json");
    // This switch's own changes leave the queue, applied or not: one not
    // applied yet has nothing to undo (the copy never takes it over, the
    // original was never put away), and one applied must not be checked
    // and applied again over the switch back.
    let pending_quiet = edits.iter().any(|edit| {
        matches!(&edit.op, Op::Quiet { edge: e, .. } if e == edge) && edit.applied_ms.is_none()
    });
    edits.retain(|edit| match &edit.op {
        Op::Quiet { edge: e, .. } | Op::Adopt { edge: e, .. } => e != edge,
        Op::Wake { .. } => true,
    });
    // The copy's automation as it is now (read where it can be).
    let copy_now = dir_of(state, &pair.target)
        .and_then(|dir| host.read(&dir, &pair.copies).ok())
        .unwrap_or_default();
    let back: Vec<(String, String)> = pair
        .copies
        .iter()
        .cloned()
        .zip(pair.originals.iter().cloned())
        .filter(|(copy, original)| copy != original)
        .collect();
    let tool = pair.tool.clone();
    if copy_now.any() {
        edits.push(Edit {
            tool: tool.clone(),
            home: pair.source.clone(),
            op: Op::Adopt {
                carried: copy_now.carried.clone(),
                renamed: back,
                edge: String::new(),
            },
            queued_ms: now,
            applied_ms: None,
        });
        edits.push(Edit {
            tool: tool.clone(),
            home: pair.target.clone(),
            op: Op::Quiet {
                conversations: pair.copies.clone(),
                edge: format!("{edge}:back"),
            },
            queued_ms: now,
            applied_ms: None,
        });
    }
    if let Some(restore) = pair.restore.clone().filter(|_| !pending_quiet) {
        edits.push(Edit {
            tool,
            home: pair.source.clone(),
            op: Op::Wake { restore },
            queued_ms: now,
            applied_ms: None,
        });
    }
    save(state, "waiting.json", &edits);
    drop(guard);
    apply(state);
    copy_now.what
}

fn dir_of(state: &Arc<AppState>, home: &Home) -> Option<PathBuf> {
    match home {
        Home::Account { dir } => Some(dir.clone()),
        Home::Place { place } => crate::features::places::instance_dir_of(state, place),
    }
}

/// Whether `edit`'s change is still in place (an app profile could have
/// written over it in the moment it started).
fn kept(host: &dyn HostAutomation, dir: &std::path::Path, edit: &Edit) -> bool {
    match &edit.op {
        Op::Quiet { conversations, .. } => host
            .read(dir, conversations)
            .is_ok_and(|found| !found.any()),
        Op::Adopt {
            carried, renamed, ..
        } => {
            let copies: Vec<String> = carried
                .as_object()
                .into_iter()
                .flatten()
                .map(|(id, _)| juggler_adapter::automation::renamed_to(renamed, id).to_owned())
                .collect();
            host.read(dir, &copies).is_ok_and(|found| found.any())
        }
        Op::Wake { .. } => true,
    }
}

/// The place ids with a change waiting (for the watch).
#[must_use]
pub fn waiting_places(state: &Arc<AppState>) -> Vec<String> {
    let edits: Vec<Edit> = load(state, "waiting.json");
    let mut places: Vec<String> = edits
        .into_iter()
        .filter_map(|edit| match edit.home {
            Home::Place { place } => Some(place),
            Home::Account { .. } => None,
        })
        .collect();
    places.sort();
    places.dedup();
    places
}

/// Applies every change that can be now: an account's at once, an app
/// profile's while it's closed; checks applied ones. `closed` names a
/// place the caller has just quit (or is about to start), so it isn't
/// looked up again.
pub fn apply_with(state: &Arc<AppState>, closed: Option<&str>) {
    let Ok(_guard) = lock().lock() else {
        return;
    };
    let edits: Vec<Edit> = load(state, "waiting.json");
    if edits.is_empty() {
        return;
    }
    let now = crate::commands::now_millis();
    let mut running: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    let mut is_running = |place: &str| -> bool {
        if closed == Some(place) {
            return false;
        }
        *running
            .entry(place.to_owned())
            .or_insert_with(|| crate::features::places::running_now(state, place).is_some())
    };
    let mut switched: Vec<Switched> = load(state, "switched.json");
    let mut left: Vec<Edit> = Vec::new();
    for mut edit in edits {
        let Some(adapter) = adapter(state, &edit.tool) else {
            continue;
        };
        let Some(host) = adapter.host_automation() else {
            continue;
        };
        let Some(dir) = dir_of(state, &edit.home) else {
            continue;
        };
        let place = match &edit.home {
            Home::Place { place } => Some(place.clone()),
            Home::Account { .. } => None,
        };
        let up = place.as_deref().is_some_and(&mut is_running);
        if let Some(applied) = edit.applied_ms {
            // Applied while closed: kept once the app has run with it.
            if up {
                if now.saturating_sub(applied) < SETTLED_MS {
                    left.push(edit);
                } else if !kept(host, &dir, &edit) {
                    tracing::info!(target: "aij::flow", flow = "automation", step = "reapply");
                    edit.applied_ms = None;
                    left.push(edit);
                }
            } else {
                left.push(edit);
            }
            continue;
        }
        if up {
            left.push(edit);
            continue;
        }
        let result = match &edit.op {
            Op::Adopt {
                carried, renamed, ..
            } => host.adopt(&dir, carried, renamed),
            Op::Quiet {
                conversations,
                edge,
            } => host.quiet(&dir, conversations).map(|restore| {
                if let Some(pair) = switched.iter_mut().find(|s| &s.edge == edge) {
                    pair.restore = Some(restore);
                }
                Applied::Done
            }),
            Op::Wake { restore } => host.wake(&dir, restore),
        };
        match result {
            Ok(Applied::Done) if place.is_some() => {
                edit.applied_ms = Some(now);
                left.push(edit);
            }
            Ok(Applied::Done) => {}
            Ok(Applied::NotYet) if now.saturating_sub(edit.queued_ms) < GIVE_UP_MS => {
                left.push(edit);
            }
            Ok(Applied::NotYet) => {
                tracing::warn!(target: "aij::flow", flow = "automation", step = "gave_up");
            }
            Err(error) => {
                tracing::warn!(target: "aij::flow", flow = "automation", step = "refused", %error);
            }
        }
    }
    save(state, "waiting.json", &left);
    save(state, "switched.json", &switched);
}

/// [`apply_with`], nothing known closed.
pub fn apply(state: &Arc<AppState>) {
    apply_with(state, None);
}

/// Everything waiting for app profile `place`, which mustn't be running
/// (the caller has just quit it or is about to start it): its family's
/// records first, then the automation changes. Returns the family's words.
pub fn apply_for_place(
    state: &Arc<AppState>,
    place: &str,
    dir: &std::path::Path,
) -> Option<String> {
    let said = super::family::place_waiting(state, place, dir);
    apply_with(state, Some(place));
    said
}

/// The watch: every few seconds, places a waiting family and applies
/// waiting changes for app profiles that are closed. Cheap when nothing
/// waits (two small files read).
pub fn spawn(state: Arc<AppState>) {
    crate::ticks::every("automation", std::time::Duration::from_secs(5), move || {
        // ADR-0031: a move changes profiles' records while they're closed;
        // nothing else writes them meanwhile.
        if crate::features::places::any_move(&state) {
            return;
        }
        for place in super::family::waiting_places(&state) {
            if crate::features::places::running_now(&state, &place).is_none()
                && let Some(dir) = crate::features::places::instance_dir_of(&state, &place)
            {
                super::family::place_waiting(&state, &place, &dir);
            }
        }
        apply(&state);
    });
}

#[cfg(test)]
mod tests {
    use super::{Edit, Home, Op};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use crate::state::AppState;
    use juggler_core::model::{Account, AccountId, HarnessId, TranscriptRef};
    use serde_json::{Value, json};

    #[test]
    fn a_waiting_change_round_trips() {
        let edit = Edit {
            tool: "claude-code".into(),
            home: Home::Place {
                place: "claude:/p".into(),
            },
            op: Op::Quiet {
                conversations: vec!["P".into()],
                edge: "e1".into(),
            },
            queued_ms: 1,
            applied_ms: Some(2),
        };
        let text = serde_json::to_string(&vec![edit.clone()]).unwrap();
        let back: Vec<Edit> = serde_json::from_str(&text).unwrap();
        assert_eq!(back, vec![edit]);
        assert!(text.contains(r#""kind":"place""#) && text.contains(r#""op":"quiet""#));
    }

    /// A Claude app profile under `root` named `name`, on `account`.
    fn profile(state: &Arc<AppState>, root: &Path, name: &str, account: &str) -> (String, PathBuf) {
        let (id, _, dir) =
            crate::features::places::remember_test_profile(state, root, name, Some(account));
        std::fs::create_dir_all(dir.join("claude-code-sessions/acct/org")).unwrap();
        (id, dir)
    }

    fn account(id: &str, dir: &Path) -> Account {
        Account {
            id: AccountId::new(id),
            harness: HarnessId::new("claude-code"),
            label: id.into(),
            plan: None,
            state_dir: dir.to_path_buf(),
            cleanup_opt_in: false,
            renewal_date: None,
            preview_opt_in: false,
            import_source_path: None,
            import_origin: None,
            imported_at: None,
            in_place: false,
        }
    }

    fn record(dir: &Path, local: &str, body: &Value) -> PathBuf {
        let path = dir.join(format!("claude-code-sessions/acct/org/{local}.json"));
        std::fs::write(&path, serde_json::to_vec(body).unwrap()).unwrap();
        path
    }

    fn get(path: &Path) -> Value {
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }

    #[test]
    fn a_switch_hands_auto_fix_over_and_switching_back_returns_it() {
        let root = tempfile::tempdir().unwrap();
        let state = Arc::new(AppState::init(root.path().join("data")).unwrap());
        let (_, north) = profile(&state, root.path(), "b-north", "north");
        let (aij_place, aij) = profile(&state, root.path(), "aij-test", "aij");
        let original = record(
            &north,
            "local_orig",
            &json!({"sessionId": "local_orig", "cliSessionId": "P", "title": "Release", "prs": [
                {"prNumber": 210, "repo": "org/repo", "state": "OPEN", "own": true, "autoFix": true}
            ]}),
        );
        let from = account("north", &root.path().join("cfg-north"));
        let to = account("aij", &root.path().join("cfg-aij"));
        let reference = TranscriptRef {
            path: root.path().join("P.jsonl"),
            native_id: Some("P".into()),
        };

        let before = super::preview(&state, &from, &to, &reference, &[]);
        assert_eq!(
            before.automation,
            vec!["Auto-fix on pull request #210 (org/repo)".to_owned()]
        );
        assert!(before.automation_needs_desktop);

        // Into a terminal: it stays with the original, untouched.
        let terminal = super::hand_over(&state, &from, &to, &reference, &[], &[], None, "e0");
        assert!(terminal.moved.is_empty() && terminal.kept.len() == 1);
        assert_eq!(get(&original).get("isArchived"), None);

        // Into the aij-test profile, neither app running: the original is
        // put away at once; the copy takes it once the app imported it.
        let done = super::hand_over(
            &state,
            &from,
            &to,
            &reference,
            &[],
            &[],
            Some(&aij_place),
            "e1",
        );
        assert_eq!(done.moved.len(), 1);
        assert!(!done.original_waits);
        assert_eq!(
            done.copy_waits.as_deref(),
            Some(aij_place.as_str()),
            "no record of the copy yet"
        );
        let put_away = get(&original);
        assert_eq!(put_away["isArchived"], true);
        assert_eq!(put_away["prs"][0]["autoFix"], false);

        let copy = record(
            &aij,
            "local_P",
            &json!({"sessionId": "local_P", "cliSessionId": "P"}),
        );
        super::apply(&state);
        assert_eq!(
            get(&copy)["prs"][0]["autoFix"],
            true,
            "the copy took it over"
        );

        // Back: the original out of the archive with Auto-fix, the copy's off.
        let back = super::switch_back(&state, "e1");
        assert_eq!(back.len(), 1);
        let original_now = get(&original);
        assert_eq!(original_now["isArchived"], false);
        assert_eq!(original_now["prs"][0]["autoFix"], true);
        let copy_now = get(&copy);
        assert_eq!(copy_now["isArchived"], true);
        assert_eq!(copy_now["prs"][0]["autoFix"], false);
    }

    #[test]
    fn switching_back_before_the_copy_took_over_leaves_it_with_the_original() {
        let root = tempfile::tempdir().unwrap();
        let state = Arc::new(AppState::init(root.path().join("data")).unwrap());
        let (_, north) = profile(&state, root.path(), "b-north", "north");
        let (aij_place, aij) = profile(&state, root.path(), "aij-test", "aij");
        let original = record(
            &north,
            "local_orig",
            &json!({"sessionId": "local_orig", "cliSessionId": "P", "prs": [
                {"prNumber": 210, "repo": "org/repo", "state": "OPEN", "own": true, "autoFix": true}
            ]}),
        );
        let from = account("north", &root.path().join("cfg-north"));
        let to = account("aij", &root.path().join("cfg-aij"));
        let reference = TranscriptRef {
            path: root.path().join("P.jsonl"),
            native_id: Some("P".into()),
        };
        // The copy isn't imported yet, so its taking over waits.
        super::hand_over(
            &state,
            &from,
            &to,
            &reference,
            &[],
            &[],
            Some(&aij_place),
            "e2",
        );
        assert_eq!(get(&original)["isArchived"], true);
        super::switch_back(&state, "e2");
        // The original is back with its Auto-fix; the copy, imported later,
        // never gets it.
        assert_eq!(get(&original)["isArchived"], false);
        assert_eq!(get(&original)["prs"][0]["autoFix"], true);
        let copy = record(
            &aij,
            "local_P",
            &json!({"sessionId": "local_P", "cliSessionId": "P"}),
        );
        super::apply(&state);
        assert!(get(&copy).get("prs").is_none());
        let waiting: Vec<super::Edit> = super::load(&state, "waiting.json");
        assert!(
            waiting
                .iter()
                .all(|edit| matches!(edit.op, super::Op::Wake { .. })),
            "only the wake waits to be seen kept: {waiting:?}"
        );
    }
}
