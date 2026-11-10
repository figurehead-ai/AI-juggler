//! Lab 2 (D-40, 2026-10-25): a tool that asks whether to trust a folder
//! and saves the answer to the user's own config. AI Juggler never lets that
//! write happen for the user. It asks the question itself, once per tool and
//! folder, in the tool's own words. It keeps the answer in its own data, and
//! passes it to each launch in memory (an argument, never a file).

/// The question a tool asks about a folder, in the tool's own words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FolderTrust {
    /// The question ("Trust this folder?").
    pub question: &'static str,
    /// The tool's words for trusting it.
    pub trust: &'static str,
    /// The tool's words for not trusting it.
    pub restrict: &'static str,
    /// What the choice does, in one line, in AI Juggler's words.
    pub note: &'static str,
}

/// How much of the screen may follow the question while it still waits:
/// the rest of the question, its choices and their hints, and a few
/// redraws of the highlighted choice (characters, spaces left out). Once
/// answered, the tool's own screen comes after it and is far longer.
const STILL_ASKING: usize = 600;

impl FolderTrust {
    /// Whether the end of a terminal's output (`tail`, raw: escape codes
    /// and all) shows this question still waiting for its answer: the
    /// question, then its "trust" choice, and nothing much after
    /// (the rc27 P1 walk: Codex's own trust screen read Working).
    #[must_use]
    pub fn on_screen(&self, tail: &str) -> bool {
        let screen = plain(tail);
        let Some(at) = screen.rfind(&plain(self.question)) else {
            return false;
        };
        let after = &screen[at..];
        after.contains(&plain(self.trust)) && after.chars().count() <= STILL_ASKING
    }
}

/// A terminal's text as words only: its escape codes (colours, cursor
/// moves, titles) taken out and every space with them, lowercased, so a
/// screen drawn cell by cell reads as one run of text.
fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            if !c.is_whitespace() && !c.is_control() {
                out.extend(c.to_lowercase());
            }
            continue;
        }
        match chars.next() {
            // CSI: parameters, then one final byte in @–~.
            Some('[') => {
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        break;
                    }
                }
            }
            // OSC (a title, a link): up to BEL or ESC \.
            Some(']') => {
                while let Some(c) = chars.next() {
                    if c == '\u{7}' {
                        break;
                    }
                    if c == '\u{1b}' {
                        chars.next();
                        break;
                    }
                }
            }
            // Any other escape is two characters.
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::FolderTrust;

    const ASKS: FolderTrust = FolderTrust {
        question: "Trust this folder?",
        trust: "Trust and continue",
        restrict: "Open restricted",
        note: "",
    };

    /// The rc27 P1 walk's screen, drawn as a terminal UI draws it: cursor
    /// moves between rows, colours inside them.
    const SCREEN: &str = "\u{1b}[?1049h\u{1b}[1;1H\u{1b}[1mFolder access\u{1b}[0m\u{1b}[2;1H/Users/you/lab/web\u{1b}[4;1HTrust this folder? Codex can read, edit, and run files here,\u{1b}[5;1Hsubject to your permission settings. Continue only if you trust\u{1b}[6;1Hthese files. Your trust decision will be saved.\u{1b}[8;1H\u{1b}[7m\u{203a} 1. Trust and continue\u{1b}[0m\u{1b}[9;1H  2. Quit\u{1b}[11;1H\u{1b}[2menter continue \u{b7} esc quit\u{1b}[0m";

    #[test]
    fn the_question_on_screen_asks_and_its_answer_ends_it() {
        assert!(ASKS.on_screen(SCREEN));
        // A highlighted choice redrawn (an arrow key) still asks.
        let moved = format!(
            "{SCREEN}\u{1b}[8;1H  1. Trust and continue\u{1b}[9;1H\u{1b}[7m\u{203a} 2. Quit\u{1b}[0m"
        );
        assert!(ASKS.on_screen(&moved));
        // Answered: the tool's own screen follows, and it asks no more.
        let answered = format!(
            "{SCREEN}\u{1b}[2J\u{1b}[1;1H{}",
            "OpenAI Codex  model: gpt  directory: ~/lab/web  To get started, describe a task or try one of these commands. ".repeat(8)
        );
        assert!(!ASKS.on_screen(&answered));
        // Neither the question alone nor other words ask.
        assert!(!ASKS.on_screen("Trust this folder? is a question in a README"));
        assert!(!ASKS.on_screen("Working (12s · esc to interrupt)"));
    }
}
