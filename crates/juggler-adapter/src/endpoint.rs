//! AJ-E1601 (D-34): accounts that run a tool through another service — a
//! subscription that publishes a Claude- or Codex-compatible address
//! (OpenCode Go, Kimi). A service is data ([`ServicePreset`], ADR-0019):
//! its published addresses, the variable each tool reads its key from,
//! its models, plans and how honest its usage can be. How a tool is
//! pointed at one of those addresses is the tool's adapter's business
//! ([`crate::HarnessAdapter::configure_endpoint`]).
//!
//! A preset is only ever read from the adapters' data file; nothing here
//! builds an address from what a user typed.

use serde::Deserialize;

/// The protocol a published address speaks, and so which tools can use
/// it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WireApi {
    /// Anthropic's Messages API (what Claude Code speaks).
    Anthropic,
    /// `OpenAI`'s Responses API (what Codex speaks: `wire_api =
    /// "responses"`).
    Responses,
}

impl WireApi {
    /// The name the tool's own config uses for it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::Responses => "responses",
        }
    }
}

/// How much AI Juggler can say about a service's limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageHonesty {
    /// Worked out from what its sessions used, against the service's
    /// published rule: never measured.
    Estimate,
    /// No source AI Juggler may read.
    Unknown,
}

impl UsageHonesty {
    /// The wire word (`estimate` / `unknown`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Estimate => "estimate",
            Self::Unknown => "unknown",
        }
    }
}

/// One address a service publishes for one protocol.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceEndpoint {
    /// The protocol it speaks.
    pub wire: WireApi,
    /// The address, exactly as the service's own guide publishes it.
    pub base_url: String,
    /// The environment variable the tool reads the key from, as the
    /// service's guide names it.
    pub key_env: String,
    /// The provider's name in the tool's config, where the tool names
    /// providers (Codex's `model_providers` table).
    pub provider_id: String,
    /// Model ids the guide names for this protocol, the usual one first.
    /// Empty when the guide names none: the user types one.
    #[serde(default)]
    pub models: Vec<String>,
    /// Where the guide is (the source of every value above).
    pub guide: String,
}

/// One service, as its published guides describe it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServicePreset {
    /// Stable id (`opencode-go`).
    pub id: String,
    /// Its name on every screen (`OpenCode Go`).
    pub name: String,
    /// Its short name, for a profile's name and a place line (`Go`).
    pub short_name: String,
    /// Who runs it.
    pub vendor: String,
    /// Its plans, by the names the service sells them under.
    pub plans: Vec<String>,
    /// How honest its usage can be here.
    pub usage: UsageHonesty,
    /// One line on how its limits work and what AI Juggler can say.
    pub usage_words: String,
    /// Where the user sees their usage and gets a key, on the service's
    /// own site.
    pub console_url: String,
    /// What the key is called on the service's site.
    pub key_name: String,
    /// Why its sessions may only be started by the user, when its terms
    /// say so (AI Juggler never starts one again by itself).
    #[serde(default)]
    pub interactive_only: Option<String>,
    /// The addresses it publishes, one per protocol.
    pub endpoints: Vec<ServiceEndpoint>,
    /// The day the values were checked against the guides
    /// (`YYYY-MM-DD`).
    pub checked: String,
}

impl ServicePreset {
    /// Its address for `wire`, if it publishes one.
    #[must_use]
    pub fn endpoint(&self, wire: WireApi) -> Option<&ServiceEndpoint> {
        self.endpoints.iter().find(|endpoint| endpoint.wire == wire)
    }
}

impl ServiceEndpoint {
    /// The host of its address (`opencode.ai`), when it is an `https`
    /// address; `None` for anything else.
    #[must_use]
    pub fn https_host(&self) -> Option<&str> {
        let rest = self.base_url.strip_prefix("https://")?;
        let host = rest.split(['/', '?', '#']).next()?;
        (!host.is_empty() && !host.contains(['@', ':', ' '])).then_some(host)
    }
}

/// Whether `key` could be a key: one line of printable characters, of a
/// sensible length. Nothing about it is ever said back (it is never shown
/// again).
#[must_use]
pub fn plausible_key(key: &str) -> bool {
    let len = key.chars().count();
    (8..=4096).contains(&len) && key.chars().all(|c| c.is_ascii_graphic())
}

/// Whether `model` could be a model id: one short word of printable
/// characters (`k3[1m]`, `minimax-m3`).
#[must_use]
pub fn plausible_model(model: &str) -> bool {
    let len = model.chars().count();
    (1..=200).contains(&len) && model.chars().all(|c| c.is_ascii_graphic())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(url: &str) -> ServiceEndpoint {
        ServiceEndpoint {
            wire: WireApi::Anthropic,
            base_url: url.to_owned(),
            key_env: "KEY".to_owned(),
            provider_id: "p".to_owned(),
            models: Vec::new(),
            guide: String::new(),
        }
    }

    #[test]
    fn only_an_https_address_has_a_host() {
        assert_eq!(
            endpoint("https://api.example.com/coding/").https_host(),
            Some("api.example.com")
        );
        assert_eq!(endpoint("http://api.example.com/").https_host(), None);
        assert_eq!(endpoint("https://user@evil.example/").https_host(), None);
        assert_eq!(endpoint("https://host:8443/").https_host(), None);
        assert_eq!(endpoint("https:///path").https_host(), None);
    }

    #[test]
    fn a_key_is_one_line_of_printable_characters() {
        assert!(plausible_key("oc_sk_0123456789abcdef"));
        assert!(!plausible_key("short"));
        assert!(!plausible_key("has a space in it"));
        assert!(!plausible_key("line\nbreak-0123456"));
        assert!(!plausible_key(&"x".repeat(5000)));
    }

    #[test]
    fn a_model_is_one_short_word() {
        assert!(plausible_model("k3[1m]"));
        assert!(!plausible_model(""));
        assert!(!plausible_model("two words"));
        assert!(!plausible_model("a\"quote\nline"));
    }
}
