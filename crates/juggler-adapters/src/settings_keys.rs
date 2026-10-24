//! A settings file with a key written in it (a provider's `apiKey`, a
//! server's token) never goes into another account's copy (AJ-E1602,
//! AJ-E1603): it stays with the original, named, and the account gets
//! its key through its own sign-in. A reference to an environment
//! variable or a file (`{env:NAME}`, `{file:path}`, `${NAME}`) isn't a
//! key. A file that can't be read as its format can't be checked, so it
//! stays behind too.

use std::path::Path;

use juggler_adapter::inventory::{ConfigFormat, looks_secret};

/// Whether a string value is a reference to where a key is kept, not a
/// key.
fn reference(text: &str) -> bool {
    let text = text.trim();
    text.starts_with("{env:") || text.starts_with("{file:") || text.starts_with("${")
}

/// Whether one value under `key` reads as a key. A setting that names
/// the variable a key is in (`api_key_env = "MOONSHOT_API_KEY"`) holds a
/// name, not a key.
fn secret(key: &str, text: &str) -> bool {
    let names_a_variable = key.to_ascii_lowercase().ends_with("env")
        && !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
    !text.is_empty() && !reference(text) && !names_a_variable && looks_secret(key, text)
}

fn json_holds(key: &str, value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(text) => secret(key, text),
        serde_json::Value::Array(items) => items.iter().any(|item| json_holds(key, item)),
        serde_json::Value::Object(map) => map.iter().any(|(key, item)| json_holds(key, item)),
        _ => false,
    }
}

fn toml_holds(key: &str, value: &toml::Value) -> bool {
    match value {
        toml::Value::String(text) => secret(key, text),
        toml::Value::Array(items) => items.iter().any(|item| toml_holds(key, item)),
        toml::Value::Table(table) => table.iter().any(|(key, item)| toml_holds(key, item)),
        _ => false,
    }
}

/// Whether settings `text` has a key written in it (or can't be read as
/// `format`, so can't be checked).
#[must_use]
pub fn holds_key(text: &str, format: ConfigFormat) -> bool {
    match format {
        ConfigFormat::Json => {
            juggler_adapter::jsonc::parse(text).map_or(true, |value| json_holds("", &value))
        }
        ConfigFormat::Toml => text.parse::<toml::Table>().map_or(true, |table| {
            table.iter().any(|(key, item)| toml_holds(key, item))
        }),
    }
}

/// The format a settings file's name says, for the ones that are.
#[must_use]
pub fn format_of(name: &str) -> Option<ConfigFormat> {
    let extension = Path::new(name)
        .extension()?
        .to_string_lossy()
        .to_ascii_lowercase();
    match extension.as_str() {
        "json" | "jsonc" => Some(ConfigFormat::Json),
        "toml" => Some(ConfigFormat::Toml),
        _ => None,
    }
}

/// Whether the settings file at `path` has a key written in it. A file
/// that isn't there holds none; one that can't be read can't be checked.
#[must_use]
pub fn file_holds_key(path: &Path, format: ConfigFormat) -> bool {
    match std::fs::read_to_string(path) {
        Ok(text) => holds_key(&text, format),
        Err(error) => error.kind() != std::io::ErrorKind::NotFound,
    }
}

/// The line a copy's skips list says for a settings file left behind.
#[must_use]
pub fn kept_back_note(name: &str) -> String {
    format!(
        "{name}: a key is written in it, and keys are never copied: sign in once inside this account"
    )
}

/// Takes each of `files` (relative to the copy `dest`) that has a key
/// written in it out of the copy again, and says so in `skipped`.
///
/// # Errors
///
/// The removal's error (the caller removes the partial copy).
pub fn keep_back_keyed(
    dest: &Path,
    files: &[&str],
    skipped: &mut Vec<String>,
) -> std::io::Result<()> {
    for name in files {
        let path = dest.join(name);
        let Some(format) = format_of(name) else {
            continue;
        };
        if path.is_file() && file_holds_key(&path, format) {
            std::fs::remove_file(&path)?;
            skipped.push(kept_back_note(name));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_is_found_by_its_name_or_shape_and_a_reference_is_not_one() {
        let json = ConfigFormat::Json;
        assert!(holds_key(
            r#"{"provider":{"x":{"options":{"apiKey":"abc"}}}}"#,
            json
        ));
        assert!(!holds_key(
            r#"{"provider":{"x":{"options":{"apiKey":"{env:X_KEY}"}}}}"#,
            json
        ));
        assert!(!holds_key(
            r#"{"theme":"system","mcp":{"a":{"command":["npx","a"]}}}"#,
            json
        ));
        assert!(holds_key("// unreadable\n{", json));
        let toml = ConfigFormat::Toml;
        assert!(holds_key(
            "[providers.platform]\napi_key = \"made-up\"\n",
            toml
        ));
        assert!(!holds_key(
            "[providers.platform]\napi_key_env = \"MOONSHOT_API_KEY\"\n",
            toml
        ));
        assert!(!holds_key("model = \"k3-256k\"\n", toml));
    }

    #[test]
    fn a_keyed_file_is_taken_out_of_the_copy_and_named() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.toml"),
            "[p]\napi_key = \"made-up\"\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("mcp.json"), r#"{"mcpServers":{}}"#).unwrap();
        let mut skipped = Vec::new();
        keep_back_keyed(
            dir.path(),
            &["config.toml", "mcp.json", "none.toml"],
            &mut skipped,
        )
        .unwrap();
        assert!(!dir.path().join("config.toml").exists());
        assert!(dir.path().join("mcp.json").exists());
        assert_eq!(skipped, vec![kept_back_note("config.toml")]);
    }
}
