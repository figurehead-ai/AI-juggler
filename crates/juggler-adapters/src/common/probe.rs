//! AJ-E2016: asking an installed tool its version, the one way every
//! adapter whose tool reports it with `--version` does.

use std::path::Path;

use juggler_adapter::InstallStatus;

/// Runs `bin --version` with the user's PATH (an npm package runs on
/// `node`) and reads the version with the tool's own `parse`. The
/// failure, ready to report, when it doesn't run, exits non-zero, or says
/// something `parse` doesn't recognise.
pub(crate) fn version(
    bin: &Path,
    parse: impl Fn(&str) -> Option<String>,
) -> Result<String, InstallStatus> {
    let output = juggler_adapter::user_path::command(bin)
        .arg("--version")
        .output()
        .map_err(|error| InstallStatus::Failed {
            message: error.to_string(),
        })?;
    if !output.status.success() {
        return Err(InstallStatus::Failed {
            message: format!("--version exited with {}", output.status),
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse(&stdout).ok_or_else(|| InstallStatus::Failed {
        message: format!("unrecognised --version output: {stdout:?}"),
    })
}
