//! What a click on a link the terminal printed opens (AJ-E1506).
//!
//! Terminal output is untrusted: anything a tool, a file or a web page
//! printed can be a "link". So a web link opens only as `http`/`https`, in
//! the browser; a path opens only if it is there — a file in the editor
//! the user chose for projects, else shown in Finder, and a folder shown
//! in Finder. Nothing is ever run, or handed to the Mac's default app for
//! its type (a `.command` file or an app would run).

use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};

/// The longest link the terminal hands over (a URL or a path).
const MAX_LINK: usize = 4096;

/// Where a clicked link leads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    /// An `http`/`https` address, for the browser.
    Web(String),
    /// A file that is there, for the editor (or Finder).
    File(PathBuf),
    /// A folder that is there, for Finder.
    Folder(PathBuf),
}

/// Resolves what the terminal printed: a web address, or a path —
/// absolute, `~/…`, or relative to the session's folder (`cwd`; a login
/// terminal has none) — with any `:line` or `:line:column` after it
/// dropped.
///
/// # Errors
///
/// [`AppError::Refused`] for anything else: another scheme, control
/// characters, a relative path with no folder to start from, or nothing at
/// that path.
pub fn resolve(target: &str, cwd: Option<&Path>, home: Option<&Path>) -> AppResult<LinkTarget> {
    let target = target.trim();
    if target.is_empty() || target.len() > MAX_LINK || target.chars().any(char::is_control) {
        return Err(AppError::Refused(
            "That isn't a link AI Juggler can open".into(),
        ));
    }
    let lower = target.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        if target.chars().any(char::is_whitespace) {
            return Err(AppError::Refused(
                "That isn't a link AI Juggler can open".into(),
            ));
        }
        return Ok(LinkTarget::Web(target.to_owned()));
    }
    if lower.contains("://") {
        return Err(AppError::Refused(
            "Only web links (http and https) open from the terminal".into(),
        ));
    }
    let typed = without_position(target);
    let path = if typed == "~" {
        home.map(Path::to_path_buf)
    } else if let Some(rest) = typed.strip_prefix("~/") {
        home.map(|home| home.join(rest))
    } else if Path::new(typed).is_absolute() {
        Some(PathBuf::from(typed))
    } else {
        cwd.map(|cwd| cwd.join(typed))
    };
    let Some(path) = path else {
        return Err(AppError::Refused(format!(
            "{typed} is relative, and this terminal has no folder to start from"
        )));
    };
    let Ok(found) = path.canonicalize() else {
        return Err(AppError::Refused(format!("Nothing is at {typed}")));
    };
    if found.is_dir() {
        Ok(LinkTarget::Folder(found))
    } else if found.is_file() {
        Ok(LinkTarget::File(found))
    } else {
        Err(AppError::Refused(format!(
            "{typed} isn't a file or a folder"
        )))
    }
}

/// `src/a.rs:42:7` → `src/a.rs`: editors' and compilers' line and column.
fn without_position(target: &str) -> &str {
    let mut rest = target;
    for _ in 0..2 {
        match rest.rsplit_once(':') {
            Some((head, tail))
                if !head.is_empty()
                    && !tail.is_empty()
                    && tail.bytes().all(|b| b.is_ascii_digit()) =>
            {
                rest = head;
            }
            _ => break,
        }
    }
    rest
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src/auth")).unwrap();
        std::fs::write(dir.path().join("src/auth/session.test.ts"), "x").unwrap();
        dir
    }

    #[test]
    fn web_links_open_only_as_http_or_https() {
        assert_eq!(
            resolve("https://example.com/a?b=1", None, None).unwrap(),
            LinkTarget::Web("https://example.com/a?b=1".into())
        );
        for refused in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "vscode://x",
            "ssh://host",
            "https://a b",
            "http://a\u{7}",
        ] {
            assert!(resolve(refused, None, None).is_err(), "{refused}");
        }
    }

    #[test]
    fn a_relative_path_starts_from_the_session_folder_and_drops_line_and_column() {
        let dir = folder();
        let file = dir
            .path()
            .join("src/auth/session.test.ts")
            .canonicalize()
            .unwrap();
        for typed in [
            "src/auth/session.test.ts",
            "src/auth/session.test.ts:42",
            "./src/auth/session.test.ts:42:7",
        ] {
            assert_eq!(
                resolve(typed, Some(dir.path()), None).unwrap(),
                LinkTarget::File(file.clone()),
                "{typed}"
            );
        }
        assert_eq!(
            resolve("src/auth", Some(dir.path()), None).unwrap(),
            LinkTarget::Folder(dir.path().join("src/auth").canonicalize().unwrap())
        );
    }

    #[test]
    fn home_and_absolute_paths_resolve_and_nothing_there_is_refused() {
        let dir = folder();
        let file = dir
            .path()
            .join("src/auth/session.test.ts")
            .canonicalize()
            .unwrap();
        assert_eq!(
            resolve("~/src/auth/session.test.ts", None, Some(dir.path())).unwrap(),
            LinkTarget::File(file.clone())
        );
        assert_eq!(
            resolve(file.to_str().unwrap(), None, None).unwrap(),
            LinkTarget::File(file)
        );
        assert!(resolve("src/missing.ts", Some(dir.path()), None).is_err());
        // A login terminal has no folder: a relative path can't be guessed.
        assert!(resolve("src/auth/session.test.ts", None, None).is_err());
        assert!(resolve("", None, None).is_err());
    }
}
