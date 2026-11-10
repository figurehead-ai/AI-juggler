//! Whether Claude Code can launch into a folder (split from `claude_code`,
//! the size budget): the files its launch reads, and its own first-run
//! setup, which a switch finishes (0.3.5).

use std::path::Path;

use juggler_adapter::AdapterError;

/// 0.3.2: Claude Code launches into `dir` only once it can read its state
/// file and projects, and its first-run setup is done there
/// (`hasCompletedOnboarding`); otherwise `--resume` opens on the setup
/// screen, not the conversation (the maintainer's walk). 0.3.4: an
/// account used every day through the Claude app's Code tab never has it
/// (the app runs Claude Code without its first run), and a sign-in change
/// clears it; the words say so, and a state file it can't parse says that
/// instead.
///
/// # Errors
///
/// What keeps the launch from working.
pub fn launch_ready_in(
    state_file: &Path,
    dir: &Path,
) -> Result<(), juggler_adapter::readiness::NotReady> {
    use juggler_adapter::readiness::{NotReady, probe_read};
    probe_read(state_file)?;
    probe_read(&dir.join("projects"))?;
    let Ok(text) = std::fs::read_to_string(state_file) else {
        return Err(NotReady::NotSetUp(
            "Claude Code isn't signed in on this account yet. Sign in to it from Accounts, then switch."
                .to_owned(),
        ));
    };
    let Ok(state) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Err(NotReady::NotSetUp(
            "AI Juggler couldn't read Claude Code's settings on this account (it may have been saving them). Try again in a moment."
                .to_owned(),
        ));
    };
    if state.get("hasCompletedOnboarding") == Some(&serde_json::Value::Bool(true)) {
        Ok(())
    } else {
        // 0.3.5: signed in, its own first run not done (a Claude app
        // profile's folder): a switch finishes it, never a refusal.
        Err(NotReady::FirstRun)
    }
}

/// 0.3.5: marks Claude Code's first run done in `state_file`, as Claude
/// Code itself records it when its first-run setup ends
/// (`hasCompletedOnboarding`); every other byte of the file is kept, and
/// the write is atomic. A missing file (never signed in) is refused.
///
/// # Errors
///
/// The file is missing, isn't a JSON object, or can't be written.
pub fn complete_first_run_in(state_file: &Path) -> Result<(), AdapterError> {
    let text = std::fs::read_to_string(state_file).map_err(|_| {
        AdapterError::Unsupported("Claude Code isn't signed in on this account yet".into())
    })?;
    let (new, before) = juggler_adapter::jsonc::set(&text, "hasCompletedOnboarding", "true")
        .map_err(AdapterError::Unsupported)?;
    if before.as_deref().map(str::trim) == Some("true") {
        return Ok(());
    }
    juggler_adapter::inventory::rewrite_in_place(state_file, &new)?;
    Ok(())
}

/// The questions Claude Code's own screens ask before its session opens
/// (its first run, a folder it doesn't trust yet, a sign-in), each by
/// the words only that screen shows and its first line as it says it.
const SCREENS: &[juggler_adapter::screen_text::ScreenQuestion] = &[
    juggler_adapter::screen_text::ScreenQuestion {
        shows: "Choose the text style that looks best with your terminal",
        asks: "Choose the text style that looks best with your terminal",
    },
    juggler_adapter::screen_text::ScreenQuestion {
        shows: "Select login method",
        asks: "Select login method",
    },
    juggler_adapter::screen_text::ScreenQuestion {
        shows: "Is this a project you created or one you trust?",
        asks: "Is this a project you created or one you trust?",
    },
    juggler_adapter::screen_text::ScreenQuestion {
        shows: "Do you trust the files in this folder?",
        asks: "Do you trust the files in this folder?",
    },
    juggler_adapter::screen_text::ScreenQuestion {
        shows: "Use Claude Code's terminal setup?",
        asks: "Use Claude Code's terminal setup?",
    },
];

/// What Claude Code's own screen at the end of `tail` asks, in its words
/// (`ToolConfig::screen_ask`); `None` once it has moved on.
#[must_use]
pub fn screen_ask(tail: &str) -> Option<String> {
    juggler_adapter::screen_text::ends_on(tail, SCREENS).map(str::to_owned)
}

#[cfg(test)]
mod launch_ready_tests {
    use super::launch_ready_in;
    use juggler_adapter::readiness::NotReady;

    /// 0.3.2: the switched session opened on Claude Code's setup screen.
    #[test]
    fn a_launch_needs_the_setup_done_there() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join(".claude.json");
        assert!(
            matches!(
                launch_ready_in(&state, dir.path()),
                Err(NotReady::NotSetUp(words)) if words.contains("isn't signed in")
            ),
            "no state file: never signed in there"
        );
        std::fs::write(&state, r#"{"numStartups":1}"#).unwrap();
        assert_eq!(
            launch_ready_in(&state, dir.path()),
            Err(NotReady::FirstRun),
            "signed in, its first run not done: a switch finishes it"
        );
        std::fs::write(&state, r#"{"hasCompletedOnboarding":true}"#).unwrap();
        assert_eq!(launch_ready_in(&state, dir.path()), Ok(()));
    }

    /// 0.3.4: a settings file it can't parse isn't "not set up".
    #[test]
    fn an_unreadable_state_file_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join(".claude.json");
        std::fs::write(&state, r#"{"hasCompletedOnboard"#).unwrap();
        let half = launch_ready_in(&state, dir.path());
        assert!(
            matches!(&half, Err(NotReady::NotSetUp(words)) if words.contains("couldn't read")),
            "a half-written file isn't ready: {half:?}"
        );
        std::fs::write(&state, r#"{"hasCompletedOnboarding":false}"#).unwrap();
        assert_eq!(launch_ready_in(&state, dir.path()), Err(NotReady::FirstRun));
    }

    /// 0.3.5: the maintainer's Claude app profiles never ran Claude Code
    /// in a terminal, so a switch into one was refused. Finishing the
    /// first run sets the one key Claude Code sets, and keeps every other
    /// byte as it was.
    #[test]
    fn finishing_the_first_run_sets_only_its_key() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join(".claude.json");
        let before = "{\n  \"numStartups\": 3,\n  \"oauthAccount\": {\n    \"emailAddress\": \"you@example.invalid\"\n  },\n  \"hasCompletedOnboarding\": false\n}\n";
        std::fs::write(&state, before).unwrap();
        super::complete_first_run_in(&state).unwrap();
        let after = std::fs::read_to_string(&state).unwrap();
        assert_eq!(
            after,
            before.replace(
                "\"hasCompletedOnboarding\": false",
                "\"hasCompletedOnboarding\": true"
            )
        );
        assert_eq!(launch_ready_in(&state, dir.path()), Ok(()));
        // Absent: added in the file's own layout; done already: unchanged.
        std::fs::write(&state, "{\n  \"numStartups\": 1\n}\n").unwrap();
        super::complete_first_run_in(&state).unwrap();
        assert_eq!(launch_ready_in(&state, dir.path()), Ok(()));
        let done = std::fs::read_to_string(&state).unwrap();
        super::complete_first_run_in(&state).unwrap();
        assert_eq!(std::fs::read_to_string(&state).unwrap(), done);
        // Never signed in there: refused, nothing made.
        let none = dir.path().join("missing.json");
        assert!(super::complete_first_run_in(&none).is_err());
        assert!(!none.exists());
    }
}
