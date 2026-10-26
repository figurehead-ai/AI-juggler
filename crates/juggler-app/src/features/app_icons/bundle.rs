//! D-38: which app bundle an icon is read from, and the checks it passes
//! first. Only a bundle AI Juggler already knows is read: AI Juggler's
//! own, a place's app (from its process, as `host_of` found it), or an app
//! in the host registry (by its bundle id, else in the usual folders).
//! The UI never names a path.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::features::conversations::places::{HostRegistry, outer_bundle};

/// The id AI Juggler's own place asks with (`OWN_PLACE` in the UI).
pub const OWN_APP_ID: &str = "ai-juggler";

/// The folders an app is looked for in when macOS can't say where its
/// bundle id is, in order, after the user's own `~/Applications`.
const FOLDERS: &[&str] = &[
    "/Applications",
    "/Applications/Utilities",
    "/System/Applications",
    "/System/Applications/Utilities",
    "/System/Library/CoreServices",
];

/// How many entries of a bundle's `Contents/Resources` are checked.
const MAX_RESOURCES: usize = 4096;

/// A bundle an icon may be read from: checked, by its real path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bundle {
    /// The `.app` folder, without links.
    pub path: PathBuf,
    /// Its name in the cache: the bundle id when known, else its folder's
    /// name (letters, digits, `.`, `-` and `_` only).
    pub key: String,
    /// When the bundle last changed (epoch seconds): a new version of the
    /// app is a new icon.
    pub version: u64,
}

/// Which app the UI asks for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ask {
    /// The host registry's id, or [`OWN_APP_ID`].
    pub app_id: Option<String>,
    /// The app's process, for an app a place found running.
    pub app_pid: Option<u32>,
}

/// Where AI Juggler looks things up (a seam, so tests use fixtures).
pub trait Lookup {
    /// The executable of process `pid`, when it runs.
    fn exe_of(&self, pid: u32) -> Option<PathBuf>;
    /// AI Juggler's own executable.
    fn own_exe(&self) -> Option<PathBuf>;
    /// Where macOS keeps the app with bundle id `id`.
    fn app_with_id(&self, id: &str) -> Option<PathBuf>;
    /// The user's home folder.
    fn home(&self) -> Option<PathBuf>;
    /// The bundle id written in bundle `path`.
    fn bundle_id(&self, path: &Path) -> Option<String>;
}

/// The real Mac.
pub struct System;

impl Lookup for System {
    fn exe_of(&self, pid: u32) -> Option<PathBuf> {
        crate::features::conversations::procs::some(&[pid])
            .remove(&pid)
            .and_then(|process| process.exe)
    }

    fn own_exe(&self) -> Option<PathBuf> {
        std::env::current_exe().ok()
    }

    fn app_with_id(&self, id: &str) -> Option<PathBuf> {
        super::render::app_with_bundle_id(id)
    }

    fn home(&self) -> Option<PathBuf> {
        dirs::home_dir()
    }

    fn bundle_id(&self, path: &Path) -> Option<String> {
        super::render::bundle_id(path)
    }
}

/// The bundle `ask` names, checked; `None` when there is none to read.
#[must_use]
pub fn locate(ask: &Ask, hosts: &HostRegistry, lookup: &dyn Lookup) -> Option<Bundle> {
    let found = if ask.app_id.as_deref() == Some(OWN_APP_ID) {
        outer_bundle(&lookup.own_exe()?).map(|(dir, _)| dir)
    } else {
        let known = ask.app_id.as_deref().and_then(|id| hosts.by_id(id));
        // A running place's app is the bundle on this Mac that runs it; a
        // process in another bundle than the one its id names is passed over.
        let running = ask
            .app_pid
            .and_then(|pid| lookup.exe_of(pid))
            .and_then(|exe| outer_bundle(&exe))
            .filter(|(_, name)| known.is_none_or(|app| app.bundle == *name))
            .map(|(dir, _)| dir);
        running.or_else(|| {
            let app = known?;
            app.bundle_id
                .as_deref()
                .and_then(|id| lookup.app_with_id(id))
                .filter(|path| {
                    path.file_name()
                        .is_some_and(|name| name.to_string_lossy() == app.bundle)
                })
                .or_else(|| installed(&app.bundle, lookup))
        })
    };
    let path = checked(&found?)?;
    let key = lookup
        .bundle_id(&path)
        .or_else(|| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        })
        .map(|key| safe_key(&key))
        .filter(|key| !key.is_empty())?;
    Some(Bundle {
        version: version_of(&path),
        path,
        key,
    })
}

/// Bundle folder `name` in the usual folders, the user's own first.
fn installed(name: &str, lookup: &dyn Lookup) -> Option<PathBuf> {
    let own = lookup.home().map(|home| home.join("Applications"));
    own.into_iter()
        .chain(FOLDERS.iter().map(PathBuf::from))
        .map(|folder| folder.join(name))
        .find(|path| path.is_dir())
}

/// `key` with only letters, digits, `.`, `-` and `_` (a file name).
#[must_use]
pub fn safe_key(key: &str) -> String {
    key.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .take(128)
        .collect::<String>()
        .trim_matches('.')
        .to_owned()
}

/// `path` as a bundle that may be read, by its real path: an absolute
/// folder named `*.app`, not a link, whose `Info.plist` and icon files
/// (`*.icns`, `Assets.car`) are not links out of it. Anything else is
/// refused.
#[must_use]
pub fn checked(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() || !is_app_name(path) {
        return None;
    }
    let meta = std::fs::symlink_metadata(path).ok()?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return None;
    }
    let real = std::fs::canonicalize(path).ok()?;
    if !is_app_name(&real) || !real.is_dir() {
        return None;
    }
    let contents = real.join("Contents");
    let resources = contents.join("Resources");
    let mut parts = vec![contents.join("Info.plist")];
    parts.extend(
        std::fs::read_dir(&resources)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .take(MAX_RESOURCES)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("icns"))
                    || path.file_name().is_some_and(|name| name == "Assets.car")
            }),
    );
    for part in [contents, resources].iter().chain(parts.iter()) {
        if !stays_inside(part, &real) {
            return None;
        }
    }
    Some(real)
}

/// A folder named `*.app` (and more than that).
fn is_app_name(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("app"))
        && path.file_stem().is_some_and(|stem| !stem.is_empty())
}

/// `part` is absent, or is (or leads by links to) something inside
/// `bundle`.
fn stays_inside(part: &Path, bundle: &Path) -> bool {
    match std::fs::symlink_metadata(part) {
        Err(_) => true,
        Ok(meta) if !meta.file_type().is_symlink() => true,
        Ok(_) => std::fs::canonicalize(part).is_ok_and(|real| real.starts_with(bundle)),
    }
}

/// When bundle `path` last changed: the latest of the bundle's folder,
/// its `Contents` and its `Info.plist` (an update rewrites one of them).
#[must_use]
pub fn version_of(path: &Path) -> u64 {
    [
        path.to_path_buf(),
        path.join("Contents"),
        path.join("Contents/Info.plist"),
    ]
    .iter()
    .filter_map(|part| std::fs::symlink_metadata(part).ok()?.modified().ok())
    .filter_map(|at| at.duration_since(UNIX_EPOCH).ok())
    .map(|since| since.as_secs())
    .max()
    .unwrap_or(0)
}
