//! Claude's limits from its own records, for sessions AI Juggler didn't
//! start (the maintainer's third report). The usage pipeline calls this
//! for each Claude profile; the formats and what is read are in
//! `juggler_adapters::claude_limits`. Here: which app instances to look
//! in, and reading each record only when it changed.
//!
//! - App instances: the tool's own app's default folder, those running now
//!   (by their `--user-data-dir`, from the process list every few minutes,
//!   never an environment) and those seen before, kept while their folder
//!   is there.
//! - Each record is read again only when its file changed, and only what
//!   is newer than the last reading kept comes in.
//! - Transcripts read before limit hits were kept are looked through once
//!   (the last week's), for the hits they hold.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use juggler_adapters::claude_limits;
use juggler_core::quota::{QuotaSample, UsageSource, WindowKind};

/// Samples gathered in a pass, with where each came from: the all-models
/// windows', and the weeks on one model only (by the model's name, as the
/// tool shows it), each a limit of its own.
#[derive(Debug, Default)]
pub(crate) struct SourcedBatch {
    /// `(account, window, sample, source)`.
    pub windows: Vec<(String, WindowKind, QuotaSample, UsageSource)>,
    /// `(account, model, sample, source)`.
    pub model_weeks: Vec<(String, String, QuotaSample, UsageSource)>,
    /// AJ-E3210 (D-135): readings that carry the Account they were read
    /// under (Claude's own records: a folder a Claude app profile shares
    /// with its CLI holds two sign-ins' readings).
    pub signed: juggler_core::storage::SignedSamples,
}

/// AJ-E3210: the Accounts a folder's Apps are on: its CLI's, and each
/// desktop App's on it (their own sign-ins, D-63 as amended).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FolderSignIns {
    /// Whose sign-ins these are (`anthropic`): an id is made as the stored
    /// Account's.
    pub vendor: String,
    /// The folder's CLI App's Account.
    pub cli: Option<String>,
    /// The desktop Apps' on it.
    pub desktop: Vec<String>,
}

impl FolderSignIns {
    /// Every Account an App on the folder is on.
    fn all(&self) -> impl Iterator<Item = &String> {
        self.cli.iter().chain(&self.desktop)
    }

    /// The Account a limit hit in a transcript there is: a run the
    /// Claude app made (its last `entrypoint`), its desktop App's when
    /// that's one Account; a terminal's, the CLI's.
    fn of_run(&self, transcript: &Path) -> RunOf {
        if self.desktop.is_empty() {
            return RunOf::Theirs(self.cli.clone());
        }
        let by_app =
            juggler_adapters::claude_sign_ins::last_entrypoint(transcript).is_some_and(|entry| {
                juggler_adapters::claude_sign_ins::kind_of(&entry)
                    == juggler_adapter::SignInWriter::DesktopApp
            });
        if !by_app {
            return RunOf::Theirs(self.cli.clone());
        }
        let mut theirs: Vec<&String> = self.desktop.iter().collect();
        theirs.dedup();
        match theirs.as_slice() {
            [one] => RunOf::Theirs(Some((*one).clone())),
            _ => RunOf::Unknown,
        }
    }
}

/// Whose a run (and a limit hit in it) was.
#[derive(Clone, Debug, PartialEq, Eq)]
enum RunOf {
    /// This Account's (`None`: the folder's, which is on none).
    Theirs(Option<String>),
    /// No one can say (two desktop Apps on two Accounts share the folder):
    /// left out, never a wrong Out.
    Unknown,
}

/// Each folder's [`FolderSignIns`], read once a pass when first asked
/// (`None`: a test's, none).
pub(crate) struct SignInsByFolder<'a> {
    state: Option<&'a crate::state::AppState>,
    read: std::cell::OnceCell<BTreeMap<String, FolderSignIns>>,
}

impl<'a> SignInsByFolder<'a> {
    /// For a pass.
    pub(crate) fn new(state: &'a crate::state::AppState) -> Self {
        Self {
            state: Some(state),
            read: std::cell::OnceCell::new(),
        }
    }

    /// None known (every reading untagged, as before V55).
    #[cfg(test)]
    pub(crate) fn none() -> Self {
        Self {
            state: None,
            read: std::cell::OnceCell::new(),
        }
    }

