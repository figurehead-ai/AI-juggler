//! Lab 2 (D-40, 2026-10-25): Codex's folder trust, passed in memory.
//!
//! In a folder Codex hasn't been told to trust, its terminal (`codex`,
//! `codex resume`) first shows "Trust this folder? … Your trust decision
//! will be saved", and the answer is written to the user's `config.toml`.
//! AI Juggler never lets that happen for the user. It asks the same
//! question itself, keeps the answer in its own data, and passes it to
//! each launch as a config override:
//! `-c projects={"<folder>"={trust_level="trusted"|"untrusted"}}`. Lab 2
//! showed this saves nothing ("Opening will not change saved trust").
//!
//! A folder the user already decided in Codex (its `config.toml` has the
//! folder's `trust_level`) gets nothing: Codex follows its own setting.

use std::path::Path;

use juggler_adapter::folder_trust::FolderTrust;

/// The question, in Codex's own words (its trust screen and its "Open
/// restricted" choice, lab 2).
pub const QUESTION: FolderTrust = FolderTrust {
    question: "Trust this folder?",
    trust: "Trust and continue",
    restrict: "Open restricted",
    note: "Codex asks this once per folder. AI Juggler keeps your answer and passes it to each Codex session here, so Codex's config.toml is never changed.",
};

/// The largest `config.toml` read for the folder's trust.
const CONFIG_MAX: u64 = 1024 * 1024;

/// Whether `home`'s `config.toml` already says how far `cwd` is trusted
/// (only that key is looked at).
#[must_use]
pub fn decided(home: &Path, cwd: &Path) -> bool {
    let file = home.join("config.toml");
    let Ok(meta) = std::fs::symlink_metadata(&file) else {
        return false;
    };
    if !meta.is_file() || meta.len() > CONFIG_MAX {
        return false;
    }
    let Some(config) = std::fs::read_to_string(&file)
        .ok()
        .and_then(|text| text.parse::<toml::Table>().ok())
    else {
        return false;
    };
    let projects = config.get("projects").and_then(toml::Value::as_table);
    [cwd.to_path_buf(), cwd.canonicalize().unwrap_or_default()]
        .iter()
        .filter_map(|path| path.to_str())
        .any(|path| {
            projects
                .and_then(|projects| projects.get(path))
                .and_then(|project| project.get("trust_level"))
                .is_some()
        })
}

/// The override for `cwd`, in front of Codex's own arguments (a root
/// option every subcommand takes).
pub fn apply(args: &mut Vec<String>, cwd: &Path, trusted: bool) {
    // Codex keys a project by its real path.
    let real = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
    let folder = toml::Value::String(real.to_string_lossy().into_owned()).to_string();
    let level = if trusted { "trusted" } else { "untrusted" };
    args.splice(
        0..0,
        [
            "-c".to_owned(),
            format!("projects={{{folder}={{trust_level=\"{level}\"}}}}"),
        ],
    );
}

#[cfg(test)]
mod tests {
    use super::{apply, decided};

    #[test]
    fn the_choice_goes_in_front_in_memory_and_quoted() {
        let mut args = vec![
            "resume".to_owned(),
            "t1".to_owned(),
            "--no-daemon".to_owned(),
        ];
        apply(&mut args, std::path::Path::new("/Users/me/lab \"x\""), true);
        assert_eq!(args[0], "-c");
        assert_eq!(args[2..], ["resume", "t1", "--no-daemon"]);
        // The override is TOML Codex reads back to the same folder, quoted.
        let parsed: toml::Table = args[1].parse().unwrap();
        assert_eq!(
            parsed["projects"]["/Users/me/lab \"x\""]["trust_level"].as_str(),
            Some("trusted")
        );
        let mut args = vec!["app-server".to_owned()];
        apply(&mut args, std::path::Path::new("/kit/project"), false);
        assert_eq!(
            args[1],
            r#"projects={"/kit/project"={trust_level="untrusted"}}"#
        );
    }

    #[test]
    fn a_folder_decided_in_codex_is_left_to_codex() {
        let home = tempfile::tempdir().unwrap();
        let lab = tempfile::tempdir().unwrap();
        assert!(!decided(home.path(), lab.path()), "no config");
        std::fs::write(
            home.path().join("config.toml"),
            format!(
                "model = \"x\"\n[projects.\"{}\"]\ntrust_level = \"trusted\"\n",
                lab.path().canonicalize().unwrap().display()
            ),
        )
        .unwrap();
        assert!(decided(home.path(), lab.path()));
        assert!(!decided(home.path(), std::path::Path::new("/elsewhere")));
    }
}
