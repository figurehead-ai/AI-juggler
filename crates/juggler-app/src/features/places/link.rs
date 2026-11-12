//! Linking a desktop App to the tool folder its sessions use: by the
//! profile's own name (the profile launcher's convention), or (ADR-0032)
//! by the folder its host app declares for an app that runs one instance
//! (the Codex app's `~/.codex`). Moved out of `places` for its size.

use std::sync::Arc;

use super::with_book;
use crate::state::AppState;

/// The folder discovery found under a profile's own `name` (the profile
/// launcher's convention).
pub(super) fn folder_for_profile<'a>(
    name: &str,
    found: &'a [juggler_adapter::environment::DiscoveredInstall],
) -> Option<&'a juggler_adapter::environment::DiscoveredInstall> {
    found
        .iter()
        .find(|one| one.path.file_name().and_then(|n| n.to_str()) == Some(name))
}

/// The go sitting (2026-11-07, finding 1): a profile added after first
/// run listed none of its sessions, because the folder its tool writes
/// them to (the profile launcher's `ClaudeCodeProfiles/<name>` beside the
/// app's `ClaudeProfiles/<name>`) was never brought in. With the same
/// yes, the folder discovery finds under the profile's own name is
/// brought in where it is, as first run does (in place, never copied),
/// unless an Account holds it already; either way it's the profile's
/// folder from then on. That link is what tells the folder's tool sign-in
/// apart (the profile signs in inside its app, never in a terminal: the
/// rerun's "needs its sign-in" came from a folder brought in unlinked) and
/// gives the App page its Account. Only one an Account holds already
/// unless `may_bring_in` (Add's yes). Nothing when none matches; a
/// failure never fails the add. Returns whether the link is new.
pub(super) fn link_its_folder(state: &Arc<AppState>, id: &str, may_bring_in: bool) -> bool {
    // Its profile's own name, or (an app that runs one instance) none:
    // then the folder its host app declares (ADR-0032: the Codex app's
    // `~/.codex`).
    let Some((app_id, name)) = with_book(state, |book| {
        book.places.get(id).map(|place| {
            let name = place
                .instance_dir
                .as_deref()
                .and_then(std::path::Path::file_name)
                .and_then(|name| name.to_str())
                .map(str::to_owned);
            (place.app_id.clone(), name)
        })
    })
    .ok()
    .flatten() else {
        return false;
    };
    let home = juggler_platform::user::home().unwrap_or_default();
    let canonical =
        |path: &std::path::Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let accounts = crate::commands::lock_storage_arc(state)
        .ok()
        .and_then(|storage| storage.accounts().list().ok())
        .unwrap_or_default();
    for harness in state.registry.ids() {
        let Some(adapter) = state.registry.get(&harness) else {
            continue;
        };
        let Some(host) = adapter
            .host_apps()
            .into_iter()
            .find(|host| host.id == app_id)
        else {
            continue;
        };
        let installs = adapter.scan_environment();
        let found = match (&name, &host.default_folder) {
            (Some(name), _) => folder_for_profile(name, &installs),
            (None, Some(folder)) => {
                let want = canonical(&home.join(folder));
                installs.iter().find(|one| canonical(&one.path) == want)
            }
            (None, None) => None,
        };
        let Some(found) = found else {
            continue;
        };
        let name = name.clone().unwrap_or_else(|| {
            found
                .path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default()
        });
        let held = accounts
            .iter()
            .find(|account| canonical(&account.state_dir) == canonical(&found.path))
            .map(|account| account.id.as_str().to_owned());
        let account = held.or_else(|| {
            if !may_bring_in {
                return None;
            }
            crate::features::account::import::account_import_mode_impl(
                state,
                harness.as_str(),
                &found.path.to_string_lossy(),
                &name,
                crate::features::account::import::ImportRequest {
                    origin: Some(found.origin.as_str().to_owned()),
                    in_place: true,
                    ..crate::features::account::import::ImportRequest::default()
                },
            )
            .map_err(|why| {
                tracing::warn!(
                    place = id,
                    "the added profile's folder wasn't brought in: {why}"
                );
            })
            .ok()
            .map(|done| done.account.id.as_str().to_owned())
        });
        return account.is_some_and(|account| {
            with_book(state, |book| {
                book.places.get_mut(id).is_some_and(|place| {
                    let new = place.account_id.as_deref() != Some(account.as_str());
                    place.account_id = Some(account);
                    // D-84 (`App::is_users`): an App on an Account the user
                    // has is theirs, now, not only once the book is read
                    // again (the 0.4.2 RC: the Codex app, linked on its
                    // first scan, stayed out of every list until a restart).
                    place.users = true;
                    place.declined = false;
                    new
                })
            })
            .unwrap_or(false)
        });
    }
    false
}

