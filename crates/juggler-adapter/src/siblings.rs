//! D-62 (AJ-E3203): a desktop app's instances that aren't running.
//!
//! An app that keeps its instances side by side in one folder (the Claude
//! app's profiles, `ClaudeProfiles/<name>`, each started with
//! `--user-data-dir`) says so as data ([`HostApp::siblings`]). Beside an
//! instance seen running, the names of that folder's other folders are
//! its other instances. Only the names are read: the folder is listed
//! once, each entry's kind comes from that listing (is it a folder), and
//! nothing inside any of them is opened, read or even looked up.
//!
//! Bounded and careful about where: never the home folder or anything
//! outside it, never the folder the default instance sits in (that holds
//! every app's data), never a path with `..` in it, and never a folder
//! with more than [`MOST`] folders (that's no app's profiles).

use std::path::{Component, Path, PathBuf};

use crate::live::HostApp;

/// The most folders a profiles folder holds: one with more isn't one
/// app's profiles, and nothing in it is offered.
pub const MOST: usize = 64;
/// The most entries (files too) read from the folder.
const READ_MOST: usize = 4_096;

/// Why no instances were listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Unlisted {
    /// The app doesn't keep its instances side by side.
    #[error("this app doesn't keep its profiles in one folder")]
    NotSideBySide,
    /// The instance isn't one whose folder AI Juggler lists (the default
    /// one, outside the home folder, or right in it).
    #[error("AI Juggler doesn't list that folder")]
    NotListed,
    /// The folder can't be read.
    #[error("that folder can't be read")]
    Unreadable,
    /// The folder holds more than [`MOST`] folders.
    #[error("that folder holds too many folders to be one app's profiles")]
    TooMany,
}

impl HostApp {
    /// The folder its instances are kept in, beside `instance` (an
    /// instance folder seen running): `instance`'s parent, when this app
    /// keeps them side by side and the parent is one AI Juggler lists.
    /// `home` is the user's home folder.
    ///
    /// # Errors
    ///
    /// [`Unlisted::NotSideBySide`] or [`Unlisted::NotListed`].
    pub fn sibling_folder(&self, instance: &Path, home: &Path) -> Result<PathBuf, Unlisted> {
        if !self.siblings || self.instance_arg.is_none() {
            return Err(Unlisted::NotSideBySide);
        }
        if !plain(instance) || !plain(home) {
            return Err(Unlisted::NotListed);
        }
        let default = self.default_instance.as_deref().map(|dir| home.join(dir));
        if default.as_deref() == Some(instance) {
            return Err(Unlisted::NotListed);
        }
        let parent = instance.parent().ok_or(Unlisted::NotListed)?;
        let inside_home = parent
            .strip_prefix(home)
            .is_ok_and(|rest| rest.components().next().is_some());
        let beside_default = default
            .as_deref()
            .and_then(Path::parent)
            .is_some_and(|shared| shared == parent);
        if !inside_home || beside_default {
            return Err(Unlisted::NotListed);
        }
        Ok(parent.to_path_buf())
    }

    /// Its instances in the folder beside `instance` (see
    /// [`HostApp::sibling_folder`]): every folder there, `instance`'s own
    /// included, by name, sorted.
    ///
    /// # Errors
    ///
    /// As [`HostApp::sibling_folder`] and [`folders_in`].
    pub fn sibling_instances(
        &self,
        instance: &Path,
        home: &Path,
    ) -> Result<Vec<PathBuf>, Unlisted> {
        folders_in(&self.sibling_folder(instance, home)?)
    }
}

