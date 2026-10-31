//! Starting a remembered place: the app the host registry knows, from
//! the bundle it last ran from, with only the instance folder it was
//! given. Checked every time: the bundle is still that app's, and its
//! executable is still one no other user can have changed (what the book
//! names is never run on its word alone).

use std::path::{Path, PathBuf};

use juggler_adapter::live::HostKind;
use juggler_platform::apps::Start;

use super::book::KnownPlace;
use crate::features::conversations::places::HostRegistry;

/// What starting an app needs from the system (a seam: the tests script
/// it).
pub trait Starter: Send + Sync {
    /// Why `exe` may not run, when it may not.
    ///
    /// # Errors
    ///
    /// The reason, in words.
    fn trusted(&self, exe: &Path) -> Result<(), String>;
    /// Starts `bundle` with `args`.
    ///
    /// # Errors
    ///
    /// The system's reason.
    fn start(&self, bundle: &Path, how: Start, args: &[String]) -> Result<(), String>;
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

    fn start(&self, bundle: &Path, how: Start, args: &[String]) -> Result<(), String> {
        juggler_platform::apps::open_app(bundle, how, args).map_err(|error| error.to_string())
    }
}

/// How `place` starts: its bundle, how, and its arguments; or why it
/// can't be.
///
/// # Errors
///
/// Why it can't be started, in words.
pub fn plan(
    place: &KnownPlace,
    hosts: &HostRegistry,
    background: bool,
) -> Result<(PathBuf, Start, Vec<String>), String> {
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
    Ok((
        place.bundle.clone(),
        Start {
            new_instance: host.instance_arg.is_some(),
            background,
        },
        args,
    ))
}

/// Starts `place` (behind the app in front when `background`).
///
/// # Errors
///
/// Why it didn't start, in words.
pub fn start(
    place: &KnownPlace,
    hosts: &HostRegistry,
    starter: &dyn Starter,
    background: bool,
) -> Result<(), String> {
    let (bundle, how, args) = plan(place, hosts, background)?;
    if !place.bundle.is_dir() {
        return Err(format!("{} isn't installed there any more", place.name));
    }
    starter
        .trusted(&place.exe)
        .map_err(|why| format!("{} wasn't started: {why}", place.name))?;
    starter
        .start(&bundle, how, &args)
        .map_err(|why| format!("{} didn't start: {why}", place.name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use juggler_adapter::live::HostApp;

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
        }
    }

    #[test]
    fn a_profile_starts_with_only_its_folder_as_a_further_instance() {
        let (bundle, how, args) = plan(&place(Some("/Users/you/P/b-work")), &hosts(), true).unwrap();
        assert_eq!(bundle, PathBuf::from("/Applications/Claude.app"));
        assert!(how.new_instance && how.background);
        assert_eq!(args, ["--user-data-dir=/Users/you/P/b-work"]);
        let (_, how, args) = plan(&place(None), &hosts(), false).unwrap();
        assert!(how.new_instance, "beside a profile that's running");
        assert!(args.is_empty(), "the default instance is started plainly");
    }

    #[test]
    fn a_place_that_isnt_the_apps_any_more_is_never_started() {
        let mut moved = place(None);
        moved.bundle = PathBuf::from("/tmp/Evil.app");
        moved.exe = PathBuf::from("/tmp/Evil.app/Contents/MacOS/Claude");
        assert!(plan(&moved, &hosts(), false).is_err(), "another bundle's name");
        let mut outside = place(None);
        outside.exe = PathBuf::from("/tmp/Claude");
        assert!(plan(&outside, &hosts(), false).is_err(), "an executable outside the bundle");
        let mut relative = place(Some("P/b-work"));
        relative.given = Some(PathBuf::from("P/b-work"));
        assert!(plan(&relative, &hosts(), false).is_err(), "a relative folder");
        let mut unknown = place(None);
        unknown.app_id = "nope".into();
        assert!(plan(&unknown, &hosts(), false).is_err());
    }

    struct Refuses;
    impl Starter for Refuses {
        fn trusted(&self, _: &Path) -> Result<(), String> {
            Err("other users can change /Applications/Claude.app".into())
        }
        fn start(&self, _: &Path, _: Start, _: &[String]) -> Result<(), String> {
            Err("STARTED-ANYWAY".into())
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
        let error = start(&here, &hosts(), &Refuses, false).unwrap_err();
        assert!(error.contains("other users can change"), "{error}");
        assert!(!error.contains("STARTED-ANYWAY"), "{error}");
    }
}
