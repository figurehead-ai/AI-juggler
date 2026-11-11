//! D-29: a delivered message typed as the next prompt into a tool's own
//! terminal UI, for a session AI Juggler runs in its terminal (not
//! driven). The decision log's words: a delivery "uses the vendor's own
//! route where one exists …, else is typed as the next prompt, through the
//! same checks".
//!
//! The adapter says how its tool takes a prompt ([`TypedPrompt`]); the app
//! types it only at the tool's idle prompt, as its hooks report it, never
//! at a question on its screen, mid-turn or over the user's typing.

/// The keys that put one prompt into a tool's terminal UI: its text, then
/// (a moment later, so the tool has taken the text in) the key that sends
/// it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedPrompt {
    /// The prompt's text, as the tool takes it in.
    pub text: Vec<u8>,
    /// The key that sends it.
    pub submit: Vec<u8>,
}

/// The start of a bracketed paste.
const PASTE_START: &str = "\x1b[200~";
/// The end of a bracketed paste.
const PASTE_END: &str = "\x1b[201~";

impl TypedPrompt {
    /// `text` as one bracketed paste, sent with Return: how a terminal UI
    /// that reads bracketed pastes (Claude Code's) takes a prompt of many
    /// lines without sending it at the first. Every control character
    /// but a newline or a tab is dropped first, so no message can end the
    /// paste early and press keys of its own (an escape, a Return, a
    /// Ctrl-C).
    #[must_use]
    pub fn bracketed(text: &str) -> Self {
        let clean: String = text
            .chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
            .collect();
        Self {
            text: format!("{PASTE_START}{clean}{PASTE_END}").into_bytes(),
            submit: b"\r".to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TypedPrompt;

    #[test]
    fn a_message_cant_end_the_paste_or_press_keys_of_its_own() {
        let typed = TypedPrompt::bracketed("line one\nline two\x1b[201~\r/exit\x03\tend");
        let text = String::from_utf8(typed.text).unwrap();
        assert_eq!(text, "\x1b[200~line one\nline two[201~/exit\tend\x1b[201~");
        assert_eq!(text.matches('\x1b').count(), 2, "only the paste's own");
        assert!(!text.contains('\r'));
        assert_eq!(typed.submit, b"\r");
    }
}
