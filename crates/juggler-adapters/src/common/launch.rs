//! What tools' terminal launches share: the folder a launch runs in, and
//! reading a running tool's command line for the command it runs.

use std::path::{Path, PathBuf};

use juggler_adapter::LaunchMode;
use juggler_core::model::Profile;

/// The folder a launch runs in: a resumed session continues work that
/// lives in `cwd`; anything else starts in the Launch settings' folder,
/// else `cwd`.
pub(crate) fn launch_cwd(profile: &Profile, cwd: &Path, mode: &LaunchMode) -> PathBuf {
    match mode {
        LaunchMode::Resume { .. } => cwd.to_path_buf(),
        _ => profile
            .default_cwd
            .clone()
            .unwrap_or_else(|| cwd.to_path_buf()),
    }
}

/// What a command line runs, read past its options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Command<'a> {
    /// An option that prints and exits (no session).
    Exits,
    /// No command word (before `--`, or at all).
    Bare,
    /// The first word that isn't an option, and where it is.
    Word(usize, &'a str),
}

/// A command line's [`Command`], read from `args`: an option in
/// `value_options` takes the next word with it, and one in `exits` prints
/// and exits.
pub(crate) fn command_word<'a>(
    args: &'a [String],
    value_options: &[&str],
    exits: &[&str],
) -> Command<'a> {
    let mut rest = args.iter().enumerate();
    while let Some((at, arg)) = rest.next() {
        if arg == "--" {
            break;
        }
        if value_options.contains(&arg.as_str()) {
            rest.next();
            continue;
        }
        if arg.starts_with('-') {
            if exits.contains(&arg.as_str()) {
                return Command::Exits;
            }
            continue;
        }
        return Command::Word(at, arg.as_str());
    }
    Command::Bare
}

/// A running tool's command line read for its session: `None` when it
/// prints and exits or runs one of its `helpers` (a server, a sign-in),
/// else what it runs ([`Command::Word`] or [`Command::Bare`]).
pub(crate) fn session_command<'a>(
    args: &'a [String],
    value_options: &[&str],
    exits: &[&str],
    helpers: &[&str],
) -> Option<Command<'a>> {
    match command_word(args, value_options, exits) {
        Command::Exits => None,
        Command::Word(_, word) if helpers.contains(&word) => None,
        command => Some(command),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_command_is_the_first_word_past_the_options() {
        let args = |words: &[&str]| words.iter().map(|w| (*w).to_owned()).collect::<Vec<_>>();
        let values = ["--model"];
        let exits = ["--version"];
        assert_eq!(
            command_word(&args(&["--model", "resume", "go", "x"]), &values, &exits),
            Command::Word(2, "go")
        );
        assert_eq!(
            command_word(&args(&["-q", "--version", "go"]), &values, &exits),
            Command::Exits
        );
        assert_eq!(
            command_word(&args(&["--", "go"]), &values, &exits),
            Command::Bare
        );
    }
}
