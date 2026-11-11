//! A tool's own terminal screen, read as words: which of the questions
//! its adapter knows the screen ends on (`ToolConfig::screen_ask`), so a
//! session waiting there says what it asks (the rc29 persona walks: a
//! Claude Code first run read "It waits on its own screen…", not its
//! "Choose the text style that looks best with your terminal").

/// How much of the screen may follow a question while it still waits:
/// the rest of the question, its choices and their hints, and a few
/// redraws of the highlighted choice. Counted in letters and digits only
/// (rc30: Claude Code's text-style screen draws two rules of `╌` as wide
/// as the terminal around its preview, so a count of every character
/// passed 600 at 160 columns and the row said "open it to see"; its
/// letters are about 300 at any width). Once answered, the tool's own
/// screen comes after it and is far longer.
pub const STILL_ASKING: usize = 600;

/// One question a tool's own screen asks before it works.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenQuestion {
    /// Words only that screen shows (the question itself, or its title).
    pub shows: &'static str,
    /// What it asks, in one line, as the row quotes it.
    pub asks: &'static str,
}

/// What the end of `tail` (a terminal's raw output, escape codes and
/// all) asks, of `questions`: the one shown last, while no more than
/// [`STILL_ASKING`] letters and digits follow it. `None`: the screen asks none of them.
#[must_use]
pub fn ends_on(tail: &str, questions: &[ScreenQuestion]) -> Option<&'static str> {
    let screen = plain(tail);
    questions
        .iter()
        .filter_map(|question| {
            let at = screen.rfind(plain(question.shows).as_str())?;
            let after = screen[at..].chars().filter(|c| c.is_alphanumeric()).count();
            (after <= STILL_ASKING).then_some((at, question.asks))
        })
        .max_by_key(|(at, _)| *at)
        .map(|(_, asks)| asks)
}

/// A terminal's text as words only: its escape codes (colours, cursor
/// moves, titles) taken out and every space with them, lowercased, so a
/// screen drawn cell by cell reads as one run of text.
#[must_use]
pub fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
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
    use super::{ScreenQuestion, ends_on};

    const KNOWN: &[ScreenQuestion] = &[
        ScreenQuestion {
            shows: "Choose the text style",
            asks: "Choose the text style that looks best with your terminal",
        },
        ScreenQuestion {
            shows: "Select login method",
            asks: "Select login method",
        },
    ];

    #[test]
    fn the_last_question_on_screen_is_what_it_asks_until_the_screen_moves_on() {
        let first = "\u{1b}[2J\u{1b}[1;1HLet's get started.\r\n\u{1b}[1mChoose\u{1b}[0m the text style that looks best with your terminal\r\n\u{1b}[36m\u{276f} 1. Dark mode \u{2714}\u{1b}[0m\r\n  2. Light mode\r\n";
        assert_eq!(
            ends_on(first, KNOWN),
            Some("Choose the text style that looks best with your terminal")
        );
        let next =
            format!("{first}\u{1b}[2JSelect login method:\r\n\u{276f} 1. Claude account\r\n");
        assert_eq!(ends_on(&next, KNOWN), Some("Select login method"));
        let working = format!("{next}{}", "\u{1b}[2K\u{2022} Working on it\r\n".repeat(60));
        assert_eq!(ends_on(&working, KNOWN), None, "answered: long gone");
        assert_eq!(ends_on("", KNOWN), None);
    }
}
