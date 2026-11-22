//! AJ-E2016: asking an installed tool its version, the one way every
//! adapter whose tool reports it with `--version` does.

use std::path::{Path, PathBuf};

use juggler_adapter::{DiscoveryReport, InstallStatus, VerificationStatus};
use juggler_core::model::HarnessId;

/// The binary a test pinned, else the file an override variable `var`
/// names (tests, dev).
pub(crate) fn pinned_bin(pinned: Option<&PathBuf>, var: &str) -> Option<PathBuf> {
    pinned.cloned().or_else(|| {
        std::env::var_os(var)
            .map(PathBuf::from)
            .filter(|path| path.is_file())
    })
}

/// What a discovery found, unverified: no version has been run live yet.
pub(crate) fn unverified_report(
    adapter: HarnessId,
    binary: Option<PathBuf>,
    install: InstallStatus,
    notes: Vec<String>,
) -> DiscoveryReport {
    DiscoveryReport {
        adapter,
        install,
        verification: VerificationStatus::Unverified,
        binary,
        notes,
    }
}

/// The discovery of a tool that is one binary: not found (saying where it
/// was `looked` for) unless `bin` is a file, else its version from
/// `version` (its failure as it is), with `note`.
pub(crate) fn discover_bin(
    adapter: HarnessId,
    bin: Option<PathBuf>,
    looked: &str,
    version: impl FnOnce(&Path) -> Result<String, InstallStatus>,
    note: &str,
) -> DiscoveryReport {
    let Some(bin) = bin.filter(|bin| bin.is_file()) else {
        return unverified_report(
            adapter,
            None,
            InstallStatus::NotFound,
            vec![looked.to_owned()],
        );
    };
    match version(&bin) {
        Ok(version) => unverified_report(
            adapter,
            Some(bin),
            InstallStatus::Installed { version },
            vec![note.to_owned()],
        ),
        Err(failed) => unverified_report(adapter, Some(bin), failed, Vec::new()),
    }
}

/// The first word of `stdout`'s first line that starts with a digit, a
/// leading `v` dropped: a version, whatever name or build surrounds it.
pub(crate) fn first_version_word(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .next()?
        .split_whitespace()
        .map(|word| word.strip_prefix('v').unwrap_or(word))
        .find(|word| word.starts_with(|c: char| c.is_ascii_digit()))
        .map(str::to_owned)
}

/// Runs `bin --version` with the user's PATH (an npm package runs on
/// `node`) and reads the version with the tool's own `parse`. The
/// failure, ready to report, when it doesn't run, exits non-zero, or says
/// something `parse` doesn't recognise.
pub(crate) fn version(
    bin: &Path,
    parse: impl Fn(&str) -> Option<String>,
) -> Result<String, InstallStatus> {
    version_with(bin, ["--version"], parse)
}

/// [`version`] for a tool that says its version another way: `program`
/// run with `args` (a script's interpreter, the script, its own word).
pub(crate) fn version_with<I, S>(
    program: &Path,
    args: I,
    parse: impl Fn(&str) -> Option<String>,
) -> Result<String, InstallStatus>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    version_with_env(program, args, &[], parse)
}

/// [`version_with`] with `env` set for the run (AJ-E1605: `ZCode`'s app
/// runs its engine only with `ELECTRON_RUN_AS_NODE=1`).
pub(crate) fn version_with_env<I, S>(
    program: &Path,
    args: I,
    env: &[(&str, &str)],
    parse: impl Fn(&str) -> Option<String>,
) -> Result<String, InstallStatus>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let args: Vec<std::ffi::OsString> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect();
    // Named in a failure by its last word (`--version`, `version`).
    let what = args
        .last()
        .map(|arg| arg.to_string_lossy().into_owned())
        .unwrap_or_default();
    let output = juggler_adapter::user_path::command(program)
        .args(&args)
        .envs(env.iter().copied())
        .output()
        .map_err(|error| InstallStatus::Failed {
            message: error.to_string(),
        })?;
    if !output.status.success() {
        return Err(InstallStatus::Failed {
            message: format!("{what} exited with {}", output.status),
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse(&stdout).ok_or_else(|| InstallStatus::Failed {
        message: format!("unrecognised {what} output: {stdout:?}"),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_version_is_the_first_numeric_word_of_the_first_line() {
        let word = super::first_version_word;
        assert_eq!(
            word("grok 1.0.77 (50ca579fa996)").as_deref(),
            Some("1.0.77")
        );
        assert_eq!(word("v0.16.9\nmore").as_deref(), Some("0.16.9"));
        assert_eq!(word("no version here"), None);
        assert_eq!(word(""), None);
    }
}
