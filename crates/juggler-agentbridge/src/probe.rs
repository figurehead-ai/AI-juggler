//! AJ-E3013 (docs/research/desktop-pool-members.md §7.0, §7.4, lab proofs
//! C2 and C3): the probe, in debug builds only. This module is compiled
//! only with `debug_assertions`; a release `juggler-agent` refuses both
//! flags ("not in this build") and the release binaries' strings check
//! (`scripts/release_seams_check.sh`) refuses [`MARKER`].
//!
//! - `juggler-agent --mcp --probe <out-file>` serves one MCP tool, `probe`.
//!   Every request's `_meta` keys are appended as it comes, with this
//!   process's pid: the keys, the keys one object down (Codex's strings
//!   name an `x-codex-turn-metadata` with `threadId`, `turnId` and
//!   `callId`, so an id may sit there), and the values of the id keys
//!   [`META_IDS`] only, never another value; each `probe` call appends
//!   this process's environment variable **names** (never a value, but for
//!   the three session and thread ids [`IDS`], which are ids, not
//!   secrets), its pid and parent chain, and the `initialize`
//!   `clientInfo`. The file is created `0600` (never over a link), only in
//!   a folder under `/tmp/aij-lab…` or `~/lab`.
//! - `juggler-agent --probe-shell` prints `CODEX_THREAD_ID` (when set) and
//!   the parent chain, nothing else.
//!
//! Nothing here reads a key, a token or a credential file: the
//! environment's names, three ids, the process table's pids and names.

use std::collections::BTreeMap;
use std::io::{BufRead, Write};
// The module is unix-only (`lib.rs`): its one use of the unix API.
use std::os::unix::{fs::OpenOptionsExt as _, process::parent_id};
use std::path::{Component, Path, PathBuf};

use serde_json::{Value, json};

/// What every line the probe writes starts with (and what a release
/// binary must never contain).
pub const MARKER: &str = "AIJ_PROBE_V1";

/// The environment variables whose values are written: ids, not secrets.
pub const IDS: [&str; 3] = [
    "CLAUDE_CODE_SESSION_ID",
    "CLAUDE_CODE_HOST_SESSION_ID",
    "CODEX_THREAD_ID",
];

/// The `_meta` keys whose values are written, at its top or one object
/// down, when the value looks like an id (letters, digits and `-_.:`, at
/// most 128): ids, not secrets.
/// A lease or a token is written as a key only.
pub const META_IDS: [&str; 10] = [
    "threadId",
    "thread_id",
    "turnId",
    "turn_id",
    "sessionId",
    "session_id",
    "conversationId",
    "conversation_id",
    "callId",
    "call_id",
];

/// The longest JSON text read as an object inside `_meta`.
const META_TEXT_MAX: usize = 4096;

/// The probe's MCP server name (the lab's `[mcp_servers.juggler-probe]`).
pub const SERVER: &str = "juggler-probe";

/// The most ancestors listed.
const CHAIN_MAX: usize = 16;

/// Where a probe may write `path`: its folder exists and resolves under
/// `/tmp/aij-lab…` (the folder itself or one inside it) or under `home`'s
/// `lab`. The resolved file path.
///
/// # Errors
///
/// Why not, in words.
pub fn lab_file(path: &Path, home: Option<&Path>) -> Result<PathBuf, String> {
    const ONLY: &str = "the probe writes only in a folder under /tmp/aij-lab… or ~/lab";
    let name = path
        .file_name()
        .filter(|name| !name.is_empty())
        .ok_or("name the file the probe writes")?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or(ONLY)?;
    let folder = std::fs::canonicalize(parent).map_err(|_| ONLY.to_owned())?;
    let mut parts = folder.components();
    let in_tmp_lab = matches!(parts.next(), Some(Component::RootDir))
        && {
            let first = parts.next();
            let second = if first == Some(Component::Normal("private".as_ref())) {
                parts.next()
            } else {
                first
            };
            second == Some(Component::Normal("tmp".as_ref()))
        }
        && parts.next().is_some_and(|lab| {
            lab.as_os_str()
                .to_str()
                .is_some_and(|lab| lab.starts_with("aij-lab"))
        });
    let in_home_lab = home
        .and_then(|home| std::fs::canonicalize(home.join("lab")).ok())
        .is_some_and(|lab| folder.starts_with(lab));
    if in_tmp_lab || in_home_lab {
        Ok(folder.join(name))
    } else {
        Err(ONLY.to_owned())
    }
}

