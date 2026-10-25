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
//!   own hooks still run; the runtime lab confirms it for `--settings`
//!   (`docs/qa/runtime-lab.md` step 3).
//! - **One `--settings`.** Claude Code takes the option once, so the file
//!   also carries the status line of the launch's own overlay (the
//!   AJ-E1530 capture, which still runs the user's status line), and that
//!   overlay's `--settings` is replaced, not repeated.

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

/// The largest overlay file whose status line is carried over.
const OVERLAY_MAX: u64 = 64 * 1024;

/// The status line of an AI Juggler overlay settings file (only that key).
fn status_line_of(path: &Path) -> Option<serde_json::Value> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    if !meta.is_file() || meta.len() > OVERLAY_MAX {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value.get("statusLine").cloned()
}

/// The injection for one terminal launch with `args`.
#[must_use]
pub fn injection(launch: &ReportsLaunch, args: &[String]) -> ReportsInjection {
    let mut rest: Vec<String> = Vec::with_capacity(args.len());
    let mut status_line = None;
    let mut words = args.iter();
    while let Some(arg) = words.next() {
        if arg == "--settings"
            && let Some(value) = words.clone().next()
            && Path::new(value).starts_with(&launch.overlays)
        {
            status_line = status_line_of(Path::new(value));
            words.next();
            continue;
        }
        rest.push(arg.clone());
    }
    let program = shell_quote(&launch.program.to_string_lossy());
    let hooks: serde_json::Map<String, serde_json::Value> = HOOKS
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
        .collect();
    let mut settings = serde_json::json!({ "hooks": hooks });
    if let Some(status_line) = status_line {
        settings["statusLine"] = status_line;
    }
    let file = launch.dir.join(SETTINGS_FILE);
    let mut out = vec!["--settings".to_owned(), file.to_string_lossy().into_owned()];
    out.extend(rest);
    ReportsInjection {
        args: out,
        files: vec![BridgeFile {
            name: SETTINGS_FILE.to_owned(),
            contents: serde_json::to_string_pretty(&settings).unwrap_or_default(),
        }],
    }
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
        let got = injection(&launch(root.path()), &args);
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

    #[test]
    fn a_settings_file_elsewhere_is_left_alone() {
        let root = tempfile::tempdir().unwrap();
        let args: Vec<String> = ["--settings", "/somewhere/theirs.json"]
            .map(String::from)
            .to_vec();
        let got = injection(&launch(root.path()), &args);
        assert_eq!(got.args[2..], args[..]);
        let settings: serde_json::Value = serde_json::from_str(&got.files[0].contents).unwrap();
        assert!(settings.get("statusLine").is_none());
    }
}
