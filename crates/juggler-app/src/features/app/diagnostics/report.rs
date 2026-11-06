//! AJ-E2911 (D-75): Report a bug — what a report says, shown whole before
//! anything leaves the computer, and sent only by the user's click.
//!
//! - [`preview`] gathers the parts (what the failure screen showed, the
//!   app and the system, the last switch's steps, recent errors, the debug
//!   log while it has lines), each scrubbed as Export diagnostics scrubs,
//!   and also of the names this computer knows (Account and App names,
//!   their folders' names, the user's full name).
//! - [`compose`] makes the one text that goes: the user's words, then the
//!   parts they kept. The window shows exactly it; [`open_issue`] opens
//!   GitHub's new-issue page with it filled in (a fixed address; the window
//!   passes no URL), cut to fit a link and saying so. AI Juggler sends
//!   nothing itself.
//! - Never in it: message contents, transcripts, terminal output,
//!   credentials, paths outside `~`, real names.

use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use super::debug_log;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Where a report goes: the project's issues (fixed; never from the
/// window).
pub const ISSUES_NEW: &str = "https://github.com/figurehead-ai/AI-juggler/issues/new";
/// The longest link a browser and GitHub take comfortably.
pub const MAX_URL: usize = 7_500;
/// Error-log lines in a report.
const ERROR_LINES: usize = 50;
/// Debug-log lines in a report.
const DEBUG_LINES: usize = 200;
/// How recent a switch must be to come.
const SWITCH_WITHIN_MS: i64 = 60 * 60 * 1000;

/// What the failure screen showed (none from Help or Settings).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BugContextArgs {
    /// Where the report started: `help`, `settings`, `switch`, `launch`, `error`.
    pub from: String,
    /// The screen's words.
    pub words: Option<String>,
    /// The failure's detail (its stage and the program's words).
    pub detail: Option<String>,
    /// Its code.
    pub code: Option<String>,
    /// When it happened, epoch ms.
    #[ts(type = "number | null")]
    pub at_ms: Option<i64>,
}

/// One part of a report.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BugPartInfo {
    /// Its key (`failure`, `app`, `switch`, `errors`, `debug`).
    pub key: String,
    /// Its heading.
    pub title: String,
    /// What it holds, in a few words.
    pub about: String,
    /// Its text, scrubbed, as it will go.
    pub text: String,
}

/// A report's parts, before the user's words.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BugReportInfo {
    /// Names this report: compose and open read exactly it.
    pub id: String,
    /// The issue's suggested title.
    pub title: String,
    /// The parts, in order.
    pub parts: Vec<BugPartInfo>,
}

/// One field of the bug form, as the link fills it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BugFieldInfo {
    /// The form's field id.
    pub id: String,
    /// The form's label.
    pub label: String,
    /// What it's filled with.
    pub value: String,
}

/// What goes.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BugComposedInfo {
    /// The issue's title.
    pub title: String,
    /// The bug form's fields as the link fills them (cut to fit).
    pub fields: Vec<BugFieldInfo>,
    /// The whole report as one text (what Copy copies).
    pub body: String,
    /// The link's logs were cut to fit (Copy has all of it).
    pub cut: bool,
}

/// The app's facts for the form's own fields.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Facts {
    version: String,
    edition: String,
    system: String,
    tools: String,
}

static PREPARED: Mutex<Option<(BugReportInfo, Facts, String)>> = Mutex::new(None);

/// Names this computer knows that a report must not carry.
fn names(state: &Arc<AppState>) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(full) = juggler_platform::system::user_display_name() {
        out.extend(full.split_whitespace().map(str::to_owned));
        out.push(full);
    }
    if let Ok(storage) = state.storage.lock() {
        if let Ok(accounts) = storage.accounts().list() {
            for account in accounts {
                out.push(account.label.clone());
                for dir in account.state_dir.iter().rev().take(2) {
                    out.push(dir.to_string_lossy().into_owned());
                }
            }
        }
        if let Ok(subscriptions) = storage.subscriptions().list() {
            out.extend(subscriptions.into_iter().filter_map(|s| s.name));
        }
    }
    let tools: Vec<String> = state
        .registry
        .ids()
        .iter()
        .map(|id| id.as_str().to_owned())
        .collect();
    clean_names(out, &tools)
}

/// Words worth scrubbing: three letters or more, never a tool's own
/// name or a common folder's (a tool's folder isn't anyone's name, and
/// "claude exited" must still read), longest first (so "Work Max" goes
/// before "Work").
fn clean_names(mut names: Vec<String>, tools: &[String]) -> Vec<String> {
    const COMMON: &[&str] = &[
        "Library",
        "Application Support",
        "Users",
        "home",
        "work",
        "Work",
        "default",
        "Default",
        "profiles",
        "Profiles",
    ];
    names.retain(|n| {
        let bare = n.trim_start_matches('.');
        n.chars().count() >= 3
            && !COMMON.contains(&n.as_str())
            && !tools
                .iter()
                .any(|t| t == bare || t.starts_with(&format!("{bare}-")))
    });
    names.sort_by_key(|n| std::cmp::Reverse(n.len()));
    names.dedup();
    names
}