/// The folders in `folder`, by name, sorted: one listing of `folder`, and
/// each entry's kind as that listing gives it. Links, files, hidden
/// folders and names that aren't plain text are left out; nothing inside
/// an entry is opened.
///
/// # Errors
///
/// [`Unlisted::Unreadable`] when `folder` can't be listed;
/// [`Unlisted::TooMany`] when it holds more than [`MOST`] folders (or
/// more than a few thousand entries).
pub fn folders_in(folder: &Path) -> Result<Vec<PathBuf>, Unlisted> {
    let entries = std::fs::read_dir(folder).map_err(|_| Unlisted::Unreadable)?;
    let mut out = Vec::new();
    for (read, entry) in entries.enumerate() {
        if read >= READ_MOST {
            return Err(Unlisted::TooMany);
        }
        let Ok(entry) = entry else { continue };
        let name = entry.file_name();
        let Some(name) = name.to_str().filter(|name| plain_name(name)) else {
            continue;
        };
        // The listing's own kind: a link is a link here, never followed.
        if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        out.push(folder.join(name));
        if out.len() > MOST {
            return Err(Unlisted::TooMany);
        }
    }
    out.sort();
    Ok(out)
}

/// An absolute path with no `.` or `..` in it.
fn plain(path: &Path) -> bool {
    path.is_absolute()
        && path.components().all(|part| {
            matches!(
                part,
                Component::RootDir | Component::Prefix(_) | Component::Normal(_)
            )
        })
}