/// Opens the probe's file for appending: created `0600` when new (never
/// through a link: `create_new` doesn't follow one), else a regular file
/// already there.
///
/// # Errors
///
/// When it's a link or another kind of file, or the system's error.
fn open(file: &Path) -> Result<std::fs::File, String> {
    match std::fs::symlink_metadata(file) {
        Ok(meta) if meta.file_type().is_file() => std::fs::OpenOptions::new()
            .append(true)
            .open(file)
            .map_err(|error| error.to_string()),
        Ok(_) => Err("the probe's file is a link or not a file".to_owned()),
        Err(_) => std::fs::OpenOptions::new()
            .append(true)
            .create_new(true)
            .mode(0o600)
            .open(file)
            .map_err(|error| error.to_string()),
    }
}

/// One process on the chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    /// Its pid.
    pub pid: u32,
    /// Its command's name, as `ps` says it.
    pub name: String,
}

/// What a step is named when `/bin/ps` couldn't be run.
pub const UNNAMED: &str = "(ps can't run here)";

/// This process and its ancestors (nearest first), from one reading of
/// the process table (`/bin/ps`, its environment cleared). `/bin/ps` is
/// setuid root, and a sandboxed process (Codex's seatbelt, the C2 lab's
/// guard) can't run it: then just this process and its parent, by pid
/// ([`UNNAMED`]), and the lab driver, outside the sandbox, names the rest.
#[must_use]
pub fn chain() -> Vec<Step> {
    let read = std::process::Command::new("/bin/ps")
        .args(["-A", "-o", "pid=,ppid=,comm="])
        .env_clear()
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output();
    let chain = read
        .map(|out| chain_from(&String::from_utf8_lossy(&out.stdout), std::process::id()))
        .unwrap_or_default();
    if chain.is_empty() {
        let step = |pid| Step {
            pid,
            name: UNNAMED.to_owned(),
        };
        vec![step(std::process::id()), step(parent_id())]
    } else {
        chain
    }
}

/// The chain from `start` up, in `ps`'s `pid ppid comm` lines.
#[must_use]
pub fn chain_from(table: &str, start: u32) -> Vec<Step> {
    let rows: BTreeMap<u32, (u32, String)> = table
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let pid = words.next()?.parse().ok()?;
            let parent = words.next()?.parse().ok()?;
            let name = words.collect::<Vec<_>>().join(" ");
            Some((pid, (parent, name)))
        })
        .collect();
    let mut out = Vec::new();
    let mut at = start;
    while out.len() < CHAIN_MAX {
        let Some((parent, name)) = rows.get(&at) else {
            break;
        };
        out.push(Step {
            pid: at,
            name: name.clone(),
        });
        if *parent == 0 || *parent == at {
            break;
        }
        at = *parent;
    }
    out
}

/// This process's environment: every name, and the values of [`IDS`]
/// only.
#[must_use]
pub fn environment() -> (Vec<String>, BTreeMap<String, String>) {
    let mut names: Vec<String> = std::env::vars_os()
        .map(|(name, _)| name.to_string_lossy().into_owned())
        .collect();
    names.sort();
    let ids = IDS
        .iter()
        .filter_map(|id| Some(((*id).to_owned(), std::env::var(id).ok()?)))
        .collect();
    (names, ids)
}

/// `value` when it is a short id: letters, digits and `-_.:`, at most 128.
fn id_like(value: &Value) -> Option<&str> {
    let text = value.as_str()?;
    let fits = !text.is_empty()
        && text.len() <= 128
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:".contains(&byte));
    fits.then_some(text)
}

