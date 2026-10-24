//! AJ-E1529, AJ-E2101: which conversations are open now. What is open
//! comes from the live refresher ([`super::scan`]): the tools' own
//! per-session files and the process table, never a process's environment
//! (D-37). Here each open session is matched to a conversation in the
//! index: on the account whose profile it runs in; a session whose
//! profile isn't known (a `codex resume <id>` holding no file) is matched
//! by its id when exactly one of the user's accounts has that
//! conversation, and is nobody's otherwise.

use std::collections::HashMap;

use super::Conversation;
use super::scan::{LiveEntry, LiveModel};

/// A live conversation: `(account, tool, id)`.
pub type Live = (String, String, String);

/// Every open conversation of the user's, with where it's open. Each
/// conversation is claimed once; two accounts' conversations can share an
/// id, so a session counts only on its own account.
#[must_use]
pub fn live_entries(conversations: &[Conversation], model: &LiveModel) -> HashMap<Live, LiveEntry> {
    let mut by_tool_id: HashMap<(&str, &str), Vec<&str>> = HashMap::new();
    for conversation in conversations {
        by_tool_id
            .entry((conversation.tool.as_str(), conversation.summary.id.as_str()))
            .or_default()
            .push(conversation.account_id.as_str());
    }
    let mut out: HashMap<Live, LiveEntry> = HashMap::new();
    for entry in &model.entries {
        let Some(accounts) = by_tool_id.get(&(entry.tool.as_str(), entry.session_id.as_str()))
        else {
            continue;
        };
        let account = match &entry.account_id {
            Some(account) if accounts.contains(&account.as_str()) => account.clone(),
            None if accounts.len() == 1 => accounts[0].to_owned(),
            // Not on this account, or on several with nothing to tell
            // them apart: nobody's.
            _ => continue,
        };
        let key = (account, entry.tool.clone(), entry.session_id.clone());
        match out.get_mut(&key) {
            // Found twice (by its profile and by what a server holds): one
            // row with every place.
            Some(known) => {
                for place in &entry.places {
                    if !known.places.iter().any(|p| p.pid == place.pid) {
                        known.places.push(place.clone());
                    }
                }
            }
            None => {
                out.insert(key, entry.clone());
            }
        }
    }
    out
}

/// The `(account, tool, id)` of each conversation in a running app's
/// sidebar that no process runs now (inactive), by the same matching.
#[must_use]
pub fn inactive_ids(
    conversations: &[Conversation],
    model: &LiveModel,
) -> std::collections::HashSet<Live> {
    conversations
        .iter()
        .filter(|conversation| {
            model
                .inactive
                .iter()
                .any(|(tool, id)| *tool == conversation.tool && *id == conversation.summary.id)
        })
        .map(|conversation| {
            (
                conversation.account_id.clone(),
                conversation.tool.clone(),
                conversation.summary.id.clone(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use juggler_adapter::conversation::ConversationSummary;

    fn conversation(id: &str, account: &str) -> Conversation {
        Conversation {
            summary: ConversationSummary {
                id: id.into(),
                ..ConversationSummary::default()
            },
            account_id: account.into(),
            tool: "t".into(),
            path: format!("/x/{account}/{id}.jsonl").into(),
            folder: None,
        }
    }

    fn entry(account: Option<&str>, id: &str, pid: u32) -> LiveEntry {
        LiveEntry {
            tool: "t".into(),
            account_id: account.map(Into::into),
            session_id: id.into(),
            places: vec![super::super::scan::Place {
                pid,
                start: None,
                host: None,
                profile: None,
            }],
            state: None,
            viewed_ms: None,
            app_title: None,
            background: false,
        }
    }

    fn model(entries: Vec<LiveEntry>) -> LiveModel {
        LiveModel {
            entries,
            ..LiveModel::default()
        }
    }

    #[test]
    fn a_session_is_live_on_its_own_account_only() {
        let all = [conversation("same", "a"), conversation("same", "b")];
        let live = live_entries(&all, &model(vec![entry(Some("a"), "same", 1)]));
        assert_eq!(live.len(), 1);
        assert!(live.contains_key(&("a".into(), "t".into(), "same".into())));
        // A profile that isn't one of the user's accounts: nobody's.
        assert!(live_entries(&all, &model(vec![entry(Some("z"), "same", 1)])).is_empty());
    }

    #[test]
    fn a_session_of_unknown_profile_is_matched_only_when_one_account_has_it() {
        let all = [
            conversation("only", "a"),
            conversation("twice", "a"),
            conversation("twice", "b"),
        ];
        let live = live_entries(
            &all,
            &model(vec![entry(None, "only", 1), entry(None, "twice", 2)]),
        );
        assert_eq!(live.len(), 1);
        assert!(live.contains_key(&("a".into(), "t".into(), "only".into())));
        // Not in the index yet: not shown (nothing to show it as).
        assert!(live_entries(&all, &model(vec![entry(Some("a"), "new", 3)])).is_empty());
    }

    #[test]
    fn found_twice_is_one_conversation_with_both_places() {
        let all = [conversation("c", "a")];
        let live = live_entries(
            &all,
            &model(vec![entry(Some("a"), "c", 1), entry(None, "c", 2)]),
        );
        assert_eq!(live.len(), 1);
        assert_eq!(live.values().next().unwrap().places.len(), 2);
    }

    #[test]
    fn an_app_sidebar_session_with_no_process_is_inactive() {
        let all = [conversation("c", "a"), conversation("d", "a")];
        let model = LiveModel {
            inactive: vec![("t".into(), "c".into())],
            ..LiveModel::default()
        };
        assert_eq!(
            inactive_ids(&all, &model),
            std::collections::HashSet::from([("a".into(), "t".into(), "c".into())])
        );
    }
}
