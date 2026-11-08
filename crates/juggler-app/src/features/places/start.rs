//! Starting a remembered place: the app the host registry knows, from
//! the bundle it last ran from, with only the instance folder it was
//! given and, for a profile paired with a tool's folder, that folder
//! (0.3.5: the Claude app reads `CLAUDE_CONFIG_DIR`, so a profile started
//! without it put its sessions in the default folder, on another
//! account). Checked every time: the bundle is still that app's, and its
//! executable is still one no other user can have changed (what the book
//! names is never run on its word alone).

use std::path::{Path, PathBuf};

use juggler_adapter::live::HostKind;
use juggler_platform::apps::Start;

use super::book::KnownPlace;
use crate::features::conversations::places::HostRegistry;

/// What starting an app needs from the system, and (0.3.8) restarting one
/// that lost its window (a seam: the tests script it).
pub trait Starter: Send + Sync {
    /// Why `exe` may not run, when it may not.
    ///
    /// # Errors
    ///
    /// The reason, in words.
    fn trusted(&self, exe: &Path) -> Result<(), String>;
    /// Starts it as `launch` says.
    ///
    /// # Errors
    ///
    /// The system's reason.
    fn start(&self, launch: &Launch) -> Result<(), String>;
    /// 0.3.8: process `pid` (started at `start`) runs on but lost this
    /// login session's window server (`juggler_platform::apps::detached`).
    fn detached(&self, pid: u32, start: Option<i64>) -> bool;
    /// 0.3.8: asks process `pid` to quit (SIGTERM).
    ///
    /// # Errors
    ///
    /// The system's reason.
    fn quit(&self, pid: u32) -> Result<(), String>;
    /// Whether `pid` is still the process that started at `start`.
    fn alive(&self, pid: u32, start: Option<i64>) -> bool;
}

/// How a place starts: its bundle, how, the one environment variable a
/// paired profile needs, and its arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Launch {
    /// The app bundle.
    pub bundle: PathBuf,
    /// A further instance, in front or behind.
    pub how: Start,
    /// `(name, value)`: the tool folder its sessions use, when paired.
    pub env: Vec<(String, String)>,
    /// Its instance folder's argument, when it has one.
    pub args: Vec<String>,
}

/// The real system.
#[derive(Clone, Copy, Debug, Default)]
pub struct System;

impl Starter for System {
    fn trusted(&self, exe: &Path) -> Result<(), String> {
        let me = juggler_platform::user::current().map_err(|error| error.to_string())?;
        juggler_platform::file_trust::executable_trust(exe, me)
            .map(|_| ())
            .map_err(|why| why.words())
    }

    fn start(&self, launch: &Launch) -> Result<(), String> {
        juggler_platform::apps::open_app(&launch.bundle, launch.how, &launch.env, &launch.args)
            .map_err(|error| error.to_string())
    }

    fn detached(&self, pid: u32, start: Option<i64>) -> bool {
        juggler_platform::apps::detached(pid, start)
    }

    fn quit(&self, pid: u32) -> Result<(), String> {
        juggler_platform::apps::quit(pid).map_err(|error| error.to_string())
    }

    fn alive(&self, pid: u32, start: Option<i64>) -> bool {
        crate::features::conversations::procs::some(&[pid])
            .get(&pid)
            .is_some_and(|process| juggler_adapter::live::same_start(start, process.start))
    }
}

/// How `place` starts, or why it can't be. `folder` is the tool folder
/// its sessions use (its account's), passed in the variable the app reads
/// it from; a profile with no folder known starts as it is.
///
/// # Errors
///
/// Why it can't be started, in words.
pub fn plan(
    place: &KnownPlace,
    hosts: &HostRegistry,
    folder: Option<&Path>,
    background: bool,
) -> Result<Launch, String> {
    let Some(host) = hosts.by_id(&place.app_id) else {
        return Err(format!("AI Juggler doesn't know {} any more", place.name));
    };
    if host.kind != HostKind::Desktop {
        return Err(format!("{} isn't an app AI Juggler starts", place.name));
    }
    let bundle_named = place
        .bundle
        .file_name()
        .is_some_and(|name| name.to_string_lossy() == host.bundle);
    if !place.bundle.is_absolute() || !bundle_named {
        return Err(format!("{} isn't where it was", place.name));
    }
    if place.exe.parent() != Some(&place.bundle.join("Contents/MacOS")) {
        return Err(format!("{} isn't where it was", place.name));
    }
    let args = match (&host.instance_arg, &place.given) {
        (Some(flag), Some(given)) if given.is_absolute() => {
            vec![format!("{flag}={}", given.to_string_lossy())]
        }
        (Some(_), Some(_)) => return Err(format!("{} has no usable folder", place.name)),
        _ => Vec::new(),
    };
    // Only a profile (an instance folder given) is paired with a folder:
    // the app's default instance keeps the tool's default.
    let env = match (&host.folder_env, folder, &place.given) {
        (Some(name), Some(folder), Some(_)) if folder.is_absolute() => {
            vec![(name.clone(), folder.to_string_lossy().into_owned())]
        }
        _ => Vec::new(),
    };
    Ok(Launch {
        bundle: place.bundle.clone(),
        how: Start {
            new_instance: host.instance_arg.is_some(),
            background,
        },
        env,
        args,
    })
}

