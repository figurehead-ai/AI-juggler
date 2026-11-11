//! Lab 2 (D-40, 2026-10-25): a tool that asks whether to trust a folder
//! and saves the answer to the user's own config. AI Juggler never lets that
//! write happen for the user. It asks the question itself, once per tool and
//! folder, in the tool's own words. It keeps the answer in its own data, and
//! passes it to each launch in memory (an argument, never a file).

use crate::screen_text::plain;

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
    /// The title every one of the tool's folder screens has ("Folder
    /// access"): the question, and its confirmation of a restricted open.
    pub title: &'static str,
    /// Words only the tool's confirmation of a restricted open shows
    /// (the folder isn't trusted; nothing is saved).
    pub restricted: &'static str,
    /// The tool's other choices on its folder screens (an existing task's).
    pub other_choices: &'static [&'static str],
}

/// Which of the tool's own folder screens a terminal shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrustScreen {
    /// Its question, whose answer it would save.
    Question,
    /// Its confirmation of a restricted open (the folder isn't trusted,
    /// nothing is saved).
    Restricted,
    /// Another of its folder screens.
    Other,
}

/// How much of the screen may follow the question while it still waits:
/// the rest of the question, its choices and their hints, and a few
/// redraws of the highlighted choice (characters, spaces left out). Once
/// answered, the tool's own screen comes after it and is far longer.
const STILL_ASKING: usize = 600;

impl FolderTrust {
    /// Whether the end of a terminal's output (`tail`, raw: escape codes
    /// and all) shows one of this tool's folder screens still waiting for
    /// its answer (the rc27 P1 walk: Codex's trust screen read Working).
    #[must_use]
    pub fn on_screen(&self, tail: &str) -> bool {
        self.screen(tail).is_some()
    }

    /// Which of the tool's folder screens the end of `tail` shows, while
    /// it waits: its title (or question), then one of its choices, and
    /// nothing much after. The rc29 walk: with the folder not trusted,
    /// Codex shows "Folder access … Open restricted / Quit", not its
    /// question, and read Working for minutes.
    #[must_use]
    pub fn screen(&self, tail: &str) -> Option<TrustScreen> {
        let screen = plain(tail);
        let at = [plain(self.title), plain(self.question)]
            .iter()
            .filter_map(|words| screen.rfind(words.as_str()))
            .min()?;
        let after = &screen[at..];
        if after.chars().count() > STILL_ASKING {
            return None;
        }
        let restrict = plain(self.restrict);
        let choices = [plain(self.trust), restrict.clone()]
            .into_iter()
            .chain(self.other_choices.iter().map(|choice| plain(choice)));
        if !choices.clone().any(|choice| after.contains(&choice)) {
            return None;
        }
        Some(
            if after.contains(&plain(self.restricted)) && after.contains(&restrict) {
                TrustScreen::Restricted
            } else if after.contains(&plain(self.question)) {
                TrustScreen::Question
            } else {
                TrustScreen::Other
            },
        )
    }

    /// What the screen asks, in the tool's words: its question, or its
    /// title and its restricted choice.
    #[must_use]
    pub fn asks(&self, screen: TrustScreen) -> String {
        match screen {
            TrustScreen::Question => self.question.to_owned(),
            TrustScreen::Restricted => format!("{}: {}?", self.title, self.restrict),
            TrustScreen::Other => self.title.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FolderTrust, TrustScreen};

    const ASKS: FolderTrust = FolderTrust {
        question: "Trust this folder?",
        trust: "Trust and continue",
        restrict: "Open restricted",
        note: "",
        title: "Folder access",
        restricted: "Opening will not change saved trust",
        other_choices: &["Open existing task"],
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

    /// The rc29 walk: with the folder not trusted (the user's Open
    /// restricted, passed in memory), Codex 0.163 shows "Folder access",
    /// then "Open restricted" and "Quit": it asks at once, in its words.
    #[test]
    fn the_restricted_confirmation_asks_at_once() {
        let screen = "\u{1b}[1;1H\u{1b}[1mFolder access\u{1b}[0m\u{1b}[2;1H/Users/you/lab/web\u{1b}[4;1HConfig, hooks, and exec policies from untrusted folders stay disabled. Trusted project folders can still contribute settings. Skills still load, and tools follow your permission settings. Opening will not change saved trust.\u{1b}[7;1H\u{1b}[7m\u{203a} 1. Open restricted\u{1b}[0m\u{1b}[8;1H  2. Quit\u{1b}[10;1Henter continue \u{b7} esc quit";
        assert_eq!(ASKS.screen(screen), Some(TrustScreen::Restricted));
        assert_eq!(
            ASKS.asks(TrustScreen::Restricted),
            "Folder access: Open restricted?"
        );
        assert_eq!(ASKS.screen(SCREEN), Some(TrustScreen::Question));
        // An existing task's screen asks too.
        assert_eq!(
            ASKS.screen("Folder access /x This existing task may retain settings. \u{203a} 1. Open existing task 2. Quit"),
            Some(TrustScreen::Other)
        );
    }
}
