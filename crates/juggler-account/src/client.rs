//! The account client: every call the cloud path may make, with the
//! service's status codes mapped to plain outcomes.

use std::sync::Arc;
use std::time::Duration;

use crate::model::{AccountView, DeviceFields, DeviceToken, LinkRequest, PollOutcome};
use crate::transport::{HttpRequest, HttpResponse, Transport};

/// Why a call didn't succeed, in plain words.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ApiError {
    /// `401`: this Mac was unlinked on the website (or the token is
    /// gone). The caller deletes the token and turns cloud off.
    #[error("This Mac was unlinked from Figurehead.")]
    Unlinked,
    /// `402`: the account's plan doesn't include `feature`.
    #[error("Your Figurehead plan doesn't include this.")]
    UpgradeRequired {
        /// The feature key.
        feature: String,
        /// Where Upgrade goes.
        upgrade_url: Option<String>,
    },
    /// `429 busy`: the service is busy; try again later.
    #[error("Figurehead is busy right now — try again in a moment.")]
    Busy {
        /// Seconds to wait, when the service said.
        retry_after_s: Option<u64>,
    },
    /// `400 invalid_or_expired_code`.
    #[error("That pairing code is wrong or has expired — generate a new one on figurehead.ai.")]
    InvalidCode,
    /// Another `4xx` with the service's error code.
    #[error("Figurehead refused the request ({code}).")]
    Rejected {
        /// The status.
        status: u16,
        /// The service's error code (or the status).
        code: String,
    },
    /// `5xx` after the retries.
    #[error("Figurehead had a problem (HTTP {0}) — try again later.")]
    Server(u16),
    /// No response (offline, DNS, TLS).
    #[error("Couldn't reach Figurehead — check your connection.")]
    Network(String),
    /// A response that doesn't match the contract.
    #[error("Figurehead sent an unexpected answer.")]
    Malformed(String),
}

#[derive(serde::Deserialize, Default)]
struct ErrorBody {
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    feature: Option<String>,
    #[serde(default, rename = "upgradeUrl")]
    upgrade_url: Option<String>,
    #[serde(default)]
    interval: Option<u64>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Claimed {
    device_id: String,
    token: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Polled {
    status: String,
    #[serde(default)]
    device_id: Option<String>,
    #[serde(default)]
    token: Option<String>,
}

/// The client (stateless: the caller owns the token and the cache).
pub struct AccountClient {
    base: String,
    transport: Arc<dyn Transport>,
    retry_delays: Vec<Duration>,
}

impl std::fmt::Debug for AccountClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccountClient")
            .field("base", &self.base)
            .finish_non_exhaustive()
    }
}

impl AccountClient {
    /// A client for `base` (see [`crate::base_url`]) over `transport`,
    /// retrying network errors and `5xx` after 0.5 s, 1 s and 2 s.
    #[must_use]
    pub fn new(base: &str, transport: Arc<dyn Transport>) -> Self {
        Self {
            base: base.trim_end_matches('/').to_owned(),
            transport,
            retry_delays: vec![
                Duration::from_millis(500),
                Duration::from_secs(1),
                Duration::from_secs(2),
            ],
        }
    }

    /// Replaces the backoff schedule (tests use zero delays).
    #[must_use]
    pub fn with_retry_delays(mut self, delays: Vec<Duration>) -> Self {
        self.retry_delays = delays;
        self
    }

    fn request(
        &self,
        method: &'static str,
        path: &str,
        bearer: Option<&str>,
        body: Option<serde_json::Value>,
    ) -> HttpRequest {
        HttpRequest {
            method,
            url: format!("{}/api/v1/{path}", self.base),
            bearer: bearer.map(str::to_owned),
            content_type: "application/json",
            headers: Vec::new(),
            body: body
                .map(|value| value.to_string().into_bytes())
                .unwrap_or_default(),
        }
    }

    /// Sends with bounded exponential backoff on network errors and
    /// `5xx` (`retry = false` for the link poll: its token is delivered
    /// once, so a lost answer means a fresh link, never a re-poll).
    fn send(&self, request: &HttpRequest, retry: bool) -> Result<HttpResponse, ApiError> {
        let mut delays = if retry {
            self.retry_delays.iter()
        } else {
            [].iter()
        };
        loop {
            let outcome = self.transport.send(request);
            let again = match &outcome {
                Err(_) => true,
                Ok(response) => response.status >= 500,
            };
            if again && let Some(delay) = delays.next() {
                std::thread::sleep(*delay);
                continue;
            }
            return match outcome {
                Err(error) => Err(ApiError::Network(error.0)),
                Ok(response) if response.status >= 500 => Err(ApiError::Server(response.status)),
                Ok(response) => Ok(response),
            };
        }
    }

    /// Maps the statuses every authenticated call shares.
    fn common(response: &HttpResponse) -> ApiError {
        let body: ErrorBody = serde_json::from_slice(&response.body).unwrap_or_default();
        match response.status {
            401 => ApiError::Unlinked,
            402 => ApiError::UpgradeRequired {
                feature: body.feature.unwrap_or_default(),
                upgrade_url: body.upgrade_url,
            },
            429 => ApiError::Busy {
                retry_after_s: response
                    .header("retry-after")
                    .and_then(|value| value.trim().parse().ok()),
            },
            status => ApiError::Rejected {
                status,
                code: body.error.unwrap_or_else(|| status.to_string()),
            },
        }
    }

