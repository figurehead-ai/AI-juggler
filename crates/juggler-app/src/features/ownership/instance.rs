//! AJ-E2315: which app instance a process runs, told so that two
//! spellings of one folder are one instance and a spelling the app might
//! read otherwise is never trusted (generalised from the agent bridge's
//! attestation, `agent_bridge::attest`, which now calls these):
//!
//! - [`instance_arg`]: the instance flag read strictly, exactly as the app
//!   itself reads it (one `flag=<folder>`, never after `--`);
//! - [`InstanceKey`]: an instance folder in its one real spelling (links,
//!   `..`, a trailing slash and letter case resolved by the system,
//!   [`juggler_platform::real_path`]);
//! - [`lock_holder`]: the process holding the instance's own lock (the
//!   app's [`HostApp::instance_lock`]), however the folder was spelled;
//! - [`instance_of`]: an app process's instance by the strict flag, and,
//!   where the flag can't be trusted, by the lock it holds;
//! - [`ProcId`]: a process told apart from a later one on its pid.

use std::path::{Path, PathBuf};

use juggler_adapter::live::HostApp;

use crate::features::conversations::procs::Proc;

/// An app instance folder (a Claude app profile's) in its one real
/// spelling: what every "is this that profile" compares.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InstanceKey(PathBuf);

impl InstanceKey {
    /// The key of the folder `dir`, however it is spelled.
    #[must_use]
    pub fn of(dir: &Path) -> Self {
        Self(juggler_platform::real_path::canonical(dir))
    }

    /// The folder, in its real spelling.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }

    /// Whether `real` (a path in its real spelling) is the folder or
    /// inside it, part by part: `/tmp/lab1` never holds `/tmp/lab10/x`.
    #[must_use]
    pub fn holds(&self, real: &Path) -> bool {
        real.starts_with(&self.0)
    }

    /// How deep the folder is: a path belongs to the deepest known folder
    /// holding it.
    pub(super) fn depth(&self) -> usize {
        self.0.components().count()
    }
}

/// Whether two instance folders (`None`: an app that runs one instance)
/// are one instance, however each is spelled.
#[must_use]
pub fn same_instance(a: Option<&Path>, b: Option<&Path>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a == b || InstanceKey::of(a) == InstanceKey::of(b),
        (None, None) => true,
        _ => false,
    }
}

/// The instance folder a process's arguments `args` give with `flag`
/// (`--user-data-dir`), read so that it can't differ from what the app
/// itself reads: only `flag=<folder>`, at most once, before a `--` (after
/// it the app reads no switch). Any other form (`flag <folder>`,
/// `-user-data-dir=…`, an empty value), a second one, or one after `--` is
/// refused. `None`: not given (its default).
///
/// # Errors
///
/// Why the arguments can't be trusted, in words.
pub fn instance_arg(args: &[String], flag: &str) -> Result<Option<String>, String> {
    let name = flag.trim_start_matches('-');
    let exact = format!("{flag}=");
    let mut found = None;
    let mut seen = 0;
    let mut ended = false;
    for arg in args.iter().skip(1) {
        if arg == "--" {
            ended = true;
            continue;
        }
        if !names_flag(arg, name) {
            continue;
        }
        seen += 1;
        match arg.strip_prefix(&exact) {
            Some(value) if !value.is_empty() && !ended => found = Some(value.to_owned()),
            _ => {
                return Err(
                    "its app was started with its profile folder given in another form".to_owned(),
                );
            }
        }
    }
    if seen > 1 {
        return Err("its app was started with its profile folder given more than once".to_owned());
    }
    Ok(found)
}

/// Whether `arg` is the switch `name` (without its dashes), with or
/// without a value: `--name`, `-name=x`, `---name=x`.
fn names_flag(arg: &str, name: &str) -> bool {
    let bare = arg.trim_start_matches('-');
    arg.starts_with('-')
        && (bare == name
            || bare
                .strip_prefix(name)
                .is_some_and(|rest| rest.starts_with('=')))
}

/// Every folder `args` name with `flag` in any form (`flag=x`, `flag x`,
/// after `--` too): never trusted as such, only the candidates the
/// instance lock decides between ([`instance_of`]).
fn named(args: &[String], flag: &str) -> Vec<PathBuf> {
    let name = flag.trim_start_matches('-');
    let mut out = Vec::new();
    for (at, arg) in args.iter().enumerate().skip(1) {
        if !names_flag(arg, name) {
            continue;
        }
        let value = match arg.split_once('=') {
            Some((_, value)) => Some(value),
            None => args.get(at + 1).map(String::as_str),
        };
        if let Some(value) = value.filter(|value| !value.is_empty()) {
            out.push(PathBuf::from(value));
        }
    }
    out
}