/// What the probe writes of a request's `_meta`: `meta_keys` (its keys),
/// `meta_nested` (for each key, the value's kind and, for an object or a
/// short JSON text holding one, its keys) and `meta_ids` (the
/// [`META_IDS`] values, at the top or one object down, by their path).
#[must_use]
pub fn meta_view(params: &Value) -> serde_json::Map<String, Value> {
    let meta = params.get("_meta").and_then(Value::as_object);
    let mut nested = serde_json::Map::new();
    let mut ids = serde_json::Map::new();
    for (key, value) in meta.into_iter().flatten() {
        if META_IDS.contains(&key.as_str())
            && let Some(id) = id_like(value)
        {
            ids.insert(key.clone(), json!(id));
        }
        let parsed = value
            .as_str()
            .filter(|text| text.len() <= META_TEXT_MAX)
            .and_then(|text| serde_json::from_str::<Value>(text).ok())
            .filter(Value::is_object);
        let (kind, inner) = match (value, parsed.as_ref()) {
            (Value::Object(inner), _) => ("object", Some(inner)),
            (_, Some(Value::Object(inner))) => ("json-text", Some(inner)),
            (Value::String(_), _) => ("string", None),
            (Value::Array(_), _) => ("array", None),
            (Value::Number(_), _) => ("number", None),
            (Value::Bool(_), _) => ("bool", None),
            (Value::Null, _) => ("null", None),
        };
        let mut keys: Vec<&String> = inner.into_iter().flat_map(|inner| inner.keys()).collect();
        keys.sort();
        for (sub, value) in inner.into_iter().flatten() {
            if META_IDS.contains(&sub.as_str())
                && let Some(id) = id_like(value)
            {
                ids.insert(format!("{key}.{sub}"), json!(id));
            }
        }
        nested.insert(key.clone(), json!({"kind": kind, "keys": keys}));
    }
    let mut top: Vec<&String> = meta.into_iter().flat_map(|meta| meta.keys()).collect();
    top.sort();
    let mut view = serde_json::Map::new();
    view.insert("meta_keys".to_owned(), json!(top));
    view.insert("meta_nested".to_owned(), Value::Object(nested));
    view.insert("meta_ids".to_owned(), Value::Object(ids));
    view
}

fn chain_json(chain: &[Step]) -> Value {
    Value::Array(
        chain
            .iter()
            .map(|step| json!({"pid": step.pid, "name": step.name}))
            .collect(),
    )
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_millis())
}

