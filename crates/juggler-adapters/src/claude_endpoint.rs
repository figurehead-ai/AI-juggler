//! AJ-E1601 (D-34): a Claude Code profile that runs through another
//! service. Its own folder — one AI Juggler made, never `~/.claude` —
//! gets the service's published address and the model in its
//! `settings.json` `env`, and its `.claude.json` says the first-run
//! screens are done, as the services' guides set them. The key is never
//! written: AI Juggler hands it to each session at launch, in the variable
//! the guide names.

use std::path::Path;

use juggler_adapter::AdapterError;
use juggler_adapter::endpoint::{ServiceEndpoint, WireApi};
use juggler_adapter::inventory::ConfigFormat;

use crate::config_edit;

/// Where Claude Code takes its provider's address from.
const ADDRESS: &str = "ANTHROPIC_BASE_URL";

/// Every variable Claude Code takes a model from. All are set: a tier left
/// out falls back to a model the service doesn't serve (Kimi's guide:
/// "Configuring only some of these variables will silently break the
/// corresponding scenarios").
const MODEL_VARIABLES: [&str; 6] = [
    "ANTHROPIC_MODEL",
    "ANTHROPIC_DEFAULT_OPUS_MODEL",
    "ANTHROPIC_DEFAULT_SONNET_MODEL",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    "ANTHROPIC_DEFAULT_FABLE_MODEL",
    "CLAUDE_CODE_SUBAGENT_MODEL",
];

/// Points the profile in `state_dir` at `endpoint` with `model`.
///
/// # Errors
///
/// [`AdapterError::Unsupported`] for an address that speaks another
/// protocol or a model id that isn't one; [`AdapterError::AccountDir`] for
/// a folder that isn't AI Juggler's own, or a write that failed (a setting
/// that's there already is never replaced).
pub(crate) fn configure(
    state_dir: &Path,
    endpoint: &ServiceEndpoint,
    model: &str,
) -> Result<(), AdapterError> {
    if endpoint.wire != WireApi::Anthropic {
        return Err(AdapterError::Unsupported(format!(
            "Claude Code speaks the Anthropic protocol, not {}",
            endpoint.wire.as_str()
        )));
    }
    check_model(model)?;
    own_folder(state_dir)?;
    let text = |value: &str| serde_json::Value::from(value).to_string();
    let mut env = vec![(
        config_edit::json_pointer(&["env", ADDRESS]),
        text(&endpoint.base_url),
    )];
    env.extend(
        MODEL_VARIABLES
            .iter()
            .map(|name| (config_edit::json_pointer(&["env", name]), text(model))),
    );
    write(&state_dir.join("settings.json"), &env, state_dir)?;
    write(
        &crate::claude_code::claude_state_file(state_dir),
        &[("hasCompletedOnboarding".to_owned(), "true".to_owned())],
        state_dir,
    )
}

/// Refuses a model id that isn't one short word.
pub(crate) fn check_model(model: &str) -> Result<(), AdapterError> {
    if juggler_adapter::endpoint::plausible_model(model) {
        Ok(())
    } else {
        Err(AdapterError::Unsupported(
            "a model is one word, as the service names it".into(),
        ))
    }
}

/// The folder is one AI Juggler made (it carries the marker), and no link.
pub(crate) fn own_folder(state_dir: &Path) -> Result<(), AdapterError> {
    let link = std::fs::symlink_metadata(state_dir).is_ok_and(|meta| meta.file_type().is_symlink());
    if link || !crate::claude_code::is_managed_dir(state_dir) {
        return Err(AdapterError::AccountDir {
            path: state_dir.to_path_buf(),
            message: "only a profile folder AI Juggler made is set up for another service".into(),
        });
    }
    Ok(())
}

/// Sets `entries` in the JSON file `file`, inside `state_dir`.
fn write(file: &Path, entries: &[(String, String)], state_dir: &Path) -> Result<(), AdapterError> {
    if !file.starts_with(state_dir) {
        return Err(AdapterError::AccountDir {
            path: file.to_path_buf(),
            message: "outside the profile's own folder".into(),
        });
    }
    config_edit::set(file, ConfigFormat::Json, entries).map_err(|message| {
        AdapterError::AccountDir {
            path: file.to_path_buf(),
            message,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(wire: WireApi) -> ServiceEndpoint {
        crate::services::presets()
            .iter()
            .find_map(|preset| preset.endpoint(wire))
            .cloned()
            .unwrap()
    }

    fn managed() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        crate::claude_code::prepare_managed_dir(dir.path()).unwrap();
        dir
    }

    #[test]
    fn the_address_and_every_model_variable_land_in_its_own_settings() {
        let dir = managed();
        let anthropic = endpoint(WireApi::Anthropic);
        configure(dir.path(), &anthropic, "model-a").unwrap();
        let settings: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("settings.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(settings["env"][ADDRESS], anthropic.base_url.as_str());
        for name in MODEL_VARIABLES {
            assert_eq!(settings["env"][name], "model-a", "{name}");
        }
        // Nothing but the address and the models: never the key's variable.
        assert_eq!(
            settings["env"].as_object().unwrap().len(),
            MODEL_VARIABLES.len() + 1
        );
        assert!(settings["env"].get(&anthropic.key_env).is_none());
        let state: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join(".claude.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(state["hasCompletedOnboarding"], true);
    }

    #[test]
    fn a_folder_ai_juggler_did_not_make_is_never_written() {
        let dir = tempfile::tempdir().unwrap();
        let error = configure(dir.path(), &endpoint(WireApi::Anthropic), "m").unwrap_err();
        assert!(matches!(error, AdapterError::AccountDir { .. }));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn another_protocol_or_a_bad_model_is_refused_and_nothing_written() {
        let dir = managed();
        assert!(configure(dir.path(), &endpoint(WireApi::Responses), "m").is_err());
        assert!(configure(dir.path(), &endpoint(WireApi::Anthropic), "two words").is_err());
        assert!(configure(dir.path(), &endpoint(WireApi::Anthropic), "").is_err());
        assert!(!dir.path().join("settings.json").exists());
    }
}
