//! AJ-E1601 (D-34): a Codex profile that runs through another service.
//! Its own `CODEX_HOME` — a folder AI Juggler made, never `~/.codex` —
//! gets a provider block in `config.toml` (the address, the variable the
//! key is read from, the protocol) and the model, as the services' guides
//! set them. The key is never written: AI Juggler hands it to each session
//! at launch, in that variable.

use std::path::Path;

use juggler_adapter::AdapterError;
use juggler_adapter::endpoint::{ServiceEndpoint, WireApi};
use juggler_adapter::inventory::ConfigFormat;

use crate::config_edit;

/// Codex's table of providers it can talk to.
const PROVIDERS: &str = "model_providers";

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
    if endpoint.wire != WireApi::Responses {
        return Err(AdapterError::Unsupported(format!(
            "Codex speaks the Responses protocol, not {}",
            endpoint.wire.as_str()
        )));
    }
    crate::claude_endpoint::check_model(model)?;
    crate::claude_endpoint::own_folder(state_dir)?;
    let text = |value: &str| toml_edit::Value::from(value).to_string();
    let block = |key: &str| format!("{PROVIDERS}.{}.{key}", endpoint.provider_id);
    let entries = vec![
        ("model".to_owned(), text(model)),
        ("model_provider".to_owned(), text(&endpoint.provider_id)),
        (block("name"), text(&endpoint.provider_id)),
        (block("base_url"), text(&endpoint.base_url)),
        (block("env_key"), text(&endpoint.key_env)),
        (block("wire_api"), text(endpoint.wire.as_str())),
    ];
    let file = state_dir.join("config.toml");
    config_edit::set(&file, ConfigFormat::Toml, &entries).map_err(|message| {
        AdapterError::AccountDir {
            path: file,
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
        crate::claude_code::prepare_managed_dir_for(dir.path(), crate::codex::CODEX_ID).unwrap();
        dir
    }

    #[test]
    fn the_provider_block_and_the_model_land_in_its_own_config() {
        let dir = managed();
        let responses = endpoint(WireApi::Responses);
        configure(dir.path(), &responses, "model-b").unwrap();
        let config: toml::Table = std::fs::read_to_string(dir.path().join("config.toml"))
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(config["model"].as_str(), Some("model-b"));
        assert_eq!(
            config["model_provider"].as_str(),
            Some(responses.provider_id.as_str())
        );
        let block = config[PROVIDERS][&responses.provider_id]
            .as_table()
            .unwrap();
        assert_eq!(
            block["base_url"].as_str(),
            Some(responses.base_url.as_str())
        );
        assert_eq!(block["env_key"].as_str(), Some(responses.key_env.as_str()));
        assert_eq!(block["wire_api"].as_str(), Some("responses"));
        // The key's variable is named, never a key.
        assert_eq!(block.len(), 4);
    }

    #[test]
    fn a_folder_ai_juggler_did_not_make_is_never_written() {
        let dir = tempfile::tempdir().unwrap();
        assert!(configure(dir.path(), &endpoint(WireApi::Responses), "m").is_err());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn another_protocol_is_refused_and_nothing_written() {
        let dir = managed();
        assert!(configure(dir.path(), &endpoint(WireApi::Anthropic), "m").is_err());
        assert!(!dir.path().join("config.toml").exists());
    }
}