/// Starts `place` (behind the app in front when `background`).
///
/// # Errors
///
/// Why it didn't start, in words.
pub fn start(
    place: &KnownPlace,
    hosts: &HostRegistry,
    folder: Option<&Path>,
    starter: &dyn Starter,
    background: bool,
) -> Result<(), String> {
    let launch = plan(place, hosts, folder, background)?;
    if !place.bundle.is_dir() {
        return Err(format!("{} isn't installed there any more", place.name));
    }
    starter
        .trusted(&place.exe)
        .map_err(|why| format!("{} wasn't started: {why}", place.name))?;
    starter
        .start(&launch)
        .map_err(|why| format!("{} didn't start: {why}", place.name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use juggler_adapter::live::HostApp;

    fn hosts() -> HostRegistry {
        HostRegistry::new(vec![HostApp {
            folder_env: Some("CLAUDE_CONFIG_DIR".into()),
            ..crate::test_support::claude_app_host(None)
        }])
    }

    fn place(given: Option<&str>) -> KnownPlace {
        KnownPlace {
            app_id: "claude-app".into(),
            name: "Claude".into(),
            instance: given.map(|_| "b-work".into()),
            bundle: PathBuf::from("/Applications/Claude.app"),
            exe: PathBuf::from("/Applications/Claude.app/Contents/MacOS/Claude"),
            given: given.map(PathBuf::from),
            instance_dir: None,
            account_id: None,
            conversations: Vec::new(),
            seen_ms: 0,
            keep_alive: false,
            paused: None,
            found: false,
            users: true,
            declined: false,
        }
    }

    #[test]
    fn a_profile_starts_with_only_its_folder_as_a_further_instance() {
        let launch = plan(&place(Some("/Users/you/P/b-work")), &hosts(), None, true).unwrap();
        assert_eq!(launch.bundle, PathBuf::from("/Applications/Claude.app"));
        assert!(launch.how.new_instance && launch.how.background);
        assert_eq!(launch.args, ["--user-data-dir=/Users/you/P/b-work"]);
        assert!(launch.env.is_empty(), "no folder known: started as it is");
        let launch = plan(&place(None), &hosts(), None, false).unwrap();
        assert!(launch.how.new_instance, "beside a profile that's running");
        assert!(
            launch.args.is_empty(),
            "the default instance is started plainly"
        );
    }

    /// 0.3.5: a profile starts on the folder its sessions use, as the
    /// user's own launcher starts it; the default instance never gets one.
    #[test]
    fn a_profile_starts_on_its_own_tool_folder() {
        let folder = Path::new("/Users/you/CodeProfiles/work-max");
        let launch = plan(
            &place(Some("/Users/you/P/b-work")),
            &hosts(),
            Some(folder),
            false,
        )
        .unwrap();
        assert_eq!(
            launch.env,
            [(
                "CLAUDE_CONFIG_DIR".to_owned(),
                "/Users/you/CodeProfiles/work-max".to_owned()
            )]
        );
        let launch = plan(&place(None), &hosts(), Some(folder), false).unwrap();
        assert!(
            launch.env.is_empty(),
            "the app's default instance keeps the default"
        );
        let relative = Path::new("CodeProfiles/work-max");
        let launch = plan(
            &place(Some("/Users/you/P/b-work")),
            &hosts(),
            Some(relative),
            false,
        )
        .unwrap();
        assert!(launch.env.is_empty(), "never a relative folder");
    }

    #[test]
    fn a_place_that_isnt_the_apps_any_more_is_never_started() {
        let mut moved = place(None);
        moved.bundle = PathBuf::from("/tmp/Evil.app");
        moved.exe = PathBuf::from("/tmp/Evil.app/Contents/MacOS/Claude");
        assert!(
            plan(&moved, &hosts(), None, false).is_err(),
            "another bundle's name"
        );
        let mut outside = place(None);
        outside.exe = PathBuf::from("/tmp/Claude");
        assert!(
            plan(&outside, &hosts(), None, false).is_err(),
            "an executable outside the bundle"
        );
        let mut relative = place(Some("P/b-work"));
        relative.given = Some(PathBuf::from("P/b-work"));
        assert!(
            plan(&relative, &hosts(), None, false).is_err(),
            "a relative folder"
        );
        let mut unknown = place(None);
        unknown.app_id = "nope".into();
        assert!(plan(&unknown, &hosts(), None, false).is_err());
    }

    struct Refuses;
    impl Starter for Refuses {
        fn trusted(&self, _: &Path) -> Result<(), String> {
            Err("other users can change /Applications/Claude.app".into())
        }
        fn start(&self, _: &Launch) -> Result<(), String> {
            Err("STARTED-ANYWAY".into())
        }
        fn detached(&self, _: u32, _: Option<i64>) -> bool {
            false
        }
        fn quit(&self, _: u32) -> Result<(), String> {
            Err("never".into())
        }
        fn alive(&self, _: u32, _: Option<i64>) -> bool {
            false
        }
    }

    #[test]
    fn an_app_other_users_can_change_is_never_started() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("Claude.app");
        std::fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
        let mut here = place(None);
        here.exe = bundle.join("Contents/MacOS/Claude");
        here.bundle = bundle;
        let error = start(&here, &hosts(), None, &Refuses, false).unwrap_err();
        assert!(error.contains("other users can change"), "{error}");
        assert!(!error.contains("STARTED-ANYWAY"), "{error}");
    }
}
