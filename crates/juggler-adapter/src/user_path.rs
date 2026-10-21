//! The user's `PATH`, for every discovery probe and every launch.
//!
//! An app opened from Finder inherits `/usr/bin:/bin:/usr/sbin:/sbin`.
//! An npm-installed CLI is a `#!/usr/bin/env node` script, so with that
//! `PATH` its `--version` fails ("env: node: No such file or directory")
//! and so would every session it starts (the 0.0.11 report: Codex was
//! installed but import only offered Claude).
//!
//! The `PATH` is resolved once: from the user's login shell when the app
//! asked for it at startup ([`resolve_at_startup`], off the UI thread,
//! [`LOGIN_SHELL_TIMEOUT`] at most), otherwise from the inherited `PATH`
//! plus the folders CLIs are usually installed in. The found binary's own
//! folder always comes first ([`path_for`]), so a CLI installed next to
//! its `node` runs either way. Its value is never logged: it names the
//! user's folders.

use std::ffi::{OsStr, OsString};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// How long the login shell gets to print its `PATH`.
pub const LOGIN_SHELL_TIMEOUT: Duration = Duration::from_secs(2);

/// Marks the `PATH` in the login shell's output, so whatever its startup
/// files print is never read as part of it.
const MARK: &str = "__AI_JUGGLER_PATH__";

/// What the login shell runs, read from its stdin — a constant: nothing
/// is spliced in, and no `-c` (SECURITY-GUIDELINES C2). The marker is
/// [`MARK`].
const SCRIPT: &[u8] = b"printf '__AI_JUGGLER_PATH__%s__AI_JUGGLER_PATH__' \"$PATH\"\nexit\n";

/// The `PATH` the login shell starts from, whatever the app inherited:
/// what launchd gives an app opened from Finder.
const SYSTEM_PATH: &str = "/usr/bin:/bin:/usr/sbin:/sbin";

/// The variables a probe or the login shell keeps from the app's own
/// environment; everything else is cleared (SECURITY-GUIDELINES C2): who
/// the user is, their locale and their temporary folder.
pub const ENV_ALLOWLIST: &[&str] = &[
    "HOME",
    "USER",
    "LOGNAME",
    "SHELL",
    "TMPDIR",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "__CF_USER_TEXT_ENCODING",
];

/// Folders under the user's home where CLIs are installed (npm prefixes,
/// installers, version managers), after Homebrew's.
const HOME_DIRS: &[&str] = &[".local/bin", ".npm-global/bin", ".volta/bin", ".bun/bin"];

/// Homebrew (Apple silicon, then Intel).
const SYSTEM_DIRS: &[&str] = &["/opt/homebrew/bin", "/usr/local/bin"];

/// The resolved `PATH`, and the login shell's own when it gave one.
struct Resolved {
    path: OsString,
    from_shell: Option<OsString>,
}

static RESOLVED: OnceLock<Resolved> = OnceLock::new();
static USE_LOGIN_SHELL: AtomicBool = AtomicBool::new(false);

/// Resolves the user's `PATH` from their login shell, on its own thread.
/// The app calls this once at startup; a probe or launch that needs the
/// `PATH` before it is done waits for it (at most the timeout). Without
/// this call — tests, tools — the fallback is used and no shell runs.
pub fn resolve_at_startup() {
    USE_LOGIN_SHELL.store(true, Ordering::SeqCst);
    let _ = std::thread::Builder::new()
        .name("user-path".into())
        .spawn(|| {
            let _ = user_path();
        });
}

/// The user's `PATH` (see the module docs), resolved on first use.
#[must_use]
pub fn user_path() -> &'static OsStr {
    &resolved().path
}

/// The `PATH` the user's own terminal has — the login shell's — when it
/// was read; `None` before that, without it, or when it failed.
#[must_use]
pub fn login_shell_path() -> Option<&'static OsStr> {
    resolved().from_shell.as_deref()
}