    /// Folder `folder`'s, when it has an App on an Account.
    pub(crate) fn of(&self, folder: &str) -> Option<&FolderSignIns> {
        self.read
            .get_or_init(|| self.state.map(read_sign_ins).unwrap_or_default())
            .get(folder)
    }
}

/// Every folder's [`FolderSignIns`], from the stored Apps.
fn read_sign_ins(state: &crate::state::AppState) -> BTreeMap<String, FolderSignIns> {
    let Ok(storage) = state.storage.lock() else {
        return BTreeMap::new();
    };
    let (Ok(folders), Ok(apps), Ok(own), Ok(rows)) = (
        storage.subscriptions().folders(),
        storage.apps().list(),
        storage.apps().accounts(),
        storage.accounts().list(),
    ) else {
        return BTreeMap::new();
    };
    let vendors: BTreeMap<String, String> = rows
        .iter()
        .map(|row| {
            let vendor = state.registry.get(&row.harness).map_or_else(
                || row.harness.to_string(),
                |adapter| adapter.sign_in_vendor(),
            );
            (row.id.to_string(), vendor)
        })
        .collect();
    let mut out: BTreeMap<String, FolderSignIns> = folders
        .into_iter()
        .map(|(folder, cli)| {
            let vendor = vendors.get(&folder).cloned().unwrap_or_default();
            (
                folder,
                FolderSignIns {
                    vendor,
                    cli,
                    desktop: Vec::new(),
                },
            )
        })
        .collect();
    for app in apps
        .iter()
        .filter(|app| app.kind == "desktop" && !app.hidden)
    {
        if let (Some(folder), Some(Some(account))) = (&app.folder_id, own.get(&app.id))
            && let Some(entry) = out.get_mut(folder)
            && !entry.desktop.contains(account)
        {
            entry.desktop.push(account.clone());
        }
    }
    out
}

/// How often (passes, ~1 s each) the records' files are looked at.
pub(crate) const RECORDS_EVERY: u32 = 15;
/// How often (passes) the running app instances are listed.
const INSTANCES_EVERY: u32 = 300;
/// The most app instances remembered.
const INSTANCES_MAX: usize = 32;
/// How far back the one-time look for limit hits goes: a week, the longest
/// window a hit can still describe.
const HITS_LOOKBACK_MS: i64 = juggler_core::quota::WEEK_MS;

/// One record file: when it last changed, and the newest reading kept.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Mark {
    /// The file's modification time when last read, epoch ms.
    pub modified_ms: i64,
    /// The newest reading taken from it, epoch ms.
    pub newest_ms: i64,
}

/// Per profile: where its records were read up to.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct RecordsState {
    /// Per app instance folder: its meter history.
    #[serde(default)]
    pub app: BTreeMap<String, Mark>,
    /// AJ-E3210: per app instance folder and Account (`<folder>#<id>`):
    /// the newest of its meter's readings taken for that Account.
    #[serde(default)]
    pub app_signed: BTreeMap<String, i64>,
    /// The profile's state file (Claude Code's last `/usage`).
    #[serde(default)]
    pub cache: Mark,
    /// The one-time look back for limit hits is done.
    #[serde(default)]
    pub hits_looked_back: bool,
}

/// Asked to read the records at the next pass rather than at their
/// cadence (0.4.4: "Measure its limits" asked the tool itself, whose
/// answer is in its own state file now, to show where the button was).
static READ_SOON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Reads the records at the next pass (about a second).
pub(crate) fn read_soon() {
    READ_SOON.store(true, std::sync::atomic::Ordering::Release);
}

/// Whether the records will be read at pass `tick` (the usage pipeline's
/// gate asks before the pass, which [`due`] then answers for good).
pub(crate) fn will_be_due(tick: u32) -> bool {
    READ_SOON.load(std::sync::atomic::Ordering::Acquire)
        || tick.is_multiple_of(RECORDS_EVERY)
        || tick == 1
}

