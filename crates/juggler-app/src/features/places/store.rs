//! D-62 (AJ-E3202): the places book is kept as the desktop Apps' rows
//! (`apps`, kind `desktop`), so an App the user names, hides or keeps
//! alive is one row, not a row and a book. 0.3's book (one JSON value
//! under [`book::KEY`]) is moved into the rows the first time it's read,
//! then cleared. The move is idempotent: rows are keyed by what they
//! stand for, so a move cut short is finished on the next read.

use std::path::PathBuf;

use juggler_core::storage::{App, Storage};

use super::book::{self, Book, KnownPlace};
use crate::error::AppResult;

/// How the App last ran, as Open starts it again (`launch_json`).
#[derive(serde::Serialize, serde::Deserialize)]
struct Ran {
    name: String,
    #[serde(default)]
    instance: Option<String>,
    bundle: PathBuf,
    #[serde(default)]
    exe: PathBuf,
    #[serde(default)]
    given: Option<PathBuf>,
    /// AJ-E3203: found beside a profile that ran, not seen running yet.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    found: bool,
}

/// The book, from the rows (0.3's book moved into them first).
///
/// # Errors
///
/// Storage errors.
pub fn load(storage: &Storage, tool_of: &dyn Fn(&str) -> String) -> AppResult<Book> {
    if let Some(old) = storage.app_settings().get(book::KEY)? {
        let moved = Book::parse(Some(&old));
        for place in moved.places.values() {
            let id = book::place_id(&place.app_id, place.instance_dir.as_deref());
            storage
                .apps()
                .put_desktop(&row(&id, place, &tool_of(&place.app_id)))?;
        }
        storage.app_settings().set(book::KEY, "")?;
    }
    let mut out = Book::default();
    for app in storage.apps().list()? {
        if let Some(place) = place(&app) {
            out.places.insert(app.id, place);
        }
    }
    Ok(out)
}

/// Writes what changed between `before` and `after`: a changed or new
/// place's row, a gone one forgotten.
///
/// # Errors
///
/// Storage errors.
pub fn save(
    storage: &Storage,
    before: &Book,
    after: &Book,
    tool_of: &dyn Fn(&str) -> String,
) -> AppResult<()> {
    for (id, place) in &after.places {
        if before.places.get(id) != Some(place) {
            storage
                .apps()
                .put_desktop(&row(id, place, &tool_of(&place.app_id)))?;
        }
    }
    for id in before.places.keys() {
        if !after.places.contains_key(id) {
            storage.apps().forget(id)?;
        }
    }
    Ok(())
}

fn row(id: &str, place: &KnownPlace, tool: &str) -> App {
    let ran = Ran {
        name: place.name.clone(),
        instance: place.instance.clone(),
        bundle: place.bundle.clone(),
        exe: place.exe.clone(),
        given: place.given.clone(),
        found: place.found,
    };
    App {
        id: id.to_owned(),
        kind: "desktop".to_owned(),
        tool: tool.to_owned(),
        host_app_id: Some(place.app_id.clone()),
        instance_dir: place
            .instance_dir
            .as_ref()
            .map(|dir| dir.to_string_lossy().into_owned()),
        folder_id: place.account_id.clone(),
        label: None,
        hidden: false,
        keep_alive: place.keep_alive,
        paused_json: place
            .paused
            .as_ref()
            .and_then(|why| serde_json::to_string(why).ok()),
        launch_json: serde_json::to_string(&ran).ok(),
        sessions_json: serde_json::to_string(&place.conversations).ok(),
        first_seen: place.seen_ms,
        last_seen: place.seen_ms,
    }
}

