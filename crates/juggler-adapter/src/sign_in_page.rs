//! AJ-E1545 R8 / AJ-E1553: what a tool's sign-in shows the user, read from
//! its terminal by its adapter ([`crate::ToolSignIn::sign_in_page`]), so the
//! sign-in can be a friendly card ("Open `<tool>`'s sign-in page", the code
//! large with Copy) instead of the tool's terminal.
//!
//! Only what the tool prints for the user to see is read: the page's
//! address and a one-time display code. Never a credential (ADR-0003): a
//! sign-in that asks for a code to be pasted back says so
//! ([`SignInPage::pastes_code`]) and the user pastes it into the tool.
//!
//! The page is opened by AI Juggler, so its address must be one the
//! adapter names: `https`, its host one of the adapter's ([`PageRule`]),
//! no user part. Anything else on the screen is not a sign-in page.

use std::time::SystemTime;

/// A sign-in's page, as its tool printed it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignInPage {
    /// The page to open (`https`, a host of the adapter's).
    pub url: String,
    /// The one-time code the page should show (a device sign-in), when
    /// the tool printed one.
    pub code: Option<String>,
    /// The page gives a code to paste back into the tool.
    pub pastes_code: bool,
}

/// How an adapter reads its tool's sign-in output.
#[derive(Clone, Copy, Debug)]
pub struct PageRule {
    /// The hosts its sign-in pages are on (a host or any name under it).
    pub hosts: &'static [&'static str],
    /// The tool prints a one-time device code to check on the page.
    pub device_codes: bool,
    /// Words (lowercase) the tool shows when it waits for a code pasted
    /// back from the page.
    pub paste_words: &'static [&'static str],
}

/// What changed when a tool's sign-in is written ([`crate::ToolSignIn::sign_in_stamp`]):
/// its sign-in file's size and time, never its contents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignInStamp {
    /// Its size in bytes.
    pub len: u64,
    /// When it was last written, when the system says.
    pub modified: Option<SystemTime>,
}

impl SignInStamp {
    /// The stamp of `meta` (a sign-in file's metadata).
    #[must_use]
    pub fn of(meta: &std::fs::Metadata) -> Self {
        Self {
            len: meta.len(),
            modified: meta.modified().ok(),
        }
    }
}

/// The longest page address taken (a sign-in page's, with its query).
const MAX_URL: usize = 4096;

/// A terminal's output as plain lines: escape codes taken out (a
/// hyperlink's address kept as text, a cursor step right as a space), a
/// carriage return as a line end.
#[must_use]
pub fn plain_lines(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\u{1b}' => match chars.next() {
                Some('[') => {
                    let mut last = ' ';
                    for c in chars.by_ref() {
                        if ('@'..='~').contains(&c) {
                            last = c;
                            break;
                        }
                    }
                    // A cursor step right stands for spaces; a move to a
                    // new line for a line end.
                    match last {
                        'C' => out.push(' '),
                        'E' | 'H' | 'f' => out.push('\n'),
                        _ => {}
                    }
                }
                Some(']') => {
                    let mut osc = String::new();
                    while let Some(c) = chars.next() {
                        if c == '\u{7}' {
                            break;
                        }
                        if c == '\u{1b}' {
                            chars.next();
                            break;
                        }
                        osc.push(c);
                    }
                    // OSC 8: `8;params;address` — the address as text.
                    if let Some((_, address)) =
                        osc.strip_prefix("8;").and_then(|rest| rest.split_once(';'))
                        && !address.is_empty()
                    {
                        out.push(' ');
                        out.push_str(address);
                        out.push(' ');
                    }
                }
                _ => {}
            },
            '\r' => {
                if chars.peek() != Some(&'\n') {
                    out.push('\n');
                }
            }
            '\n' | '\t' => out.push(if c == '\t' { ' ' } else { '\n' }),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// Whether `c` can be part of an address.
fn url_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || "-._~:/?#[]@!$&'()*+,;=%".contains(c)
}