fn without_names(text: &str, names: &[String]) -> String {
    names.iter().fold(text.to_owned(), |text, name| {
        text.replace(name.as_str(), "<name>")
    })
}

fn part(key: &str, title: &str, about: &str, text: String) -> BugPartInfo {
    BugPartInfo {
        key: key.to_owned(),
        title: title.to_owned(),
        about: about.to_owned(),
        text,
    }
}

fn last_lines(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

fn facts(state: &Arc<AppState>) -> Facts {
    let app = crate::commands::AppInfo::current();
    let channel = crate::updates::current_channel().as_str();
    let edition = match (app.edition.as_str(), channel) {
        ("nightly", _) => "AI Juggler Nightly",
        ("dev", _) => "Built from source",
        (_, "beta") => "AI Juggler, Beta",
        _ => "AI Juggler, Stable",
    };
    let os = sysinfo::System::long_os_version().unwrap_or_else(|| std::env::consts::OS.to_owned());
    let chip = sysinfo::System::cpu_arch();
    let tools = state
        .provider_versions
        .lock()
        .map(|versions| {
            versions
                .iter()
                .map(|(tool, version)| format!("{tool} {version}"))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    Facts {
        version: app.version,
        edition: edition.to_owned(),
        system: format!("{os}, {chip}"),
        tools,
    }
}

fn facts_text(facts: &Facts) -> String {
    let mut out = format!(
        "AI Juggler {} ({})\n{}",
        facts.version, facts.edition, facts.system
    );
    if !facts.tools.is_empty() {
        let _ = write!(out, "\n{}", facts.tools);
    }
    out
}

fn failure_part(context: &BugContextArgs) -> Option<String> {
    let mut out = String::new();
    if let Some(words) = context.words.as_deref().filter(|w| !w.trim().is_empty()) {
        let _ = writeln!(out, "Shown: {}", words.trim());
    }
    if let Some(detail) = context.detail.as_deref().filter(|w| !w.trim().is_empty()) {
        let _ = writeln!(out, "Detail: {}", detail.trim());
    }
    if let Some(code) = context.code.as_deref().filter(|w| !w.trim().is_empty()) {
        let _ = writeln!(out, "Code: {code}");
    }
    if let Some(at) = context.at_ms {
        let _ = writeln!(out, "At: {at} (epoch ms)");
    }
    (!out.is_empty()).then(|| out.trim_end().to_owned())
}

/// Gathers a report's parts, scrubbed, and keeps it for [`compose`].
///
/// # Errors
///
/// A poisoned lock.
pub fn preview(state: &Arc<AppState>, context: &BugContextArgs) -> AppResult<BugReportInfo> {
    let scrub = super::Scrub::here();
    let names = names(state);
    let clean = |text: &str| without_names(&scrub.text(text), &names);
    let mut parts = Vec::new();
    if let Some(text) = failure_part(context) {
        parts.push(part(
            "failure",
            "What AI Juggler showed",
            "the message and its detail",
            clean(&text),
        ));
    }
    let facts = facts(state);
    let facts = Facts {
        tools: clean(&facts.tools),
        ..facts
    };
    parts.push(part(
        "app",
        "AI Juggler and this computer",
        "versions and the system",
        facts_text(&facts),
    ));
    let now = juggler_core::model::Timestamp::now().millis();
    let switch = debug_log::last_flow("switch", now, SWITCH_WITHIN_MS);
    if !switch.is_empty() {
        parts.push(part(
            "switch",
            "The last switch",
            "its steps, sizes and timings",
            clean(&switch.join("\n")),
        ));
    }
    let errors = std::fs::read_to_string(state.data_dir.join(crate::error_log::RELATIVE))
        .unwrap_or_default();
    if !errors.trim().is_empty() {
        parts.push(part(
            "errors",
            "Recent errors",
            "the last lines of the error log",
            clean(&last_lines(&errors, ERROR_LINES)),
        ));
    }
    let debug = debug_log::text(&state.data_dir, 512 * 1024);
    if !debug.trim().is_empty() {
        parts.push(part(
            "debug",
            "Debug log",
            "its last lines (debug mode)",
            clean(&last_lines(&debug, DEBUG_LINES)),
        ));
    }
    let title = match context.from.as_str() {
        "switch" => "A switch failed",
        "launch" => "A session didn't start",
        "error" => "Something went wrong",
        _ => "",
    };
    let id = format!("{now:x}");
    let report = BugReportInfo {
        id,
        title: title.to_owned(),
        parts,
    };
    let area = match context.from.as_str() {
        "switch" => "Switching to another account or tool",
        "launch" => "Sessions, projects or History",
        _ => "",
    };
    *PREPARED.lock().map_err(|_| AppError::StatePoisoned)? =
        Some((report.clone(), facts, area.to_owned()));
    Ok(report)
}

/// What goes, from the prepared report `id`, the user's words and the
/// parts they kept: the bug form's fields (the logs cut to fit a link) and
/// the whole text for Copy.
///
/// # Errors
///
/// [`AppError::Validation`] when the report is gone (prepare it again).
pub fn compose(id: &str, title: &str, words: &str, keep: &[String]) -> AppResult<BugComposedInfo> {
    let (report, facts, area) = PREPARED
        .lock()
        .map_err(|_| AppError::StatePoisoned)?
        .clone()
        .filter(|(r, ..)| r.id == id)
        .ok_or_else(|| AppError::Validation {
            subject: "report",
            message: "This report is out of date. Open Report a bug again.".into(),
        })?;
    // The user's own words are theirs: kept as typed, keys still removed.
    let words = crate::error_log::without_keys(words.trim());
    let kept = |key: &str| keep.iter().any(|k| k == key);
    let title = if title.trim().is_empty() {
        report.title.clone()
    } else {
        title.trim().to_owned()
    };
    let mut what = words.clone();
    if let Some(failure) = report
        .parts
        .iter()
        .find(|p| p.key == "failure" && kept("failure"))
    {
        if !what.is_empty() {
            what.push_str("\n\n");
        }
        what.push_str(&failure.text);
    }
    let mut fields = vec![field("what-happened", "What happened?", what)];
    if !area.is_empty() {
        fields.push(field("area", "Where in AI Juggler?", area));
    }
    if kept("app") {
        fields.push(field(
            "version",
            "AI Juggler version",
            facts.version.clone(),
        ));
        fields.push(field(
            "edition",
            "Which app and channel?",
            facts.edition.clone(),
        ));
        fields.push(field("macos", "System", facts.system.clone()));
        if !facts.tools.is_empty() {
            fields.push(field("tool-versions", "Tool versions", facts.tools.clone()));
        }
    }
    let logs: Vec<&BugPartInfo> = report
        .parts
        .iter()
        .filter(|p| ["switch", "errors", "debug"].contains(&p.key.as_str()) && kept(&p.key))
        .collect();
    let all_logs = logs_of(&logs, usize::MAX).0;
    let room = MAX_URL.saturating_sub(issue_url(&title, &fields).len() + "&logs=".len());
    let (fitted, cut) = logs_of(&logs, room);
    let mut body = format!(
        "### What happened?\n\n{}\n",
        if fields[0].value.is_empty() {
            "_(not filled in)_"
        } else {
            &fields[0].value
        }
    );
    for f in &fields[1..] {
        let _ = write!(body, "\n### {}\n\n{}\n", f.label, f.value);
    }
    if !all_logs.is_empty() {
        let _ = write!(body, "\n### Logs or screenshots\n\n```\n{all_logs}\n```\n");
        fields.push(field("logs", "Logs or screenshots", fitted));
    }
    Ok(BugComposedInfo {
        title,
        fields,
        body,
        cut,
    })
}

fn field(id: &str, label: &str, value: String) -> BugFieldInfo {
    BugFieldInfo {
        id: id.to_owned(),
        label: label.to_owned(),
        value,
    }
}

const CUT_NOTE: &str = "(cut to fit a link: Copy in AI Juggler's Report a bug has all of it)";

/// The logs field: each part under its title; within `room` encoded
/// (the oldest lines of the longest parts go first); whether it was cut.
fn logs_of(parts: &[&BugPartInfo], room: usize) -> (String, bool) {
    let mut lines: Vec<Vec<String>> = parts
        .iter()
        .map(|p| p.text.lines().map(str::to_owned).collect())
        .collect();
    let render = |lines: &[Vec<String>]| {
        parts
            .iter()
            .zip(lines)
            .map(|(p, text)| format!("# {}\n{}", p.title, text.join("\n")))
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    let mut out = render(&lines);
    if encoded_len(&out) <= room {
        return (out, false);
    }
    while encoded_len(&out) + encoded_len(CUT_NOTE) + 3 > room {
        let Some(longest) = lines
            .iter_mut()
            .filter(|l| l.len() > 1)
            .max_by_key(|l| l.iter().map(String::len).sum::<usize>())
        else {
            break;
        };
        longest.remove(0);
        out = render(&lines);
    }
    out = format!("{CUT_NOTE}\n{out}");
    (out, true)
}

/// `text` as a URL's query value.
fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

fn encoded_len(text: &str) -> usize {
    text.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                1
            } else {
                3
            }
        })
        .sum()
}

/// The bug form's address with `title` and `fields` filled in.
#[must_use]
pub fn issue_url(title: &str, fields: &[BugFieldInfo]) -> String {
    let mut url = format!(
        "{ISSUES_NEW}?template=bug_report.yml&title={}",
        encode(title)
    );
    for f in fields {
        let _ = write!(url, "&{}={}", f.id, encode(&f.value));
    }
    url
}

/// Opens GitHub's new-issue page with the composed report filled in (in
/// the browser; nothing is sent until the user submits it there).
///
/// # Errors
///
/// As [`compose`]; [`AppError::Io`] when the browser can't be opened.
pub fn open_issue(
    app: &tauri::AppHandle,
    id: &str,
    title: &str,
    words: &str,
    keep: &[String],
) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt as _;
    let composed = compose(id, title, words, keep)?;
    app.opener()
        .open_url(issue_url(&composed.title, &composed.fields), None::<&str>)
        .map_err(|e| AppError::Io {
            context: "report",
            message: e.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    // One test at a time sets the prepared report (they share it).
    static ONE: Mutex<()> = Mutex::new(());

    fn prepare(parts: Vec<BugPartInfo>) {
        let facts = Facts {
            version: "0.4.0".into(),
            edition: "AI Juggler, Beta".into(),
            system: "Darwin 26.1, arm64".into(),
            tools: "claude 2.1.0".into(),
        };
        let report = BugReportInfo {
            id: "r1".into(),
            title: "A switch failed".into(),
            parts,
        };
        *PREPARED.lock().unwrap() =
            Some((report, facts, "Switching to another account or tool".into()));
    }

    fn value<'a>(c: &'a BugComposedInfo, id: &str) -> Option<&'a str> {
        c.fields
            .iter()
            .find(|f| f.id == id)
            .map(|f| f.value.as_str())
    }

    #[test]
    fn names_are_scrubbed_longest_first_and_common_folders_kept() {
        let names = clean_names(
            vec![
                "Sam".into(),
                "Sam Rivera".into(),
                ".claude".into(),
                "ab".into(),
                "acme-lab".into(),
            ],
            &["claude-code".into()],
        );
        assert_eq!(names, vec!["Sam Rivera", "acme-lab", "Sam"]);
        assert_eq!(
            without_names("~/Profiles/acme-lab by Sam Rivera and Sam", &names),
            "~/Profiles/<name> by <name> and <name>"
        );
    }

    #[test]
    fn the_form_is_filled_from_the_kept_parts_and_the_logs_cut_to_fit() {
        let _one = ONE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let long: String = (0..2000).map(|i| format!("line {i}\n")).collect();
        prepare(vec![
            part(
                "failure",
                "What AI Juggler showed",
                "",
                "Shown: The switch stopped".into(),
            ),
            part("app", "AI Juggler and this computer", "", String::new()),
            part("errors", "Recent errors", "", long),
        ]);
        let few = compose("r1", "", "It stopped", &["failure".into()]).unwrap();
        assert_eq!(few.title, "A switch failed");
        assert_eq!(
            value(&few, "what-happened"),
            Some("It stopped\n\nShown: The switch stopped")
        );
        assert_eq!(value(&few, "version"), None, "the app's facts weren't kept");
        assert_eq!(value(&few, "logs"), None);
        let all = compose("r1", "Mine", "It stopped", &["app".into(), "errors".into()]).unwrap();
        assert_eq!(value(&all, "version"), Some("0.4.0"));
        assert_eq!(value(&all, "edition"), Some("AI Juggler, Beta"));
        assert_eq!(
            value(&all, "area"),
            Some("Switching to another account or tool")
        );
        assert!(all.cut, "two thousand lines don't fit a link");
        assert!(issue_url(&all.title, &all.fields).len() <= MAX_URL);
        let logs = value(&all, "logs").unwrap();
        assert!(
            logs.contains("line 1999") && !logs.contains("line 0\n"),
            "the newest lines stay"
        );
        assert!(all.body.contains("line 0\n"), "Copy has all of it");
        assert!(
            compose("old", "", "", &[]).is_err(),
            "an old report is refused"
        );
    }

    #[test]
    fn a_key_typed_into_the_words_is_removed() {
        let _one = ONE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        prepare(vec![]);
        let c = compose("r1", "", "my key is sk-FAKE0000NOTAREALKEY111", &[]).unwrap();
        assert!(!c.body.contains("NOTAREALKEY"));
    }

    #[test]
    fn the_address_is_the_bug_form_and_encoded() {
        let url = issue_url("A & B", &[field("logs", "Logs", "x y\n".into())]);
        assert!(url.starts_with("https://github.com/figurehead-ai/AI-juggler/issues/new?template=bug_report.yml&title=A%20%26%20B"));
        assert!(url.ends_with("&logs=x%20y%0A"));
    }
}