    /// Starts a browser link (`POST devices/link-requests`).
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`]; `Rejected` carries a field-limit violation.
    pub fn start_link(&self, device: &DeviceFields) -> Result<LinkRequest, ApiError> {
        device.validate().map_err(|message| ApiError::Rejected {
            status: 400,
            code: message,
        })?;
        let body = serde_json::to_value(device).map_err(|e| ApiError::Malformed(e.to_string()))?;
        let response = self.send(
            &self.request("POST", "devices/link-requests", None, Some(body)),
            true,
        )?;
        if response.status == 201 || response.status == 200 {
            return serde_json::from_slice(&response.body)
                .map_err(|e| ApiError::Malformed(e.to_string()));
        }
        Err(Self::common(&response))
    }

    /// One poll of a browser link (never retried; see [`Self::send`]).
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] for network failures and unexpected answers.
    pub fn poll_link(&self, link: &LinkRequest) -> Result<PollOutcome, ApiError> {
        let path = format!("devices/link-requests/{}", link.request_id);
        let response = self.send(
            &self.request("GET", &path, Some(link.poll_token()), None),
            false,
        )?;
        match response.status {
            200 => {
                let polled: Polled = serde_json::from_slice(&response.body)
                    .map_err(|e| ApiError::Malformed(e.to_string()))?;
                match (polled.status.as_str(), polled.device_id, polled.token) {
                    ("pending", _, _) => Ok(PollOutcome::Pending),
                    ("approved", Some(device_id), Some(token)) if !token.is_empty() => {
                        Ok(PollOutcome::Approved {
                            device_id,
                            token: DeviceToken::new(token),
                        })
                    }
                    (other, _, _) => Err(ApiError::Malformed(format!("poll status {other}"))),
                }
            }
            410 => {
                let body: ErrorBody = serde_json::from_slice(&response.body).unwrap_or_default();
                Ok(if body.status.as_deref() == Some("denied") {
                    PollOutcome::Denied
                } else {
                    PollOutcome::Expired
                })
            }
            429 => {
                let body: ErrorBody = serde_json::from_slice(&response.body).unwrap_or_default();
                Ok(PollOutcome::SlowDown {
                    interval_s: body.interval.unwrap_or(link.interval_s.max(1) * 2),
                })
            }
            404 => Ok(PollOutcome::NotFound),
            _ => Err(Self::common(&response)),
        }
    }

    /// Links with a pairing code from the dashboard (`POST devices/claim`).
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::InvalidCode`] for a wrong or expired code.
    pub fn claim(
        &self,
        code: &str,
        device: &DeviceFields,
    ) -> Result<(String, DeviceToken), ApiError> {
        device.validate().map_err(|message| ApiError::Rejected {
            status: 400,
            code: message,
        })?;
        let normalised: String = code
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .map(|c| c.to_ascii_uppercase())
            .collect();
        if normalised.is_empty() || normalised.len() > 32 {
            return Err(ApiError::InvalidCode);
        }
        let mut body =
            serde_json::to_value(device).map_err(|e| ApiError::Malformed(e.to_string()))?;
        if let Some(map) = body.as_object_mut() {
            map.insert("code".to_owned(), serde_json::Value::String(normalised));
        }
        let response = self.send(
            &self.request("POST", "devices/claim", None, Some(body)),
            true,
        )?;
        match response.status {
            200 | 201 => {
                let claimed: Claimed = serde_json::from_slice(&response.body)
                    .map_err(|e| ApiError::Malformed(e.to_string()))?;
                Ok((claimed.device_id, DeviceToken::new(claimed.token)))
            }
            400 => {
                let body: ErrorBody = serde_json::from_slice(&response.body).unwrap_or_default();
                if body.error.as_deref() == Some("invalid_or_expired_code") {
                    Err(ApiError::InvalidCode)
                } else {
                    Err(Self::common(&response))
                }
            }
            _ => Err(Self::common(&response)),
        }
    }

    /// The account, this Mac, the plan and entitlements (`GET account`);
    /// a `404` (no account endpoint yet) answers the Phase-1 defaults.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Unlinked`] on `401`, or another [`ApiError`].
    pub fn account(&self, token: &DeviceToken) -> Result<AccountView, ApiError> {
        let response = self.send(
            &self.request("GET", "account", Some(token.expose()), None),
            true,
        )?;
        match response.status {
            200 => serde_json::from_slice(&response.body)
                .map_err(|e| ApiError::Malformed(e.to_string())),
            404 => Ok(AccountView::phase1_defaults()),
            _ => Err(Self::common(&response)),
        }
    }

    /// Renames this Mac on the service (`PATCH devices/self`).
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`].
    pub fn rename(
        &self,
        token: &DeviceToken,
        name: &str,
        app_version: &str,
    ) -> Result<(), ApiError> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 64 {
            return Err(ApiError::Rejected {
                status: 400,
                code: "Use a name of 1–64 characters for this Mac.".to_owned(),
            });
        }
        let body = serde_json::json!({ "name": name, "appVersion": app_version });
        let response = self.send(
            &self.request("PATCH", "devices/self", Some(token.expose()), Some(body)),
            true,
        )?;
        if response.status == 200 || response.status == 204 {
            Ok(())
        } else {
            Err(Self::common(&response))
        }
    }

    /// Unlinks this Mac (`DELETE devices/self`); the token stops working.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] — the caller cleans up locally regardless.
    pub fn unlink(&self, token: &DeviceToken) -> Result<(), ApiError> {
        let response = self.send(
            &self.request("DELETE", "devices/self", Some(token.expose()), None),
            true,
        )?;
        match response.status {
            200 | 204 | 401 | 404 => Ok(()),
            _ => Err(Self::common(&response)),
        }
    }
}
