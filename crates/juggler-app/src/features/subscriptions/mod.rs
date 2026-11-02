//! AJ-E2109 (D-35): accounts are subscriptions; profiles sit under them.
//!
//! What AI Juggler stores per folder is a **profile** (a Claude Code
//! folder, a Claude app profile, a `CODEX_HOME`). The **account** is the
//! subscription a profile is signed in to: worked out here when read,
//! from each profile's sign-in (its id and organisation, through the
//! adapter's `probe_identity`: metadata only, never a token). Nothing is
//! stored and the schema doesn't change (0.1.x).
//!
//! One login on two profiles is one account (one runway). A personal plan
//! and a team seat on one email are two (their organisations differ).
//! Profiles that aren't signed in are grouped last, per tool; one whose
//! sign-in can't be read stands alone (it can't be told apart).
//!
//! D-62 (AJ-E3202): each sign-in is stored as its Account, and its plan,
//! renewal and fee are kept there ([`terms`]), moved from its folders
//! where they agree; where they don't, the Account says what to choose.

pub mod commands;
pub mod terms;

use std::collections::BTreeMap;
use std::sync::Arc;

use juggler_adapter::IdentityProbe;

use crate::commands::lock_storage_arc;
use crate::error::AppResult;
use crate::state::AppState;

/// One profile under an account.
#[derive(Clone, Debug, PartialEq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SubscriptionProfileInfo {
    /// The profile (AI Juggler's stored account row).
    pub id: String,
    /// Its name, as the user gave it or the import named it: what the
    /// account is called on screen (never a vendor's id or organisation).
    pub label: String,
    /// Its folder, `~`-relative.
    pub folder: String,
    /// Its own plan, as set on it (before 0.4, or while it has no Account).
    pub plan: Option<String>,
    /// Its own renewal date, as set on it.
    pub renewal_date: Option<String>,
    /// Its own monthly fee, USD, as set on it.
    #[ts(type = "number | null")]
    pub fee_usd: Option<f64>,
}

/// One account: a subscription and the profiles signed in to it.
#[derive(Clone, Debug, PartialEq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct SubscriptionInfo {
    /// Stable for this read (the UI's key); never a vendor id.
    pub key: String,
    /// D-62 (AJ-E3202): the stored Account (`acct:<vendor>:<hash>`), for
    /// a sign-in that could be read; never a vendor id.
    #[ts(optional = nullable)]
    pub account_id: Option<String>,
    /// D-62 (AJ-E3205): the name you gave the Account; `null`: its default.
    #[ts(optional = nullable)]
    pub name: Option<String>,
    /// AJ-E3206: the stored Account's colour, given once (an index into
    /// the UI's `PROFILE_HUES`); `null` for one not stored.
    #[ts(optional = nullable)]
    pub colour_slot: Option<u8>,
    /// The tool its profiles run.
    pub tool: String,
    /// `signed_in`, `signed_out` (its profiles aren't signed in) or
    /// `unverified` (a sign-in AI Juggler can't read).
    pub state: String,
    /// The sign-in's email, masked (`o…@work.com`); `None` when the
    /// sign-in records no email (never an id or a fragment of one).
    pub email: Option<String>,
    /// The team's name, for a seat in one (never a personal one's).
    pub organisation: Option<String>,
    /// Why a sign-in can't be read.
    pub reason: Option<String>,
    /// Its profiles, in the order AI Juggler lists them.
    pub profiles: Vec<SubscriptionProfileInfo>,
    /// Its plan: the stored Account's; for one not stored (not signed in,
    /// a sign-in that can't be read), what every profile naming one says.
    pub plan: Option<String>,
    /// Its renewal date, the same way.
    pub renewal_date: Option<String>,
    /// Its monthly fee, USD, the same way.
    #[ts(type = "number | null")]
    pub fee_usd: Option<f64>,
    /// Which of `plan`, `renewal` and `fee` its profiles disagree on (each
    /// then shows per profile, and reads its own until chosen).
    pub differs: Vec<String>,
    /// D-62 (AJ-E3202 req. 4): what the user chooses once, for a stored
    /// Account whose folders disagree.
    pub choices: Vec<terms::TermChoiceInfo>,
    /// AJ-E1601 (D-34): set for an account through another service (its
    /// kind is then "endpoint"): the service, and how honest its usage can
    /// be (`estimate` or `unknown`). Its profiles may run different tools;
    /// `tool` is its first's.
    #[ts(optional = nullable)]
    pub service: Option<crate::features::service_accounts::AccountServiceInfo>,
}

