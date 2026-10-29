//! The shapes of the Figurehead API contract the app uses.

use std::collections::BTreeMap;

/// What this Mac tells the service when linking.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceFields {
    /// This Mac's name (≤ 64 characters; user-editable).
    pub name: String,
    /// For example `macOS 26 · arm64` (≤ 32 characters).
    pub platform: String,
    /// The app version (≤ 32 characters).
    pub app_version: String,
    /// `ed25519:<base64>;x25519:<base64>` — the Fleet identity key and
    /// the key-transfer key (ADR-0014).
    pub public_key: String,
}

impl DeviceFields {
    /// Checks the field limits the service enforces.
    ///
    /// # Errors
    ///
    /// Returns a plain-words reason.
    pub fn validate(&self) -> Result<(), String> {
        let name = self.name.trim();
        if name.is_empty() || name.chars().count() > 64 {
            return Err("Use a name of 1–64 characters for this computer.".to_owned());
        }
        if self.platform.chars().count() > 32 || self.app_version.chars().count() > 32 {
            return Err("The platform or version text is too long.".to_owned());
        }
        if self.public_key.len() > 1024 {
            return Err("The identity key is too long.".to_owned());
        }
        Ok(())
    }
}

/// The device token: a secret. Never printed, never serialised — it
/// goes from the response straight into the Keychain.
#[derive(Clone, PartialEq, Eq)]
pub struct DeviceToken(String);

impl DeviceToken {
    /// Wraps a token read back from the Keychain.
    #[must_use]
    pub fn new(token: String) -> Self {
        Self(token)
    }

    /// The secret, for the `Authorization` header and the Keychain only.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for DeviceToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DeviceToken(…)")
    }
}

/// A started browser link: show `user_code`, open `verify_url`, poll
/// every `interval_s`. The poll token is a secret and stays inside.
#[derive(Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkRequest {
    /// The request id.
    pub request_id: String,
    /// The code the user checks in the browser (e.g. `QVM3-GABM`).
    pub user_code: String,
    /// Where the user approves.
    pub verify_url: String,
    poll_token: String,
    /// Seconds until the request expires.
    #[serde(rename = "expiresIn")]
    pub expires_in_s: u64,
    /// The minimum seconds between polls.
    #[serde(rename = "interval")]
    pub interval_s: u64,
}

impl LinkRequest {
    pub(crate) fn poll_token(&self) -> &str {
        &self.poll_token
    }
}

impl std::fmt::Debug for LinkRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinkRequest")
            .field("request_id", &self.request_id)
            .field("user_code", &self.user_code)
            .field("interval_s", &self.interval_s)
            .finish_non_exhaustive()
    }
}

/// One poll's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PollOutcome {
    /// Not decided yet.
    Pending,
    /// Approved: store the token NOW (it is delivered once).
    Approved {
        /// The new device id.
        device_id: String,
        /// The device token.
        token: DeviceToken,
    },
    /// The user clicked Deny.
    Denied,
    /// The request expired (or its token was already delivered).
    Expired,
    /// Polled too fast: wait this many seconds from now on.
    SlowDown {
        /// The new interval.
        interval_s: u64,
    },
    /// Unknown request (wrong id or poll token): start a fresh link.
    NotFound,
}

/// One feature's entitlement. Unknown limit fields are kept as-is.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Entitlement {
    /// Whether this account may use it.
    pub enabled: bool,
    /// `free` | `paid`.
    #[serde(default = "free")]
    pub tier: String,
    /// `false` = the feature doesn't exist yet: hide it.
    #[serde(default = "yes")]
    pub available: bool,
    /// Limits (`maxBackups`, `maxBytes`, …); `null` = unlimited.
    #[serde(flatten)]
    pub limits: BTreeMap<String, serde_json::Value>,
}

fn free() -> String {
    "free".to_owned()
}

const fn yes() -> bool {
    true
}

impl Entitlement {
    /// Shown as a disabled toggle with Upgrade.
    #[must_use]
    pub fn needs_upgrade(&self) -> bool {
        self.available && !self.enabled && self.tier == "paid"
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
struct Named {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    email: Option<String>,
}

/// The account, this Mac and the plan, as `GET /api/v1/account` says
/// (cached; holds no secret).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    #[serde(default)]
    user: Named,
    #[serde(default)]
    device: Named,
    #[serde(default)]
    plan: Named,
    /// Feature key → entitlement. Unknown keys are kept but never shown.
    #[serde(default)]
    pub entitlements: BTreeMap<String, Entitlement>,
    /// Where Upgrade goes.
    #[serde(default)]
    pub upgrade_url: Option<String>,
    /// True when the service had no account endpoint and these are the
    /// Phase-1 defaults.
    #[serde(default, skip_deserializing)]
    pub fallback: bool,
}

/// The Phase-1 cloud features (enabled by default when the service has
/// no account endpoint yet).
pub const PHASE1_FEATURES: [&str; 3] = ["cloudBackup", "fleetDiscovery", "usageSync"];

impl AccountView {
    /// The account e-mail, when sent.
    #[must_use]
    pub fn email(&self) -> Option<&str> {
        self.user.email.as_deref()
    }

    /// The plan's display name, when sent.
    #[must_use]
    pub fn plan_name(&self) -> Option<&str> {
        self.plan.name.as_deref()
    }

    /// This Mac's name as the service knows it.
    #[must_use]
    pub fn device_name(&self) -> Option<&str> {
        self.device.name.as_deref()
    }

    /// The Phase-1 defaults (`GET /api/v1/account` answered `404`).
    #[must_use]
    pub fn phase1_defaults() -> Self {
        Self {
            user: Named::default(),
            device: Named::default(),
            plan: Named::default(),
            entitlements: PHASE1_FEATURES
                .iter()
                .map(|key| {
                    (
                        (*key).to_owned(),
                        Entitlement {
                            enabled: true,
                            tier: free(),
                            available: true,
                            limits: BTreeMap::new(),
                        },
                    )
                })
                .collect(),
            upgrade_url: None,
            fallback: true,
        }
    }

    /// Whether `feature` may be used.
    #[must_use]
    pub fn allows(&self, feature: &str) -> bool {
        self.entitlements
            .get(feature)
            .is_some_and(|entitlement| entitlement.available && entitlement.enabled)
    }
}