/// Whether the records are due at pass `tick`: every [`RECORDS_EVERY`]
/// passes, the first, and the one after [`read_soon`].
fn due(tick: u32) -> bool {
    READ_SOON.swap(false, std::sync::atomic::Ordering::AcqRel)
        || tick.is_multiple_of(RECORDS_EVERY)
        || tick == 1
}

/// The app instance folders to look in when the records are due this pass
/// ([`due`]); `None` otherwise.
pub(crate) fn due_instances(
    state: &crate::state::AppState,
    known: &mut BTreeSet<String>,
    tick: u32,
) -> Option<Vec<PathBuf>> {
    due(tick).then(|| app_instances(&state.registry, known, tick))
}

/// The app instance folders to look in: the tools' own apps' default
/// folders, those running (listed every [`INSTANCES_EVERY`] passes) and
/// those seen before (`known`, updated), each only while it is there.
pub(crate) fn app_instances(
    registry: &juggler_adapter::AdapterRegistry,
    known: &mut BTreeSet<String>,
    tick: u32,
) -> Vec<PathBuf> {
    let Some(home) = juggler_platform::user::home() else {
        return Vec::new();
    };
    let apps: Vec<juggler_adapter::live::HostApp> = registry
        .ids()
        .iter()
        .filter_map(|id| registry.get(id))
        .flat_map(|adapter| adapter.host_apps())
        .collect();
    for app in &apps {
        if let Some(default) = &app.default_instance {
            known.insert(home.join(default).to_string_lossy().into_owned());
        }
    }
    if tick.is_multiple_of(INSTANCES_EVERY) || tick == 1 {
        let hosts = crate::features::conversations::places::HostRegistry::new(apps);
        let table = crate::features::conversations::procs::table();
        for (_, dir) in
            crate::features::conversations::places::running_instances(&table, &hosts, &home)
        {
            known.insert(dir.to_string_lossy().into_owned());
        }
    }
    known.retain(|dir| Path::new(dir).is_dir());
    while known.len() > INSTANCES_MAX {
        let Some(first) = known.iter().next().cloned() else {
            break;
        };
        known.remove(&first);
    }
    known.iter().map(PathBuf::from).collect()
}

fn modified_ms(path: &Path) -> Option<i64> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    let ms = modified
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis();
    i64::try_from(ms).ok()
}

/// Reads one Claude profile's records into `out`: Claude Code's last
/// `/usage` and the app instances' meters, each only when its file
/// changed and only what is newer than `state` kept.
///
/// AJ-E3210 (D-135): each reading carries the Account it was read under:
/// the `/usage` cache the state file's sign-in's; a meter's, each Account
/// an App on the folder is on (`sign_ins`) or the state file names, read
/// for each (the instance's record folder names hashed as the stored
/// Account's id: nothing new is read).
pub(crate) fn read_claude_records(
    account: &str,
    profile_dir: &Path,
    instances: &[PathBuf],
    sign_ins: Option<&FolderSignIns>,
    state: &mut RecordsState,
    out: &mut SourcedBatch,
) {
    let state_file = claude_limits::profile_state_file(profile_dir);
    let Some(changed) = modified_ms(&state_file) else {
        return;
    };
    // The sign-in comes from the same file: read it when the file or any
    // meter changed (a meter changes about every 15 minutes).
    let meters: Vec<(PathBuf, String, i64)> = instances
        .iter()
        .filter_map(|dir| {
            let file = dir.join(claude_limits::APP_USAGE_FILE);
            let modified = modified_ms(&file)?;
            let key = dir.to_string_lossy().into_owned();
            let seen = state.app.get(&key).map_or(0, |mark| mark.modified_ms);
            (modified != seen).then_some((dir.clone(), key, modified))
        })
        .collect();
    if changed == state.cache.modified_ms && meters.is_empty() {
        return;
    }
    let Some(limits) = claude_limits::profile_limits(profile_dir) else {
        return;
    };
    let vendor = sign_ins.map_or("anthropic", |known| known.vendor.as_str());
    let id_of = |sign_in: &claude_limits::SignIn| {
        crate::features::subscriptions::app_accounts::account_id_of(
            vendor,
            &sign_in.account,
            Some(&sign_in.org),
        )
    };
    let read_as = limits.sign_in.as_ref().map(id_of);
    if changed != state.cache.modified_ms {
        state.cache.modified_ms = changed;
        if let Some(cache) = limits
            .cache
            .filter(|cache| cache.fetched_at > state.cache.newest_ms)
        {
            state.cache.newest_ms = cache.fetched_at;
            out.signed
                .windows
                .extend(cache.samples.into_iter().map(|(kind, sample)| {
                    let (who, source) = (read_as.clone(), UsageSource::UsageCache);
                    (account.to_owned(), kind, sample, source, who)
                }));
            out.signed
                .model_weeks
                .extend(cache.model_weeks.into_iter().map(|(model, sample)| {
                    let (who, source) = (read_as.clone(), UsageSource::UsageCache);
                    (account.to_owned(), model, sample, source, who)
                }));
        }
    }
    // Every Account an App here is on, and the one the state file names.
    let wanted: BTreeSet<String> = sign_ins
        .into_iter()
        .flat_map(FolderSignIns::all)
        .cloned()
        .chain(read_as)
        .collect();
    if wanted.is_empty() {
        return;
    }
    for (dir, key, modified) in meters {
        let mark = state.app.entry(key.clone()).or_default();
        mark.modified_ms = modified;
        for sign_in in claude_limits::instance_sign_ins(&dir) {
            let id = id_of(&sign_in);
            if !wanted.contains(&id) {
                continue;
            }
            let newest = state.app_signed.entry(format!("{key}#{id}")).or_default();
            let samples = claude_limits::app_usage_samples(&dir, &sign_in, *newest);
            if let Some(at) = samples.iter().map(|(_, sample)| sample.ts).max() {
                *newest = at;
            }
            out.signed
                .windows
                .extend(samples.into_iter().map(|(kind, sample)| {
                    let source = UsageSource::DesktopApp;
                    (account.to_owned(), kind, sample, source, Some(id.clone()))
                }));
        }
    }
}

