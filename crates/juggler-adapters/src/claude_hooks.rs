//! AJ-E2306 (D-40, ADR-0022 decision 4): Claude Code's terminal sessions
//! report their exact state through Claude Code's own hooks, from one
//! per-launch `--settings` file in AI Juggler's folder (never the user's
//! `settings.json`).
//!
//! - **Hooks.** `SessionStart`, `UserPromptSubmit`, `PreToolUse` (for
//!   `AskUserQuestion` only), `PermissionRequest`, `PostToolUse`,
//!   `Notification`, `Stop`, `StopFailure` and `SessionEnd` run
//!   `juggler-agent hook <event>`, which reports and prints nothing: a
//!   `PermissionRequest` hook that prints nothing decides nothing, so the
//!   user answers Claude Code's own dialog. Each has a 5 s timeout.
//! - **Merging.** Claude Code merges hook entries across its settings
//!   levels ("Hook entries merge across settings levels rather than
//!   replacing each other", code.claude.com/docs/en/hooks), so the user's
//!   and the project's own hooks still run (runtime lab 2, 2026-10-25).
//! - **One `--settings`.** Claude Code applies only the **last**
//!   `--settings` (lab 2: with `--settings A --settings B` only B's hooks
//!   ran). So the launch gets exactly one, AI Juggler's file, and every
//!   `--settings` the launch already had is folded into it:
//!   - the launch's own overlay (AJ-E1530's capture) gives only its status
//!     line;
//!   - a user's own (a file, or JSON written in the option) gives all of
//!     its settings, its hooks combined with AI Juggler's (theirs first),
//!     and its status line wins, as it did when it came last.
//!   - A user's own that can't be read, or that holds a setting which can
//!     carry a secret (`env`, a key helper and the like), is never copied
//!     into AI Juggler's folder: the launch is left as it is, and its
//!     state is the usual guess.

use std::path::Path;

use juggler_adapter::adapter::shell_quote;
use juggler_adapter::agent_bridge::BridgeFile;
use juggler_adapter::reports::{ReportsInjection, ReportsLaunch};

/// The per-launch file's name in the launch's folder.
pub const SETTINGS_FILE: &str = "settings.json";

/// Each hook's timeout, seconds (the command itself gives up in 2 s).
pub const HOOK_TIMEOUT_SECS: u64 = 5;

/// The events and their matcher (a tool name), when they take one.
const HOOKS: [(&str, Option<&str>); 9] = [
    ("SessionStart", None),
    ("UserPromptSubmit", None),
    ("PreToolUse", Some("AskUserQuestion")),
    ("PermissionRequest", None),
    ("PostToolUse", None),
    ("Notification", None),
    ("Stop", None),
    ("StopFailure", None),
    ("SessionEnd", None),
];

/// The largest settings file that is folded in.
const OVERLAY_MAX: u64 = 64 * 1024;

/// Whether a setting can carry a secret: `env`, a command that prints one
/// (a `…Helper`), or the cloud sign-in ones (`aws…`). A user's `--settings`
/// holding one is never copied into AI Juggler's folder.
fn may_carry_secret(key: &str) -> bool {
    key == "env" || key.ends_with("Helper") || key.starts_with("aws")
}

/// A settings file's JSON object: a regular file (never a link) of at most
/// [`OVERLAY_MAX`] bytes.
fn settings_file(path: &Path) -> Option<serde_json::Map<String, serde_json::Value>> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    if !meta.is_file() || meta.len() > OVERLAY_MAX {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()?
        .as_object()
        .cloned()
}

/// A user's own `--settings` value as an object: JSON written in the
/// option, or a file.
fn users_settings(value: &str) -> Option<serde_json::Map<String, serde_json::Value>> {
    if value.trim_start().starts_with('{') {
        return serde_json::from_str::<serde_json::Value>(value)
            .ok()?
            .as_object()
            .cloned();
    }
    settings_file(Path::new(value))
}