fn resolved() -> &'static Resolved {
    RESOLVED.get_or_init(|| {
        let from_shell = if USE_LOGIN_SHELL.load(Ordering::SeqCst) {
            login_shell(&shell_program(), LOGIN_SHELL_TIMEOUT)
        } else {
            None
        };
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let path = combine(
            from_shell.as_deref(),
            std::env::var_os("PATH").as_deref(),
            home.as_deref(),
            std::env::var_os("NVM_DIR").map(PathBuf::from).as_deref(),
        );
        Resolved { path, from_shell }
    })
}

/// The `PATH` to run `binary` with: its own folder, then [`user_path`].
#[must_use]
pub fn path_for(binary: &Path) -> OsString {
    prepend_dir(binary, user_path())
}

/// `binary`'s folder in front of `path` (once).
#[must_use]
pub fn prepend_dir(binary: &Path, path: &OsStr) -> OsString {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(dir) = binary.parent().filter(|dir| dir.is_absolute()) {
        dirs.push(dir.to_path_buf());
    }
    dirs.extend(std::env::split_paths(path));
    join(dedup(dirs))
}

/// A command for `binary` that runs with [`path_for`] and only the
/// [`ENV_ALLOWLIST`] — for the discovery probes (`--version`).
#[must_use]
pub fn command(binary: &Path) -> Command {
    let mut command = Command::new(binary);
    allowlisted_env(&mut command);
    command.env("PATH", path_for(binary));
    command
}

/// Clears `command`'s environment, then keeps the [`ENV_ALLOWLIST`].
fn allowlisted_env(command: &mut Command) {
    command.env_clear();
    for key in ENV_ALLOWLIST {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
}

/// The first `program` on [`user_path`].
#[must_use]
pub fn find(program: &str) -> Option<PathBuf> {
    find_in(program, user_path())
}

/// The first `program` on `path`.
#[must_use]
pub fn find_in(program: &str, path: &OsStr) -> Option<PathBuf> {
    std::env::split_paths(path)
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
}

/// The fallback and the merge: the login shell's `PATH` (when it gave
/// one), the inherited `PATH`, Homebrew, the home folders that exist, and
/// the active nvm `node` — each folder once, relative entries dropped.
#[must_use]
pub fn combine(
    from_shell: Option<&OsStr>,
    inherited: Option<&OsStr>,
    home: Option<&Path>,
    nvm_dir: Option<&Path>,
) -> OsString {
    let mut dirs: Vec<PathBuf> = Vec::new();
    for path in [from_shell, inherited].into_iter().flatten() {
        dirs.extend(std::env::split_paths(path));
    }
    dirs.extend(SYSTEM_DIRS.iter().map(PathBuf::from));
    if let Some(home) = home {
        dirs.extend(
            HOME_DIRS
                .iter()
                .map(|dir| home.join(dir))
                .filter(|dir| dir.is_dir()),
        );
        let nvm = nvm_dir.map_or_else(|| home.join(".nvm"), Path::to_path_buf);
        dirs.extend(nvm_bin(&nvm));
    }
    join(dedup(dirs))
}

/// The active nvm `node`'s folder: nvm's `default` alias (a version like
/// `v22.11.0`, a prefix like `22`, or `node`/`stable`/`lts/*`), else the
/// newest installed version.
#[must_use]
pub fn nvm_bin(nvm_dir: &Path) -> Option<PathBuf> {
    let versions = nvm_dir.join("versions/node");
    let mut installed: Vec<((u64, u64, u64), String)> = std::fs::read_dir(&versions)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            semver_of(&name).map(|version| (version, name))
        })
        .collect();
    installed.sort();
    let alias = std::fs::read_to_string(nvm_dir.join("alias/default"))
        .map(|text| text.trim().trim_start_matches('v').to_owned())
        .unwrap_or_default();
    let numeric = !alias.is_empty() && alias.chars().all(|c| c.is_ascii_digit() || c == '.');
    let chosen = installed
        .iter()
        .rev()
        .find(|(_, name)| {
            let name = name.trim_start_matches('v');
            name == alias || (numeric && name.starts_with(&format!("{alias}.")))
        })
        .or_else(|| installed.last())?;
    let bin = versions.join(&chosen.1).join("bin");
    bin.is_dir().then_some(bin)
}

