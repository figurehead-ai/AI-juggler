//! The apps and profiles AI Juggler has seen running, remembered (one
//! JSON value in app settings under [`KEY`], which the window's setting
//! writer refuses): enough to start one again exactly as it ran (its
//! bundle and the instance folder it was given, nothing else) and to say
//! which of the user's sessions were open in it.
//!
//! Recorded from the live view's full process scan (at most once a
//! minute). At most [`MAX`] places; one not seen for [`FORGET_MS`] is
//! forgotten, unless the user keeps it alive.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::features::conversations::places::{HostRegistry, RunningApp};
use crate::features::conversations::scan::LiveEntry;

/// The app setting the book is kept under.
pub const KEY: &str = "places.known";
/// The keys only this module writes (the window's setting writer refuses
/// them: a place names a program AI Juggler may start).
pub const RESERVED_PREFIX: &str = "places.";
/// Places remembered at most.
pub const MAX: usize = 32;
/// Forgotten after this long unseen (90 days), unless kept alive.
pub const FORGET_MS: i64 = 90 * 24 * 60 * 60 * 1000;
/// "Last seen" moves in steps this long (an hour), so a running app
/// doesn't rewrite the book every scan.
const SEEN_STEP_MS: i64 = 60 * 60 * 1000;
/// Sessions remembered per place (the newest).
const CONVERSATIONS: usize = 100;

/// One app or profile, as it last ran.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct KnownPlace {
    /// The host registry's id for the app.
    pub app_id: String,
    /// The app's name on screen (`Claude app`).
    pub name: String,
    /// The instance's name when it isn't the app's default one (`b-work`).
    #[serde(default)]
    pub instance: Option<String>,
    /// The bundle it ran from.
    pub bundle: PathBuf,
    /// Its main executable, as it ran (checked before every start).
    #[serde(default)]
    pub exe: PathBuf,
    /// The instance folder it was started with, when it was given one:
    /// the only argument starting it again passes.
    #[serde(default)]
    pub given: Option<PathBuf>,
    /// Its instance folder (given, else the default); `None` for an app
    /// that runs one instance.
    #[serde(default)]
    pub instance_dir: Option<PathBuf>,
    /// The account its sessions ran on, when one did.
    #[serde(default)]
    pub account_id: Option<String>,
    /// `tool/id` of the sessions seen open in it, newest last.
    #[serde(default)]
    pub conversations: Vec<String>,
    /// When it was last seen running, epoch ms (to the hour).
    pub seen_ms: i64,
    /// Keep alive: start it again when it stops.
    #[serde(default)]
    pub keep_alive: bool,
    /// Why keep alive is paused (it stopped too often, it couldn't start,
    /// it was stopped from AI Juggler); `None` while it watches.
    #[serde(default)]
    pub paused: Option<String>,
}

impl KnownPlace {
    /// `Claude · b-work`, `Codex`.
    #[must_use]
    pub fn label(&self) -> String {
        match &self.instance {
            Some(instance) => format!("{} · {instance}", self.name),
            None => self.name.clone(),
        }
    }

    /// The profile in words, as the rail and Home name it: its account's
    /// name (`account`, when its sessions ran on one), else the
    /// instance's; none for an app that runs one instance.
    #[must_use]
    pub fn profile(&self, account: Option<&str>) -> Option<String> {
        self.instance_dir.as_ref()?;
        account.map(str::to_owned).or_else(|| self.instance.clone())
    }

    /// `Claude · Work Max`: the app, then [`KnownPlace::profile`].
    #[must_use]
    pub fn named(&self, account: Option<&str>) -> String {
        self.profile(account).map_or_else(
            || self.name.clone(),
            |profile| format!("{} · {profile}", self.name),
        )
    }

    /// Kept alive and watching (not paused).
    #[must_use]
    pub const fn watched(&self) -> bool {
        self.keep_alive && self.paused.is_none()
    }
}

/// A place's id: its app and instance folder.
#[must_use]
pub fn place_id(app_id: &str, instance_dir: Option<&Path>) -> String {
    format!(
        "{app_id}|{}",
        instance_dir
            .map(|dir| dir.to_string_lossy())
            .unwrap_or_default()
    )
}

/// `tool/id`: how a session is remembered.
#[must_use]
pub fn conversation_key(tool: &str, id: &str) -> String {
    format!("{tool}/{id}")
}

/// Every place remembered, by id.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Book {
    /// By [`place_id`].
    pub places: BTreeMap<String, KnownPlace>,
}

