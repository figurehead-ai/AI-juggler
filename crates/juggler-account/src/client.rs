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
    #[error("This computer was unlinked from Figurehead.")]
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
    /// `413`, or larger than the plan's limit before sending.
    #[error("This backup is larger than your plan allows ({limit_mb} MB) — nothing was uploaded.")]
    TooLarge {
        /// The limit, in whole MB as Finder counts a file (1,000,000 bytes),
        /// as the app's backup sizes are shown (0.1.8: the panel said
        /// "26 MB" for the same limit this said was "25 MB").
        limit_mb: u64,
    },
    /// The bytes don't match the checksum the service gave.
    #[error("The backup changed on the way — nothing was restored. Try again.")]
    ChecksumMismatch,
}

/// One backup the service holds (`GET backups`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRecord {
    /// The service's id (`bak_…`).
    pub id: String,
    /// The label sent with it, if any.
    #[serde(default)]
    pub label: Option<String>,
    /// Its size.
    pub size_bytes: u64,
    /// sha256 of the ciphertext, as stored.
    pub sha256: String,
    /// The Mac that uploaded it.
    #[serde(default)]
    pub device_id: Option<String>,
    /// When, as the service writes it (RFC 3339).
    pub created_at: String,
}

/// One Mac in the account's directory (`GET devices`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRecord {
    /// The service's device id (`dev_…`).
    pub id: String,
    /// Its name.
    #[serde(default)]
    pub name: String,
    /// Its platform.
    #[serde(default)]
    pub platform: Option<String>,
    /// Its app version.
    #[serde(default)]
    pub app_version: Option<String>,
    /// Its Fleet identity public key (standard base64): a candidate to
    /// pin, never trust by itself.
    #[serde(default)]
    pub public_key: Option<String>,
    /// When it was last seen (RFC 3339).
    #[serde(default)]
    pub last_seen_at: Option<String>,
}

/// The account's linked Macs.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct DeviceDirectory {
    /// This Mac's own device id.
    #[serde(rename = "self")]
    pub self_id: String,
    /// Every linked Mac, this one included.
    pub devices: Vec<DeviceRecord>,
}

/// What an upload stored (`201`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadedBackup {
    /// The service's id.
    pub id: String,
    /// sha256 of what it stored.
    pub sha256: String,
    /// Its size.
    pub size_bytes: u64,
}

/// The backup size limit when the plan names none: 25 MB.
pub const DEFAULT_BACKUP_LIMIT: u64 = 25 * 1024 * 1024;

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    use std::fmt::Write as _;
    sha2::Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
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
            long_transfer: false,
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

    /// One poll of a browser link (never retried: the token is delivered
    /// once, so a lost answer means a fresh link).
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
                code: "Use a name of 1–64 characters for this computer.".to_owned(),
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

    /// Uploads an encrypted backup (`POST backups`, octet-stream). Never
    /// sends more than `limit_bytes`; checks the stored checksum.
    ///
    /// # Errors
    ///
    /// [`ApiError::TooLarge`] (before sending, or on `413`),
    /// [`ApiError::UpgradeRequired`] on `402`, [`ApiError::Malformed`] when
    /// the stored checksum differs, or another [`ApiError`].
    pub fn upload_backup(
        &self,
        token: &DeviceToken,
        label: Option<&str>,
        ciphertext: &[u8],
        limit_bytes: u64,
    ) -> Result<UploadedBackup, ApiError> {
        let limit_mb = limit_bytes.saturating_add(500_000) / 1_000_000;
        if ciphertext.len() as u64 > limit_bytes {
            return Err(ApiError::TooLarge { limit_mb });
        }
        let mut request = self.request("POST", "backups", Some(token.expose()), None);
        request.content_type = "application/octet-stream";
        request.body = ciphertext.to_vec();
        request.long_transfer = true;
        if let Some(label) = label {
            let label: String = label.chars().take(120).collect();
            request.headers.push(("x-backup-label", label));
        }
        let response = self.send(&request, true)?;
        match response.status {
            200 | 201 => {
                let stored: UploadedBackup = serde_json::from_slice(&response.body)
                    .map_err(|e| ApiError::Malformed(e.to_string()))?;
                if stored.sha256 != sha256_hex(ciphertext) {
                    return Err(ApiError::Malformed(
                        "the stored checksum doesn't match what was sent".to_owned(),
                    ));
                }
                Ok(stored)
            }
            413 => Err(ApiError::TooLarge { limit_mb }),
            _ => Err(Self::common(&response)),
        }
    }

    /// The account's linked Macs (`GET devices`) — a directory of
    /// candidates, not a relay.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`].
    pub fn list_devices(&self, token: &DeviceToken) -> Result<DeviceDirectory, ApiError> {
        let response = self.send(
            &self.request("GET", "devices", Some(token.expose()), None),
            true,
        )?;
        if response.status != 200 {
            return Err(Self::common(&response));
        }
        serde_json::from_slice(&response.body).map_err(|e| ApiError::Malformed(e.to_string()))
    }

    /// Lists the account's backups, newest first (`GET backups`).
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`].
    pub fn list_backups(&self, token: &DeviceToken) -> Result<Vec<BackupRecord>, ApiError> {
        #[derive(serde::Deserialize)]
        struct Listed {
            backups: Vec<BackupRecord>,
        }
        let response = self.send(
            &self.request("GET", "backups", Some(token.expose()), None),
            true,
        )?;
        if response.status != 200 {
            return Err(Self::common(&response));
        }
        let mut listed: Listed = serde_json::from_slice(&response.body)
            .map_err(|e| ApiError::Malformed(e.to_string()))?;
        listed
            .backups
            .sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(listed.backups)
    }

    /// Downloads one backup's ciphertext (`GET backups/{id}`) and checks
    /// it against `X-Backup-Sha256`.
    ///
    /// # Errors
    ///
    /// [`ApiError::ChecksumMismatch`] when the bytes don't match, or
    /// another [`ApiError`].
    pub fn download_backup(&self, token: &DeviceToken, id: &str) -> Result<Vec<u8>, ApiError> {
        if id.is_empty()
            || !id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(ApiError::Malformed("an invalid backup id".to_owned()));
        }
        let mut request = self.request("GET", &format!("backups/{id}"), Some(token.expose()), None);
        request.long_transfer = true;
        let response = self.send(&request, true)?;
        if response.status != 200 {
            return Err(Self::common(&response));
        }
        let expected = response
            .header("x-backup-sha256")
            .ok_or_else(|| ApiError::Malformed("no X-Backup-Sha256".to_owned()))?;
        if !expected.eq_ignore_ascii_case(&sha256_hex(&response.body)) {
            return Err(ApiError::ChecksumMismatch);
        }
        Ok(response.body)
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