/// AJ-E3210: a limit hit read in transcript `transcript`, spent on folder
/// `on`: the Account whose run it was ([`FolderSignIns::of_run`]) where
/// the folder's Apps are known; left out when that can't be said (never a
/// wrong Out on the other Account); as before where no App is known.
pub(crate) fn push_hit(
    out: &mut SourcedBatch,
    (on, kind, sample): (String, WindowKind, QuotaSample),
    transcript: &Path,
    sign_ins: &SignInsByFolder<'_>,
) {
    match sign_ins.of(&on).map(|known| known.of_run(transcript)) {
        None => out.windows.push((on, kind, sample, UsageSource::ApiRetry)),
        Some(RunOf::Unknown) => {}
        Some(RunOf::Theirs(who)) => {
            out.signed
                .windows
                .push((on, kind, sample, UsageSource::ApiRetry, who));
        }
    }
}

/// Once per profile: the limit hits in the last week's transcripts, which
/// a pass read before hits were kept (`files`: the profile's transcripts),
/// each the folder's it was spent on (`spent`: a moved conversation's, the
/// one it left).
pub(crate) fn look_back_for_hits(
    (account, spent, sign_ins): (
        &str,
        &crate::features::handover::moved_lines::SpentOn<'_>,
        &SignInsByFolder<'_>,
    ),
    files: &[PathBuf],
    now_ms: i64,
    state: &mut RecordsState,
    out: &mut SourcedBatch,
) {
    if state.hits_looked_back {
        return;
    }
    state.hits_looked_back = true;
    for path in files {
        if modified_ms(path).is_none_or(|at| now_ms.saturating_sub(at) > HITS_LOOKBACK_MS) {
            continue;
        }
        for (kind, sample) in claude_limits::limit_hits_in(path)
            .into_iter()
            .filter(|(_, sample)| now_ms.saturating_sub(sample.ts) <= HITS_LOOKBACK_MS)
        {
            if let Some(on) = spent.folder(account, path, sample.ts) {
                push_hit(out, (on, kind, sample), path, sign_ins);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0.4.4: after the tool was asked ("Measure its limits"), the records
    /// are read at the next pass, once; otherwise at their cadence.
    #[test]
    fn asked_now_the_records_are_read_at_the_next_pass_once() {
        assert!(due(1) && due(RECORDS_EVERY) && !due(2));
        read_soon();
        assert!(due(2), "the next pass reads them");
        assert!(!due(3), "once");
    }

    const T0: i64 = 1_792_000_000_000;

    fn profile(dir: &Path, cache_at: i64) {
        std::fs::write(
            dir.join(".claude.json"),
            format!(
                r#"{{"oauthAccount":{{"accountUuid":"acct-a","organizationUuid":"org-a"}},
                "cachedUsageUtilization":{{"fetchedAtMs":{cache_at},"accountUuid":"acct-a",
                "utilization":{{"five_hour":{{"utilization":40,"resets_at":"2026-10-25T19:00:00+00:00"}},"seven_day":null,
                "limits":[{{"kind":"weekly_scoped","percent":3,"resets_at":"2026-10-29T20:00:00+00:00","scope":{{"model":{{"display_name":"Fable"}}}}}}]}}}}}}"#
            ),
        )
        .unwrap();
    }

    fn instance(dir: &Path, samples: &str) {
        std::fs::create_dir_all(dir.join("claude-code-sessions/acct-a/org-a")).unwrap();
        std::fs::write(
            dir.join(claude_limits::APP_USAGE_FILE),
            format!(r#"{{"version":1,"samples":[{samples}]}}"#),
        )
        .unwrap();
    }

    #[test]
    fn records_come_in_once_each_and_again_only_when_newer() {
        let profile_dir = tempfile::tempdir().unwrap();
        let app = tempfile::tempdir().unwrap();
        profile(profile_dir.path(), T0);
        instance(
            app.path(),
            &format!(r#"{{"org":"org-a","t":{T0},"u":{{"fh":12}}}}"#),
        );
        let instances = vec![app.path().to_path_buf()];
        let mut state = RecordsState::default();
        let mut out = SourcedBatch::default();
        read_claude_records(
            "acc",
            profile_dir.path(),
            &instances,
            None,
            &mut state,
            &mut out,
        );
        assert_eq!(
            out.signed.windows.len(),
            2,
            "the /usage reading and the meter's sample"
        );
        assert!(
            out.signed
                .windows
                .iter()
                .any(|row| row.3 == UsageSource::UsageCache)
        );
        assert!(
            out.signed
                .windows
                .iter()
                .any(|row| row.3 == UsageSource::DesktopApp)
        );
        assert_eq!(
            out.signed
                .model_weeks
                .iter()
                .map(|row| (row.1.as_str(), row.3))
                .collect::<Vec<_>>(),
            [("Fable", UsageSource::UsageCache)],
            "the model's own week comes with the /usage reading"
        );

        // Nothing changed: nothing is read again.
        out = SourcedBatch::default();
        read_claude_records(
            "acc",
            profile_dir.path(),
            &instances,
            None,
            &mut state,
            &mut out,
        );
        assert!(out.signed.windows.is_empty() && out.signed.model_weeks.is_empty());

        // The meter grows: only its new sample comes.
        instance(
            app.path(),
            &format!(
                r#"{{"org":"org-a","t":{T0},"u":{{"fh":12}}}},{{"org":"org-a","t":{},"u":{{"fh":15}}}}"#,
                T0 + 900_000
            ),
        );
        state.app.values_mut().for_each(|mark| mark.modified_ms = 0); // as if it changed
        read_claude_records(
            "acc",
            profile_dir.path(),
            &instances,
            None,
            &mut state,
            &mut out,
        );
        assert_eq!(out.signed.windows.len(), 1);
        assert_eq!(out.signed.windows[0].2.used_pct, 15.0);
        assert!(
            out.signed.model_weeks.is_empty(),
            "the /usage reading didn't change"
        );
    }

    /// AJ-E3210 acceptance test 5: the folder a Claude app profile shares
    /// with its CLI. Its state file holds the app's sign-in (the personal
    /// plan) while the CLI's is the Team seat's: the meter's readings of
    /// each are read, each tagged with its own Account, never one for the
    /// other.
    #[test]
    fn a_shared_folders_readings_carry_the_account_they_were_read_under() {
        use crate::features::subscriptions::app_accounts::account_id_of;
        let profile_dir = tempfile::tempdir().unwrap();
        let app = tempfile::tempdir().unwrap();
        std::fs::write(
            profile_dir.path().join(".claude.json"),
            r#"{"oauthAccount":{"accountUuid":"acct-a","organizationUuid":"org-me"}}"#,
        )
        .unwrap();
        // One instance's meter holds both sign-ins' readings (the default
        // app on the Team seat, the profile on the personal plan).
        std::fs::create_dir_all(app.path().join("claude-code-sessions/acct-a/org-me")).unwrap();
        std::fs::create_dir_all(app.path().join("claude-code-sessions/acct-a/org-team")).unwrap();
        std::fs::write(
            app.path().join(claude_limits::APP_USAGE_FILE),
            format!(
                r#"{{"version":1,"samples":[{{"org":"org-team","t":{T0},"u":{{"sd":7}}}},{{"org":"org-me","t":{},"u":{{"sd":100}}}}]}}"#,
                T0 + 1
            ),
        )
        .unwrap();
        let team = account_id_of("anthropic", "acct-a", Some("org-team"));
        let me = account_id_of("anthropic", "acct-a", Some("org-me"));
        let sign_ins = FolderSignIns {
            vendor: "anthropic".into(),
            cli: Some(team.clone()),
            desktop: vec![me.clone()],
        };
        let mut state = RecordsState::default();
        let mut out = SourcedBatch::default();
        read_claude_records(
            "shared",
            profile_dir.path(),
            &[app.path().to_path_buf()],
            Some(&sign_ins),
            &mut state,
            &mut out,
        );
        let read: Vec<(f64, Option<&str>)> = out
            .signed
            .windows
            .iter()
            .map(|row| (row.2.used_pct, row.4.as_deref()))
            .collect();
        assert!(read.contains(&(7.0, Some(team.as_str()))), "{read:?}");
        assert!(read.contains(&(100.0, Some(me.as_str()))), "{read:?}");
        assert_eq!(read.len(), 2, "each once, under its own Account");
        assert!(out.windows.is_empty(), "nothing untagged");

        // A folder with no App on another Account reads only its own.
        let mut state = RecordsState::default();
        let mut out = SourcedBatch::default();
        let alone = FolderSignIns {
            vendor: "anthropic".into(),
            cli: Some(me.clone()),
            desktop: Vec::new(),
        };
        read_claude_records(
            "personal",
            profile_dir.path(),
            &[app.path().to_path_buf()],
            Some(&alone),
            &mut state,
            &mut out,
        );
        assert!(
            out.signed
                .windows
                .iter()
                .all(|row| row.4.as_deref() == Some(me.as_str()))
        );
    }

    #[test]
    fn the_look_back_runs_once_and_keeps_only_the_last_weeks_hits() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.jsonl");
        let line = |at: &str, reset_s: i64| {
            format!(
                r#"{{"type":"assistant","timestamp":"{at}","error":"rate_limit","quotaLimits":{{"status":"rejected","rateLimitType":"five_hour","resetsAt":{reset_s}}}}}"#
            )
        };
        let now = juggler_core::time::timestamp_ms("2026-10-25T12:00:00Z").unwrap();
        std::fs::write(
            &path,
            [
                line("2026-10-25T10:00:00Z", now / 1000 + 3600),
                line("2026-10-01T10:00:00Z", now / 1000 - 86_400 * 20),
            ]
            .join("\n"),
        )
        .unwrap();
        let mut state = RecordsState::default();
        let mut out = SourcedBatch::default();
        let spent = crate::features::handover::moved_lines::SpentOn::none();
        let sign_ins = SignInsByFolder::none();
        look_back_for_hits(
            ("acc", &spent, &sign_ins),
            std::slice::from_ref(&path),
            now,
            &mut state,
            &mut out,
        );
        assert_eq!(
            out.windows.len(),
            1,
            "a hit older than a week describes nothing now"
        );
        assert_eq!(out.windows[0].3, UsageSource::ApiRetry);
        out = SourcedBatch::default();
        look_back_for_hits(
            ("acc", &spent, &sign_ins),
            &[path],
            now,
            &mut state,
            &mut out,
        );
        assert!(out.windows.is_empty(), "once");
    }
}