impl Book {
    /// The book as stored; anything unreadable is an empty book (it is
    /// rebuilt from what runs), never an error.
    #[must_use]
    pub fn parse(stored: Option<&str>) -> Self {
        stored
            .and_then(|json| serde_json::from_str(json).ok())
            .unwrap_or_default()
    }

    /// The value to store.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Records what runs now (`running`, from a full scan) and which
    /// sessions are open where (`entries`). Returns whether the book
    /// changed.
    pub fn observe(
        &mut self,
        running: &[RunningApp],
        entries: &[LiveEntry],
        hosts: &HostRegistry,
        home: &Path,
        now_ms: i64,
    ) -> bool {
        let before = self.clone();
        for app in running {
            let id = place_id(&app.app_id, app.instance.as_deref());
            if !self.places.contains_key(&id) && !self.make_room() {
                continue;
            }
            let host = hosts.by_id(&app.app_id);
            let default_dir = host
                .and_then(|host| host.default_instance.as_deref())
                .map(|dir| home.join(dir));
            let instance = app
                .instance
                .as_ref()
                .filter(|dir| Some(*dir) != default_dir.as_ref())
                .and_then(|dir| dir.file_name())
                .map(|name| name.to_string_lossy().into_owned());
            let place = self.places.entry(id).or_insert_with(|| KnownPlace {
                app_id: app.app_id.clone(),
                name: String::new(),
                instance: None,
                bundle: PathBuf::new(),
                exe: PathBuf::new(),
                given: None,
                instance_dir: app.instance.clone(),
                account_id: None,
                conversations: Vec::new(),
                seen_ms: now_ms,
                keep_alive: false,
                paused: None,
            });
            place.name =
                host.map_or_else(|| app.app_id.clone(), |host| host.shown_name().to_owned());
            place.instance = instance;
            place.bundle.clone_from(&app.bundle);
            place.exe.clone_from(&app.exe);
            place.given.clone_from(&app.given);
            if now_ms.saturating_sub(place.seen_ms) >= SEEN_STEP_MS {
                place.seen_ms = now_ms;
            }
        }
        for entry in entries {
            for seen in &entry.places {
                let Some(host) = &seen.host else { continue };
                let Some(app_id) = &host.app_id else { continue };
                let id = place_id(app_id, host.instance_dir.as_deref());
                let Some(place) = self.places.get_mut(&id) else {
                    continue;
                };
                let key = conversation_key(&entry.tool, &entry.session_id);
                if !place.conversations.contains(&key) {
                    place.conversations.push(key);
                    let over = place.conversations.len().saturating_sub(CONVERSATIONS);
                    place.conversations.drain(..over);
                }
                if entry.account_id.is_some() && place.account_id != entry.account_id {
                    place.account_id.clone_from(&entry.account_id);
                }
            }
        }
        self.tidy(now_ms);
        *self != before
    }

    /// Forgets places unseen for [`FORGET_MS`] that aren't kept alive.
    pub fn tidy(&mut self, now_ms: i64) {
        self.places.retain(|_, place| {
            place.keep_alive || now_ms.saturating_sub(place.seen_ms) < FORGET_MS
        });
    }

    /// Room for one more: below [`MAX`], or the place seen longest ago
    /// that isn't kept alive forgotten. `false` when all are kept alive.
    fn make_room(&mut self) -> bool {
        if self.places.len() < MAX {
            return true;
        }
        let oldest = self
            .places
            .iter()
            .filter(|(_, place)| !place.keep_alive)
            .min_by_key(|(_, place)| place.seen_ms)
            .map(|(id, _)| id.clone());
        oldest.is_some_and(|id| self.places.remove(&id).is_some())
    }