/// `v22.11.0` → `(22, 11, 0)`.
fn semver_of(name: &str) -> Option<(u64, u64, u64)> {
    let mut parts = name.strip_prefix('v')?.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some((major, minor, patch))
}

/// The user's shell: `$SHELL` when it is one of the system's login
/// shells (`/etc/shells`), else macOS's default.
fn shell_program() -> PathBuf {
    let listed = std::fs::read_to_string("/etc/shells").unwrap_or_default();
    std::env::var_os("SHELL")
        .map(PathBuf::from)
        .filter(|shell| known_shell(shell, &listed))
        .unwrap_or_else(|| PathBuf::from("/bin/zsh"))
}

/// True when `shell` is absolute and listed in `etc_shells` (comments and
/// blank lines skipped).
fn known_shell(shell: &Path, etc_shells: &str) -> bool {
    shell.is_absolute()
        && etc_shells
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .any(|line| Path::new(line) == shell)
}

/// The `PATH` a login shell sets up: `shell -l`, reading a constant script from
/// its stdin, prints it between markers; it is killed after `timeout`.
/// It starts from the [`ENV_ALLOWLIST`] and the system `PATH`. Anything
/// else it prints, its errors and its exit status are ignored; `None`
/// when it gave no usable `PATH`.
#[must_use]
pub fn login_shell(shell: &Path, timeout: Duration) -> Option<OsString> {
    let mut command = Command::new(shell);
    allowlisted_env(&mut command);
    let mut child = command
        .env("PATH", SYSTEM_PATH)
        .arg("-l")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    // Far below a pipe's buffer: the write never waits on the shell.
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(SCRIPT);
    }
    let mut stdout = child.stdout.take()?;
    let (send, receive) = std::sync::mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("user-path-read".into())
        .spawn(move || {
            let mut bytes = Vec::new();
            let _ = stdout.read_to_end(&mut bytes);
            let _ = send.send(bytes);
        });
    let output = receive.recv_timeout(timeout).ok();
    // Done or not, the shell goes: a slow startup file never holds it.
    let _ = child.kill();
    let _ = child.wait();
    parse_marked(&String::from_utf8_lossy(&output?))
}

/// The `PATH` between the markers, absolute folders only.
fn parse_marked(output: &str) -> Option<OsString> {
    let start = output.find(MARK)? + MARK.len();
    let end = start + output[start..].find(MARK)?;
    let dirs: Vec<PathBuf> = std::env::split_paths(&output[start..end])
        .filter(|dir| dir.is_absolute())
        .collect();
    (!dirs.is_empty()).then(|| join(dirs))
}

fn dedup(dirs: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    dirs.into_iter()
        .filter(|dir| dir.is_absolute() && seen.insert(dir.clone()))
        .collect()
}

