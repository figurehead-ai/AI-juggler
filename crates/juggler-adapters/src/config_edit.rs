//! AJ-E1508: settings set in a tool's config file, and taken out again,
//! with every other byte of the file kept: comments, order and spacing.
//! A key that's already there is never replaced, and an undo takes a key
//! out only while it still holds the value that was set (compared by
//! fingerprint, so a tool re-saving the file in its own layout doesn't
//! count as a change); anything else is the user's.

use std::path::Path;

use juggler_adapter::inventory::{ConfigFormat, rewrite_in_place};
use juggler_adapter::jsonc;

/// What taking the settings out did.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Removed {
    /// They were there as set, and are gone now.
    Done,
    /// None of them is there any more (the user took them out).
    Gone,
}

/// A value's fingerprint, its layout aside: the same value written on one
/// line or many gives the same one. Recorded for an undo instead of the
/// value itself.
#[must_use]
pub fn fingerprint(format: ConfigFormat, value: &str) -> String {
    let canonical = match format {
        ConfigFormat::Json => serde_json::from_str::<serde_json::Value>(value.trim())
            .map_or_else(|_| without_layout(value), |parsed| parsed.to_string()),
        ConfigFormat::Toml => {
            parse_value(value).map_or_else(|_| without_layout(value), |parsed| normal(&parsed))
        }
    };
    juggler_adapter::handover::sha256_hex(canonical.as_bytes())
}

/// `text` without the spaces and line breaks outside its strings.
fn without_layout(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let (mut quoted, mut escaped) = (false, false);
    for c in text.chars() {
        if quoted {
            out.push(c);
            match (escaped, c) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => quoted = false,
                _ => {}
            }
        } else if c == '"' {
            quoted = true;
            out.push(c);
        } else if !c.is_whitespace() {
            out.push(c);
        }
    }
    out
}

/// Every string in `value` (keys and values alike), for the secret check.
#[must_use]
pub fn strings_of(format: ConfigFormat, value: &str) -> Vec<String> {
    let mut out = Vec::new();
    match format {
        ConfigFormat::Json => {
            fn walk(value: &serde_json::Value, out: &mut Vec<String>) {
                match value {
                    serde_json::Value::String(text) => out.push(text.clone()),
                    serde_json::Value::Array(items) => {
                        for item in items {
                            walk(item, out);
                        }
                    }
                    serde_json::Value::Object(map) => {
                        for (key, item) in map {
                            out.push(key.clone());
                            walk(item, out);
                        }
                    }
                    _ => {}
                }
            }
            match serde_json::from_str::<serde_json::Value>(value.trim()) {
                Ok(parsed) => walk(&parsed, &mut out),
                // Not plain JSON (a comment in it): all of it, as text.
                Err(_) => out.push(value.to_owned()),
            }
        }
        ConfigFormat::Toml => {
            fn walk(value: &toml_edit::Value, out: &mut Vec<String>) {
                match value {
                    toml_edit::Value::String(text) => out.push(text.value().clone()),
                    toml_edit::Value::Array(items) => {
                        for item in items {
                            walk(item, out);
                        }
                    }
                    toml_edit::Value::InlineTable(table) => {
                        for (key, item) in table {
                            out.push(key.to_owned());
                            walk(item, out);
                        }
                    }
                    _ => {}
                }
            }
            match parse_value(value) {
                Ok(parsed) => walk(&parsed, &mut out),
                Err(_) => out.push(value.to_owned()),
            }
        }
    }
    out
}