/// The host of an `https` address, when it has one and no user part.
fn https_host(url: &str) -> Option<String> {
    let rest = url.strip_prefix("https://")?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.contains('@') || authority.is_empty() {
        return None;
    }
    let host = authority.split(':').next().unwrap_or_default();
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

/// Whether `host` is one of `hosts` or a name under one.
fn host_allowed(host: &str, hosts: &[&str]) -> bool {
    hosts
        .iter()
        .any(|allowed| host == *allowed || host.ends_with(&format!(".{allowed}")))
}

/// A line's frame (a box's sides, a prompt's marks) taken off its ends.
fn unframed(line: &str) -> &str {
    line.trim_matches(|c: char| c.is_whitespace() || "│┃║|".contains(c))
}

/// The addresses on `lines`, each joined to the lines a terminal width
/// broke it over: a long line ending in an address, followed by a line
/// that is one run of address characters, is one address.
fn addresses(lines: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let mut search = *line;
        while let Some(at) = search.find("https://") {
            let tail = &search[at..];
            let end = tail.find(|c: char| !url_char(c)).unwrap_or(tail.len());
            let mut url = tail[..end].to_owned();
            // At the line's end, on a line as long as a terminal's width:
            // the rest may be on the lines after it.
            let rest_of_line = unframed(&tail[end..]);
            if rest_of_line.is_empty() && line.chars().count() >= 70 {
                for next in &lines[index + 1..] {
                    let next = unframed(next);
                    if next.is_empty() || !next.chars().all(url_char) || url.len() > MAX_URL {
                        break;
                    }
                    url.push_str(next);
                    if next.chars().count() < 40 {
                        break; // a short piece is the last
                    }
                }
            }
            let url = url
                .trim_end_matches(['.', ',', ';', ':', ')', '\'', '"', ']'])
                .to_owned();
            if url.len() <= MAX_URL {
                found.push(url);
            }
            search = &tail[end.max(1)..];
        }
    }
    found
}

/// Whether `word` looks like a one-time device code: groups of capital
/// letters and digits joined by dashes (`WDJB-MJHT`, `ABCD-1234`).
fn device_code(word: &str) -> bool {
    let groups: Vec<&str> = word.split('-').collect();
    (2..=3).contains(&groups.len())
        && groups.iter().all(|group| {
            (3..=6).contains(&group.len())
                && group
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        })
        && word.chars().any(|c| c.is_ascii_uppercase())
}

/// The value of `name` in `url`'s query.
fn query_value(url: &str, name: &str) -> Option<String> {
    let query = url.split_once('?')?.1.split('#').next()?;
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == name && device_code(value)).then(|| value.to_owned())
    })
}

/// The device code on `lines`: on a line that says "code", or the line
/// after one.
fn code_on(lines: &[&str]) -> Option<String> {
    let mut after_code_line = false;
    for line in lines {
        let says_code = line.to_ascii_lowercase().contains("code");
        if says_code || after_code_line {
            let code = line
                .split(|c: char| c.is_whitespace() || ":`'\"()[]".contains(c))
                .map(|word| word.trim_matches(|c: char| c == '.' || c == ','))
                .find(|word| device_code(word));
            if let Some(code) = code {
                return Some(code.to_owned());
            }
        }
        after_code_line = says_code;
    }
    None
}

/// The sign-in page `text` (a terminal's [`plain_lines`]) shows, by
/// `rule`: the first address on one of its hosts, the device code with it
/// (when the tool prints one), and whether it waits for a code pasted
/// back. `None`: no page of the tool's on the screen (yet).
#[must_use]
pub fn read(text: &str, rule: &PageRule) -> Option<SignInPage> {
    let lines: Vec<&str> = text.lines().collect();
    let url = addresses(&lines)
        .into_iter()
        .find(|url| https_host(url).is_some_and(|host| host_allowed(&host, rule.hosts)))?;
    let code = if rule.device_codes {
        query_value(&url, "user_code").or_else(|| code_on(&lines))
    } else {
        None
    };
    let lower = text.to_ascii_lowercase();
    let pastes_code = code.is_none() && rule.paste_words.iter().any(|words| lower.contains(words));
    Some(SignInPage {
        url,
        code,
        pastes_code,
    })
}

/// The last `count` lines of `text` worth showing (not blank, nor a line
/// of rules), each made safe to keep: an address without its query (it
/// can carry a sign-in's state), a long token-like word taken out.
#[must_use]
pub fn last_lines(text: &str, count: usize) -> Vec<String> {
    let mut kept: Vec<String> = text
        .lines()
        .map(str::trim_end)
        .filter(|line| line.chars().any(char::is_alphanumeric))
        .map(safe_line)
        .collect();
    let skip = kept.len().saturating_sub(count);
    kept.drain(..skip);
    kept
}

