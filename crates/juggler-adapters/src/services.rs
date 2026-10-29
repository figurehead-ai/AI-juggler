//! AJ-E1601 (D-34): the services a Claude Code or Codex profile can run
//! through, as data (ADR-0019). Every value comes from the service's own
//! published guide, recorded in `services.json` with the guide's address
//! and the day it was checked (`docs/research/opencode.md` §5,
//! `docs/research/kimi.md` §9). A third service is a new entry there, and
//! no code.
//!
//! This file is the only reader of that data: every address a tool is ever
//! pointed at comes from it (the Privacy page's `no-proxy` line, checked by
//! `ai_juggler_never_points_a_tool_at_another_address`).

use std::sync::OnceLock;

use juggler_adapter::endpoint::ServicePreset;

/// The data, as shipped.
const SERVICES_JSON: &str = include_str!("services.json");

/// Every service a profile can run through, in the order Add account
/// offers them.
///
/// # Panics
///
/// Never at runtime: the shipped data is parsed and checked by this
/// module's tests.
#[must_use]
pub fn presets() -> &'static [ServicePreset] {
    static PRESETS: OnceLock<Vec<ServicePreset>> = OnceLock::new();
    PRESETS.get_or_init(|| serde_json::from_str(SERVICES_JSON).unwrap_or_default())
}

/// The service `id`, if there is one.
#[must_use]
pub fn preset(id: &str) -> Option<&'static ServicePreset> {
    presets().iter().find(|preset| preset.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use juggler_adapter::endpoint::WireApi;

    #[test]
    fn the_shipped_data_parses_and_every_address_is_https() {
        let parsed: Vec<ServicePreset> = serde_json::from_str(SERVICES_JSON).unwrap();
        assert!(!parsed.is_empty());
        assert_eq!(presets(), parsed.as_slice());
        for preset in presets() {
            assert!(!preset.plans.is_empty(), "{} names no plan", preset.id);
            assert!(preset.console_url.starts_with("https://"), "{}", preset.id);
            assert!(
                !preset.endpoints.is_empty(),
                "{} publishes nothing",
                preset.id
            );
            for endpoint in &preset.endpoints {
                assert!(
                    endpoint.https_host().is_some(),
                    "{}: {} is not an https address",
                    preset.id,
                    endpoint.base_url
                );
                assert!(endpoint.guide.starts_with("https://"), "{}", preset.id);
                assert!(
                    endpoint
                        .key_env
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'),
                    "{}: {}",
                    preset.id,
                    endpoint.key_env
                );
                assert!(
                    endpoint
                        .provider_id
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                    "{}: {}",
                    preset.id,
                    endpoint.provider_id
                );
            }
        }
    }

    #[test]
    fn ids_are_unique_and_each_protocol_is_published_once() {
        let mut ids: Vec<&str> = presets().iter().map(|p| p.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), presets().len());
        for preset in presets() {
            for wire in [WireApi::Anthropic, WireApi::Responses] {
                assert!(
                    preset.endpoints.iter().filter(|e| e.wire == wire).count() <= 1,
                    "{} publishes two {} addresses",
                    preset.id,
                    wire.as_str()
                );
            }
        }
    }

    #[test]
    fn an_unknown_service_is_none() {
        assert!(preset("no-such-service").is_none());
        assert!(preset(&presets()[0].id).is_some());
    }
}