/// What one profile's read found.
#[derive(Clone, Debug)]
pub struct ProfileRead {
    /// The profile.
    pub profile: SubscriptionProfileInfo,
    /// Its tool.
    pub tool: String,
    /// Who it's signed in as.
    pub probe: IdentityProbe,
    /// AJ-E1601: the service it runs through, when it does.
    pub service: Option<crate::features::service_accounts::ProfileService>,
}

/// Groups profiles into accounts: signed-in accounts first (in the order
/// their first profile is listed), then sign-ins that can't be read, then
/// the profiles not signed in.
#[must_use]
pub fn group(rows: Vec<ProfileRead>) -> Vec<SubscriptionInfo> {
    let mut groups: Vec<(u8, String, SubscriptionInfo)> = Vec::new();
    let mut index: BTreeMap<String, usize> = BTreeMap::new();
    for row in rows {
        // AJ-E1601: through another service, the account is the one its
        // key belongs to, whichever tool each profile runs.
        let found = row.service.as_ref().map(|service| {
            (
                0,
                format!("svc|{}", service.account),
                if service.key {
                    "signed_in"
                } else {
                    "unverified"
                },
                None,
                None,
                (!service.key)
                    .then(|| format!("its {} isn't in this Mac's Keychain", service.info.key_name)),
            )
        });
        let (rank, key, state, email, organisation, reason) = match (found, &row.probe) {
            (Some(found), _) => found,
            (
                None,
                IdentityProbe::SignedIn {
                    subject,
                    label,
                    org,
                },
            ) => {
                let (email, org_name) = split_label(label);
                (
                    0,
                    format!("in|{}|{subject}|{}", row.tool, org.as_deref().unwrap_or("")),
                    "signed_in",
                    email,
                    org_name,
                    None,
                )
            }
            (None, IdentityProbe::Unverifiable { reason }) => (
                1,
                format!("unverified|{}", row.profile.id),
                "unverified",
                None,
                None,
                Some(reason.clone()),
            ),
            (None, IdentityProbe::SignedOut) => (
                2,
                format!("out|{}", row.tool),
                "signed_out",
                None,
                None,
                None,
            ),
        };
        if let Some(&at) = index.get(&key) {
            groups[at].2.profiles.push(row.profile);
            continue;
        }
        index.insert(key.clone(), groups.len());
        groups.push((
            rank,
            key.clone(),
            SubscriptionInfo {
                key: opaque(&key),
                account_id: None,
                name: None,
                colour_slot: None,
                tool: row.tool,
                state: state.to_owned(),
                email,
                organisation,
                reason,
                profiles: vec![row.profile],
                plan: None,
                renewal_date: None,
                fee_usd: None,
                differs: Vec::new(),
                choices: Vec::new(),
                service: row.service.map(|service| service.info),
            },
        ));
    }
    groups.sort_by_key(|(rank, ..)| *rank);
    groups
        .into_iter()
        .map(|(.., mut account)| {
            agree(&mut account);
            account
        })
        .collect()
}

/// The account's plan, renewal and fee where its profiles agree.
fn agree(account: &mut SubscriptionInfo) {
    let mut differs = Vec::new();
    account.plan = one(
        account.profiles.iter().map(|p| p.plan.clone()),
        "plan",
        &mut differs,
    );
    account.renewal_date = one(
        account.profiles.iter().map(|p| p.renewal_date.clone()),
        "renewal",
        &mut differs,
    );
    let fees = account
        .profiles
        .iter()
        .map(|p| p.fee_usd.map(|fee| format!("{fee}")));
    account.fee_usd = one(fees, "fee", &mut differs).and_then(|fee| fee.parse().ok());
    account.differs = differs;
}

/// The one value the known ones agree on; none, and `name` noted, when
/// they differ.
fn one(
    values: impl Iterator<Item = Option<String>>,
    name: &str,
    differs: &mut Vec<String>,
) -> Option<String> {
    let known: std::collections::BTreeSet<String> = values.flatten().collect();
    if known.len() > 1 {
        differs.push(name.to_owned());
        return None;
    }
    known.into_iter().next()
}

/// The probe's masked label as its email and its organisation's name.
/// A personal organisation (Claude names it after the email) is no team.
/// A label that isn't an email (Codex's account label, the
/// tail of an id) gives no email: an id is never shown (the 0.2 walk, F3).
fn split_label(label: &str) -> (Option<String>, Option<String>) {
    let (email, org) = label
        .split_once(" · ")
        .map_or((label, None), |(email, org)| (email, Some(org)));
    let org = org.map(str::trim).filter(|org| {
        !org.is_empty()
            && !org.contains('@')
            && !org.to_ascii_lowercase().ends_with("'s organization")
    });
    let email = email.trim();
    (
        email.contains('@').then(|| email.to_owned()),
        org.map(str::to_owned),
    )
}

