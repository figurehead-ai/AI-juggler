//! ADR-0028: a switch moves a conversation's whole family.
//!
//! Before the switch, [`find`] asks the source tool's adapter for the
//! sessions its desktop app records as started from the conversation (the
//! Claude app's side sessions) and finds each one's conversation file, so
//! the pipeline moves them together ([`juggler_handover::pipeline::execute_family`]).
//!
//! After it, their records go to the target profile of that app, beside
//! the conversation's own record there, so they nest as they did. The app
//! reads records only when it starts, so they are placed while it isn't
//! running: at once when it isn't, else at its next start (a restart the
//! user confirms, or AI Juggler's own Open of it). Until then they wait in
//! AI Juggler's data folder (`family-pending/<place>.json`), so a waiting
//! family survives AI Juggler quitting.
//!
//! Nothing here names a tool: the adapter's
//! [`juggler_adapter::live::HostFamily`] does the tool's part.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::live::HostFamilyMember;
use juggler_core::model::{Account, TranscriptRef};

use crate::state::AppState;

/// The family the switch moves with a conversation.
#[derive(Default)]
pub struct Family {
    /// Each member, parents first, as the source app records it.
    pub members: Vec<HostFamilyMember>,
    /// Each member's conversation file, in the same order.
    pub refs: Vec<TranscriptRef>,
    /// Members whose conversation file isn't there (by title): named
    /// before the switch, never moved half.
    pub missing: Vec<String>,
}

/// The family of the conversation `reference` holds on `account`: the
/// sessions started from it in any profile of its tool's desktop app that
/// works on `account` (the first that records any).
#[must_use]
pub fn find(state: &Arc<AppState>, account: &Account, reference: &TranscriptRef) -> Family {
    let Some(adapter) = state.registry.get(&account.harness) else {
        return Family::default();
    };
    let Some(host) = adapter.host_family() else {
        return Family::default();
    };
    let Some(native) = reference.native_id.clone().or_else(|| {
        reference
            .path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
    }) else {
        return Family::default();
    };
    let members = instance_dirs(state, account.id.as_str())
        .iter()
        .map(|dir| host.descendants(dir, &native))
        .find(|members| !members.is_empty())
        .unwrap_or_default();
    let mut family = Family::default();
    for member in members {
        match state
            .conversations
            .conversation_of(account.id.as_str(), &member.conversation)
            .map(|conversation| conversation.path)
            .filter(|path| path.is_file())
        {
            Some(path) => {
                family.refs.push(TranscriptRef {
                    path,
                    native_id: Some(member.conversation.clone()),
                });
                family.members.push(member);
            }
            None => family.missing.push(
                member
                    .title
                    .clone()
                    .unwrap_or_else(|| "an untitled session".to_owned()),
            ),
        }
    }
    family
}

/// The instance folders of the app profiles AI Juggler knows that work on
/// `account_id`.
fn instance_dirs(state: &Arc<AppState>, account_id: &str) -> Vec<PathBuf> {
    crate::features::places::with_book(state, |book| {
        book.places
            .values()
            .filter(|place| place.account_id.as_deref() == Some(account_id))
            .filter_map(|place| place.instance_dir.clone())
            .collect()
    })
    .unwrap_or_default()
}

/// A family whose records wait for its target app's next start.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Pending {
    /// The target app profile (a place id).
    pub place: String,
    /// The tool, whose adapter places the records.
    pub tool: String,
    /// The conversation the switch moved, as it is on the target.
    pub parent: String,
    /// The members, as the source app recorded them.
    pub members: Vec<PendingMember>,
    /// Conversation ids the move forked: old → new.
    pub renamed: Vec<(String, String)>,
}

/// [`HostFamilyMember`], as it waits on disk.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct PendingMember {
    /// The source app's record of it.
    pub record: PathBuf,
    /// The record's id in the app.
    pub local_id: String,
    /// Its conversation id.
    pub conversation: String,
    /// The conversation it was started from.
    pub parent_conversation: String,
    /// Its title, when it has one.
    pub title: Option<String>,
}

impl From<&HostFamilyMember> for PendingMember {
    fn from(member: &HostFamilyMember) -> Self {
        Self {
            record: member.record.clone(),
            local_id: member.local_id.clone(),
            conversation: member.conversation.clone(),
            parent_conversation: member.parent_conversation.clone(),
            title: member.title.clone(),
        }
    }
}

impl From<&PendingMember> for HostFamilyMember {
    fn from(member: &PendingMember) -> Self {
        Self {
            record: member.record.clone(),
            local_id: member.local_id.clone(),
            conversation: member.conversation.clone(),
            parent_conversation: member.parent_conversation.clone(),
            title: member.title.clone(),
        }
    }
}

