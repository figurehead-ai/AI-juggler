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
//! sign-in can't be read stands alone (it can't be told apart), unless it
//! was read as an Account before: a check that can't tell (0.4.1) keeps it
//! there, marked [`SubscriptionProfileInfo::unchecked`], until a read says
//! it's signed out or signed in as another.
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
    /// 0.4.1: why its sign-in couldn't be checked on this read (a file
    /// mid-write, a timeout, a tool not installed), for a folder that
    /// stays in the Account it was last read as until a read says
    /// otherwise; `null` when this read checked it (or it never had one).
    #[ts(optional = nullable)]
    pub unchecked: Option<String>,
    /// AJ-E3202 req. 3 (0.4.1): the name the user accepted it under at
    /// import (first run's found step, or one they typed), as stored
    /// (`accounts.import_name`, v52); `null` for one added another way or
    /// named without them (never a folder's name they didn't see).
    #[ts(optional = nullable)]
    pub accepted_name: Option<String>,
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
    /// D-62 (AJ-E3202 req. 3): what every screen and notice calls it,
    /// worked out once, where the list is built: the name you gave it; else
    /// its service's; else its team, its tool and plan ("Claude Max
    /// 20x"), the one name all its Apps were accepted under at import
    /// ("Work"), or its masked email, told apart from another that reads
    /// the same. Folders not signed in are "Not signed in"; a sign-in that
    /// can't be read is "Account unknown". Never a folder's or an app's
    /// name the user didn't accept.
    pub display_name: String,
    /// AJ-E3206: the stored Account's colour, given once (an index into
    /// the UI's `MARK_COLOURS`); `null` for one not stored.
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
    /// rc22 (A3): its Apps, by id: the user's Apps
    /// (`apps::users_apps`, the Apps list) whose folder is one of its
    /// profiles'. Every screen that lists or counts an Account's Apps
    /// reads this (`lib/account-items`'s `appsOfAccount`); absent where
    /// it wasn't read ([`list_impl`] alone) and in the preview.
    #[ts(optional)]
    pub apps: Option<Vec<String>>,
    /// AJ-E3202 req. 3: a desktop App the user added sits on one of its
    /// folders: an App not brought in by import, so its Apps weren't all
    /// accepted under one name. Read where the list is built; not sent.
    #[serde(skip)]
    #[ts(skip)]
    pub desktop_apps: bool,
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
    group_linked(rows, &BTreeMap::new())
}

