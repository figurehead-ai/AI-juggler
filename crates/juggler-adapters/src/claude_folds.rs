//! 0.3.5: the line the Claude app's Code tab draws for a run of work (the
//! tool calls between two texts), titled by its own rules, read from the
//! app's code (`docs/research/native-conversation-display.md` §1):
//!
//! - one call: a command's own description with its verbs put into the
//!   past tense ("Record X and commit Y" → "Recorded X and committed Y"),
//!   only when it opens with a known verb, else as written; with no
//!   description the command itself, and "Ran a command" with neither.
//!   A read is "Read <file>", a write or edit "Created" or "Updated"
//!   <file> (from its result), a subagent "Ran agent <task>", a web
//!   search "Searched web <words>";
//! - several calls: one phrase per kind of call, in the order each kind
//!   first came ("ran 3 commands", "read a file"), counting distinct
//!   commands and files; when every file the run touched is one file,
//!   it is named ("read accounts-screen-light.png"). The first phrase is
//!   capitalised and the phrases joined with ", ";
//! - while the run is going, the "-ing" form ("Recording X", "Reading
//!   a.ts").
//!
//! Pure: no file, no clock. The tool names are Claude Code's own, so
//! this lives with its adapter.

use crate::conversation_feed::{Step, StepRun};

/// A run's title, done or `running`.
#[must_use]
pub fn title(run: &StepRun, running: bool) -> String {
    match run.steps.as_slice() {
        [] => (if running { "Thinking" } else { "Thought" }).to_owned(),
        [step] => one(step, running),
        steps if running => steps
            .last()
            .map_or_else(String::new, |step| one(step, true)),
        steps => several(steps),
    }
}

/// What kind of call a step is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Command,
    Read,
    Edit,
    SearchCode,
    SearchFiles,
    Agent,
    Web,
    Other,
}

fn kind_of(tool: &str) -> Kind {
    match tool {
        "Bash" => Kind::Command,
        "Read" => Kind::Read,
        "Write" | "Edit" | "MultiEdit" | "NotebookEdit" => Kind::Edit,
        "Grep" => Kind::SearchCode,
        "Glob" => Kind::SearchFiles,
        "Task" | "Agent" => Kind::Agent,
        "WebSearch" => Kind::Web,
        _ => Kind::Other,
    }
}

fn one(step: &Step, running: bool) -> String {
    let input = &step.input;
    let pick = |done: &str, going: &str| if running { going } else { done }.to_owned();
    let name = input.file_path.as_deref().map(file_name);
    match kind_of(&step.tool) {
        Kind::Command => match (&input.description, &input.command) {
            (Some(description), _) => {
                tensed(description, if running { Form::Going } else { Form::Done })
            }
            (None, Some(command)) => short(command),
            (None, None) => pick("Ran a command", "Running a command"),
        },
        Kind::Read => with(pick("Read", "Reading"), name.as_deref(), "a file"),
        Kind::Edit => {
            let made = step
                .result
                .as_ref()
                .and_then(|result| result.kind.as_deref());
            let verb = match (running && step.result.is_none(), made, step.tool.as_str()) {
                (true, _, "Write") => "Writing",
                (true, _, _) => "Editing",
                (false, Some("create"), _) | (false, None, "Write") => "Created",
                (false, _, _) => "Updated",
            };
            with(verb.to_owned(), name.as_deref(), "a file")
        }
        Kind::Agent => with(
            pick("Ran agent", "Running agent"),
            input.description.as_deref(),
            "",
        ),
        Kind::Web => with(
            pick("Searched web", "Searching web"),
            input.query.as_deref(),
            "",
        ),
        Kind::SearchCode => with(
            pick("Searched code for", "Searching code for"),
            input.pattern.as_deref(),
            "a pattern",
        ),
        Kind::SearchFiles => with(
            pick("Searched for files", "Searching for files"),
            input.pattern.as_deref(),
            "",
        ),
        Kind::Other => capitalised(&step.line),
    }
}

/// `verb detail`, or `verb fallback` (just `verb` when that is empty).
fn with(verb: String, detail: Option<&str>, fallback: &str) -> String {
    match detail.map(str::trim).filter(|detail| !detail.is_empty()) {
        Some(detail) => format!("{verb} {detail}"),
        None if fallback.is_empty() => verb,
        None => format!("{verb} {fallback}"),
    }
}

