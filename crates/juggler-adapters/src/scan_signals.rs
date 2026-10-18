//! AJ-E302 — the widened scan's pure probes (S2/S4/S5/S9/S10,
//! pass2/03 §3). Every function takes its roots as parameters so the
//! fixture-home tests drive exactly what the real home would; every
//! read is METADATA-ONLY: existence, counts, directory names —
//! never credential file contents, never secret-store reads
//! (existence only), never general shell evaluation (line-scoped
//! parsing of two known variable names only).

use std::path::Path;

use juggler_adapter::environment::{AliasHint, ManagerHint, ScanHealth};

/// The two variable names the rc parse recognises (nothing else).
pub const ALIAS_VARIABLES: [&str; 2] = ["CLAUDE_CONFIG_DIR", "CODEX_HOME"];

/// The rc files scanned, in order (Req.2 — these and ONLY these).
#[must_use]
pub fn rc_files(home: &Path) -> Vec<std::path::PathBuf> {
    let mut out = vec![home.join(".zshrc"), home.join(".bashrc")];
    if let Ok(entries) = std::fs::read_dir(home.join(".config").join("fish")) {
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|ext| ext == "fish") {
                out.push(entry.path());
            }
        }
    }
    out
}

/// Extracts a path value from one rc assignment line for a known
/// variable. Handles `export VAR=path`, `VAR=path`, quoting, `~`
/// expansion, and trailing comments — nothing else is interpreted.
fn assignment_path(line: &str, home: &Path) -> Option<(String, std::path::PathBuf)> {
    let trimmed = line.trim();
    // fish syntax: `set -x VAR value` (flags sit between set and the
    // variable name; the value is the next bare token).
    if trimmed.starts_with("set ") {
        let mut tokens = trimmed.split_whitespace().skip(1);
        let name = tokens.find(|token| !token.starts_with('-'))?;
        if ALIAS_VARIABLES.contains(&name)
            && let Some(value) = tokens.next()
        {
            return expand_value(value, home).map(|path| (name.to_owned(), path));
        }
        return None;
    }
    // POSIX syntax: `export VAR=value` / `VAR=value`.
    let body = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    for variable in ALIAS_VARIABLES {
        let prefix = format!("{variable}=");
        if let Some(value) = body.strip_prefix(&prefix) {
            return expand_value(value, home).map(|path| (variable.to_owned(), path));
        }
    }
    None
}

/// Normalizes one raw value: strips trailing comments and quotes,
/// expands a leading `~` against `home`. Empty values yield `None`.
fn expand_value(raw: &str, home: &Path) -> Option<std::path::PathBuf> {
    let mut value = raw.split(" #").next().unwrap_or(raw).trim();
    if (value.starts_with('"') && value.ends_with('"') && value.len() >= 2)
        || (value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2)
    {
        value = &value[1..value.len() - 1];
    }
    if value.is_empty() {
        return None;
    }
    Some(if let Some(rest) = value.strip_prefix("~/") {
        home.join(rest)
    } else {
        std::path::PathBuf::from(value)
    })
}

/// S5: parse the rc files for `CLAUDE_CONFIG_DIR=` / `CODEX_HOME=`
/// assignments — path values only, source file + line recorded
/// (Req.2). Malformed lines are skipped, never fatal.
#[must_use]
pub fn scan_rc_assignments(home: &Path) -> (Vec<AliasHint>, ScanHealth) {
    let mut hints = Vec::new();
    let mut skipped = 0_u32;
    for file in rc_files(home) {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue; // not present — fine
        };
        for (index, line) in text.lines().enumerate() {
            // Lossy parse: a non-UTF-8-ish line simply won't match.
            if let Some((variable, path)) = assignment_path(line, home) {
                let target_exists = path.is_dir();
                if !target_exists {
                    skipped = skipped.saturating_add(1);
                }
                hints.push(AliasHint {
                    variable,
                    target_exists,
                    path,
                    source_file: file.to_string_lossy().into_owned(),
                    line: u32::try_from(index + 1).unwrap_or_default(),
                });
            }
        }
    }
    let health = if hints.is_empty() {
        ScanHealth::Ok
    } else if skipped > 0 {
        ScanHealth::Partial(format!(
            "{skipped} assignment(s) point at directories that no longer exist"
        ))
    } else {
        ScanHealth::Ok
    };
    (hints, health)
}

