//! One seam for every request the account client makes.

use std::sync::Mutex;
use std::time::Duration;

/// The per-request timeout.
pub const TIMEOUT: Duration = Duration::from_secs(15);

/// One HTTP request, fully described (no hidden headers).
#[derive(Clone, PartialEq, Eq)]
pub struct HttpRequest {
    /// `GET` | `POST` | `PATCH` | `DELETE`.
    pub method: &'static str,
    /// The absolute URL.
    pub url: String,
    /// The bearer secret, when the call is authenticated.
    pub bearer: Option<String>,
    /// The content type (always sent — the service rejects requests
    /// without one, even bodiless `DELETE`s).
    pub content_type: &'static str,
    /// Extra headers (e.g. a backup label).
    pub headers: Vec<(&'static str, String)>,
    /// The body.
    pub body: Vec<u8>,
    /// A large transfer (backup upload/download): allowed
    /// [`LONG_TIMEOUT`] instead of [`TIMEOUT`].
    pub long_transfer: bool,
}

/// The timeout for a backup upload or download (up to 25 MB).
pub const LONG_TIMEOUT: Duration = Duration::from_secs(180);

impl std::fmt::Debug for HttpRequest {
    // Never print the bearer or the body: either may be a secret.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpRequest")
            .field("method", &self.method)
            .field("url", &self.url)
            .field("authenticated", &self.bearer.is_some())
            .field("content_type", &self.content_type)
            .field("body_bytes", &self.body.len())
            .finish_non_exhaustive()
    }
}

/// One HTTP response.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct HttpResponse {
    /// The status code.
    pub status: u16,
    /// Response headers, lower-cased names.
    pub headers: Vec<(String, String)>,
    /// The body.
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// A header value by (lower-case) name.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// A network failure (no HTTP status was received).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct NetworkError(pub String);

/// Sends requests. The production transport is [`UreqTransport`]; tests
/// use [`RecordingTransport`].
pub trait Transport: Send + Sync {
    /// Sends `request`.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError`] when no response arrived.
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse, NetworkError>;
}

/// The production transport: rustls (ring) with the platform's trust
/// store, 15 s timeout, no redirects, no cookies.
pub struct UreqTransport {
    agent: ureq::Agent,
    user_agent: String,
}

impl std::fmt::Debug for UreqTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UreqTransport").finish_non_exhaustive()
    }
}

impl UreqTransport {
    /// A transport identifying as `AIJuggler/<app_version>`.
    #[must_use]
    pub fn new(app_version: &str) -> Self {
        // The operating system's trust store (so a managed Mac's own
        // roots work); no bundled CA list.
        let roots: Vec<ureq::tls::Certificate<'static>> = rustls_native_certs::load_native_certs()
            .certs
            .into_iter()
            .map(|der| ureq::tls::Certificate::from_der(der.as_ref()).to_owned())
            .collect();
        let tls = ureq::tls::TlsConfig::builder()
            .provider(ureq::tls::TlsProvider::Rustls)
            .root_certs(ureq::tls::RootCerts::Specific(std::sync::Arc::new(roots)))
            .unversioned_rustls_crypto_provider(std::sync::Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .build();
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .max_redirects(0)
            .http_status_as_error(false)
            .tls_config(tls)
            .build()
            .new_agent();
        Self {
            agent,
            user_agent: format!("AIJuggler/{app_version}"),
        }
    }
}

impl Transport for UreqTransport {
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse, NetworkError> {
        let mut builder = ureq::http::Request::builder()
            .method(request.method)
            .uri(&request.url)
            .header("user-agent", &self.user_agent)
            .header("content-type", request.content_type)
            .header("accept", "application/json");
        if let Some(bearer) = &request.bearer {
            builder = builder.header("authorization", format!("Bearer {bearer}"));
        }
        for (name, value) in &request.headers {
            builder = builder.header(*name, value);
        }
        let http = builder
            .body(request.body.clone())
            .map_err(|e| NetworkError(e.to_string()))?;
        let mut response = if request.long_transfer {
            let configured = self
                .agent
                .configure_request(http)
                .timeout_global(Some(LONG_TIMEOUT))
                .build();
            self.agent.run(configured)
        } else {
            self.agent.run(http)
        }
        .map_err(|e| NetworkError(e.to_string()))?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_ascii_lowercase(),
                    value.to_str().unwrap_or_default().to_owned(),
                )
            })
            .collect();
        let body = response
            .body_mut()
            .with_config()
            .limit(32 * 1024 * 1024)
            .read_to_vec()
            .map_err(|e| NetworkError(e.to_string()))?;
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

/// A scripted transport that records every request (guardrail and unit
/// tests: it proves what was sent — and that nothing was).
#[derive(Default)]
pub struct RecordingTransport {
    requests: Mutex<Vec<HttpRequest>>,
    responses: Mutex<std::collections::VecDeque<Result<HttpResponse, NetworkError>>>,
}

impl std::fmt::Debug for RecordingTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecordingTransport")
            .field("requests", &self.count())
            .finish_non_exhaustive()
    }
}

impl RecordingTransport {
    /// Queues the next response.
    pub fn respond(&self, status: u16, body: &str) {
        self.push(Ok(HttpResponse {
            status,
            headers: Vec::new(),
            body: body.as_bytes().to_vec(),
        }));
    }

    /// Queues a full response or a network failure.
    pub fn push(&self, response: Result<HttpResponse, NetworkError>) {
        if let Ok(mut queue) = self.responses.lock() {
            queue.push_back(response);
        }
    }

    /// Every request sent so far.
    #[must_use]
    pub fn requests(&self) -> Vec<HttpRequest> {
        self.requests.lock().map(|r| r.clone()).unwrap_or_default()
    }

    /// How many requests were sent.
    #[must_use]
    pub fn count(&self) -> usize {
        self.requests.lock().map_or(0, |r| r.len())
    }
}

impl Transport for RecordingTransport {
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse, NetworkError> {
        if let Ok(mut requests) = self.requests.lock() {
            requests.push(request.clone());
        }
        self.responses
            .lock()
            .ok()
            .and_then(|mut queue| queue.pop_front())
            .unwrap_or_else(|| Err(NetworkError("no scripted response".to_owned())))
    }
}
