//! AJ-E3210 (F-rc3c-3): whose sign-in a Claude Code folder holds, where a
//! Claude app profile and its CLI share it. Started on the folder
//! (`CLAUDE_CONFIG_DIR`), the app's Code tab runs the folder's `claude` on
//! the app's own sign-in, and that `claude` writes it into the folder's
//! state file; a terminal `claude` there writes the CLI's. So:
//!
//! - **The app's own sign-in** is the newest of its records' folders
//!   (`claude-code-sessions/<account>/<org>/`), by their names and their
//!   records' times only: nothing in them is opened (D-63 as amended).
//! - **Who wrote the folder's sign-in last** is the kind of the newest run
//!   there: a live pid file's `entrypoint`, else the newest transcript's
//!   last `entrypoint`. Each saves its whole state, its sign-in with it, as
//!   it works, so the newest run's is the one in the file. Both are read
//!   for sessions already; here only the `entrypoint` word is taken.

use std::io::{Read as _, Seek as _, SeekFrom};
use std::path::{Path, PathBuf};

use juggler_adapter::{SignInNames, SignInWriter};

/// The `entrypoint`s of the Claude app's runs: its Code tab, and its own
/// Agent SDK's (`claude_conversations`' rule).
const DESKTOP: [&str; 2] = ["claude-desktop", "sdk-ts"];
/// A terminal's, and a script's (`claude -p`, the Agent SDKs run from a
/// shell): the folder's own sign-in.
const CLI: [&str; 3] = ["cli", "sdk-cli", "sdk-py"];
/// The most of a transcript's end read for its last `entrypoint`.
const TAIL_MAX: u64 = 64 * 1024;
/// The most transcripts looked at for the newest.
const TRANSCRIPTS_MAX: usize = 20_000;

/// Which kind of run an `entrypoint` says.
#[must_use]
pub fn kind_of(entrypoint: &str) -> SignInWriter {
    if DESKTOP.contains(&entrypoint) {
        SignInWriter::DesktopApp
    } else if CLI.contains(&entrypoint) {
        SignInWriter::Cli
    } else {
        SignInWriter::Unknown
    }
}

/// The sign-in the Claude app's instance `instance_dir` runs its Code
/// sessions under: its one record folder's, else the one whose records
/// changed last. `None` before its first record.
#[must_use]
pub fn desktop_sign_in(instance_dir: &Path) -> Option<SignInNames> {
    let sign_ins = crate::claude_limits::instance_sign_ins(instance_dir);
    let root = crate::claude_live::app_root(instance_dir);
    let newest = match sign_ins.as_slice() {
        [] => return None,
        [one] => one,
        many => many
            .iter()
            .max_by_key(|one| newest_record_ms(&root.join(&one.account).join(&one.org)))?,
    };
    Some(SignInNames {
        subject: newest.account.clone(),
        org: Some(newest.org.clone()),
    })
}

/// The newest change among a record folder's records (their times only).
fn newest_record_ms(dir: &Path) -> i64 {
    std::fs::read_dir(dir).map_or(0, |entries| {
        entries
            .flatten()
            .filter(|entry| {
                let path = entry.path();
                path.extension().is_some_and(|ext| ext == "json")
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with("local_"))
            })
            .filter_map(|entry| modified_ms(&entry.path()))
            .max()
            .unwrap_or(0)
    })
}

fn modified_ms(path: &Path) -> Option<i64> {
    let at = std::fs::metadata(path).ok()?.modified().ok()?;
    i64::try_from(at.duration_since(std::time::UNIX_EPOCH).ok()?.as_millis()).ok()
}

/// The last `entrypoint` a transcript names, from its last
/// [`TAIL_MAX`] bytes (the word only).
#[must_use]
pub fn last_entrypoint(transcript: &Path) -> Option<String> {
    let mut file = std::fs::File::open(transcript).ok()?;
    let len = file.metadata().ok()?.len();
    file.seek(SeekFrom::Start(len.saturating_sub(TAIL_MAX)))
        .ok()?;
    let mut tail = Vec::new();
    file.take(TAIL_MAX).read_to_end(&mut tail).ok()?;
    last_entrypoint_in(&String::from_utf8_lossy(&tail))
}

/// [`last_entrypoint`] of some text.
#[must_use]
pub fn last_entrypoint_in(text: &str) -> Option<String> {
    const KEY: &str = "\"entrypoint\"";
    let at = text.rfind(KEY)? + KEY.len();
    let rest = text[at..].trim_start().strip_prefix(':')?.trim_start();
    let word = rest.strip_prefix('"')?;
    let end = word.find('"')?;
    let word = &word[..end];
    (!word.is_empty() && word.len() <= 32 && word.bytes().all(|b| b.is_ascii_graphic()))
        .then(|| word.to_owned())
}