/// The probe's MCP server on `input` and `output`, writing to `out`
/// (checked with [`lab_file`] against `home`) until `input` ends.
///
/// # Errors
///
/// Why it doesn't start (the file's place or kind), or a write failing.
pub fn serve(
    out: &Path,
    home: Option<&Path>,
    input: impl BufRead,
    mut output: impl Write,
) -> Result<(), String> {
    let file = lab_file(out, home)?;
    let mut sink = open(&file)?;
    let mut client_info = Value::Null;
    for line in input.lines() {
        let line = line.map_err(|error| error.to_string())?;
        let Ok(request) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        let method = request
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let params = request.get("params").cloned().unwrap_or(Value::Null);
        let meta = meta_view(&params);
        let mut seen =
            json!({"marker": MARKER, "at": now_ms(), "pid": std::process::id(), "request": method});
        if let Some(seen) = seen.as_object_mut() {
            seen.extend(meta.clone());
        }
        writeln!(sink, "{seen}").map_err(|error| error.to_string())?;
        if method == "initialize" {
            client_info = params.get("clientInfo").cloned().unwrap_or(Value::Null);
        }
        let Some(id) = request.get("id").cloned() else {
            continue; // a notification
        };
        let result = match method.as_str() {
            "initialize" => json!({
                "protocolVersion": params.get("protocolVersion").cloned().unwrap_or(json!("2025-06-18")),
                "capabilities": {"tools": {"listChanged": false}},
                "serverInfo": {"name": SERVER, "version": env!("CARGO_PKG_VERSION")},
            }),
            "ping" => json!({}),
            "tools/list" => json!({"tools": [{
                "name": "probe",
                "description": "Lab only: records what this server can see of its caller (environment names, its process chain, the client's name, request metadata keys).",
                "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false},
                "annotations": {"readOnlyHint": true},
            }]}),
            "tools/call" if params.get("name").and_then(Value::as_str) == Some("probe") => {
                let (names, ids) = environment();
                let mut record = json!({
                    "marker": MARKER,
                    "at": now_ms(),
                    "pid": std::process::id(),
                    "chain": chain_json(&chain()),
                    "env_names": names,
                    "ids": ids,
                    "client_info": client_info,
                });
                if let Some(record) = record.as_object_mut() {
                    record.extend(meta);
                }
                writeln!(sink, "{record}").map_err(|error| error.to_string())?;
                json!({"content": [{"type": "text", "text": "recorded"}]})
            }
            _ => {
                let error = json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": "not something the probe does"}});
                writeln!(output, "{error}").map_err(|error| error.to_string())?;
                output.flush().map_err(|error| error.to_string())?;
                continue;
            }
        };
        let answer = json!({"jsonrpc": "2.0", "id": id, "result": result});
        writeln!(output, "{answer}").map_err(|error| error.to_string())?;
        output.flush().map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// `--probe-shell`'s lines: `CODEX_THREAD_ID` when set, then the chain.
#[must_use]
pub fn shell_lines() -> Vec<String> {
    let mut lines = Vec::new();
    if let Ok(thread) = std::env::var("CODEX_THREAD_ID") {
        lines.push(format!("CODEX_THREAD_ID={thread}"));
    }
    lines.extend(
        chain()
            .into_iter()
            .map(|step| format!("{} {}", step.pid, step.name)),
    );
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chain_is_read_from_pid_up_and_bounded() {
        let table = "  1     0 /sbin/launchd\n 50     1 /Applications/Codex.app/Contents/MacOS/Codex\n 60    50 codex app-server\n 70    60 juggler-agent\n";
        let chain = chain_from(table, 70);
        let pids: Vec<u32> = chain.iter().map(|step| step.pid).collect();
        assert_eq!(pids, [70, 60, 50, 1]);
        assert_eq!(chain[1].name, "codex app-server");
        assert!(chain_from(table, 999).is_empty());
        let looped = "5 6 a\n6 5 b\n";
        assert!(chain_from(looped, 5).len() <= CHAIN_MAX);
    }

    #[test]
    fn meta_shows_keys_one_object_down_and_id_values_only() {
        let params = json!({"_meta": {
            "progressToken": 7,
            "x-codex-turn-metadata": {"threadId": "thr-1", "turnId": "t-2", "note": "keep-out"},
            "as-text": "{\"callId\":\"c-3\",\"token\":\"keep-out\"}",
            "codex/connectionLease": "lease-keep-out",
            "threadId": "has space",
        }});
        let view = Value::Object(meta_view(&params));
        let text = view.to_string();
        assert!(!text.contains("keep-out"), "no value but an id's: {text}");
        assert_eq!(view["meta_ids"]["x-codex-turn-metadata.threadId"], "thr-1");
        assert_eq!(view["meta_ids"]["x-codex-turn-metadata.turnId"], "t-2");
        assert_eq!(view["meta_ids"]["as-text.callId"], "c-3");
        assert!(view["meta_ids"].get("threadId").is_none(), "not id-like");
        assert_eq!(view["meta_nested"]["as-text"]["kind"], "json-text");
        assert_eq!(
            view["meta_nested"]["codex/connectionLease"]["kind"],
            "string"
        );
        assert_eq!(
            view["meta_nested"]["x-codex-turn-metadata"]["keys"],
            json!(["note", "threadId", "turnId"])
        );
        assert_eq!(view["meta_keys"].as_array().map(Vec::len), Some(5));
        let none = Value::Object(meta_view(&json!({})));
        assert_eq!(none["meta_keys"], json!([]));
    }

    #[test]
    fn it_writes_only_in_a_lab_folder() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join("lab/x")).unwrap();
        assert!(lab_file(&home.path().join("lab/x/out.jsonl"), Some(home.path())).is_ok());
        assert!(lab_file(&home.path().join("out.jsonl"), Some(home.path())).is_err());
        assert!(lab_file(&home.path().join("labs/out.jsonl"), Some(home.path())).is_err());
        assert!(lab_file(Path::new("/tmp/out.jsonl"), None).is_err());
        assert!(lab_file(Path::new("out.jsonl"), None).is_err());
        assert!(
            lab_file(Path::new("/tmp/aij-lab-no-such-folder/out.jsonl"), None).is_err(),
            "its folder must be there"
        );
    }
}
