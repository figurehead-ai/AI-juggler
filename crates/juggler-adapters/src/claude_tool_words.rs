//! 0.3.5: the words the Claude app's Code tab gives one call that isn't
//! a command, a file, a search or a subagent, read from its code
//! (`docs/research/native-conversation-display.md` §1):
//!
//! - its own tools by name and, where it says, by action: the in-app
//!   browser ("Opened a page", a click "Clicked", a batch "Clicked,
//!   typed, captured page +2"), Claude in Chrome in the same words,
//!   computer use, a simulator, and its session tools ("Updated plan");
//! - its built-in tools by kind ("Loaded tools", "Proposed a plan");
//! - any other as "Used <server>: <Tool>" ("Used visualize: Show
//!   widget"), or "Used <Tool>" outside MCP: never a tool's raw name.
//!
//! The tables in [`table`] are the app's own, generated from it. Pure.

mod table;

use crate::conversation_feed::CallInput;

const BROWSER: &str = "mcp__Claude_Browser__";
const CHROME: &str = "mcp__claude-in-chrome__";
const COMPUTER_USE: &str = "mcp__computer-use__";
const SIMULATOR: &str = "mcp__Claude_Code_";

/// How many of a batch's actions are named; the rest are "+N".
const BATCH_NAMED: usize = 3;

/// One call's words, done or `running`.
#[must_use]
pub fn words(tool: &str, input: &CallInput, running: bool) -> String {
    known(tool, input, running).unwrap_or_else(|| {
        let verb = if running { "Using" } else { "Used" };
        format!("{verb} {}", label(tool))
    })
}

type Pair = (&'static str, &'static str);

fn find(table: &[(&'static str, &'static str, &'static str)], key: &str) -> Option<Pair> {
    table
        .binary_search_by(|(name, _, _)| (*name).cmp(key))
        .ok()
        .map(|at| (table[at].1, table[at].2))
}

fn find_one(table: &[(&'static str, &'static str)], key: &str) -> Option<&'static str> {
    table
        .binary_search_by(|(name, _)| (*name).cmp(key))
        .ok()
        .map(|at| table[at].1)
}

const fn form((done, going): Pair, running: bool) -> &'static str {
    if running { going } else { done }
}

fn known(tool: &str, input: &CallInput, running: bool) -> Option<String> {
    let action = input.action.as_deref().unwrap_or_default();
    if let Some(name) = tool.strip_prefix(BROWSER) {
        match name {
            "preview_start" => {
                let pair = if input.url_without_name {
                    ("Opened a page", "Opening a page")
                } else {
                    ("Started preview", "Starting preview")
                };
                return Some(form(pair, running).to_owned());
            }
            "browser_batch" => return Some(batch(input, running, page_action)),
            "computer" => {
                if let Some(pair) = find(table::POINTER, action) {
                    return Some(form(pair, running).to_owned());
                }
            }
            _ => {}
        }
    }
    if let Some(name) = tool.strip_prefix(CHROME) {
        if let Some(pair) = find(table::MCP, tool) {
            return Some(form(pair, running).to_owned());
        }
        if name == "browser_batch" {
            return Some(batch(input, running, chrome_action));
        }
        let same = find_one(table::CHROME, name)?;
        let pair = (same == "computer")
            .then(|| find(table::POINTER, action))
            .flatten()
            .or_else(|| find(table::MCP, &format!("{BROWSER}{same}")))?;
        return Some(form(pair, running).to_owned());
    }
    if tool == format!("{COMPUTER_USE}computer_batch") {
        return Some(list(
            input.batch.iter().map(|(_, action)| {
                find(table::COMPUTER, action).map_or("Computer action", |pair| form(pair, running))
            }),
            form(("Used your computer", "Using your computer"), running),
        ));
    }
    if let Some(kind) = simulator(tool) {
        let pair = if kind == "control" {
            find(table::DEVICE, action).unwrap_or(("Used the device", "Using the device"))
        } else {
            find(table::DEVICE_OTHER, kind)?
        };
        return Some(form(pair, running).to_owned());
    }
    if let Some(pair) = find(table::MCP, tool) {
        return Some(form(pair, running).to_owned());
    }
    if tool.starts_with("mcp__") {
        return None;
    }
    // A built-in tool the app words by kind (only those with a done form
    // of their own: the others read "Used <Tool>").
    let kind = snake(tool);
    let done = find_one(table::BUILTIN_DONE, &kind)?;
    Some(if running {
        find_one(table::BUILTIN_RUNNING, &kind)
            .unwrap_or(done)
            .to_owned()
    } else {
        done.to_owned()
    })
}

/// `mcp__Claude_Code_<iOS_Simulator|Android_Emulator>__<kind>[_<8 hex>]`:
/// the kind.
fn simulator(tool: &str) -> Option<&str> {
    let rest = tool.strip_prefix(SIMULATOR)?;
    let rest = rest
        .strip_prefix("iOS_Simulator__")
        .or_else(|| rest.strip_prefix("Android_Emulator__"))?;
    let kind = match rest.split_once('_') {
        Some((kind, id)) if id.len() == 8 && id.bytes().all(|b| b.is_ascii_hexdigit()) => kind,
        Some(_) => return None,
        None => rest,
    };
    ["control", "build", "logs", "debug"]
        .contains(&kind)
        .then_some(kind)
}