/// A folder name shown as a profile's: not hidden, no control characters,
/// not too long.
fn plain_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name.chars().count() <= 128
        && !name.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::HostKind;

    const DEFAULT: &str = "Library/Application Support/Desk";

    fn app(siblings: bool) -> HostApp {
        HostApp {
            id: "desk".into(),
            name: "Desk".into(),
            bundle: "Desk.app".into(),
            bundle_id: None,
            kind: HostKind::Desktop,
            instance_arg: Some("--user-data-dir".into()),
            default_instance: Some(DEFAULT.into()),
            thread_link: None,
            folder_link: None,
            folder_command: None,
            tab_script: None,
            shown: None,
            siblings,
            folder_env: None,
            handoff_link: None,
            handoff_record: None,
            session_link: None,
        }
    }

    /// A home with a profiles folder holding `names`; the folder.
    fn profiles(home: &Path, names: &[&str]) -> PathBuf {
        let folder = home.join("Library/Application Support/DeskProfiles");
        for name in names {
            std::fs::create_dir_all(folder.join(name)).unwrap();
        }
        folder
    }

    fn names(found: &[PathBuf]) -> Vec<String> {
        found
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn the_profiles_beside_one_running_are_its_folders_by_name() {
        let home = tempfile::tempdir().unwrap();
        let folder = profiles(home.path(), &["b-work", "b-side", "b-home", ".cache"]);
        std::fs::write(folder.join("notes.txt"), "not a profile").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(folder.join("b-side"), folder.join("linked")).unwrap();
        let found = app(true)
            .sibling_instances(&folder.join("b-work"), home.path())
            .unwrap();
        assert_eq!(names(&found), ["b-home", "b-side", "b-work"]);
        assert!(
            found
                .iter()
                .all(|path| path.parent() == Some(folder.as_path()))
        );
    }

    /// The Privacy page's line: nothing inside a profile folder is opened.
    /// Each profile folder is closed to everyone (so any look inside fails)
    /// and holds files that would fail to parse; the folder beside them
    /// also holds a pipe, which would block forever if it were opened.
    #[cfg(unix)]
    #[test]
    fn no_file_inside_a_profile_folder_is_opened() {
        use std::os::unix::fs::PermissionsExt;
        let home = tempfile::tempdir().unwrap();
        let folder = profiles(home.path(), &["b-work", "b-side", "b-play"]);
        for name in ["b-work", "b-side", "b-play"] {
            let inside = folder.join(name);
            std::fs::write(inside.join("Preferences"), [0xff, 0xfe, 0x00, 0x7b]).unwrap();
            std::fs::create_dir_all(inside.join("claude-code-sessions/a/o")).unwrap();
            std::fs::write(
                inside.join("claude-code-sessions/a/o/local_x.json"),
                "{garbage",
            )
            .unwrap();
        }
        let piped = std::process::Command::new("mkfifo")
            .arg(folder.join("pipe"))
            .status()
            .is_ok_and(|status| status.success());
        for name in ["b-work", "b-side", "b-play"] {
            std::fs::set_permissions(folder.join(name), std::fs::Permissions::from_mode(0o000))
                .unwrap();
        }
        let found = app(true).sibling_instances(&folder.join("b-work"), home.path());
        for name in ["b-work", "b-side", "b-play"] {
            std::fs::set_permissions(folder.join(name), std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }
        assert_eq!(names(&found.unwrap()), ["b-play", "b-side", "b-work"]);
        assert!(
            piped,
            "the pipe was made, so the listing passed it unopened"
        );
    }

    #[test]
    fn only_a_folder_that_holds_one_apps_profiles_is_listed() {
        let home = tempfile::tempdir().unwrap();
        let h = home.path();
        profiles(h, &["b-work"]);
        let listed = |app: &HostApp, instance: &Path| app.sibling_instances(instance, h);
        let work = h.join("Library/Application Support/DeskProfiles/b-work");
        assert_eq!(listed(&app(false), &work), Err(Unlisted::NotSideBySide));
        // The default instance, and a folder beside it (every app's data).
        assert_eq!(
            listed(&app(true), &h.join(DEFAULT)),
            Err(Unlisted::NotListed)
        );
        std::fs::create_dir_all(h.join("Library/Application Support/Desk-2")).unwrap();
        assert_eq!(
            listed(&app(true), &h.join("Library/Application Support/Desk-2")),
            Err(Unlisted::NotListed)
        );
        // Right in the home folder, outside it, or with `..` in the path.
        std::fs::create_dir_all(h.join("b-home")).unwrap();
        assert_eq!(
            listed(&app(true), &h.join("b-home")),
            Err(Unlisted::NotListed)
        );
        let outside = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(outside.path().join("P/b-x")).unwrap();
        assert_eq!(
            listed(&app(true), &outside.path().join("P/b-x")),
            Err(Unlisted::NotListed)
        );
        assert_eq!(
            listed(&app(true), &work.join("../../DeskProfiles/b-work")),
            Err(Unlisted::NotListed)
        );
        assert_eq!(
            listed(&app(true), Path::new("relative/b-work")),
            Err(Unlisted::NotListed)
        );
        // A folder that isn't there any more.
        assert_eq!(
            listed(&app(true), &h.join("Gone/b-work")),
            Err(Unlisted::Unreadable)
        );
    }

    #[test]
    fn a_folder_with_more_folders_than_profiles_is_no_ones_profiles() {
        let home = tempfile::tempdir().unwrap();
        let many: Vec<String> = (0..=MOST).map(|n| format!("p{n}")).collect();
        let refs: Vec<&str> = many.iter().map(String::as_str).collect();
        let folder = profiles(home.path(), &refs);
        assert_eq!(
            app(true).sibling_instances(&folder.join("p0"), home.path()),
            Err(Unlisted::TooMany)
        );
        std::fs::remove_dir(folder.join("p1")).unwrap();
        assert_eq!(
            app(true)
                .sibling_instances(&folder.join("p0"), home.path())
                .unwrap()
                .len(),
            MOST
        );
    }

    /// AJ-E3203's budget: the listing takes 50 ms at most (best of three,
    /// so a loaded machine's one slow run doesn't decide).
    #[test]
    fn listing_the_most_profiles_takes_under_50_ms() {
        let home = tempfile::tempdir().unwrap();
        let many: Vec<String> = (0..MOST).map(|n| format!("p{n}")).collect();
        let refs: Vec<&str> = many.iter().map(String::as_str).collect();
        let folder = profiles(home.path(), &refs);
        for n in 0..500 {
            std::fs::write(folder.join(format!("f{n}.log")), "x").unwrap();
        }
        let best = (0..3)
            .map(|_| {
                let at = std::time::Instant::now();
                let found = app(true)
                    .sibling_instances(&folder.join("p0"), home.path())
                    .unwrap();
                assert_eq!(found.len(), MOST);
                at.elapsed()
            })
            .min()
            .unwrap();
        assert!(best < std::time::Duration::from_millis(50), "{best:?}");
    }
}