/// A key the UI can hold: a hash, never the vendor's ids.
fn opaque(key: &str) -> String {
    juggler_adapter::handover::sha256_hex(key.as_bytes())[..16].to_owned()
}

/// Core: every profile, grouped into its account.
///
/// # Errors
///
/// Storage errors; a poisoned lock.
pub fn list_impl(state: &Arc<AppState>) -> AppResult<Vec<SubscriptionInfo>> {
    let (accounts, fees) = {
        let storage = lock_storage_arc(state)?;
        let accounts = storage.accounts().list()?;
        let fees: BTreeMap<String, f64> = accounts
            .iter()
            .filter_map(|account| {
                let own = storage.terms().own(account.id.as_str()).ok()??;
                Some((account.id.to_string(), own.fee_usd?))
            })
            .collect();
        (accounts, fees)
    };
    let mut services = crate::features::service_accounts::profile_services(state);
    let rows: Vec<ProfileRead> = accounts
        .iter()
        .map(|account| ProfileRead {
            service: services.remove(account.id.as_str()),
            probe: state.registry.get(&account.harness).map_or_else(
                || IdentityProbe::Unverifiable {
                    reason: "this tool isn't installed here".into(),
                },
                |adapter| adapter.probe_identity(account),
            ),
            tool: account.harness.to_string(),
            profile: SubscriptionProfileInfo {
                id: account.id.to_string(),
                label: account.label.clone(),
                folder: crate::features::quota::capture::home_relative(&account.state_dir),
                plan: account.plan.as_ref().map(|plan| plan.as_str().to_owned()),
                renewal_date: account.renewal_date.clone(),
                fee_usd: fees.get(account.id.as_str()).copied(),
            },
        })
        .collect();
    let stored = store_accounts(state, &rows);
    let mut accounts = group(rows);
    let storage = lock_storage_arc(state)?;
    let names: BTreeMap<String, String> = storage
        .subscriptions()
        .list()?
        .into_iter()
        .filter_map(|account| Some((account.id, account.name?)))
        .collect();
    let slots = storage.subscriptions().colour_slots()?;
    for account in &mut accounts {
        account.account_id = account
            .profiles
            .iter()
            .find_map(|profile| stored.get(&profile.id).cloned());
        let Some(id) = account.account_id.clone() else {
            continue;
        };
        account.name = names.get(&id).cloned();
        account.colour_slot = slots.get(&id).copied();
        on_account(&storage, &id, account)?;
    }
    Ok(accounts)
}

/// D-62 (AJ-E3202 req. 4): a stored Account's plan, renewal and fee are
/// its own, moved from its folders where they agree; where they don't,
/// the choice it waits for (each folder reads its own until then).
fn on_account(
    storage: &juggler_core::storage::Storage,
    id: &str,
    account: &mut SubscriptionInfo,
) -> AppResult<()> {
    let choices = storage.terms().settle(id)?;
    let terms = storage.terms().account(id)?.unwrap_or_default();
    account.plan = terms.plan;
    account.renewal_date = terms.renewal_date;
    account.fee_usd = terms.fee_usd;
    account.differs = choices
        .iter()
        .map(|choice| choice.term.as_str().to_owned())
        .collect();
    account.choices = choices.into_iter().map(Into::into).collect();
    Ok(())
}

/// Names Account `id` (`None` or blank: its default name).
///
/// # Errors
///
/// [`AppError::Refused`] for an Account that isn't stored, or a name
/// over 60 characters; storage errors.
pub fn rename_impl(state: &Arc<AppState>, id: &str, name: Option<&str>) -> AppResult<()> {
    if name.is_some_and(|name| name.trim().chars().count() > 60) {
        return Err(crate::error::AppError::Refused(
            "A name is 60 characters at most".to_owned(),
        ));
    }
    let storage = lock_storage_arc(state)?;
    if storage.subscriptions().get(id)?.is_none() {
        return Err(crate::error::AppError::Refused(
            "AI Juggler doesn't know that account".to_owned(),
        ));
    }
    storage.subscriptions().rename(id, name)?;
    Ok(())
}