/// Sets each of `entries` (key, value text) in `file`, creating the file
/// when there's none. Refused, with nothing written, when any key is there
/// already.
///
/// # Errors
///
/// The key that's there already, a file that isn't valid `format`, or the
/// write's error, in words.
pub fn set(file: &Path, format: ConfigFormat, entries: &[(String, String)]) -> Result<(), String> {
    let existed = std::fs::symlink_metadata(file).is_ok();
    let text = if existed {
        std::fs::read_to_string(file).map_err(|error| error.to_string())?
    } else {
        match format {
            ConfigFormat::Json => "{}\n".to_owned(),
            ConfigFormat::Toml => String::new(),
        }
    };
    let new = match format {
        ConfigFormat::Json => {
            let mut text = text;
            for (key, value) in entries {
                if jsonc::value_text(&text, key)?.is_some() {
                    return Err(format!("it has its own {key} already"));
                }
                text = jsonc::set(&text, key, value)?.0;
            }
            text
        }
        ConfigFormat::Toml => {
            let mut doc: toml_edit::DocumentMut = text
                .parse()
                .map_err(|error: toml_edit::TomlError| error.to_string())?;
            for (key, value) in entries {
                let parsed = parse_value(value)?;
                let (table, last) = table_for(&mut doc, key)?;
                if table.contains_key(last) {
                    return Err(format!("it has its own {key} already"));
                }
                table.insert(last, toml_edit::Item::Value(parsed));
            }
            doc.to_string()
        }
    };
    if existed {
        return rewrite_in_place(file, &new).map_err(|error| error.to_string());
    }
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    // A new file only: never through a link left where it would go.
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(file)
        .and_then(|mut created| std::io::Write::write_all(&mut created, new.as_bytes()))
        .map_err(|error| error.to_string())
}

/// Takes each of `entries` (key, [`fingerprint`] of the value that was
/// set) out of `file` again, while every one that's still there holds that
/// value.
///
/// # Errors
///
/// When one of them holds another value now (the user changed it: nothing
/// is taken out), or the file can't be read or written, in words.
pub fn remove(
    file: &Path,
    format: ConfigFormat,
    entries: &[(String, String)],
) -> Result<Removed, String> {
    let Ok(text) = std::fs::read_to_string(file) else {
        return Ok(Removed::Gone);
    };
    let new = match format {
        ConfigFormat::Json => {
            let mut out = text.clone();
            for (key, was) in entries {
                match jsonc::value_text(&out, key)? {
                    None => {}
                    Some(now) if fingerprint(format, &now) == *was => {
                        out = jsonc::unset(&out, key, &now, None)?.unwrap_or(out);
                    }
                    Some(_) => return Err(format!("its {key} has changed since")),
                }
            }
            out
        }
        ConfigFormat::Toml => {
            let mut doc: toml_edit::DocumentMut = text
                .parse()
                .map_err(|error: toml_edit::TomlError| error.to_string())?;
            for (key, was) in entries {
                let mut parts: Vec<&str> = key.split('.').collect();
                let last = parts.pop().unwrap_or_default();
                // Walk to it without making any table on the way.
                let mut table = Some(doc.as_table_mut());
                for part in &parts {
                    table = table
                        .and_then(|t| t.get_mut(part))
                        .and_then(toml_edit::Item::as_table_mut);
                }
                let Some(table) = table else { continue };
                match table.get(last) {
                    None => continue,
                    Some(item) => {
                        let now = item.as_value().map(normal).unwrap_or_default();
                        if juggler_adapter::handover::sha256_hex(now.as_bytes()) != *was {
                            return Err(format!("its {key} has changed since"));
                        }
                    }
                }
                table.remove(last);
                // A table left empty stays: it may be the user's, with
                // their comments (re-review of AJ-E1508).
            }
            doc.to_string()
        }
    };
    if new == text {
        return Ok(Removed::Gone);
    }
    rewrite_in_place(file, &new).map_err(|error| error.to_string())?;
    Ok(Removed::Done)
}

/// The text of `key`'s value in `file` (JSON's as written, TOML's without
/// its spacing), when it's there.
#[must_use]
pub fn read(file: &Path, format: ConfigFormat, key: &str) -> Option<String> {
    let text = std::fs::read_to_string(file).ok()?;
    match format {
        ConfigFormat::Json => jsonc::value_text(&text, key).ok().flatten(),
        ConfigFormat::Toml => {
            let doc: toml_edit::DocumentMut = text.parse().ok()?;
            let mut item = doc.as_item();
            for part in key.split('.') {
                item = item.get(part)?;
            }
            item.as_value().map(normal)
        }
    }
}

/// A value's text, parsed.
fn parse_value(text: &str) -> Result<toml_edit::Value, String> {
    text.trim()
        .parse::<toml_edit::Value>()
        .map_err(|error| format!("not a TOML value: {error}"))
}