/// The process holding a write lock on the file `lock` under the instance
/// folder `instance` (`fcntl`'s, asked with `F_GETLK`, which takes none),
/// walked to without a link: `None` when nothing holds one or it isn't
/// there.
///
/// # Errors
///
/// A link on the way, a file that isn't a regular one of the owner's, or
/// the system's error ([`juggler_platform::safe_fs::lock_holder_in`]).
pub fn lock_holder(instance: &InstanceKey, lock: &str) -> std::io::Result<Option<u32>> {
    juggler_platform::safe_fs::lock_holder_in(instance.path(), Path::new(lock))
}

/// Who holds an instance's lock (`lock`, under the instance's folder):
/// the system's answer ([`lock_holder`]), or a test's.
pub type Holder<'a> = &'a dyn Fn(&InstanceKey, &str) -> Option<u32>;

/// [`Holder`] by the system.
#[must_use]
pub fn system_holder(instance: &InstanceKey, lock: &str) -> Option<u32> {
    lock_holder(instance, lock).ok().flatten()
}

/// The instance an app's process runs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Instance {
    /// The folder it was given, as given: what starting it again passes.
    /// `None`: its default.
    pub given: Option<PathBuf>,
    /// Its folder: the one given, else the app's default. `None` for an
    /// app that runs one instance.
    pub dir: Option<PathBuf>,
}

/// The instance `process`, a process of `app`, runs: by `app`'s instance
/// flag read strictly ([`instance_arg`]), else the app's default folder
/// under `home`. Where the flag can't be trusted (given twice, in another
/// form, after `--`), the operating system decides: the folder, of those
/// it names or the default, whose instance lock `process` holds (only a
/// main process holds it).
///
/// # Errors
///
/// The flag can't be trusted and no lock says otherwise, in words.
pub fn instance_of(process: &Proc, app: &HostApp, home: &Path) -> Result<Instance, String> {
    instance_with(process, app, Some(home), &system_holder)
}

/// [`instance_of`], the lock's holder asked of `held`; `home: None` looks
/// at no default folder (only a folder the process was given counts: a
/// lab's own, whose home isn't the process's).
///
/// # Errors
///
/// As [`instance_of`].
pub fn instance_with(
    process: &Proc,
    app: &HostApp,
    home: Option<&Path>,
    held: Holder<'_>,
) -> Result<Instance, String> {
    let holds_lock = |dir: &Path| {
        app.instance_lock
            .as_deref()
            .is_some_and(|lock| held(&InstanceKey::of(dir), lock) == Some(process.pid))
    };
    let Some(flag) = app.instance_arg.as_deref() else {
        return Ok(Instance::default());
    };
    let default = app
        .default_instance
        .as_deref()
        .and_then(|dir| Some(home?.join(dir)));
    let given = match instance_arg(&process.args, flag) {
        Ok(given) => given.map(PathBuf::from),
        Err(why) => {
            let named = named(&process.args, flag);
            if let Some(dir) = named.iter().find(|dir| holds_lock(dir)) {
                Some(dir.clone())
            } else if default.as_deref().is_some_and(holds_lock) {
                None
            } else {
                return Err(why);
            }
        }
    };
    let dir = given.clone().or(default);
    Ok(Instance { given, dir })
}

/// A process told apart from any later one given its pid: its pid and
/// when it started, epoch microseconds (the kernel's listing).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProcId {
    /// Its pid.
    pub pid: u32,
    /// When it started, epoch microseconds.
    pub start_micros: u128,
}

impl ProcId {
    /// The identity of `listed`.
    #[must_use]
    pub fn of(listed: &juggler_proc_list::Listed) -> Self {
        Self {
            pid: listed.pid,
            start_micros: u128::from(listed.start_secs) * 1_000_000
                + u128::from(listed.start_micros),
        }
    }
}

/// The main process of `app` (run from `bundle`) proven to run the
/// instance at `dir` by holding its lock, however its folder was spelled
/// on its command line: `None` when nothing holds it, the app declares no
/// lock, or the holder isn't the app's main process in `table`.
#[must_use]
pub fn proven_running(
    table: &crate::features::conversations::procs::Table,
    app: &HostApp,
    bundle: &Path,
    dir: Option<&Path>,
    held: Holder<'_>,
) -> Option<u32> {
    let lock = app.instance_lock.as_deref()?;
    let holder = held(&InstanceKey::of(dir?), lock)?;
    let mains = juggler_platform::real_path::lexical(&bundle.join("Contents/MacOS"));
    table
        .get(&holder)
        .and_then(|process| process.exe.as_deref())
        .is_some_and(|exe| juggler_platform::real_path::lexical(exe).parent() == Some(&mains))
        .then_some(holder)
}
