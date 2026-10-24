//! AJ-E2101 (D-37): Codex's open threads, from the files its processes
//! hold open. The Codex app serves its threads from one `codex
//! app-server`, which holds `<CODEX_HOME>/thread-writer-locks/<id>.lock`
//! and the thread's rollout open while the thread is loaded; a lock file
//! nobody holds is left over, not open. A `codex` session in a terminal
//! holds its rollout open too. Only the paths are looked at, never the
//! files' contents.

use std::path::{Path, PathBuf};

use juggler_adapter::live::{HostApp, HostKind};

/// Whether `text` is a thread id (a UUID's shape).
fn is_thread_id(text: &str) -> bool {
    text.len() == 36
        && text.char_indices().all(|(at, c)| match at {
            8 | 13 | 18 | 23 => c == '-',
            _ => c.is_ascii_hexdigit(),
        })
}

/// The thread a held file belongs to, when it is a writer lock or a
/// rollout inside `profile_dir`.
#[must_use]
pub fn thread_of(profile_dir: &Path, path: &Path) -> Option<String> {
    let inside = path.strip_prefix(profile_dir).ok()?;
    let name = inside.file_name()?.to_str()?;
    let first = inside.components().next()?.as_os_str().to_str()?;
    let id = match first {
        "thread-writer-locks" => name.strip_suffix(".lock")?,
        "sessions" | "archived_sessions" if name.starts_with("rollout-") => {
            let stem = name.strip_suffix(".zst").unwrap_or(name);
            let stem = stem.strip_suffix(".jsonl")?;
            stem.get(stem.len().checked_sub(36)?..)?
        }
        _ => return None,
    };
    is_thread_id(id).then(|| id.to_owned())
}

/// The threads the files a Codex process holds open (`open_files`) name,
/// in `profile_dir`: each once.
#[must_use]
pub fn held_sessions(profile_dir: &Path, open_files: &[PathBuf]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for path in open_files {
        if let Some(id) = thread_of(profile_dir, path)
            && !out.contains(&id)
        {
            out.push(id);
        }
    }
    out
}

/// Whether a process is the Codex app's server (`codex … app-server`),
/// which holds its loaded threads open.
#[must_use]
pub fn is_session_server(exe: Option<&Path>, args: &[String]) -> bool {
    let codex = exe.is_some_and(|exe| exe.file_name().is_some_and(|name| name == "codex"))
        || args.first().is_some_and(|first| {
            Path::new(first)
                .file_name()
                .is_some_and(|name| name == "codex")
        });
    codex && args.iter().skip(1).any(|arg| arg == "app-server")
}

/// The Codex app, as data: the Dock calls it `ChatGPT`, and a thread opens
/// with its documented link.
#[must_use]
pub fn host_apps() -> Vec<HostApp> {
    vec![HostApp {
        id: "codex-app".to_owned(),
        name: "ChatGPT".to_owned(),
        bundle: "ChatGPT.app".to_owned(),
        bundle_id: Some("com.openai.codex".to_owned()),
        kind: HostKind::Desktop,
        instance_arg: None,
        default_instance: None,
        thread_link: Some("codex://threads/{id}".to_owned()),
        folder_link: None,
        folder_command: None,
        tab_script: None,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "019a1b2c-3d4e-7f80-9a1b-2c3d4e5f6a7b";
    const OTHER: &str = "019a1b2c-3d4e-7f80-9a1b-2c3d4e5f6a7c";

    #[test]
    fn held_locks_and_rollouts_name_their_threads_once() {
        let home = Path::new("/Users/you/.codex");
        let open = vec![
            home.join(format!("thread-writer-locks/{ID}.lock")),
            home.join(format!(
                "sessions/2026/10/24/rollout-2026-10-24T08-12-34-{ID}.jsonl"
            )),
            home.join(format!(
                "sessions/2026/10/24/rollout-2026-10-24T08-12-34-{OTHER}.jsonl.zst"
            )),
            home.join("state_5.sqlite"),
            home.join("thread-writer-locks/not-an-id.lock"),
            // Another home's lock is not this profile's.
            PathBuf::from(format!(
                "/Users/you/.codex-work/thread-writer-locks/{ID}.lock"
            )),
            home.join(format!("tmp/{ID}.lock")),
        ];
        assert_eq!(
            held_sessions(home, &open),
            vec![ID.to_owned(), OTHER.to_owned()]
        );
    }

    #[test]
    fn only_the_app_server_is_a_session_server() {
        let exe = Path::new("/Applications/ChatGPT.app/Contents/Resources/codex-cli/codex");
        let args = |list: &[&str]| list.iter().map(|a| (*a).to_owned()).collect::<Vec<_>>();
        assert!(is_session_server(
            Some(exe),
            &args(&[
                "codex",
                "-c",
                "x=1",
                "app-server",
                "--analytics-default-enabled"
            ])
        ));
        assert!(!is_session_server(Some(exe), &args(&["codex", "sandbox"])));
        assert!(!is_session_server(
            Some(exe),
            &args(&["codex", "resume", ID])
        ));
        assert!(!is_session_server(
            Some(Path::new("/bin/zsh")),
            &args(&["zsh", "app-server"])
        ));
    }
}