/// S4: the cc-switch probe — `~/.cc-switch/cc-switch.db` opened
/// READ-ONLY (immutable URI: no locks, no WAL, no writes). Returns
/// existence + provider-row count + provider NAMES only; a locked,
/// missing-table, or corrupt DB degrades to existence-only and NEVER
/// fails the scan (Req.3).
#[must_use]
pub fn scan_cc_switch(home: &Path) -> ManagerHint {
    let db = home.join(".cc-switch").join("cc-switch.db");
    if !db.is_file() {
        return ManagerHint {
            manager: "cc-switch".to_owned(),
            present: false,
            detail: String::new(),
            health: ScanHealth::Ok,
        };
    }
    let uri = format!("file:{}?mode=ro&immutable=1", db.to_string_lossy());
    let Ok(connection) = rusqlite::Connection::open_with_flags(
        &uri,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) else {
        return ManagerHint {
            manager: "cc-switch".to_owned(),
            present: true,
            detail: "database present (not readable — existence only)".to_owned(),
            health: ScanHealth::Denied("cc-switch database is locked or unreadable".to_owned()),
        };
    };
    // Names + count ONLY — never token columns (the query lists its
    // columns explicitly; a schema drift degrades, never guesses).
    let rows: Result<Vec<(i64, String)>, _> = connection
        .prepare("SELECT id, name FROM providers ORDER BY id") // cc-switch v3 schema
        .and_then(|mut statement| {
            let names = statement
                .query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(names)
        });
    match rows {
        Ok(names) => ManagerHint {
            manager: "cc-switch".to_owned(),
            present: true,
            detail: format!(
                "{} provider(s): {}",
                names.len(),
                names
                    .iter()
                    .map(|(_, name)| name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            health: ScanHealth::Ok,
        },
        Err(_) => ManagerHint {
            manager: "cc-switch".to_owned(),
            present: true,
            detail: "database present (schema not recognised — existence only)".to_owned(),
            health: ScanHealth::Partial(
                "cc-switch schema not recognised — count/names unavailable".to_owned(),
            ),
        },
    }
}

/// S9: project seed names from a config dir — the `projects/`
/// children's names and the transcript-cwd dir names, DIRECTORY
/// NAMES ONLY (never file contents, never full paths beyond the
/// encoded name the vendor already uses).
#[must_use]
pub fn project_seed_names(config_dirs: &[std::path::PathBuf]) -> Vec<String> {
    let mut names = std::collections::BTreeSet::new();
    for dir in config_dirs {
        let Ok(entries) = std::fs::read_dir(dir.join("projects")) else {
            continue;
        };
        for entry in entries.flatten() {
            if entry.path().is_dir()
                && let Some(name) = entry.file_name().to_str()
            {
                names.insert(name.to_owned());
            }
        }
    }
    names.into_iter().collect()
}

/// S10: desktop harness presence — `/Applications/*.app` existence
/// only (names, never bundle contents).
#[must_use]
pub fn desktop_apps_present(applications: &Path) -> Vec<String> {
    const KNOWN: [&str; 2] = ["Claude.app", "ZCode.app"];
    let mut out = Vec::new();
    for app in KNOWN {
        if applications.join(app).exists() {
            out.push(app.to_owned());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assignment_parse_handles_export_quote_tilde_and_comments() {
        let home = Path::new("/Users/dev");
        let (variable, path) =
            assignment_path("export CLAUDE_CONFIG_DIR=~/work-claude # daily", home).unwrap();
        assert_eq!(variable, "CLAUDE_CONFIG_DIR");
        assert_eq!(path, home.join("work-claude"));

        let (_, quoted) = assignment_path("CODEX_HOME=\"/opt/codex-a\"", home).unwrap();
        assert_eq!(quoted, std::path::PathBuf::from("/opt/codex-a"));

        assert!(assignment_path("export OTHER_VAR=x", home).is_none());
        assert!(assignment_path("CLAUDE_CONFIG_DIR=", home).is_none());
        assert!(assignment_path("echo CLAUDE_CONFIG_DIR=", home).is_none());
    }

    #[test]
    fn rc_scan_records_source_line_and_dangling_targets() {
        let work = tempfile::tempdir().unwrap();
        std::fs::write(
            work.path().join(".zshrc"),
            "# setup\nexport CLAUDE_CONFIG_DIR=~/gone\nexport CLAUDE_CONFIG_DIR=~/there\n",
        )
        .unwrap();
        std::fs::create_dir_all(work.path().join("there")).unwrap();
        let (hints, health) = scan_rc_assignments(work.path());
        assert_eq!(hints.len(), 2);
        assert!(
            hints[0].line == 2 && !hints[0].target_exists,
            "dangling recorded"
        );
        assert!(hints[1].target_exists);
        assert!(matches!(health, ScanHealth::Partial(_)));
    }
}