/// The newest transcript in a folder's `projects/` (by when it changed).
fn newest_transcript(state_dir: &Path) -> Option<(i64, PathBuf)> {
    let projects = std::fs::read_dir(state_dir.join("projects")).ok()?;
    let mut newest: Option<(i64, PathBuf)> = None;
    let mut seen = 0usize;
    for project in projects.flatten() {
        if !project.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        let Ok(files) = std::fs::read_dir(project.path()) else {
            continue;
        };
        for file in files.flatten() {
            seen += 1;
            if seen > TRANSCRIPTS_MAX {
                return newest;
            }
            let path = file.path();
            if path.extension().is_none_or(|ext| ext != "jsonl") {
                continue;
            }
            if let Some(at) = modified_ms(&path)
                && newest.as_ref().is_none_or(|(was, _)| at > *was)
            {
                newest = Some((at, path));
            }
        }
    }
    newest
}

/// Which kind of run wrote the sign-in in `state_dir`'s state last: the
/// newest run there whose `entrypoint` says (a live one's pid file, else
/// the newest transcript). Kept while the folder's state file is
/// unchanged (every run writes it): the Accounts are read every half
/// minute while shown, and the transcripts' walk is only needed again
/// after a run.
#[must_use]
pub fn writer(state_dir: &Path) -> SignInWriter {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static KEPT: OnceLock<Mutex<HashMap<PathBuf, (i64, SignInWriter)>>> = OnceLock::new();
    let Some(stamp) = modified_ms(&crate::claude_code::claude_state_file(state_dir)) else {
        return writer_now(state_dir);
    };
    let kept = KEPT.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(kept) = kept.lock()
        && let Some((at, kind)) = kept.get(state_dir)
        && *at == stamp
    {
        return *kind;
    }
    let kind = writer_now(state_dir);
    if let Ok(mut kept) = kept.lock() {
        kept.insert(state_dir.to_path_buf(), (stamp, kind));
    }
    kind
}

/// [`writer`], read now.
fn writer_now(state_dir: &Path) -> SignInWriter {
    let mut runs: Vec<(i64, SignInWriter)> = crate::claude_live::pid_files(state_dir)
        .iter()
        .filter_map(|path| {
            let kind = kind_of(
                crate::claude_live::read_pid_file(path)?
                    .entrypoint
                    .as_deref()?,
            );
            Some((modified_ms(path)?, kind))
        })
        .collect();
    if let Some((at, path)) = newest_transcript(state_dir)
        && let Some(entry) = last_entrypoint(&path)
    {
        runs.push((at, kind_of(&entry)));
    }
    runs.into_iter()
        .filter(|(_, kind)| *kind != SignInWriter::Unknown)
        .max_by_key(|(at, _)| *at)
        .map_or(SignInWriter::Unknown, |(_, kind)| kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(path: &Path, text: &str, at_s: u64) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
        let at = std::time::UNIX_EPOCH + std::time::Duration::from_secs(at_s);
        std::fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(at)
            .unwrap();
    }

    #[test]
    fn the_apps_sign_in_is_its_newest_record_folders() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            desktop_sign_in(dir.path()),
            None,
            "no record: not known yet"
        );
        let root = dir.path().join("claude-code-sessions");
        touch(&root.join("acct-a/org-team/local_1.json"), "{}", 1_000);
        assert_eq!(
            desktop_sign_in(dir.path()),
            Some(SignInNames {
                subject: "acct-a".into(),
                org: Some("org-team".into())
            })
        );
        touch(&root.join("acct-a/org-me/local_2.json"), "{}", 2_000);
        assert_eq!(
            desktop_sign_in(dir.path()).unwrap().org.as_deref(),
            Some("org-me"),
            "signed in again: the newest records are"
        );
    }

    #[test]
    fn the_last_entrypoint_is_read_from_the_end() {
        assert_eq!(
            last_entrypoint_in(
                r#"{"entrypoint":"cli","x":1}
{"type":"user","entrypoint": "claude-desktop"}"#
            )
            .as_deref(),
            Some("claude-desktop")
        );
        assert_eq!(last_entrypoint_in(r#"{"entrypoint":""}"#), None);
        assert_eq!(last_entrypoint_in("no word"), None);
    }

    #[test]
    fn the_newest_run_says_who_wrote_the_sign_in() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(writer(dir.path()), SignInWriter::Unknown);
        let projects = dir.path().join("projects/-w-p");
        touch(
            &projects.join("a.jsonl"),
            r#"{"type":"user","entrypoint":"cli"}"#,
            1_000,
        );
        assert_eq!(writer(dir.path()), SignInWriter::Cli);
        touch(
            &projects.join("b.jsonl"),
            r#"{"type":"user","entrypoint":"claude-desktop"}"#,
            2_000,
        );
        assert_eq!(writer(dir.path()), SignInWriter::DesktopApp);
        // A terminal run now, still open: its pid file is the newest.
        touch(
            &dir.path().join("sessions/77.json"),
            r#"{"pid":77,"sessionId":"s-1","entrypoint":"cli"}"#,
            3_000,
        );
        assert_eq!(writer(dir.path()), SignInWriter::Cli);
    }
}