fn pending_path(state: &Arc<AppState>, place: &str) -> Option<PathBuf> {
    let safe = place
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':' || c == '.');
    (safe && !place.is_empty() && !place.contains("..")).then(|| {
        state
            .data_dir
            .join("family-pending")
            .join(format!("{}.json", place.replace(':', "_")))
    })
}

/// Keeps `pending` until its app's next start (one per app profile: a
/// later switch's family joins the one waiting).
///
/// # Errors
///
/// The write fails.
pub fn wait(state: &Arc<AppState>, pending: &Pending) -> std::io::Result<()> {
    let path = pending_path(state, &pending.place)
        .ok_or_else(|| std::io::Error::other("not an app profile id"))?;
    let mut all = waiting(state, &pending.place);
    all.push(pending.clone());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let staged = path.with_extension("json.new");
    std::fs::write(
        &staged,
        serde_json::to_vec_pretty(&all).map_err(std::io::Error::other)?,
    )?;
    std::fs::rename(staged, path)
}

/// The families waiting for `place`'s next start.
#[must_use]
pub fn waiting(state: &Arc<AppState>, place: &str) -> Vec<Pending> {
    pending_path(state, place)
        .and_then(|path| std::fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// The app profiles with a family waiting.
#[must_use]
pub fn waiting_places(state: &Arc<AppState>) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(state.data_dir.join("family-pending")) else {
        return Vec::new();
    };
    let mut places: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .filter_map(|entry| std::fs::read(entry.path()).ok())
        .filter_map(|bytes| serde_json::from_slice::<Vec<Pending>>(&bytes).ok())
        .filter_map(|all| all.first().map(|pending| pending.place.clone()))
        .collect();
    places.sort();
    places.dedup();
    places
}

/// How many sessions wait to nest in `place` at its next start.
#[must_use]
pub fn waiting_count(state: &Arc<AppState>, place: &str) -> usize {
    waiting(state, place)
        .iter()
        .map(|pending| pending.members.len())
        .sum()
}

/// Places every family waiting for `place`, which mustn't be running
/// (the caller has just quit it, or is about to start it): each beside
/// its parent's record in `instance_dir`. One that can't be placed (its
/// parent's record isn't there yet) keeps waiting; one that was refused
/// is dropped, and said. Returns the words for what happened, if any.
pub fn place_waiting(state: &Arc<AppState>, place: &str, instance_dir: &Path) -> Option<String> {
    let all = waiting(state, place);
    if all.is_empty() {
        return None;
    }
    let mut kept = Vec::new();
    let mut nested = 0;
    let mut refused = Vec::new();
    for pending in all {
        let Some(adapter) = state
            .registry
            .get(&juggler_core::model::HarnessId::new(pending.tool.clone()))
        else {
            continue;
        };
        let Some(host) = adapter.host_family() else {
            continue;
        };
        let Some(folder) = host.records_folder_of(instance_dir, &pending.parent) else {
            kept.push(pending);
            continue;
        };
        let members: Vec<HostFamilyMember> = pending.members.iter().map(Into::into).collect();
        match host.place(&members, &folder, &pending.renamed) {
            Ok(placed) => nested += placed.len(),
            Err(error) => refused.push(error.to_string()),
        }
    }
    if let Some(path) = pending_path(state, place) {
        if kept.is_empty() {
            let _ = std::fs::remove_file(path);
        } else if let Ok(bytes) = serde_json::to_vec_pretty(&kept) {
            let _ = std::fs::write(path, bytes);
        }
    }
    tracing::info!(target: "aij::flow", flow = "family", step = "placed", nested, refused = refused.len());
    match (nested, refused.first()) {
        (_, Some(why)) => Some(why.clone()),
        (0, None) => None,
        (n, None) => Some(format!(
            "{n} session{} now sit under the one that started {}",
            if n == 1 { "" } else { "s" },
            if n == 1 { "it" } else { "them" }
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{Pending, PendingMember};

    #[test]
    fn a_waiting_family_round_trips() {
        let pending = Pending {
            place: "claude:/p".into(),
            tool: "claude-code".into(),
            parent: "P".into(),
            members: vec![PendingMember {
                record: "/a/local_c.json".into(),
                local_id: "local_c".into(),
                conversation: "C".into(),
                parent_conversation: "P".into(),
                title: Some("Child".into()),
            }],
            renamed: vec![("P".into(), "P2".into())],
        };
        let text = serde_json::to_string(&vec![pending.clone()]).unwrap();
        let back: Vec<Pending> = serde_json::from_str(&text).unwrap();
        assert_eq!(back, vec![pending]);
    }
}