fn several(steps: &[Step]) -> String {
    let mut kinds: Vec<(Kind, Vec<String>)> = Vec::new();
    let mut files: Vec<&str> = Vec::new();
    for step in steps {
        let kind = kind_of(&step.tool);
        let input = &step.input;
        let key = match kind {
            Kind::Command => input.command.clone().or_else(|| input.description.clone()),
            Kind::Read | Kind::Edit => input.file_path.clone(),
            Kind::SearchCode | Kind::SearchFiles => input.pattern.clone(),
            Kind::Agent => input.description.clone(),
            Kind::Web => input.query.clone(),
            Kind::Other => None,
        }
        .unwrap_or_else(|| step.line.clone());
        if matches!(kind, Kind::Read | Kind::Edit)
            && let Some(path) = input.file_path.as_deref()
            && !files.contains(&path)
        {
            files.push(path);
        }
        match kinds.iter_mut().find(|(seen, _)| *seen == kind) {
            Some((_, keys)) => {
                if !keys.contains(&key) {
                    keys.push(key);
                }
            }
            None => kinds.push((kind, vec![key])),
        }
    }
    let one_file = match files.as_slice() {
        [only] => Some(file_name(only)),
        _ => None,
    };
    let phrases: Vec<String> = kinds
        .iter()
        .map(|(kind, keys)| phrase(*kind, keys.len(), one_file.as_deref()))
        .collect();
    capitalised(&phrases.join(", "))
}

fn phrase(kind: Kind, count: usize, one_file: Option<&str>) -> String {
    let counted = |one: &str, many: &str| {
        if count == 1 {
            one.to_owned()
        } else {
            many.replace('N', &count.to_string())
        }
    };
    match (kind, one_file) {
        (Kind::Read, Some(name)) => format!("read {name}"),
        (Kind::Edit, Some(name)) => format!("edited {name}"),
        (Kind::Command, _) => counted("ran a command", "ran N commands"),
        (Kind::Read, None) => counted("read a file", "read N files"),
        (Kind::Edit, None) => counted("edited a file", "edited N files"),
        (Kind::SearchCode, _) => "searched code".to_owned(),
        (Kind::SearchFiles, _) => "searched for files".to_owned(),
        (Kind::Agent, _) => counted("ran an agent", "ran N agents"),
        (Kind::Web, _) => "searched the web".to_owned(),
        (Kind::Other, _) => counted("used a tool", "used N tools"),
    }
}

/// A path's last part.
fn file_name(path: &str) -> String {
    path.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
        .to_owned()
}

/// A command in one short line.
fn short(command: &str) -> String {
    const MAX: usize = 80;
    let line = command.lines().next().unwrap_or_default().trim();
    let more = command.trim().lines().nth(1).is_some();
    if line.chars().count() > MAX {
        let cut: String = line.chars().take(MAX).collect();
        format!("{}…", cut.trim_end())
    } else if more {
        format!("{line} …")
    } else {
        line.to_owned()
    }
}

fn capitalised(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Which form a description's verbs take.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Form {
    /// The past tense ("Recorded").
    Done,
    /// The "-ing" form ("Recording").
    Going,
}

/// `description` with its first verb, and each verb right after "and"
/// or "then", in `form`, when it opens with a known verb; else as
/// written.
#[must_use]
pub fn tensed(description: &str, form: Form) -> String {
    let words: Vec<&str> = description.split(' ').collect();
    let Some(first) = words.first() else {
        return description.to_owned();
    };
    if verb_of(first).is_none() {
        return description.to_owned();
    }
    let mut out: Vec<String> = Vec::with_capacity(words.len());
    for (index, word) in words.iter().enumerate() {
        let joined = index > 0
            && matches!(
                bare(words[index - 1]).to_ascii_lowercase().as_str(),
                "and" | "then"
            );
        if index == 0 || joined {
            out.push(converted(word, form).unwrap_or_else(|| (*word).to_owned()));
        } else {
            out.push((*word).to_owned());
        }
    }
    out.join(" ")
}

/// A word without the punctuation around it.
fn bare(word: &str) -> &str {
    word.trim_matches(|c: char| !c.is_alphanumeric())
}

/// `word` in `form`, keeping its capital and the punctuation after it,
/// when it is a known verb.
fn converted(word: &str, form: Form) -> Option<String> {
    let core = bare(word);
    let verb = verb_of(core)?;
    let start = word.find(core)?;
    let (before, after) = (&word[..start], &word[start + core.len()..]);
    let changed = match form {
        Form::Done => verb.past(),
        Form::Going => verb.going(),
    };
    let changed = if core.starts_with(|c: char| c.is_uppercase()) {
        capitalised(&changed)
    } else {
        changed
    };
    Some(format!("{before}{changed}{after}"))
}

