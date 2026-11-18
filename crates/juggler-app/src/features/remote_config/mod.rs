//! ADR-0035: the remote configuration's fetch over the app's own HTTPS
//! client (juggler-account's ureq + rustls: the platform trust store, no
//! cookies, no redirects), with the 10 s timeout of §7.
//!
//! The request is exactly what [`juggler_remote_config::ConfigRequest`]
//! holds (a URL, and `If-None-Match` only when the policy allows it) plus
//! the account client's generic headers (`User-Agent: AIJuggler/<version>`,
//! `Accept`, `Content-Type`): never a bearer, a cookie or an id. A URL
//! off the allow-listed host (D-129) is refused before anything is sent.
//!
//! [`spawn`] runs the daily fetch and applies what's in effect: the
//! adapters' compat gate, and the [`Snapshot`] every other section's
//! reader (the flags, the notices) and the window read from memory
//! ([`current`]); a change is told to the window
//! (`remote-config://changed`). [`commands`] are `useRemote(section)`'s
//! values and "About this configuration".

use std::sync::Arc;

use juggler_remote_config::{ConfigRequest, ConfigResponse, NetworkError, Transport};

pub mod commands;
pub mod dto;
mod service;
mod snapshot;

pub use service::{CHECK_KEY, STATE_FILE, registry, spawn};
#[cfg(test)]
pub(crate) use snapshot::fixtures as snapshot_fixtures;
pub use snapshot::{FileFacts, Snapshot, current, install};

/// The remote-configuration transport over the account client's.
pub struct RemoteConfigTransport(pub Arc<dyn juggler_account::Transport>);

impl std::fmt::Debug for RemoteConfigTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteConfigTransport")
            .finish_non_exhaustive()
    }
}

impl RemoteConfigTransport {
    /// The production transport.
    #[must_use]
    pub fn production() -> Self {
        Self(Arc::new(juggler_account::UreqTransport::with_timeout(
            env!("CARGO_PKG_VERSION"),
            juggler_remote_config::fetch::FETCH_TIMEOUT,
        )))
    }
}

impl Transport for RemoteConfigTransport {
    fn get(&self, request: &ConfigRequest) -> Result<ConfigResponse, NetworkError> {
        if !juggler_remote_config::fetch::allowed_url(&request.url) {
            return Err(NetworkError("not the configuration host".to_owned()));
        }
        let headers = request
            .if_none_match
            .iter()
            .map(|tag| ("if-none-match", tag.clone()))
            .collect();
        let response = self
            .0
            .send(&juggler_account::HttpRequest {
                method: "GET",
                url: request.url.clone(),
                bearer: None,
                content_type: "application/json",
                headers,
                body: Vec::new(),
                long_transfer: false,
                // The cap is enforced while reading, not after (§7).
                max_body: Some(juggler_remote_config::MAX_FILE_BYTES as u64 + 1),
            })
            .map_err(|e| NetworkError(e.0))?;
        Ok(ConfigResponse {
            status: response.status,
            etag: response.header("etag").map(str::to_owned),
            body: response.body,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_request_is_a_bare_get() {
        let account = Arc::new(juggler_account::RecordingTransport::default());
        account.respond(200, "{}");
        let transport = RemoteConfigTransport(account.clone());
        let response = transport
            .get(&ConfigRequest {
                url: "https://config.figurehead.ai/v1/stable.json".to_owned(),
                if_none_match: None,
            })
            .unwrap();
        assert_eq!(
            (response.status, response.body.as_slice()),
            (200, b"{}".as_slice())
        );
        let sent = account.requests();
        assert_eq!(sent.len(), 1);
        let request = &sent[0];
        assert_eq!(request.method, "GET");
        assert!(request.bearer.is_none(), "never authenticated");
        assert!(request.headers.is_empty(), "no extra header");
        assert!(request.body.is_empty());
        assert_eq!(
            request.max_body,
            Some(juggler_remote_config::MAX_FILE_BYTES as u64 + 1),
            "the size cap is enforced while reading"
        );
        assert!(!request.url.contains('?'));
    }

    #[test]
    fn a_url_off_the_configuration_host_is_never_sent() {
        let account = Arc::new(juggler_account::RecordingTransport::default());
        let transport = RemoteConfigTransport(account.clone());
        for url in [
            "https://config.figurehead.ai.example.test/v1/stable.json",
            "http://config.figurehead.ai/v1/stable.json",
            "https://example.test/v1/stable.json",
        ] {
            let request = ConfigRequest {
                url: url.to_owned(),
                if_none_match: None,
            };
            assert!(transport.get(&request).is_err(), "{url}");
        }
        assert!(account.requests().is_empty());
    }
}