/// One line, made safe to keep (see [`last_lines`]).
#[must_use]
pub fn safe_line(line: &str) -> String {
    line.split(' ')
        .map(|word| {
            if let Some(at) = word.find("https://").or_else(|| word.find("http://")) {
                let (before, address) = word.split_at(at);
                let bare = address.split(['?', '#']).next().unwrap_or(address);
                return format!("{before}{bare}");
            }
            let tokenish = word.len() >= 24
                && word
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_.=+/".contains(c));
            if tokenish {
                "…".to_owned()
            } else {
                word.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// AJ-E1553: one sign-in run's "signed in?" signal: the adapter's own
/// check ([`crate::ToolSignIn::login_state`], metadata only) once its
/// folder changed, against how it stood when the sign-in started. A
/// folder signed out at the start is done once it reads signed in; one
/// already signed in (signing in again) only once its sign-in file was
/// written anew ([`crate::ToolSignIn::sign_in_stamp`]), so an old sign-in
/// never ends a new one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignInWatch {
    was_signed_in: bool,
    was_stamp: Option<SignInStamp>,
}

impl SignInWatch {
    /// How `state_dir` stands as its sign-in starts.
    pub fn start<A: crate::ToolSignIn + ?Sized>(adapter: &A, state_dir: &std::path::Path) -> Self {
        Self {
            was_signed_in: signed_in(adapter, state_dir),
            was_stamp: adapter.sign_in_stamp(state_dir),
        }
    }

    /// Whether the sign-in is done now (see [`SignInWatch`]).
    pub fn done<A: crate::ToolSignIn + ?Sized>(
        &self,
        adapter: &A,
        state_dir: &std::path::Path,
    ) -> bool {
        if !signed_in(adapter, state_dir) {
            return false;
        }
        !self.was_signed_in
            || adapter
                .sign_in_stamp(state_dir)
                .is_some_and(|stamp| Some(stamp) != self.was_stamp)
    }
}

/// Whether `state_dir` reads signed in (a sign-in or a key) by the
/// adapter's own check: a sign-in entry left for it after a restart is
/// stale (AJ-E1553 R3).
pub fn signed_in<A: crate::ToolSignIn + ?Sized>(adapter: &A, state_dir: &std::path::Path) -> bool {
    adapter.login_state(state_dir).probe() == crate::environment::LoginProbe::LoggedIn
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULE: PageRule = PageRule {
        hosts: &["auth.example.com", "example.org"],
        device_codes: true,
        paste_words: &["paste code"],
    };

    #[test]
    fn escape_codes_go_and_a_hyperlinks_address_stays() {
        let raw = b"\x1b[1mOpen\x1b[0m\x1b[1Cthis:\r\n\x1b]8;;https://auth.example.com/device\x1b\\here\x1b]8;;\x1b\\\r\n";
        let text = plain_lines(raw);
        assert!(text.contains("Open this:"), "{text}");
        assert!(text.contains("https://auth.example.com/device"), "{text}");
    }

    #[test]
    fn a_page_on_the_tools_host_is_read_with_its_code() {
        let text = "Your one-time code: WDJB-MJHT\nOpen this page to sign in: https://auth.example.com/device\n";
        let page = read(text, &RULE).unwrap();
        assert_eq!(page.url, "https://auth.example.com/device");
        assert_eq!(page.code.as_deref(), Some("WDJB-MJHT"));
        assert!(!page.pastes_code);
    }

    #[test]
    fn a_page_on_another_host_or_with_a_user_part_is_never_read() {
        assert_eq!(
            read("Go to https://evil.test/auth.example.com\n", &RULE),
            None
        );
        assert_eq!(
            read("Go to https://auth.example.com@evil.test/x\n", &RULE),
            None
        );
        assert_eq!(read("Go to http://auth.example.com/x\n", &RULE), None);
        assert_eq!(read("Go to https://notexample.org/x\n", &RULE), None);
        assert!(read("Go to https://login.example.org/x\n", &RULE).is_some());
    }

    #[test]
    fn a_code_in_the_address_counts_and_lowercase_words_are_no_code() {
        let page = read(
            "Visit https://auth.example.com/d?user_code=ABCD-1234 now\n",
            &RULE,
        )
        .unwrap();
        assert_eq!(page.code.as_deref(), Some("ABCD-1234"));
        let none = read(
            "Enter the code below\nhttps://auth.example.com/d\nwell-known\n",
            &RULE,
        )
        .unwrap();
        assert_eq!(none.code, None);
    }

    #[test]
    fn an_address_broken_over_the_terminals_width_is_joined() {
        let first = format!(
            "Browser didn't open? Use this: https://auth.example.com/oauth?{}",
            "a".repeat(50)
        );
        let text = format!(
            "{first}\n{}\nb=2\n\nPaste code here if prompted >\n",
            "c".repeat(60)
        );
        let page = read(&text, &RULE).unwrap();
        assert_eq!(
            page.url,
            format!(
                "https://auth.example.com/oauth?{}{}b=2",
                "a".repeat(50),
                "c".repeat(60)
            )
        );
        assert!(page.pastes_code);
        // A short line's address is never joined to the code under it.
        let page = read("Go to https://auth.example.com/d\nABCD-EFGH\n", &RULE).unwrap();
        assert_eq!(page.url, "https://auth.example.com/d");
    }

    #[test]
    fn kept_lines_lose_queries_and_tokens() {
        let text = "Starting…\n\n────\nOpen https://auth.example.com/o?state=s3cr3t&x=1 now\nkey abcdefghijklmnopqrstuvwxyz0123 used\nError: timed out\n";
        let lines = last_lines(text, 3);
        assert_eq!(
            lines,
            [
                "Open https://auth.example.com/o now",
                "key … used",
                "Error: timed out"
            ]
        );
    }
}