/// A known verb (lower case) and its past tense when irregular.
struct Verb {
    base: String,
    irregular: Option<&'static str>,
}

impl Verb {
    fn past(&self) -> String {
        if let Some(past) = self.irregular {
            return past.to_owned();
        }
        let base = self.base.as_str();
        if base.ends_with('e') {
            format!("{base}d")
        } else if let Some(stem) = base.strip_suffix('y') {
            // `play` → `played`; `copy` → `copied`.
            if stem.ends_with(['a', 'e', 'i', 'o', 'u']) {
                format!("{base}ed")
            } else {
                format!("{stem}ied")
            }
        } else if DOUBLED.contains(&base) {
            format!("{base}{}ed", base.chars().last().unwrap_or_default())
        } else {
            format!("{base}ed")
        }
    }

    fn going(&self) -> String {
        let base = self.base.as_str();
        if DOUBLED.contains(&base) || DOUBLED_GOING.contains(&base) {
            format!("{base}{}ing", base.chars().last().unwrap_or_default())
        } else if let Some(stem) = base.strip_suffix("ie") {
            format!("{stem}ying")
        } else if base.ends_with('e')
            && !base.ends_with("ee")
            && !base.ends_with("ye")
            && base.len() > 2
        {
            format!("{}ing", &base[..base.len() - 1])
        } else {
            format!("{base}ing")
        }
    }
}

fn verb_of(word: &str) -> Option<Verb> {
    let lower = bare(word).to_ascii_lowercase();
    if let Some((_, past)) = IRREGULAR.iter().find(|(base, _)| *base == lower) {
        return Some(Verb {
            base: lower,
            irregular: Some(past),
        });
    }
    REGULAR.contains(&lower.as_str()).then_some(Verb {
        base: lower,
        irregular: None,
    })
}

/// Verbs whose last letter doubles before `-ed` and `-ing`.
const DOUBLED: &[&str] = &[
    "commit", "submit", "admit", "emit", "omit", "permit", "stop", "drop", "ship", "skip", "strip",
    "wrap", "unwrap", "trim", "scan", "rescan", "plan", "replan", "tag", "retag", "log", "grep",
    "pin", "unpin", "prefer", "map", "remap", "zip", "unzip", "step", "chop", "plug", "unplug",
    "tap", "snap", "swap", "nag", "jog", "flag", "debug", "rerun",
];

/// Verbs whose last letter doubles before `-ing` (irregular past).
const DOUBLED_GOING: &[&str] = &[
    "run", "set", "reset", "get", "forget", "put", "cut", "begin", "spin", "split", "sit", "hit",
    "upset", "shut", "win", "swim",
];

const IRREGULAR: &[(&str, &str)] = &[
    ("run", "ran"),
    ("rerun", "reran"),
    ("read", "read"),
    ("reread", "reread"),
    ("write", "wrote"),
    ("rewrite", "rewrote"),
    ("overwrite", "overwrote"),
    ("build", "built"),
    ("rebuild", "rebuilt"),
    ("make", "made"),
    ("remake", "remade"),
    ("find", "found"),
    ("get", "got"),
    ("forget", "forgot"),
    ("set", "set"),
    ("reset", "reset"),
    ("put", "put"),
    ("take", "took"),
    ("retake", "retook"),
    ("give", "gave"),
    ("see", "saw"),
    ("keep", "kept"),
    ("send", "sent"),
    ("resend", "resent"),
    ("spend", "spent"),
    ("split", "split"),
    ("cut", "cut"),
    ("shut", "shut"),
    ("hit", "hit"),
    ("begin", "began"),
    ("bring", "brought"),
    ("think", "thought"),
    ("rethink", "rethought"),
    ("undo", "undid"),
    ("redo", "redid"),
    ("do", "did"),
    ("tell", "told"),
    ("hold", "held"),
    ("feed", "fed"),
    ("lead", "led"),
    ("leave", "left"),
    ("lose", "lost"),
    ("meet", "met"),
    ("pay", "paid"),
    ("say", "said"),
    ("seek", "sought"),
    ("sell", "sold"),
    ("stand", "stood"),
    ("understand", "understood"),
    ("teach", "taught"),
    ("catch", "caught"),
    ("buy", "bought"),
    ("throw", "threw"),
    ("draw", "drew"),
    ("redraw", "redrew"),
    ("choose", "chose"),
    ("drive", "drove"),
    ("feel", "felt"),
    ("freeze", "froze"),
    ("unfreeze", "unfroze"),
    ("hide", "hid"),
    ("unhide", "unhid"),
    ("know", "knew"),
    ("shoot", "shot"),
    ("sleep", "slept"),
    ("slide", "slid"),
    ("speak", "spoke"),
    ("spin", "spun"),
    ("stick", "stuck"),
    ("strike", "struck"),
    ("sweep", "swept"),
    ("wake", "woke"),
    ("win", "won"),
    ("wind", "wound"),
    ("unwind", "unwound"),
    ("bind", "bound"),
    ("rebind", "rebound"),
    ("upset", "upset"),
    ("withdraw", "withdrew"),
    ("quit", "quit"),
    ("sit", "sat"),
    ("swim", "swam"),
    ("go", "went"),
    ("come", "came"),
    ("become", "became"),
];