/// The Account a profile's read names (`vendor`, sign-in hash): a
/// vendor's sign-in (subject and organisation), or the key a service
/// profile runs on. `None` when there's no sign-in to read.
fn sign_in(state: &Arc<AppState>, row: &ProfileRead) -> Option<(String, String)> {
    if let Some(service) = &row.service {
        return service.key.then(|| {
            (
                "service".to_owned(),
                hash(&format!("svc|{}", service.account)),
            )
        });
    }
    let IdentityProbe::SignedIn { subject, org, .. } = &row.probe else {
        return None;
    };
    let vendor = state
        .registry
        .get(&juggler_core::model::HarnessId::new(row.tool.clone()))
        .map_or_else(|| row.tool.clone(), |adapter| adapter.sign_in_vendor());
    let identity = hash(&format!(
        "{vendor}|{subject}|{}",
        org.as_deref().unwrap_or("")
    ));
    Some((vendor, identity))
}

fn hash(key: &str) -> String {
    juggler_adapter::handover::sha256_hex(key.as_bytes())
}

/// D-62 (AJ-E3202): each profile's sign-in stored as its Account (made
/// the first time it's read), and the folder pointed at it; a folder
/// with no sign-in to read points at none. The stored Account by folder.
fn store_accounts(state: &Arc<AppState>, rows: &[ProfileRead]) -> BTreeMap<String, String> {
    let found: Vec<(String, Option<(String, String)>)> = rows
        .iter()
        .map(|row| (row.profile.id.clone(), sign_in(state, row)))
        .collect();
    let Ok(storage) = lock_storage_arc(state) else {
        return BTreeMap::new();
    };
    let now = crate::commands::now_millis();
    let mut stored = BTreeMap::new();
    for (folder, sign_in) in found {
        let account = sign_in.and_then(|(vendor, identity)| {
            storage.subscriptions().upsert(&vendor, &identity, now).ok()
        });
        let _ = storage
            .subscriptions()
            .set_folder(&folder, account.as_deref());
        if let Some(account) = account {
            stored.insert(folder, account);
        }
    }
    stored
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, tool: &str, probe: IdentityProbe) -> ProfileRead {
        ProfileRead {
            profile: SubscriptionProfileInfo {
                id: id.to_owned(),
                label: format!("Account {id}"),
                folder: format!("~/{id}"),
                plan: None,
                renewal_date: None,
                fee_usd: None,
            },
            tool: tool.to_owned(),
            probe,
            service: None,
        }
    }

    fn through(id: &str, tool: &str, account: &str, key: bool) -> ProfileRead {
        let preset = &juggler_adapters::services::presets()[0];
        ProfileRead {
            service: Some(crate::features::service_accounts::ProfileService {
                account: account.to_owned(),
                info: crate::features::service_accounts::AccountServiceInfo::of(preset),
                key,
            }),
            ..row(id, tool, IdentityProbe::SignedOut)
        }
    }

    #[test]
    fn profiles_on_one_service_key_are_one_account_whatever_their_tools() {
        let accounts = group(vec![
            row("out", "claude-code", IdentityProbe::SignedOut),
            through("go-claude", "claude-code", "svc-1", true),
            through("go-codex", "codex", "svc-1", true),
            through("other", "claude-code", "svc-2", false),
        ]);
        assert_eq!(accounts.len(), 3);
        assert_eq!(ids(&accounts[0]), ["go-claude", "go-codex"]);
        assert_eq!(accounts[0].state, "signed_in");
        let service = accounts[0].service.as_ref().unwrap();
        assert_eq!(service.usage, "estimate");
        // A key that isn't here is said, never guessed.
        assert_eq!(ids(&accounts[1]), ["other"]);
        assert_eq!(accounts[1].state, "unverified");
        assert!(accounts[1].reason.as_deref().unwrap().contains("Keychain"));
        assert!(accounts[2].service.is_none());
        assert!(!accounts[0].key.contains("svc"));
    }

    fn signed(subject: &str, org: Option<&str>, label: &str) -> IdentityProbe {
        IdentityProbe::SignedIn {
            subject: subject.to_owned(),
            label: label.to_owned(),
            org: org.map(str::to_owned),
        }
    }

    fn ids(account: &SubscriptionInfo) -> Vec<&str> {
        account.profiles.iter().map(|p| p.id.as_str()).collect()
    }

    #[test]
    fn two_profiles_on_one_login_are_one_account() {
        let accounts = group(vec![
            row(
                "work",
                "claude-code",
                signed("acct-1", Some("org-1"), "o…@work.com"),
            ),
            row(
                "work-2",
                "claude-code",
                signed("acct-1", Some("org-1"), "o…@work.com"),
            ),
            row(
                "home",
                "claude-code",
                signed("acct-2", Some("org-2"), "o…@home.com"),
            ),
        ]);
        assert_eq!(accounts.len(), 2);
        assert_eq!(ids(&accounts[0]), ["work", "work-2"]);
        assert_eq!(accounts[0].email.as_deref(), Some("o…@work.com"));
        assert_eq!(ids(&accounts[1]), ["home"]);
    }

    #[test]
    fn a_personal_plan_and_a_team_seat_on_one_email_stay_apart() {
        let accounts = group(vec![
            row(
                "personal",
                "claude-code",
                signed(
                    "acct-1",
                    Some("org-me"),
                    "s…@work.com · sam@work.com's Organization",
                ),
            ),
            row(
                "team",
                "claude-code",
                signed("acct-1", Some("org-acme"), "o…@work.com · Acme"),
            ),
        ]);
        assert_eq!(accounts.len(), 2);
        // A personal organisation is named after the email: never shown.
        assert_eq!(accounts[0].organisation, None);
        assert_eq!(accounts[1].organisation.as_deref(), Some("Acme"));
    }

    #[test]
    fn one_login_on_two_tools_is_two_accounts() {
        let accounts = group(vec![
            row("a", "claude-code", signed("same", None, "x")),
            row("b", "codex", signed("same", None, "x")),
        ]);
        assert_eq!(accounts.len(), 2);
    }

    #[test]
    fn not_signed_in_comes_last_and_an_unreadable_sign_in_stands_alone() {
        let accounts = group(vec![
            row("out-1", "claude-code", IdentityProbe::SignedOut),
            row(
                "key-1",
                "codex",
                IdentityProbe::Unverifiable {
                    reason: "API key".into(),
                },
            ),
            row(
                "key-2",
                "codex",
                IdentityProbe::Unverifiable {
                    reason: "API key".into(),
                },
            ),
            row("in", "claude-code", signed("acct", None, "o…@a.com")),
            row("out-2", "claude-code", IdentityProbe::SignedOut),
        ]);
        let states: Vec<&str> = accounts.iter().map(|a| a.state.as_str()).collect();
        assert_eq!(
            states,
            ["signed_in", "unverified", "unverified", "signed_out"]
        );
        assert_eq!(ids(&accounts[3]), ["out-1", "out-2"]);
    }

    #[test]
    fn plan_renewal_and_fee_are_the_accounts_when_its_profiles_agree() {
        let mut a = row("a", "claude-code", signed("acct", None, "o…@a.com"));
        a.profile.plan = Some("Max".into());
        a.profile.fee_usd = Some(100.0);
        a.profile.renewal_date = Some("2026-11-01".into());
        let mut b = row("b", "claude-code", signed("acct", None, "o…@a.com"));
        b.profile.plan = Some("Max".into());
        b.profile.fee_usd = Some(200.0);
        let accounts = group(vec![a, b]);
        assert_eq!(accounts[0].plan.as_deref(), Some("Max"));
        assert_eq!(accounts[0].renewal_date.as_deref(), Some("2026-11-01"));
        assert_eq!(accounts[0].fee_usd, None);
        assert_eq!(accounts[0].differs, ["fee"]);
    }

    #[test]
    fn an_id_is_never_an_email_and_each_profile_keeps_its_own_name() {
        // The 0.2 walk (F3): Codex's label is the tail of an account id.
        let accounts = group(vec![
            row(
                "codex",
                "codex",
                signed("acct-123456789", None, "ChatGPT account …6789"),
            ),
            row(
                "team",
                "claude-code",
                signed("acct-1", Some("org-acme"), "o…@work.com · Acme"),
            ),
        ]);
        assert_eq!(accounts[0].email, None);
        assert_eq!(accounts[0].profiles[0].label, "Account codex");
        assert_eq!(accounts[1].email.as_deref(), Some("o…@work.com"));
        assert_eq!(accounts[1].organisation.as_deref(), Some("Acme"));
        assert_eq!(accounts[1].profiles[0].label, "Account team");
    }

    #[test]
    fn the_key_never_carries_the_vendor_ids() {
        let accounts = group(vec![row(
            "a",
            "claude-code",
            signed("acct-secretish", Some("org-x"), "o…@a.com"),
        )]);
        assert!(!accounts[0].key.contains("acct"));
        assert_eq!(accounts[0].key.len(), 16);
    }
}