    /// The place a session was last seen open in: the most recent one
    /// that lists it.
    #[must_use]
    pub fn place_of(&self, tool: &str, id: &str) -> Option<(&String, &KnownPlace)> {
        let key = conversation_key(tool, id);
        self.places
            .iter()
            .filter(|(_, place)| place.conversations.contains(&key))
            .max_by_key(|(_, place)| place.seen_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::conversations::places::Host;
    use crate::features::conversations::scan::Place;
    use juggler_adapter::live::{HostApp, HostKind};

    fn hosts() -> HostRegistry {
        HostRegistry::new(vec![HostApp {
            id: "claude-app".into(),
            name: "Claude".into(),
            bundle: "Claude.app".into(),
            bundle_id: None,
            kind: HostKind::Desktop,
            instance_arg: Some("--user-data-dir".into()),
            default_instance: Some("Library/Application Support/Claude".into()),
            thread_link: None,
            folder_link: None,
            folder_command: None,
            tab_script: None,
            shown: None,
        }])
    }

    fn running(dir: Option<&str>, given: bool) -> RunningApp {
        let home = Path::new("/Users/you");
        let instance = Some(dir.map_or_else(
            || home.join("Library/Application Support/Claude"),
            PathBuf::from,
        ));
        RunningApp {
            app_id: "claude-app".into(),
            bundle: PathBuf::from("/Applications/Claude.app"),
            exe: PathBuf::from("/Applications/Claude.app/Contents/MacOS/Claude"),
            pid: 10,
            start: Some(100),
            given: if given { instance.clone() } else { None },
            instance,
        }
    }

    const WORK: &str = "/Users/you/Profiles/b-work";

    fn entry(session: &str, dir: &str) -> LiveEntry {
        LiveEntry {
            tool: "claude-code".into(),
            account_id: Some("acc-work".into()),
            session_id: session.into(),
            places: vec![Place {
                pid: 12,
                start: None,
                host: Some(Host {
                    app_id: Some("claude-app".into()),
                    name: "Claude".into(),
                    shown: "Claude app".into(),
                    kind: Some(HostKind::Desktop),
                    pid: 10,
                    start: Some(100),
                    instance_dir: Some(PathBuf::from(dir)),
                    instance: None,
                }),
                profile: None,
            }],
            state: None,
            probably: false,
            viewed_ms: None,
            app_title: None,
            background: false,
        }
    }

    #[test]
    fn a_profile_is_remembered_with_its_folder_and_its_sessions() {
        let mut book = Book::default();
        let home = Path::new("/Users/you");
        let changed = book.observe(
            &[running(Some(WORK), true), running(None, false)],
            &[entry("s-1", WORK)],
            &hosts(),
            home,
            1_000,
        );
        assert!(changed);
        let work = &book.places[&place_id("claude-app", Some(Path::new(WORK)))];
        assert_eq!(work.label(), "Claude · b-work");
        assert_eq!(work.given.as_deref(), Some(Path::new(WORK)));
        assert_eq!(work.conversations, ["claude-code/s-1"]);
        assert_eq!(work.account_id.as_deref(), Some("acc-work"));
        let default = book
            .places
            .values()
            .find(|place| place.instance.is_none())
            .unwrap();
        assert_eq!(default.label(), "Claude");
        assert_eq!(
            default.given, None,
            "the default instance is started plainly"
        );
        assert_eq!(
            book.place_of("claude-code", "s-1")
                .map(|(_, place)| place.label()),
            Some("Claude · b-work".to_owned())
        );
        // The same again a minute later: nothing to write.
        assert!(!book.observe(
            &[running(Some(WORK), true), running(None, false)],
            &[entry("s-1", WORK)],
            &hosts(),
            home,
            61_000,
        ));
    }

    #[test]
    fn unseen_places_are_forgotten_unless_kept_alive_and_the_book_stays_small() {
        let mut book = Book::default();
        let home = Path::new("/Users/you");
        for at in 0..MAX + 3 {
            let dir = format!("/Users/you/Profiles/p{at}");
            book.observe(
                &[running(Some(&dir), true)],
                &[],
                &hosts(),
                home,
                i64::try_from(at).unwrap(),
            );
        }
        assert_eq!(book.places.len(), MAX, "the oldest went to make room");
        assert!(!book.places.contains_key(&place_id(
            "claude-app",
            Some(Path::new("/Users/you/Profiles/p0"))
        )));
        let kept = place_id("claude-app", Some(Path::new("/Users/you/Profiles/p5")));
        book.places.get_mut(&kept).unwrap().keep_alive = true;
        book.tidy(FORGET_MS + 1_000);
        assert_eq!(book.places.keys().collect::<Vec<_>>(), [&kept]);
    }

    #[test]
    fn an_unreadable_book_is_an_empty_one() {
        assert_eq!(Book::parse(Some("{not json")), Book::default());
        assert_eq!(Book::parse(None), Book::default());
        let mut book = Book::default();
        book.observe(
            &[running(None, false)],
            &[],
            &hosts(),
            Path::new("/Users/you"),
            5,
        );
        assert_eq!(Book::parse(Some(&book.to_json())), book);
    }
}