/// A desktop row as a place; `None` for a CLI App, or a desktop one
/// never seen running (nothing to start it with).
fn place(app: &App) -> Option<KnownPlace> {
    if app.kind != "desktop" {
        return None;
    }
    let ran: Ran = serde_json::from_str(app.launch_json.as_deref()?).ok()?;
    Some(KnownPlace {
        app_id: app.host_app_id.clone()?,
        name: ran.name,
        instance: ran.instance,
        bundle: ran.bundle,
        exe: ran.exe,
        given: ran.given,
        instance_dir: app.instance_dir.as_ref().map(PathBuf::from),
        account_id: app.folder_id.clone(),
        conversations: app
            .sessions_json
            .as_deref()
            .and_then(|json| serde_json::from_str(json).ok())
            .unwrap_or_default(),
        seen_ms: app.last_seen,
        keep_alive: app.keep_alive,
        paused: app
            .paused_json
            .as_deref()
            .and_then(|json| serde_json::from_str(json).ok()),
        found: ran.found,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claude_code(_: &str) -> String {
        "claude-code".to_owned()
    }

    fn place(dir: &str, keep_alive: bool) -> KnownPlace {
        KnownPlace {
            app_id: "claude-app".into(),
            name: "Claude app".into(),
            instance: Some("b-lab".into()),
            bundle: "/Applications/Claude.app".into(),
            exe: "/Applications/Claude.app/Contents/MacOS/Claude".into(),
            given: Some(dir.into()),
            instance_dir: Some(dir.into()),
            account_id: None,
            conversations: vec!["claude-code/s-1".into(), "claude-code/s-2".into()],
            seen_ms: 3_600_000,
            keep_alive,
            paused: keep_alive.then(|| "it stopped too often".to_owned()),
            found: false,
        }
    }

    #[test]
    fn the_book_survives_the_rows_whole() {
        let storage = Storage::open_in_memory().unwrap();
        let mut ours = Book::default();
        for (dir, kept) in [("/u/b-lab", true), ("/u/b-old", false)] {
            let p = place(dir, kept);
            ours.places
                .insert(book::place_id(&p.app_id, p.instance_dir.as_deref()), p);
        }
        save(&storage, &Book::default(), &ours, &claude_code).unwrap();
        assert_eq!(load(&storage, &claude_code).unwrap(), ours);
        // One forgotten: its row goes; the user's name for the other stays.
        let lab = book::place_id("claude-app", Some(std::path::Path::new("/u/b-lab")));
        storage.apps().rename(&lab, Some("Lab")).unwrap();
        let mut fewer = ours.clone();
        fewer.places.retain(|id, _| *id == lab);
        save(&storage, &ours, &fewer, &claude_code).unwrap();
        assert_eq!(load(&storage, &claude_code).unwrap(), fewer);
        assert_eq!(
            storage.apps().get(&lab).unwrap().unwrap().label.as_deref(),
            Some("Lab")
        );
    }

    #[test]
    fn the_old_book_is_moved_once_and_cleared() {
        let storage = Storage::open_in_memory().unwrap();
        let p = place("/u/b-lab", true);
        let mut old = Book::default();
        // 0.3's ids were `<app>|<folder>`: the move keys by what it is.
        old.places.insert("claude-app|/u/b-lab".into(), p.clone());
        storage
            .app_settings()
            .set(book::KEY, &old.to_json())
            .unwrap();
        let book = load(&storage, &claude_code).unwrap();
        let id = book::place_id("claude-app", Some(std::path::Path::new("/u/b-lab")));
        assert_eq!(book.places.get(&id), Some(&p));
        assert_eq!(storage.app_settings().get(book::KEY).unwrap(), None);
        assert_eq!(
            load(&storage, &claude_code).unwrap(),
            book,
            "read again: the same"
        );
        assert_eq!(
            storage.apps().get(&id).unwrap().unwrap().tool,
            "claude-code"
        );
    }

    #[test]
    fn a_cli_app_is_no_place() {
        let storage = Storage::open_in_memory().unwrap();
        storage.apps().upsert_cli("claude-code", "work", 1).ok();
        assert!(load(&storage, &claude_code).unwrap().places.is_empty());
    }
}