const REGULAR: &[&str] = &[
    "add",
    "adjust",
    "align",
    "analyze",
    "analyse",
    "answer",
    "apply",
    "archive",
    "ask",
    "assert",
    "audit",
    "backfill",
    "bump",
    "cache",
    "calculate",
    "call",
    "cancel",
    "capture",
    "change",
    "check",
    "chop",
    "clean",
    "clear",
    "clone",
    "close",
    "collect",
    "commit",
    "compare",
    "compile",
    "compress",
    "compute",
    "configure",
    "confirm",
    "connect",
    "convert",
    "copy",
    "count",
    "crop",
    "create",
    "debug",
    "decode",
    "decompress",
    "delete",
    "deploy",
    "describe",
    "detect",
    "diff",
    "disable",
    "download",
    "drop",
    "dump",
    "edit",
    "emit",
    "enable",
    "encode",
    "ensure",
    "examine",
    "execute",
    "expand",
    "explain",
    "explore",
    "export",
    "extract",
    "fetch",
    "fill",
    "filter",
    "fix",
    "flag",
    "follow",
    "format",
    "generate",
    "grep",
    "handle",
    "import",
    "improve",
    "inspect",
    "install",
    "kill",
    "label",
    "launch",
    "link",
    "lint",
    "list",
    "load",
    "locate",
    "lock",
    "log",
    "look",
    "map",
    "mark",
    "measure",
    "merge",
    "migrate",
    "mount",
    "move",
    "note",
    "notify",
    "normalize",
    "open",
    "optimize",
    "pack",
    "parse",
    "patch",
    "paste",
    "pin",
    "ping",
    "plan",
    "play",
    "poll",
    "prepare",
    "preview",
    "print",
    "probe",
    "profile",
    "prune",
    "publish",
    "pull",
    "push",
    "query",
    "rebase",
    "recheck",
    "record",
    "recompute",
    "refactor",
    "refresh",
    "regenerate",
    "reinstall",
    "reload",
    "remove",
    "rename",
    "render",
    "reorder",
    "replace",
    "replay",
    "reproduce",
    "request",
    "rescan",
    "resize",
    "resolve",
    "restart",
    "restore",
    "retag",
    "retry",
    "return",
    "revert",
    "review",
    "rotate",
    "save",
    "scan",
    "search",
    "seed",
    "select",
    "serve",
    "ship",
    "sign",
    "simulate",
    "skip",
    "snapshot",
    "sort",
    "squash",
    "stage",
    "stamp",
    "start",
    "stash",
    "step",
    "stop",
    "strip",
    "submit",
    "summarize",
    "summarise",
    "switch",
    "sync",
    "tag",
    "test",
    "tidy",
    "time",
    "toggle",
    "touch",
    "trace",
    "track",
    "trigger",
    "trim",
    "try",
    "tweak",
    "type",
    "uninstall",
    "unlock",
    "unpack",
    "unpin",
    "unstage",
    "unzip",
    "update",
    "upgrade",
    "upload",
    "use",
    "validate",
    "verify",
    "view",
    "wait",
    "walk",
    "watch",
    "wrap",
    "zip",
    "show",
    "open",
    "rerender",
    "recreate",
    "remap",
    "swap",
    "unwrap",
    "plug",
];

#[cfg(test)]
#[path = "claude_folds_tests.rs"]
mod tests;