/// The in-app browser's batch: a page action by its name, a pointer
/// action by `computer`'s action.
fn page_action((name, action): &(String, String), running: bool) -> &'static str {
    let pair = if name == "computer" {
        find(table::POINTER, action)
    } else {
        find(table::PAGE, name)
    };
    pair.map_or("Browser action", |pair| form(pair, running))
}

/// Claude in Chrome's batch: its own names mapped onto the browser's.
fn chrome_action((name, action): &(String, String), running: bool) -> &'static str {
    let pair = if name == "computer" {
        find(table::POINTER, action)
    } else if let Some(same) = find_one(table::CHROME, name) {
        find(table::PAGE, same).or_else(|| find(table::MCP, &format!("{BROWSER}{same}")))
    } else {
        find(table::MCP, &format!("{CHROME}{name}"))
    };
    pair.map_or("Browser action", |pair| form(pair, running))
}

fn batch(
    input: &CallInput,
    running: bool,
    action: fn(&(String, String), bool) -> &'static str,
) -> String {
    list(
        input.batch.iter().map(|step| action(step, running)),
        form(
            ("Performed browser actions", "Performing browser actions"),
            running,
        ),
    )
}

/// The first [`BATCH_NAMED`] different words, the first capitalised and
/// the others not, then "+N" for the rest; `none` when there are none.
fn list<'a>(words: impl Iterator<Item = &'a str>, none: &str) -> String {
    let mut named: Vec<&str> = Vec::new();
    let mut more = 0_usize;
    for word in words {
        if named.contains(&word) {
            continue;
        }
        if named.len() < BATCH_NAMED {
            named.push(word);
        } else {
            more += 1;
        }
    }
    if named.is_empty() {
        return none.to_owned();
    }
    let joined = named
        .iter()
        .enumerate()
        .map(|(at, word)| {
            if at == 0 {
                (*word).to_owned()
            } else {
                lower_first(word)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    if more > 0 {
        format!("{joined} +{more}")
    } else {
        joined
    }
}

fn lower_first(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|first| first.to_lowercase().chain(chars).collect())
        .unwrap_or_default()
}

/// What follows "Used": an MCP tool's server, then the tool in words
/// ("visualize: Show widget"); another tool in words ("Tool search").
fn label(tool: &str) -> String {
    match mcp_parts(tool) {
        Some((server, name)) => {
            let server = server
                .split('_')
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            format!("{server}: {}", in_words(name))
        }
        None => in_words(
            tool.rsplit("__")
                .find(|part| !part.is_empty())
                .unwrap_or(tool),
        ),
    }
}

/// `mcp__<server>__<tool>`: its server and tool.
fn mcp_parts(tool: &str) -> Option<(&str, &str)> {
    let rest = tool.strip_prefix("mcp__")?.trim_end_matches('_');
    let (server, name) = rest.rsplit_once("__")?;
    let server = server.trim_matches('_');
    (!server.is_empty() && !name.is_empty()).then_some((server, name))
}

/// A tool's name in words, as the app writes it: split at `_`, `.`, `-`
/// and spaces (else between a lower and an upper letter), lower case but
/// for a word with capitals inside ("GitHub"), the first capitalised.
fn in_words(name: &str) -> String {
    let name = name.trim_start_matches('_');
    let base = if name.chars().any(char::is_lowercase) {
        name.to_owned()
    } else {
        name.to_lowercase()
    };
    let split = if base.contains(['_', '.', '-', ' ']) {
        base
    } else {
        camel_spaced(&base)
    };
    let words: Vec<&str> = split
        .split(['_', '.', '-', ' '])
        .filter(|word| !word.is_empty())
        .collect();
    let Some(first) = words.first() else {
        return name.to_owned();
    };
    let joined = words
        .iter()
        .map(|word| {
            if inner_capital(word) {
                (*word).to_owned()
            } else {
                word.to_lowercase()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    if inner_capital(first) {
        joined
    } else {
        let mut chars = joined.chars();
        chars
            .next()
            .map(|head| head.to_uppercase().chain(chars).collect())
            .unwrap_or_default()
    }
}

fn inner_capital(word: &str) -> bool {
    word.chars().skip(1).any(char::is_uppercase)
}

/// `ToolSearch` → `Tool Search`: a space between a lower letter or a
/// digit and an upper one.
fn camel_spaced(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    let mut before: Option<char> = None;
    for c in name.chars() {
        if c.is_uppercase() && before.is_some_and(|b| b.is_lowercase() || b.is_ascii_digit()) {
            out.push(' ');
        }
        out.push(c);
        before = Some(c);
    }
    out
}

/// `ToolSearch` → `tool_search`, the key the built-in tables use.
fn snake(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    let mut before: Option<char> = None;
    for c in name.chars() {
        if c.is_uppercase() && before.is_some_and(char::is_lowercase) {
            out.push('_');
        }
        out.extend(c.to_lowercase());
        before = Some(c);
    }
    out
}

#[cfg(test)]
#[path = "claude_tool_words_tests.rs"]
mod tests;