fn join(dirs: Vec<PathBuf>) -> OsString {
    std::env::join_paths(dirs).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(path: &OsStr) -> Vec<PathBuf> {
        std::env::split_paths(path).collect()
    }

    #[test]
    fn the_fallback_adds_homebrew_and_the_home_folders_that_exist() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join(".npm-global/bin")).unwrap();
        let path = combine(
            None,
            Some(OsStr::new("/usr/bin:/bin")),
            Some(home.path()),
            None,
        );
        let dirs = split(&path);
        assert_eq!(dirs[0], PathBuf::from("/usr/bin"));
        assert!(dirs.contains(&PathBuf::from("/opt/homebrew/bin")));
        assert!(dirs.contains(&PathBuf::from("/usr/local/bin")));
        assert!(dirs.contains(&home.path().join(".npm-global/bin")));
        assert!(
            !dirs.contains(&home.path().join(".volta/bin")),
            "missing folders stay out"
        );
    }

    #[test]
    fn the_login_shell_comes_first_and_nothing_repeats_or_is_relative() {
        let path = combine(
            Some(OsStr::new("/custom/bin:/usr/bin:relative")),
            Some(OsStr::new("/usr/bin:/bin")),
            None,
            None,
        );
        let dirs = split(&path);
        assert_eq!(dirs[0], PathBuf::from("/custom/bin"));
        assert_eq!(
            dirs.iter()
                .filter(|d| d.as_path() == Path::new("/usr/bin"))
                .count(),
            1
        );
        assert!(!dirs.iter().any(|d| d.is_relative()));
    }

    #[test]
    fn the_binarys_own_folder_comes_first() {
        let path = prepend_dir(
            Path::new("/opt/tool/bin/codex"),
            OsStr::new("/usr/bin:/opt/tool/bin"),
        );
        assert_eq!(
            split(&path),
            vec![PathBuf::from("/opt/tool/bin"), PathBuf::from("/usr/bin")]
        );
    }

    #[test]
    fn the_copyable_line_names_the_binary_when_the_terminal_wont_find_it() {
        use crate::adapter::snippet_program;
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("codex"), "").unwrap();
        let found = || Some(PathBuf::from("/Users/you/my tools/codex"));
        // Unknown terminal PATH: what the user would type.
        assert_eq!(snippet_program("codex", None, found), "codex");
        // Their terminal finds it: the bare name.
        let path = join(vec![dir.path().to_path_buf()]);
        assert_eq!(snippet_program("codex", Some(&path), found), "codex");
        // It doesn't: the found binary, quoted.
        assert_eq!(
            snippet_program("codex", Some(OsStr::new("/usr/bin:/bin")), found),
            "'/Users/you/my tools/codex'"
        );
        assert_eq!(
            snippet_program("codex", Some(OsStr::new("/usr/bin")), || None),
            "codex"
        );
    }

    #[test]
    fn nvm_follows_its_default_alias_else_the_newest() {
        let nvm = tempfile::tempdir().unwrap();
        for version in ["v18.20.0", "v22.1.0", "v22.11.0", "v9.0.0"] {
            std::fs::create_dir_all(nvm.path().join("versions/node").join(version).join("bin"))
                .unwrap();
        }
        let bin = |v: &str| nvm.path().join("versions/node").join(v).join("bin");
        assert_eq!(
            nvm_bin(nvm.path()),
            Some(bin("v22.11.0")),
            "no alias: the newest"
        );
        std::fs::create_dir_all(nvm.path().join("alias")).unwrap();
        std::fs::write(nvm.path().join("alias/default"), "18\n").unwrap();
        assert_eq!(nvm_bin(nvm.path()), Some(bin("v18.20.0")));
        std::fs::write(nvm.path().join("alias/default"), "v22.1.0").unwrap();
        assert_eq!(nvm_bin(nvm.path()), Some(bin("v22.1.0")));
        std::fs::write(nvm.path().join("alias/default"), "lts/*").unwrap();
        assert_eq!(nvm_bin(nvm.path()), Some(bin("v22.11.0")));
        assert_eq!(nvm_bin(&nvm.path().join("absent")), None);
    }

    #[test]
    fn the_script_uses_the_marker() {
        let script = std::str::from_utf8(SCRIPT).unwrap();
        assert_eq!(script.matches(MARK).count(), 2);
    }

    #[test]
    fn only_a_listed_absolute_shell_runs() {
        let listed = "# List of acceptable shells\n/bin/zsh\n/opt/homebrew/bin/fish\n\n";
        assert!(known_shell(Path::new("/bin/zsh"), listed));
        assert!(known_shell(Path::new("/opt/homebrew/bin/fish"), listed));
        assert!(!known_shell(Path::new("/tmp/evil"), listed));
        assert!(!known_shell(Path::new("zsh"), "zsh\n"));
        assert!(!known_shell(
            Path::new("# List of acceptable shells"),
            listed
        ));
    }

    #[test]
    fn only_the_marked_path_is_read() {
        let out = format!("Welcome!\n{MARK}/a/bin:/b/bin{MARK}more noise");
        assert_eq!(parse_marked(&out), Some(OsString::from("/a/bin:/b/bin")));
        assert_eq!(parse_marked("no marks"), None);
        assert_eq!(parse_marked(&format!("{MARK}{MARK}")), None);
    }

    #[cfg(unix)]
    fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        std::fs::write(&path, body).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[cfg(unix)]
    #[test]
    fn a_login_shell_that_prints_its_path_is_read() {
        let dir = tempfile::tempdir().unwrap();
        // A stand-in shell: noise from its startup files, then what `-c` asks.
        let shell = script(
            dir.path(),
            "shell",
            "#!/bin/sh\necho 'rc noise'\nPATH=/from/login:/usr/bin\nexport PATH\nexec /bin/sh\n",
        );
        // What is read, not how fast: a loaded CI runner can take longer
        // than the app's 2 s to start a shell (it failed there once). The
        // timeout has its own test below.
        assert_eq!(
            login_shell(&shell, Duration::from_secs(30)),
            Some(OsString::from("/from/login:/usr/bin"))
        );
    }

    #[cfg(unix)]
    #[test]
    fn the_login_shell_and_probes_start_from_the_allowlist() {
        let dir = tempfile::tempdir().unwrap();
        // A stand-in shell that reports its whole environment as its PATH.
        let shell = script(
            dir.path(),
            "shell",
            "#!/bin/sh\nPATH=\"/env/$(/usr/bin/env | /usr/bin/cut -d= -f1 | /usr/bin/sort | /usr/bin/tr '\\n' :)\"\nexport PATH\nexec /bin/sh\n",
        );
        let names = login_shell(&shell, Duration::from_secs(30)).unwrap();
        let names = names.to_string_lossy();
        for name in names
            .trim_start_matches("/env/")
            .split(':')
            .filter(|n| !n.is_empty())
        {
            assert!(
                ENV_ALLOWLIST.contains(&name)
                    || ["PATH", "PWD", "SHLVL", "_", "OLDPWD"].contains(&name),
                "the login shell got {name}"
            );
        }
        let probe = command(Path::new("/usr/bin/true"));
        let set: Vec<_> = probe
            .get_envs()
            .filter(|(_, v)| v.is_some())
            .map(|(k, _)| k.to_string_lossy().into_owned())
            .collect();
        assert!(
            set.iter()
                .all(|k| k == "PATH" || ENV_ALLOWLIST.contains(&k.as_str())),
            "{set:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_login_shell_that_hangs_falls_back_within_the_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let shell = script(dir.path(), "shell", "#!/bin/sh\nexec /bin/sleep 30\n");
        let started = std::time::Instant::now();
        let from_shell = login_shell(&shell, Duration::from_millis(300));
        assert_eq!(from_shell, None);
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the shell was not waited out"
        );
        // The fallback still finds Homebrew's folder.
        let path = combine(
            from_shell.as_deref(),
            Some(OsStr::new("/usr/bin:/bin")),
            None,
            None,
        );
        assert!(split(&path).contains(&PathBuf::from("/opt/homebrew/bin")));
    }

    #[cfg(unix)]
    #[test]
    fn a_login_shell_that_fails_or_is_missing_gives_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let shell = script(dir.path(), "shell", "#!/bin/sh\nexit 1\n");
        assert_eq!(login_shell(&shell, LOGIN_SHELL_TIMEOUT), None);
        assert_eq!(
            login_shell(&dir.path().join("absent"), LOGIN_SHELL_TIMEOUT),
            None
        );
    }
}