/// 0.4.3 (D-102): what a profile's folder link should be, by where its own
/// records' sessions are, when the folder it's linked to holds none of
/// them (`held`: each Account folder's count of them, switch copies
/// included; `own`: the folder its launcher names it by, its Account if it
/// is one, and its count). `None`: leave it; `Some(link)`: the new link,
/// `None` inside for "not one of the user's Accounts" (Add brings it in).
/// Its own folder wins a tie: a switch copies a conversation into another
/// profile's folder under the same id, so an Account holding copies isn't
/// the profile's for that.
#[must_use]
pub(super) fn link_by_records(
    held: &std::collections::HashMap<String, usize>,
    own: Option<(Option<String>, usize)>,
) -> Option<Option<String>> {
    let best = held
        .iter()
        .filter(|(_, n)| **n > 0)
        .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)));
    match (own, best) {
        (Some((account, mine)), best) if mine > 0 && best.is_none_or(|(_, n)| mine >= *n) => {
            Some(account)
        }
        (_, Some((account, _))) => Some(Some(account.clone())),
        _ => None,
    }
}

/// 0.4.3 (D-102): a profile linked to a folder that holds none of its own
/// records' sessions is linked again by them ([`link_by_records`]). The
/// test profile was linked to the default folder by sessions it once ran
/// there, while its sessions now run in its own folder, not an Account:
/// it wore the default Account's mark and listed none of its 23. Returns
/// whether the link changed.
pub(super) fn correct_by_records(
    state: &Arc<AppState>,
    id: &str,
    tool: &str,
    ids: &[String],
) -> bool {
    let Some((current, name, app_id)) = with_book(state, |book| {
        book.places.get(id).and_then(|place| {
            let name = place
                .instance_dir
                .as_deref()
                .and_then(std::path::Path::file_name)
                .and_then(|n| n.to_str())
                .map(str::to_owned);
            Some((place.account_id.clone()?, name, place.app_id.clone()))
        })
    })
    .ok()
    .flatten() else {
        return false;
    };
    if ids.is_empty()
        || ids
            .iter()
            .any(|sid| state.conversations.path_of(&current, sid).is_some())
    {
        return false;
    }
    let wanted: std::collections::HashSet<&str> = ids.iter().map(String::as_str).collect();
    let mut pairs = std::collections::HashSet::new();
    let mut held: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for one in state.conversations.all().unwrap_or_default() {
        if one.tool == tool
            && wanted.contains(one.summary.id.as_str())
            && pairs.insert((one.account_id.clone(), one.summary.id.clone()))
        {
            *held.entry(one.account_id).or_default() += 1;
        }
    }
    let canonical =
        |path: &std::path::Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let own = name.and_then(|name| {
        let adapter = state
            .registry
            .ids()
            .into_iter()
            .filter_map(|h| state.registry.get(&h))
            .find(|a| a.host_apps().iter().any(|host| host.id == app_id))?;
        let installs = adapter.scan_environment();
        let dir = folder_for_profile(&name, &installs)?.path.clone();
        let stems: std::collections::HashSet<String> = adapter
            .conversation_files(&dir)
            .iter()
            .filter_map(|path| path.file_stem().and_then(|s| s.to_str()).map(str::to_owned))
            .collect();
        let count = ids.iter().filter(|sid| stems.contains(*sid)).count();
        let account = crate::commands::lock_storage_arc(state)
            .ok()
            .and_then(|storage| storage.accounts().list().ok())
            .unwrap_or_default()
            .into_iter()
            .find(|account| canonical(&account.state_dir) == canonical(&dir))
            .map(|account| account.id.as_str().to_owned());
        Some((account, count))
    });
    let Some(link) = link_by_records(&held, own) else {
        return false;
    };
    if link.as_deref() == Some(current.as_str()) {
        return false;
    }
    tracing::info!(
        place = id,
        relinked = link.is_some(),
        "a profile's own records hold none of their sessions in its folder: linked by them"
    );
    with_book(state, |book| {
        book.places.get_mut(id).is_some_and(|place| {
            place.account_id = link;
            true
        })
    })
    .unwrap_or(false)
}

#[cfg(test)]
mod link_tests {
    use super::link_by_records;
    use std::collections::HashMap;

    fn held(pairs: &[(&str, usize)]) -> HashMap<String, usize> {
        pairs.iter().map(|(k, n)| ((*k).to_owned(), *n)).collect()
    }

    #[test]
    fn its_own_folder_wins_over_an_account_holding_switch_copies() {
        // The test profile: 21 in its own folder (no Account), 11 copies in
        // another profile's: unlinked, never the other's.
        assert_eq!(
            link_by_records(&held(&[("f-other", 11)]), Some((None, 21))),
            Some(None)
        );
        // Its own folder an Account: linked to it.
        assert_eq!(
            link_by_records(
                &held(&[("f-other", 11), ("f-own", 21)]),
                Some((Some("f-own".into()), 21))
            ),
            Some(Some("f-own".into()))
        );
    }

    #[test]
    fn an_account_holding_them_where_its_own_holds_none() {
        assert_eq!(
            link_by_records(&held(&[("f-a", 3), ("f-b", 9)]), Some((None, 0))),
            Some(Some("f-b".into()))
        );
        assert_eq!(
            link_by_records(&held(&[("f-a", 3)]), None),
            Some(Some("f-a".into()))
        );
    }

    #[test]
    fn no_evidence_leaves_it() {
        assert_eq!(link_by_records(&held(&[]), Some((None, 0))), None);
        assert_eq!(link_by_records(&held(&[("f-a", 0)]), None), None);
    }
}