/// A value without its surrounding spacing, for comparing.
fn normal(value: &toml_edit::Value) -> String {
    let mut value = value.clone();
    value.decor_mut().clear();
    value.to_string()
}

/// The table holding dotted `key`'s last part (created when missing), and
/// that part.
fn table_for<'a, 'k>(
    doc: &'a mut toml_edit::DocumentMut,
    key: &'k str,
) -> Result<(&'a mut toml_edit::Table, &'k str), String> {
    let mut parts: Vec<&str> = key.split('.').collect();
    let last = parts.pop().ok_or_else(|| "an empty key".to_owned())?;
    let mut table = doc.as_table_mut();
    for part in parts {
        let entry = table
            .entry(part)
            .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()));
        table = entry
            .as_table_mut()
            .ok_or_else(|| format!("{part} isn't a table"))?;
    }
    Ok((table, last))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    /// What an undo records for `entries`.
    fn recorded(format: ConfigFormat, entries: &[(String, String)]) -> Vec<(String, String)> {
        entries
            .iter()
            .map(|(key, value)| (key.clone(), fingerprint(format, value)))
            .collect()
    }

    #[test]
    fn a_json_setting_is_set_beside_the_rest_and_taken_out_again() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.json");
        let before = "{\n  // mine\n  \"theme\": \"dark\"\n}\n";
        std::fs::write(&file, before).unwrap();
        let set_these = entries(&[("permissions", "{\"allow\": [\"Bash(ls)\"]}")]);
        set(&file, ConfigFormat::Json, &set_these).unwrap();
        let now = std::fs::read_to_string(&file).unwrap();
        assert!(
            now.contains("// mine") && now.contains("\"permissions\": {\"allow\": [\"Bash(ls)\"]}"),
            "{now}"
        );
        assert_eq!(
            read(&file, ConfigFormat::Json, "permissions").as_deref(),
            Some("{\"allow\": [\"Bash(ls)\"]}")
        );
        // Never over the user's own.
        assert!(set(&file, ConfigFormat::Json, &set_these).is_err());
        let undo = recorded(ConfigFormat::Json, &set_these);
        assert_eq!(
            remove(&file, ConfigFormat::Json, &undo).unwrap(),
            Removed::Done
        );
        assert_eq!(std::fs::read_to_string(&file).unwrap(), before);
        assert_eq!(
            remove(&file, ConfigFormat::Json, &undo).unwrap(),
            Removed::Gone
        );
    }

    #[test]
    fn a_json_setting_the_tool_laid_out_again_still_comes_out() {
        // Review of AJ-E1508 (M2): Claude Code re-saves settings.json in its
        // own layout; the value is the same.
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.json");
        let set_these = entries(&[("permissions", "{\"allow\": [\"Bash(ls)\"], \"deny\": []}")]);
        set(&file, ConfigFormat::Json, &set_these).unwrap();
        std::fs::write(
            &file,
            "{\n  \"permissions\": {\n    \"allow\": [\n      \"Bash(ls)\"\n    ],\n    \"deny\": []\n  },\n  \"theme\": \"dark\"\n}\n",
        )
        .unwrap();
        let undo = recorded(ConfigFormat::Json, &set_these);
        assert_eq!(
            remove(&file, ConfigFormat::Json, &undo).unwrap(),
            Removed::Done
        );
        let after: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert!(after.get("permissions").is_none());
        assert_eq!(after["theme"], "dark");
    }

    #[test]
    fn a_json_setting_changed_since_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.json");
        let set_these = entries(&[(
            "statusLine",
            "{\"type\": \"command\", \"command\": \"echo hi\"}",
        )]);
        set(&file, ConfigFormat::Json, &set_these).unwrap();
        let edited = std::fs::read_to_string(&file)
            .unwrap()
            .replace("echo hi", "echo mine");
        std::fs::write(&file, &edited).unwrap();
        assert!(
            remove(
                &file,
                ConfigFormat::Json,
                &recorded(ConfigFormat::Json, &set_these)
            )
            .is_err()
        );
        assert_eq!(std::fs::read_to_string(&file).unwrap(), edited);
    }

    #[test]
    fn toml_settings_go_at_the_top_or_in_their_table_and_come_out_again() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        let before = "# mine\nmodel = \"gpt-6\"\n\n[mcp_servers.docs]\ncommand = \"docs\"\n";
        std::fs::write(&file, before).unwrap();
        let set_these = entries(&[
            ("approval_policy", "\"on-request\""),
            ("sandbox_mode", "\"workspace-write\""),
            ("tui.status_line", "[\"model\", \"context\"]"),
        ]);
        set(&file, ConfigFormat::Toml, &set_these).unwrap();
        let now = std::fs::read_to_string(&file).unwrap();
        let parsed: toml::Table = now.parse().unwrap();
        assert_eq!(
            parsed["approval_policy"].as_str(),
            Some("on-request"),
            "{now}"
        );
        assert_eq!(
            parsed["mcp_servers"]["docs"]["command"].as_str(),
            Some("docs"),
            "the user's tables stay"
        );
        assert_eq!(parsed["tui"]["status_line"].as_array().unwrap().len(), 2);
        assert!(now.starts_with("# mine\nmodel = \"gpt-6\""), "{now}");
        assert_eq!(
            read(&file, ConfigFormat::Toml, "sandbox_mode").as_deref(),
            Some("\"workspace-write\"")
        );
        assert_eq!(
            remove(
                &file,
                ConfigFormat::Toml,
                &recorded(ConfigFormat::Toml, &set_these)
            )
            .unwrap(),
            Removed::Done
        );
        let after: toml::Table = std::fs::read_to_string(&file).unwrap().parse().unwrap();
        assert!(after.get("approval_policy").is_none() && after.get("sandbox_mode").is_none());
        assert!(
            after
                .get("tui")
                .and_then(|tui| tui.get("status_line"))
                .is_none(),
            "the setting goes; a table stays, in case it's the user's"
        );
        assert_eq!(after["model"].as_str(), Some("gpt-6"));
    }

    #[test]
    fn an_undo_never_makes_a_table_the_user_took_out() {
        // Review of AJ-E1508 (Low): the undo made an empty [tui] again.
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        let set_these = entries(&[("tui.status_line", "[\"model\"]")]);
        set(&file, ConfigFormat::Toml, &set_these).unwrap();
        std::fs::write(&file, "model = \"gpt-6\"\n").unwrap();
        assert_eq!(
            remove(
                &file,
                ConfigFormat::Toml,
                &recorded(ConfigFormat::Toml, &set_these)
            )
            .unwrap(),
            Removed::Gone
        );
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "model = \"gpt-6\"\n"
        );
    }

    #[test]
    fn a_toml_setting_changed_since_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        let set_these = entries(&[("approval_policy", "\"on-request\"")]);
        set(&file, ConfigFormat::Toml, &set_these).unwrap();
        std::fs::write(&file, "approval_policy = \"never\"\n").unwrap();
        assert!(
            remove(
                &file,
                ConfigFormat::Toml,
                &recorded(ConfigFormat::Toml, &set_these)
            )
            .is_err()
        );
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "approval_policy = \"never\"\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_new_file_is_never_written_through_a_link() {
        let dir = tempfile::tempdir().unwrap();
        let elsewhere = dir.path().join("elsewhere.json");
        let file = dir.path().join("settings.json");
        std::os::unix::fs::symlink(&elsewhere, &file).unwrap();
        // A dangling link: the "file" reads as missing, and is never created
        // through.
        assert!(
            set(
                &file,
                ConfigFormat::Json,
                &entries(&[("permissions", "{}")])
            )
            .is_err()
        );
        assert!(!elsewhere.exists());
    }

    #[test]
    fn every_string_in_a_value_is_found_for_the_secret_check() {
        let json = strings_of(
            ConfigFormat::Json,
            r#"{"allow": ["Bash(curl -H 'Authorization: Bearer sk-x')"], "env": {"K": "v"}}"#,
        );
        assert!(json.iter().any(|s| s.contains("Bearer sk-x")) && json.contains(&"K".to_owned()));
        let toml = strings_of(ConfigFormat::Toml, r#"["model", { token = "ghp_x" }]"#);
        assert!(toml.contains(&"ghp_x".to_owned()) && toml.contains(&"token".to_owned()));
    }
}