/// AI Juggler's hooks, one entry per event.
fn our_hooks(launch: &ReportsLaunch) -> serde_json::Map<String, serde_json::Value> {
    let program = shell_quote(&launch.program.to_string_lossy());
    HOOKS
        .iter()
        .map(|(event, matcher)| {
            let mut entry = serde_json::json!({"hooks": [{
                "type": "command",
                "command": format!("{program} hook {event}"),
                "timeout": HOOK_TIMEOUT_SECS,
            }]});
            if let Some(matcher) = matcher {
                entry["matcher"] = serde_json::json!(matcher);
            }
            ((*event).to_owned(), serde_json::json!([entry]))
        })
        .collect()
}

/// The injection for one terminal launch with `args`; `None` leaves the
/// launch as it is (a user's own `--settings` that isn't copied).
#[must_use]
pub fn injection(launch: &ReportsLaunch, args: &[String]) -> Option<ReportsInjection> {
    let mut rest: Vec<String> = Vec::with_capacity(args.len());
    let mut overlay_status = None;
    let mut theirs = serde_json::Map::new();
    let mut words = args.iter();
    while let Some(arg) = words.next() {
        let Some(value) = (arg == "--settings")
            .then(|| words.clone().next())
            .flatten()
        else {
            rest.push(arg.clone());
            continue;
        };
        if Path::new(value).starts_with(&launch.overlays) {
            overlay_status =
                settings_file(Path::new(value)).and_then(|s| s.get("statusLine").cloned());
        } else {
            match users_settings(value) {
                Some(more) if !more.keys().any(|key| may_carry_secret(key)) => {
                    // Later ones override earlier ones, as Claude Code's
                    // last one would.
                    theirs.extend(more);
                }
                // Unreadable, or it could carry a secret: the launch keeps
                // its own settings alone (never copied here).
                _ => return None,
            }
        }
        words.next();
    }
    let mut hooks = match theirs.remove("hooks") {
        Some(serde_json::Value::Object(hooks)) => hooks,
        _ => serde_json::Map::new(),
    };
    for (event, ours) in our_hooks(launch) {
        let mut entries = match hooks.remove(&event) {
            Some(serde_json::Value::Array(entries)) => entries,
            _ => Vec::new(),
        };
        entries.extend(ours.as_array().cloned().unwrap_or_default());
        hooks.insert(event, serde_json::Value::Array(entries));
    }
    let mut settings = serde_json::Value::Object(theirs);
    settings["hooks"] = serde_json::Value::Object(hooks);
    if settings.get("statusLine").is_none()
        && let Some(status_line) = overlay_status
    {
        settings["statusLine"] = status_line;
    }
    let file = launch.dir.join(SETTINGS_FILE);
    let mut out = vec!["--settings".to_owned(), file.to_string_lossy().into_owned()];
    out.extend(rest);
    Some(ReportsInjection {
        args: out,
        files: vec![BridgeFile {
            name: SETTINGS_FILE.to_owned(),
            contents: serde_json::to_string_pretty(&settings).unwrap_or_default(),
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::{SETTINGS_FILE, injection};
    use juggler_adapter::reports::ReportsLaunch;

    fn launch(root: &std::path::Path) -> ReportsLaunch {
        ReportsLaunch {
            program: root.join("App Support/juggler-agent"),
            dir: root.join("overlays/runtime/sess-1"),
            overlays: root.join("overlays"),
        }
    }

    #[test]
    fn the_overlay_settings_are_folded_in_once() {
        let root = tempfile::tempdir().unwrap();
        let overlay = root.path().join("overlays/acct/launch-settings.json");
        std::fs::create_dir_all(overlay.parent().unwrap()).unwrap();
        std::fs::write(
            &overlay,
            r#"{"statusLine":{"type":"command","command":"'/x/juggler-statusline.sh'"},"apiKey":"never-copied"}"#,
        )
        .unwrap();
        let args: Vec<String> = [
            "--model",
            "opus",
            "--settings",
            overlay.to_str().unwrap(),
            "--resume",
            "abc",
        ]
        .map(String::from)
        .to_vec();
        let got = injection(&launch(root.path()), &args).unwrap();
        assert_eq!(got.args.iter().filter(|a| *a == "--settings").count(), 1);
        assert_eq!(&got.args[2..], ["--model", "opus", "--resume", "abc"]);
        assert!(got.args[1].ends_with(&format!("sess-1/{SETTINGS_FILE}")));
        let settings: serde_json::Value = serde_json::from_str(&got.files[0].contents).unwrap();
        assert_eq!(
            settings["statusLine"]["command"],
            "'/x/juggler-statusline.sh'"
        );
        assert!(
            !got.files[0].contents.contains("never-copied"),
            "only the status line"
        );
        let permission = &settings["hooks"]["PermissionRequest"][0]["hooks"][0];
        assert_eq!(
            permission["command"],
            format!(
                "'{}' hook PermissionRequest",
                root.path().join("App Support/juggler-agent").display()
            )
        );
        assert_eq!(permission["timeout"], 5);
        assert_eq!(
            settings["hooks"]["PreToolUse"][0]["matcher"],
            "AskUserQuestion"
        );
    }

    /// Lab 2 (2026-10-25): Claude Code applies only the last `--settings`,
    /// so a user's own is folded into AI Juggler's one file, its hooks
    /// combined with ours, and never passed beside it.
    #[test]
    fn a_users_own_settings_are_folded_into_the_one_file() {
        let root = tempfile::tempdir().unwrap();
        let overlay = root.path().join("overlays/acct/launch-settings.json");
        std::fs::create_dir_all(overlay.parent().unwrap()).unwrap();
        std::fs::write(
            &overlay,
            r#"{"statusLine":{"type":"command","command":"capture"}}"#,
        )
        .unwrap();
        let theirs = root.path().join("theirs.json");
        std::fs::write(
            &theirs,
            r#"{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"echo mine"}]}],"PreCompact":[{"hooks":[{"type":"command","command":"echo pre"}]}]},"statusLine":{"type":"command","command":"my-line"},"model":"opus"}"#,
        )
        .unwrap();
        let args: Vec<String> = [
            "--settings",
            overlay.to_str().unwrap(),
            "--settings",
            theirs.to_str().unwrap(),
            "--settings",
            r#"{"outputStyle":"Explanatory"}"#,
            "--resume",
            "abc",
        ]
        .map(String::from)
        .to_vec();
        let got = injection(&launch(root.path()), &args).unwrap();
        assert_eq!(
            got.args.iter().filter(|a| *a == "--settings").count(),
            1,
            "one --settings"
        );
        assert_eq!(&got.args[2..], ["--resume", "abc"]);
        let settings: serde_json::Value = serde_json::from_str(&got.files[0].contents).unwrap();
        let start = settings["hooks"]["SessionStart"].as_array().unwrap();
        assert_eq!(start.len(), 2, "theirs and ours");
        assert_eq!(start[0]["hooks"][0]["command"], "echo mine");
        assert!(
            start[1]["hooks"][0]["command"]
                .as_str()
                .unwrap()
                .ends_with("hook SessionStart")
        );
        assert_eq!(
            settings["hooks"]["PreCompact"][0]["hooks"][0]["command"],
            "echo pre"
        );
        assert!(settings["hooks"]["PermissionRequest"].is_array());
        assert_eq!(
            settings["statusLine"]["command"], "my-line",
            "theirs came last"
        );
        assert_eq!(settings["model"], "opus");
        assert_eq!(settings["outputStyle"], "Explanatory");
    }

    /// A user's own `--settings` that could carry a secret, or that can't
    /// be read, is never copied: the launch keeps it alone.
    #[test]
    fn a_users_settings_with_a_secret_or_unreadable_are_left_alone() {
        let root = tempfile::tempdir().unwrap();
        for value in [
            r#"{"env":{"ANTHROPIC_API_KEY":"x"}}"#,
            "/nowhere/theirs.json",
        ] {
            let args: Vec<String> = ["--settings", value].map(String::from).to_vec();
            assert!(injection(&launch(root.path()), &args).is_none());
        }
    }
}
