//! AJ-E1508: settings set in a tool's config file, and taken out again,
//! with every other byte of the file kept: comments, order and spacing.
//! A key that's already there is never replaced, and an undo takes a key
//! out only while it still holds the value that was set; anything else is
//! the user's.

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

/// Sets each of `entries` (key, value text) in `file`, creating the file
/// when there's none. Refused, with nothing written, when any key is there
/// already.
///
/// # Errors
///
/// The key that's there already, a file that isn't valid `format`, or the
/// write's error, in words.
pub fn set(file: &Path, format: ConfigFormat, entries: &[(String, String)]) -> Result<(), String> {
    let existed = file.exists();
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
        rewrite_in_place(file, &new).map_err(|error| error.to_string())
    } else {
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        std::fs::write(file, new).map_err(|error| error.to_string())
    }
}

/// Takes each of `entries` out of `file` again, while every one that's
/// still there holds the value that was set.
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
            for (key, value) in entries {
                match jsonc::value_text(&out, key)? {
                    None => {}
                    Some(now) if now == *value => {
                        out = jsonc::unset(&out, key, value, None)?.unwrap_or(out);
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
            for (key, value) in entries {
                let want = normal(&parse_value(value)?);
                let (table, last) = table_for(&mut doc, key)?;
                match table.get(last).and_then(toml_edit::Item::as_value) {
                    None if !table.contains_key(last) => {}
                    Some(now) if normal(now) == want => {
                        table.remove(last);
                    }
                    _ => return Err(format!("its {key} has changed since")),
                }
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
        assert_eq!(
            remove(&file, ConfigFormat::Json, &set_these).unwrap(),
            Removed::Done
        );
        assert_eq!(std::fs::read_to_string(&file).unwrap(), before);
        assert_eq!(
            remove(&file, ConfigFormat::Json, &set_these).unwrap(),
            Removed::Gone
        );
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
        assert!(remove(&file, ConfigFormat::Json, &set_these).is_err());
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
            remove(&file, ConfigFormat::Toml, &set_these).unwrap(),
            Removed::Done
        );
        let after: toml::Table = std::fs::read_to_string(&file).unwrap().parse().unwrap();
        assert!(after.get("approval_policy").is_none() && after.get("sandbox_mode").is_none());
        assert_eq!(after["model"].as_str(), Some("gpt-6"));
    }

    #[test]
    fn a_toml_setting_changed_since_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        let set_these = entries(&[("approval_policy", "\"on-request\"")]);
        set(&file, ConfigFormat::Toml, &set_these).unwrap();
        std::fs::write(&file, "approval_policy = \"never\"\n").unwrap();
        assert!(remove(&file, ConfigFormat::Toml, &set_these).is_err());
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "approval_policy = \"never\"\n"
        );
    }
}