/// [`group`], with each folder's stored Account (`linked`, by folder):
/// a folder whose sign-in couldn't be checked on this read
/// ([`SubscriptionProfileInfo::unchecked`]) stays in the Account it's
/// linked to, beside the folders signed in to it now.
#[must_use]
pub fn group_linked(
    rows: Vec<ProfileRead>,
    linked: &BTreeMap<String, String>,
) -> Vec<SubscriptionInfo> {
    let keys = account_keys(&rows, linked);
    let mut groups: Vec<(u8, String, SubscriptionInfo)> = Vec::new();
    let mut index: BTreeMap<String, usize> = BTreeMap::new();
    for row in rows {
        // 0.4.1: couldn't tell this time; it stays where it was.
        let kept = row
            .profile
            .unchecked
            .as_ref()
            .and(linked.get(&row.profile.id))
            .map(|account| {
                keys.get(account)
                    .cloned()
                    .unwrap_or_else(|| format!("kept|{account}"))
            });
        let Place {
            rank,
            key,
            state,
            email,
            organisation,
            reason,
        } = place(&row, kept);
        if let Some(&at) = index.get(&key) {
            let account = &mut groups[at].2;
            // A folder kept in it may have made it: one signed in now says
            // who it is.
            account.email = account.email.take().or(email);
            account.organisation = account.organisation.take().or(organisation);
            account.profiles.push(row.profile);
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
                display_name: String::new(),
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
                apps: None,
                desktop_apps: false,
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

/// Where one profile's read puts it: its group's rank and key, and what
/// the group says.
struct Place {
    rank: u8,
    key: String,
    state: &'static str,
    email: Option<String>,
    organisation: Option<String>,
    reason: Option<String>,
}

/// The group key each stored Account's signed-in folders read as now.
fn account_keys(
    rows: &[ProfileRead],
    linked: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut keys = BTreeMap::new();
    for row in rows {
        if let (None, IdentityProbe::SignedIn { subject, org, .. }) = (&row.service, &row.probe)
            && let Some(account) = linked.get(&row.profile.id)
        {
            keys.entry(account.clone())
                .or_insert_with(|| signed_in_key(&row.tool, subject, org.as_deref()));
        }
    }
    keys
}

/// [`Place`] for one profile's read; `kept`, the group of the Account a
/// folder that couldn't be checked stays in.
fn place(row: &ProfileRead, kept: Option<String>) -> Place {
    let plain = |rank, key, state| Place {
        rank,
        key,
        state,
        email: None,
        organisation: None,
        reason: None,
    };
    // AJ-E1601: through another service, the account is the one its key
    // belongs to, whichever tool each profile runs.
    if let Some(service) = &row.service {
        return Place {
            reason: (!service.key)
                .then(|| format!("its {} isn't in this Mac's Keychain", service.info.key_name)),
            ..plain(
                0,
                format!("svc|{}", service.account),
                if service.key {
                    "signed_in"
                } else {
                    "unverified"
                },
            )
        };
    }
    if let Some(key) = kept {
        // Who it was the last time it was read, where that's known.
        let (email, organisation) = match &row.probe {
            IdentityProbe::SignedIn { label, .. } => split_label(label),
            _ => (None, None),
        };
        return Place {
            email,
            organisation,
            ..plain(0, key, "signed_in")
        };
    }
    match &row.probe {
        IdentityProbe::SignedIn {
            subject,
            label,
            org,
        } => {
            let (email, organisation) = split_label(label);
            Place {
                email,
                organisation,
                ..plain(
                    0,
                    signed_in_key(&row.tool, subject, org.as_deref()),
                    "signed_in",
                )
            }
        }
        // A key names no account: it stands alone, said as it always was.
        IdentityProbe::SignedInWithKey { reason } | IdentityProbe::Unverifiable { reason } => {
            Place {
                reason: Some(reason.clone()),
                ..plain(1, format!("unverified|{}", row.profile.id), "unverified")
            }
        }
        IdentityProbe::SignedOut => plain(2, format!("out|{}", row.tool), "signed_out"),
    }
}

/// The group a signed-in folder's sign-in reads as.
fn signed_in_key(tool: &str, subject: &str, org: Option<&str>) -> String {
    format!("in|{tool}|{subject}|{}", org.unwrap_or(""))
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
    let (accounts, fees, accepted) = {
        let storage = lock_storage_arc(state)?;
        let accounts = storage.accounts().list()?;
        let accepted = storage.accounts().import_names()?;
        let fees: BTreeMap<String, f64> = accounts
            .iter()
            .filter_map(|account| {
                let own = storage.terms().own(account.id.as_str()).ok()??;
                Some((account.id.to_string(), own.fee_usd?))
            })
            .collect();
        (accounts, fees, accepted)
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
                unchecked: None,
                accepted_name: accepted.get(account.id.as_str()).cloned(),
            },
        })
        .collect();
    let Links {
        accounts: stored,
        kept,
        moved,
    } = store_accounts(state, &rows);
    let mut rows = rows;
    for row in &mut rows {
        if let (true, IdentityProbe::Unverifiable { reason }) =
            (kept.contains(&row.profile.id), &row.probe)
        {
            row.profile.unchecked = Some(reason.clone());
        }
    }
    // A kept folder reads as who it was last (its email, its team).
    state.folder_accounts.remember(&mut rows, &kept);
    let mut accounts = group_linked(rows, &stored);
    let storage = lock_storage_arc(state)?;
    let names: BTreeMap<String, String> = storage
        .subscriptions()
        .list()?
        .into_iter()
        .filter_map(|account| Some((account.id, account.name?)))
        .collect();
    let slots = storage.subscriptions().colour_slots()?;
    let desktop = desktop_folders(storage.apps().list()?);
    for account in &mut accounts {
        account.desktop_apps = account
            .profiles
            .iter()
            .any(|profile| desktop.contains(&profile.id));
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
    drop(storage);
    name_all(&mut accounts, |tool| tool_words(state, tool));
    // D-62 (P5 step 4): the pools a sign-in change moved open sessions
    // out of and into say who, by the Accounts' names now.
    if !moved.is_empty() {
        let names = by_folder(&accounts);
        crate::features::pools::sign_in::tell(state, &moved, |id| {
            names.values().find(|account| account.id == id).map_or_else(
                || "another account".to_owned(),
                |account| account.name.clone(),
            )
        });
    }
    Ok(accounts)
}

/// The window's list ([`list_impl`]), each Account with its Apps
/// ([`with_apps`]).
///
/// # Errors
///
/// As [`list_impl`] and `apps::list_impl`.
pub fn list_with_apps_impl(state: &Arc<AppState>) -> AppResult<Vec<SubscriptionInfo>> {
    let mut accounts = list_impl(state)?;
    with_apps(&mut accounts, &crate::features::apps::list_impl(state)?);
    Ok(accounts)
}

/// The folders a desktop App the user added uses (one they hid isn't
/// among an Account's Apps).
fn desktop_folders(apps: Vec<juggler_core::storage::App>) -> std::collections::BTreeSet<String> {
    apps.into_iter()
        .filter(|app| app.kind != "cli" && !app.hidden && app.is_users())
        .filter_map(|app| app.folder_id)
        .collect()
}

/// rc22 (A3): each Account's Apps, of the Apps list (`listed`): those on
/// one of its profiles' folders.
pub fn with_apps(
    accounts: &mut [SubscriptionInfo],
    listed: &[crate::features::apps::StoredAppInfo],
) {
    for account in accounts {
        account.apps = Some(
            listed
                .iter()
                .filter(|app| {
                    app.folder_id
                        .as_deref()
                        .is_some_and(|folder| account.profiles.iter().any(|p| p.id == folder))
                })
                .map(|app| app.id.clone())
                .collect(),
        );
    }
}

/// A tool as an Account's name says it: its short name ("Claude"), else
/// its full one; a tool not registered here, its id.
fn tool_words(state: &AppState, tool: &str) -> String {
    state
        .registry
        .get(&juggler_core::model::HarnessId::new(tool.to_owned()))
        .map_or_else(
            || tool.to_owned(),
            |adapter| {
                let spec = adapter.spec();
                if spec.short_name.is_empty() {
                    spec.display_name
                } else {
                    spec.short_name
                }
            },
        )
}

/// A plan as it's said: "max-20x" → "Max 20x", "pro" → "Pro".
#[must_use]
pub fn plan_words(plan: &str) -> String {
    plan.split(|c: char| c == '-' || c == '_' || c.is_whitespace())
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect::<String>()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Some, and not blank.
fn given(value: Option<&String>) -> Option<&str> {
    value
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
}

/// AJ-E3202 req. 3 (0.4.1): the one name every one of the Account's
/// Apps was accepted under at import ("Work", seen and kept in first
/// run's found step). None when one of them wasn't (added another way, a
/// desktop App on one of its folders, a name worked out from a folder
/// that the user never saw) or they were accepted under different names.
fn accepted_name(row: &SubscriptionInfo) -> Option<&str> {
    if row.desktop_apps {
        return None;
    }
    let mut names = row
        .profiles
        .iter()
        .map(|profile| given(profile.accepted_name.as_ref()).map(str::trim));
    let first = names.next()??;
    names.all(|name| name == Some(first)).then_some(first)
}

/// An Account's default name, before it's told apart from its twins:
/// its team; else its tool and plan; else the one name all its Apps were
/// accepted under at import ([`accepted_name`]); else its masked email;
/// else the tool's ("Claude account").
fn default_name(row: &SubscriptionInfo, tool: &str) -> String {
    if let Some(organisation) = given(row.organisation.as_ref()) {
        return organisation.to_owned();
    }
    if let Some(plan) = given(row.plan.as_ref()) {
        return format!("{tool} {}", plan_words(plan));
    }
    if let Some(name) = accepted_name(row) {
        return name.to_owned();
    }
    given(row.email.as_ref()).map_or_else(|| format!("{tool} account"), str::to_owned)
}

/// Gives every Account its [`SubscriptionInfo::display_name`] (D-62,
/// AJ-E3202 req. 3; AJ-E3205): the one rule every screen and notice
/// reads. `tool_words` says a tool ("Claude"). Two signed-in Accounts
/// with no name of their own that would read the same are told apart:
/// by their masked email; where that is the name itself, by their tool
/// where theirs differ, else by their place among them (by id).
/// Renaming one settles it.
pub fn name_all(accounts: &mut [SubscriptionInfo], tool_words: impl Fn(&str) -> String) {
    let defaults: Vec<Option<String>> = accounts
        .iter()
        .map(|row| {
            let plain = row.state == "signed_in"
                && given(row.name.as_ref()).is_none()
                && row.service.is_none();
            plain.then(|| default_name(row, &tool_words(&row.tool)))
        })
        .collect();
    let names: Vec<String> = accounts
        .iter()
        .map(|row| name_of(row, accounts, &defaults, &tool_words))
        .collect();
    for (row, name) in accounts.iter_mut().zip(names) {
        row.display_name = name;
    }
}

fn name_of(
    row: &SubscriptionInfo,
    all: &[SubscriptionInfo],
    defaults: &[Option<String>],
    tool_words: &impl Fn(&str) -> String,
) -> String {
    if row.state == "signed_out" {
        return "Not signed in".to_owned();
    }
    if let Some(name) = given(row.name.as_ref()) {
        return name.trim().to_owned();
    }
    // AJ-E1601: an account through another service is named for it.
    if let Some(service) = &row.service {
        return service.name.clone();
    }
    if row.state == "unverified" {
        return "Account unknown".to_owned();
    }
    let tool = tool_words(&row.tool);
    let base = default_name(row, &tool);
    let twins: Vec<&SubscriptionInfo> = all
        .iter()
        .zip(defaults)
        .filter(|(_, default)| default.as_deref() == Some(base.as_str()))
        .map(|(other, _)| other)
        .collect();
    if !twins.iter().any(|other| other.key != row.key) {
        return base;
    }
    if let Some(email) = given(row.email.as_ref())
        && email != base
    {
        return format!("{base} · {email}");
    }
    // Its masked email is its name and another's reads the same (masking
    // keeps one letter): its tool says which where theirs differ.
    let tools: std::collections::BTreeSet<&str> =
        twins.iter().map(|other| other.tool.as_str()).collect();
    if tools.len() == twins.len() {
        return format!("{tool} · {base}");
    }
    let id = |other: &SubscriptionInfo| {
        other
            .account_id
            .clone()
            .unwrap_or_else(|| other.key.clone())
    };
    let mut order: Vec<String> = twins.iter().map(|other| id(other)).collect();
    order.sort();
    let place = order
        .iter()
        .position(|other| *other == id(row))
        .unwrap_or(0)
        + 1;
    format!("{base} ({place})")
}

/// D-62: the Account a folder is signed in to, as a notice names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FolderAccount {
    /// The stored Account (`acct:<vendor>:<hash>`).
    pub id: String,
    /// Its [`SubscriptionInfo::display_name`].
    pub name: String,
    /// Every folder signed in to it, in the order AI Juggler lists them.
    pub folders: Vec<String>,
}

/// Each folder signed in to a stored Account, with that Account. A
/// folder in none (not signed in, a sign-in that can't be read) isn't
/// listed: it stands for itself.
#[must_use]
pub fn by_folder(accounts: &[SubscriptionInfo]) -> BTreeMap<String, FolderAccount> {
    let mut out = BTreeMap::new();
    for account in accounts {
        let Some(id) = &account.account_id else {
            continue;
        };
        let found = FolderAccount {
            id: id.clone(),
            name: account.display_name.clone(),
            folders: account.profiles.iter().map(|p| p.id.clone()).collect(),
        };
        for folder in &found.folders {
            out.insert(folder.clone(), found.clone());
        }
    }
    out
}

/// What the last reads gave, kept for one that can't tell.
#[derive(Debug, Default)]
pub struct LastRead {
    /// The Accounts by folder the last whole read gave
    /// ([`folder_accounts`]), kept for a read that fails.
    names: std::sync::Mutex<Option<BTreeMap<String, FolderAccount>>>,
    /// 0.4.1: each folder's last sign-in that was read (an
    /// [`IdentityProbe::SignedIn`]), so one that can't be checked for a
    /// while keeps its email and team (and the name they give it). Held
    /// for this run only.
    sign_ins: std::sync::Mutex<BTreeMap<String, IdentityProbe>>,
}

impl LastRead {
    /// Notes this read's sign-ins, and stands in the last one for each
    /// folder kept in its Account (`kept`) whose sign-in this read
    /// couldn't check.
    fn remember(&self, rows: &mut [ProfileRead], kept: &std::collections::BTreeSet<String>) {
        let Ok(mut known) = self.sign_ins.lock() else {
            return;
        };
        for row in rows {
            let folder = &row.profile.id;
            match &row.probe {
                IdentityProbe::SignedIn { .. } if row.service.is_none() => {
                    known.insert(folder.clone(), row.probe.clone());
                }
                IdentityProbe::Unverifiable { .. } if kept.contains(folder) => {
                    if let Some(last) = known.get(folder) {
                        row.probe = last.clone();
                    }
                }
                IdentityProbe::Unverifiable { .. } => {}
                _ => {
                    known.remove(folder);
                }
            }
        }
    }
}

/// [`by_folder`], read now ([`list_impl`]: each folder's sign-in is read
/// again). A read that fails is logged with its reason and answers what
/// the last whole read gave: a notice is posted once, and a failure for a
/// moment (the database busy) must not name an Account by its folder in
/// it. Before any read has worked it's empty (each folder stands for
/// itself).
#[must_use]
pub fn folder_accounts(state: &Arc<AppState>) -> BTreeMap<String, FolderAccount> {
    let last = &state.folder_accounts.names;
    match list_impl(state) {
        Ok(accounts) => {
            let read = by_folder(&accounts);
            if let Ok(mut last) = last.lock() {
                *last = Some(read.clone());
            }
            read
        }
        Err(error) => {
            let kept = last.lock().ok().and_then(|last| last.clone());
            tracing::warn!(
                %error,
                kept = kept.is_some(),
                "the Accounts couldn't be read; folders keep the names the last read gave"
            );
            kept.unwrap_or_default()
        }
    }
}

/// A folder's Account's name, the Accounts read now ([`folder_accounts`]);
/// a folder no Account lists keeps `own`.
#[must_use]
pub fn account_name_of(state: &Arc<AppState>, folder: &str, own: &str) -> String {
    folder_name(&folder_accounts(state), folder, own)
}

/// What a limit, reset or renewal calls folder `folder` (D-62: they are
/// its Account's): the Account's name, from `accounts` ([`by_folder`]);
/// a folder no Account lists keeps `own`, its own name.
#[must_use]
pub fn folder_name(accounts: &BTreeMap<String, FolderAccount>, folder: &str, own: &str) -> String {
    accounts
        .get(folder)
        .map_or_else(|| own.to_owned(), |account| account.name.clone())
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
/// [`AppError::Refused`](crate::error::AppError::Refused) for an Account that isn't stored, or a name
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

/// What [`store_accounts`] found.
struct Links {
    /// The stored Account by folder (a kept one too).
    accounts: BTreeMap<String, String>,
    /// The folders whose sign-in couldn't be checked, kept in the Account
    /// they were last read as.
    kept: std::collections::BTreeSet<String>,
    /// The open sessions a change of sign-in moved between pools (told
    /// once the Accounts are named).
    moved: Vec<crate::features::pools::sign_in::Moved>,
}

/// D-62 (AJ-E3202): each profile's sign-in stored as its Account (made
/// the first time it's read), and the folder pointed at it. Only a
/// definite answer moves a folder: not signed in points it at none, a
/// sign-in as another Account at that one. A sign-in that couldn't be
/// checked (`Unverifiable`: a file mid-write, a timeout, a tool not
/// installed) leaves the folder where it was (0.4.1): kept in the Account
/// it was last read as, or in none if it never had one (a Codex API key).
fn store_accounts(state: &Arc<AppState>, rows: &[ProfileRead]) -> Links {
    let mut out = Links {
        accounts: BTreeMap::new(),
        kept: std::collections::BTreeSet::new(),
        moved: Vec::new(),
    };
    let Ok(storage) = lock_storage_arc(state) else {
        return out;
    };
    let linked: BTreeMap<String, Option<String>> = if rows.iter().any(unsure) {
        match storage.subscriptions().folders() {
            Ok(folders) => folders.into_iter().collect(),
            Err(error) => {
                tracing::warn!(%error, "the folders' Accounts weren't read; each unchecked one stays as it is");
                BTreeMap::new()
            }
        }
    } else {
        BTreeMap::new()
    };
    let now = crate::commands::now_millis();
    for row in rows {
        let folder = row.profile.id.clone();
        if unsure(row) {
            // Couldn't tell: never a sign-out. The stored link stays.
            if let Some(Some(account)) = linked.get(&folder) {
                out.accounts.insert(folder.clone(), account.clone());
                out.kept.insert(folder);
            }
            continue;
        }
        let account = match sign_in(state, row) {
            None => None,
            Some((vendor, identity)) => {
                match storage.subscriptions().upsert(&vendor, &identity, now) {
                    Ok(account) => Some(account),
                    // A sign-in that couldn't be stored isn't a sign-out:
                    // the folder keeps the Account it points at (its
                    // sessions stay in their pools) until a read stores it.
                    Err(error) => {
                        tracing::warn!(%error, folder, "a folder's sign-in wasn't stored");
                        continue;
                    }
                }
            }
        };
        if let Ok(Some(moves)) =
            crate::features::pools::sign_in::set_folder(&storage, &folder, account.as_deref())
        {
            out.moved.extend(moves);
            // AJ-E3102: the folder's sessions count on another Account now.
            crate::features::relations::changed(
                state,
                crate::features::relations::Change::AccountsChanged,
            );
        }
        if let Some(account) = account {
            out.accounts.insert(folder, account);
        }
    }
    out
}

/// A sign-in this read couldn't tell (not a service's key, which is
/// there or not).
fn unsure(row: &ProfileRead) -> bool {
    row.service.is_none() && matches!(row.probe, IdentityProbe::Unverifiable { .. })
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
                unchecked: None,
                accepted_name: None,
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

    fn unchecked(id: &str, reason: &str) -> ProfileRead {
        let mut read = row(
            id,
            "claude-code",
            IdentityProbe::Unverifiable {
                reason: reason.into(),
            },
        );
        read.profile.unchecked = Some(reason.to_owned());
        read
    }

    /// 0.4.1: a sign-in that couldn't be checked stays in the Account it
    /// was linked to, before or after the folders signed in to it now, and
    /// the Account still says who it is.
    #[test]
    fn a_folder_that_couldnt_be_checked_stays_in_its_account() {
        let linked: BTreeMap<String, String> = [
            ("work".to_owned(), "acct:anthropic:1".to_owned()),
            ("busy".to_owned(), "acct:anthropic:1".to_owned()),
            ("alone".to_owned(), "acct:anthropic:2".to_owned()),
        ]
        .into();
        let accounts = group_linked(
            vec![
                unchecked("busy", "mid-write"),
                row("work", "claude-code", signed("a", None, "o…@work.com")),
                unchecked("alone", "timed out"),
                unchecked("never", "API key"),
            ],
            &linked,
        );
        assert_eq!(accounts.len(), 3, "{accounts:?}");
        assert_eq!(ids(&accounts[0]), ["busy", "work"]);
        assert_eq!(accounts[0].state, "signed_in");
        assert_eq!(accounts[0].email.as_deref(), Some("o…@work.com"));
        assert_eq!(ids(&accounts[1]), ["alone"]);
        assert_eq!(accounts[1].state, "signed_in", "its Account, not «unknown»");
        assert_eq!(accounts[1].reason, None);
        // Never linked (a Codex API key): it stands alone, as before.
        assert_eq!(ids(&accounts[2]), ["never"]);
        assert_eq!(accounts[2].state, "unverified");
        // Without a link it's grouped as it always was.
        assert_eq!(group(vec![unchecked("busy", "x")])[0].state, "unverified");
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

    /// A signed-in Account as the list hands it, before it's named.
    fn named_row(key: &str, extra: impl FnOnce(&mut SubscriptionInfo)) -> SubscriptionInfo {
        let mut account = group(vec![row(
            key,
            "claude-code",
            signed(key, None, "o…@work.com"),
        )])
        .remove(0);
        account.key = key.to_owned();
        account.plan = Some("Max".to_owned());
        extra(&mut account);
        account
    }

    fn tool_words(tool: &str) -> String {
        match tool {
            "claude-code" => "Claude".to_owned(),
            "codex" => "Codex".to_owned(),
            other => other.to_owned(),
        }
    }

    fn names(mut rows: Vec<SubscriptionInfo>) -> Vec<String> {
        name_all(&mut rows, tool_words);
        rows.into_iter().map(|row| row.display_name).collect()
    }

    /// D-62 (AJ-E3202 req. 3; AJ-E3205), moved here from the UI: your
    /// name, else its team, else its tool and plan, else the one name all
    /// its Apps were accepted under at import, else its email. A folder's
    /// name the user never accepted never names it.
    #[test]
    fn an_account_is_named_its_own_way_never_by_a_folder() {
        assert_eq!(
            names(vec![named_row("t", |_| {})]),
            ["Claude Max"],
            "its tool and plan"
        );
        assert_eq!(
            names(vec![named_row("t", |row| row.plan = Some("max-20x".into()))]),
            ["Claude Max 20x"]
        );
        assert_eq!(
            names(vec![named_row("me", |row| {
                row.plan = None;
                row.email = Some("o…@me.com".into());
            })]),
            ["o…@me.com"],
            "no plan: its email"
        );
        assert_eq!(
            names(vec![named_row("t", |row| {
                row.plan = None;
                row.email = None;
            })]),
            ["Claude account"],
            "nothing: the tool's"
        );
        assert_eq!(
            names(vec![named_row("w", |row| row.name = Some("Day job".into()))]),
            ["Day job"],
            "named by you, whatever its folders are called"
        );
        assert_eq!(
            names(vec![named_row("w", |row| row.name = Some("  ".into()))]),
            ["Claude Max"],
            "a blank name is its default"
        );
        assert_eq!(
            names(vec![named_row("u", |row| row.state = "unverified".into())]),
            ["Account unknown"]
        );
        assert_eq!(
            names(vec![named_row("o", |row| row.state = "signed_out".into())]),
            ["Not signed in"]
        );
        let folder_named = named_row("f", |_| {});
        assert_eq!(folder_named.profiles[0].label, "Account f");
        assert_eq!(names(vec![folder_named]), ["Claude Max"]);
        // A label the user never accepted (no plan, no team): its email.
        let unaccepted = named_row("f", |row| {
            row.plan = None;
            row.profiles[0].label = "Work".into();
        });
        assert_eq!(names(vec![unaccepted]), ["o…@work.com"]);
        assert_eq!(
            names(vec![accepted_as(&["Work"], |_| {})]),
            ["Work"],
            "accepted at import: that name"
        );
    }

    /// An Account with no team and no plan, its folders' names as accepted
    /// at import (`None`: one added another way).
    fn accepted_as(
        accepted: &[&str],
        extra: impl FnOnce(&mut SubscriptionInfo),
    ) -> SubscriptionInfo {
        named_row("acc", |row| {
            row.plan = None;
            row.profiles = accepted
                .iter()
                .enumerate()
                .map(|(at, name)| SubscriptionProfileInfo {
                    id: format!("f{at}"),
                    label: format!("Folder {at}"),
                    accepted_name: (!name.is_empty()).then(|| (*name).to_owned()),
                    ..row.profiles[0].clone()
                })
                .collect();
            extra(row);
        })
    }

    /// AJ-E3202 req. 3 (0.4.1): every App accepted under one name at
    /// import ("Work", from first run's found step): the Account takes it.
    #[test]
    fn an_account_whose_apps_were_all_accepted_under_one_name_takes_it() {
        assert_eq!(names(vec![accepted_as(&["Work"], |_| {})]), ["Work"]);
        assert_eq!(
            names(vec![accepted_as(&["Work", " Work "], |_| {})]),
            ["Work"],
            "two folders, one name"
        );
    }

    #[test]
    fn your_name_wins_over_the_name_accepted_at_import() {
        assert_eq!(names(vec![accepted_as(&["Work"], |_| {})]), ["Work"]);
        assert_eq!(
            names(vec![
                accepted_as(&["Work"], |row| row.name = Some("Day job".into()))
            ]),
            ["Day job"]
        );
    }

    #[test]
    fn apps_accepted_under_different_names_fall_back_to_the_email() {
        assert_eq!(
            names(vec![accepted_as(&["Work", "Work"], |_| {})]),
            ["Work"]
        );
        assert_eq!(
            names(vec![accepted_as(&["Work", "Home"], |_| {})]),
            ["o…@work.com"]
        );
    }

    /// One App not brought in by import (made in AI Juggler, a desktop
    /// profile's folder, a desktop App on one of its folders): the order
    /// it always had.
    #[test]
    fn an_app_not_accepted_at_import_falls_back() {
        assert_eq!(names(vec![accepted_as(&["Work"], |_| {})]), ["Work"]);
        assert_eq!(
            names(vec![accepted_as(&["Work", ""], |_| {})]),
            ["o…@work.com"],
            "a folder added another way"
        );
        assert_eq!(
            names(vec![accepted_as(&["Work"], |row| row.desktop_apps = true)]),
            ["o…@work.com"],
            "a desktop App on its folder"
        );
        assert_eq!(
            names(vec![accepted_as(&["Work"], |row| row.email = None)]),
            ["Work"]
        );
        assert_eq!(
            names(vec![accepted_as(&[""], |row| row.email = None)]),
            ["Claude account"]
        );
    }

    #[test]
    fn a_team_or_a_plan_names_it_before_the_name_accepted_at_import() {
        assert_eq!(names(vec![accepted_as(&["Work"], |_| {})]), ["Work"]);
        assert_eq!(
            names(vec![
                accepted_as(&["Work"], |row| row.organisation = Some("Acme".into()))
            ]),
            ["Acme"]
        );
        assert_eq!(
            names(vec![
                accepted_as(&["Work"], |row| row.plan = Some("max-20x".into()))
            ]),
            ["Claude Max 20x"]
        );
    }

    /// Two Accounts accepted under one name are twins, told apart by email.
    #[test]
    fn two_accounts_accepted_under_one_name_are_told_apart() {
        let other = accepted_as(&["Work"], |row| {
            row.key = "acc-2".into();
            row.email = Some("s…@home.com".into());
        });
        assert_eq!(
            names(vec![accepted_as(&["Work"], |_| {}), other]),
            ["Work · o…@work.com", "Work · s…@home.com"]
        );
    }

    #[test]
    fn an_account_through_a_service_is_named_for_it() {
        let accounts = group(vec![through("go", "claude-code", "svc-1", true)]);
        let service = accounts[0].service.as_ref().unwrap().name.clone();
        assert_eq!(names(accounts), [service]);
    }

    /// The 0.2 walk (F3): five seats in one team are told apart by their
    /// email, never five "Acme"s; twins on one masked email by their tool,
    /// else by their place among them.
    #[test]
    fn two_accounts_that_would_read_the_same_are_told_apart() {
        let seats: Vec<SubscriptionInfo> = ["a", "b", "c"]
            .into_iter()
            .map(|id| {
                named_row(id, |row| {
                    row.organisation = Some("Acme".into());
                    row.email = Some(format!("{id}…@acme.com"));
                })
            })
            .collect();
        assert_eq!(
            names(seats.clone()),
            [
                "Acme · a…@acme.com",
                "Acme · b…@acme.com",
                "Acme · c…@acme.com"
            ]
        );
        assert_eq!(
            names(vec![seats[0].clone()]),
            ["Acme"],
            "one alone: the team"
        );
        let masked: Vec<SubscriptionInfo> = ["x1", "x2"]
            .into_iter()
            .map(|id| {
                named_row(id, |row| {
                    row.email = Some("c…@example.com".into());
                    row.plan = None;
                    row.account_id = Some(format!("acct:{id}"));
                })
            })
            .collect();
        assert_eq!(
            names(vec![masked[1].clone(), masked[0].clone()]),
            ["c…@example.com (2)", "c…@example.com (1)"],
            "by their place among them, by id"
        );
        let mut codex = masked[1].clone();
        codex.tool = "codex".into();
        assert_eq!(
            names(vec![masked[0].clone(), codex]),
            ["Claude · c…@example.com", "Codex · c…@example.com"]
        );
        // One you named isn't a twin: the other reads its plain name.
        let mut named = seats[1].clone();
        named.name = Some("Side".into());
        assert_eq!(names(vec![seats[0].clone(), named]), ["Acme", "Side"]);
    }

    #[test]
    fn a_plan_is_said_in_words() {
        assert_eq!(plan_words("max-20x"), "Max 20x");
        assert_eq!(plan_words("pro"), "Pro");
        assert_eq!(plan_words("team_premium  seat"), "Team Premium Seat");
        assert_eq!(plan_words(""), "");
    }

    #[test]
    fn each_folder_on_a_stored_account_maps_to_it_and_others_stand_alone() {
        let mut accounts = group(vec![
            row("work", "claude-code", signed("a", None, "o…@work.com")),
            row("work-2", "claude-code", signed("a", None, "o…@work.com")),
            row("out", "claude-code", IdentityProbe::SignedOut),
        ]);
        accounts[0].account_id = Some("acct:anthropic:1".into());
        accounts[0].plan = Some("max-20x".into());
        name_all(&mut accounts, tool_words);
        let map = by_folder(&accounts);
        assert_eq!(map.len(), 2, "the folder not signed in stands alone");
        let work = &map["work"];
        assert_eq!(work.id, "acct:anthropic:1");
        assert_eq!(work.name, "Claude Max 20x");
        assert_eq!(work.folders, ["work", "work-2"]);
        assert_eq!(map["work-2"], *work);
        assert!(!map.contains_key("out"));
        assert_eq!(folder_name(&map, "work-2", "Work Max"), "Claude Max 20x");
        assert_eq!(
            folder_name(&map, "out", "Spare"),
            "Spare",
            "a folder no Account lists keeps its own name"
        );
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
