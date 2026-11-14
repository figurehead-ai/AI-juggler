//! A throwaway lab `HOME` (`scripts/qa/lab_scenario.sh`'s): what the lab
//! write mode (ADR-0027) and the lab's Muse switch (AJ-E1612) both stand
//! on. One check, so the two can't drift apart.

use std::path::{Path, PathBuf};

/// `home`, resolved, when it is a throwaway lab by its place and name:
/// absolute, spelled as it resolves (only macOS's `/tmp` →
/// `/private/tmp` allowed: a symlink anywhere in it, a trailing `/` or
/// `/.`, is refused), directly under `/tmp`, named `aij-lab…`. Who owns
/// it and what else it overlaps are the caller's to check.
///
/// # Errors
///
/// What makes it not a lab, in words.
pub fn throwaway_lab(home: &Path) -> Result<PathBuf, String> {
    if !home.is_absolute() {
        return Err(format!("HOME {} isn't an absolute path", home.display()));
    }
    let resolved =
        std::fs::canonicalize(home).map_err(|e| format!("HOME {}: {e}", home.display()))?;
    let spelled = home.to_string_lossy();
    let shown = resolved.to_string_lossy();
    if shown != spelled && shown != format!("/private{spelled}") {
        return Err(format!(
            "HOME {} is a symlink or isn't spelled as it resolves ({shown})",
            home.display()
        ));
    }
    let in_tmp = matches!(
        resolved.parent().and_then(Path::to_str),
        Some("/tmp" | "/private/tmp")
    );
    let named = resolved
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("aij-lab"));
    if !in_tmp || !named {
        return Err(format!(
            "HOME {} isn't a lab folder (/tmp/aij-lab…)",
            resolved.display()
        ));
    }
    Ok(resolved)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "a test fails loudly on a broken fixture"
)]
mod tests {
    use super::*;

    /// A real lab folder under /tmp, removed when dropped.
    struct TmpLab(PathBuf);

    impl TmpLab {
        fn new() -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos());
            let lab = PathBuf::from(format!(
                "/tmp/aij-lab-core-unit-{}-{nanos}",
                std::process::id()
            ));
            std::fs::create_dir_all(&lab).unwrap();
            Self(lab)
        }
    }

    impl Drop for TmpLab {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn only_a_throwaway_lab_home_is_a_lab() {
        let lab = TmpLab::new();
        assert!(throwaway_lab(&lab.0).is_ok());
        for not_lab in [
            "/Users/you",
            "/tmp",
            "/tmp/other",
            "/var/aij-lab-x",
            "relative/aij-lab",
        ] {
            assert!(throwaway_lab(Path::new(not_lab)).is_err(), "{not_lab}");
        }
        // A trailing `/.` isn't how it resolves.
        assert!(throwaway_lab(&lab.0.join(".")).is_err());
        // Inside a lab isn't the lab.
        let inner = lab.0.join("aij-lab-inner");
        std::fs::create_dir_all(&inner).unwrap();
        assert!(throwaway_lab(&inner).is_err());
    }
}
