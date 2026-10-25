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

/// Samples gathered in a pass, with where each came from.
pub(crate) type SourcedBatch = Vec<(String, WindowKind, QuotaSample, UsageSource)>;

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
    /// The profile's state file (Claude Code's last `/usage`).
    #[serde(default)]
    pub cache: Mark,
    /// The one-time look back for limit hits is done.
    #[serde(default)]
    pub hits_looked_back: bool,
}

/// The app instance folders to look in when the records are due this pass
/// (every [`RECORDS_EVERY`] passes, and the first); `None` otherwise.
pub(crate) fn due_instances(
    state: &crate::state::AppState,
    known: &mut BTreeSet<String>,
    tick: u32,
) -> Option<Vec<PathBuf>> {
    (tick.is_multiple_of(RECORDS_EVERY) || tick == 1)
        .then(|| app_instances(&state.registry, known, tick))
}

/// The app instance folders to look in: the tools' own apps' default
/// folders, those running (listed every [`INSTANCES_EVERY`] passes) and
/// those seen before (`known`, updated), each only while it is there.
pub(crate) fn app_instances(
    registry: &juggler_adapter::AdapterRegistry,
    known: &mut BTreeSet<String>,
    tick: u32,
) -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
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
pub(crate) fn read_claude_records(
    account: &str,
    profile_dir: &Path,
    instances: &[PathBuf],
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
    if changed != state.cache.modified_ms {
        state.cache.modified_ms = changed;
        if let Some(cache) = limits
            .cache
            .filter(|cache| cache.fetched_at > state.cache.newest_ms)
        {
            state.cache.newest_ms = cache.fetched_at;
            out.extend(
                cache.samples.into_iter().map(|(kind, sample)| {
                    (account.to_owned(), kind, sample, UsageSource::UsageCache)
                }),
            );
        }
    }
    let Some(sign_in) = limits.sign_in else {
        return;
    };
    for (dir, key, modified) in meters {
        let mark = state.app.entry(key).or_default();
        mark.modified_ms = modified;
        let samples = claude_limits::app_usage_samples(&dir, &sign_in, mark.newest_ms);
        if let Some(newest) = samples.iter().map(|(_, sample)| sample.ts).max() {
            mark.newest_ms = newest;
        }
        out.extend(
            samples
                .into_iter()
                .map(|(kind, sample)| (account.to_owned(), kind, sample, UsageSource::DesktopApp)),
        );
    }
}

/// Once per profile: the limit hits in the last week's transcripts, which
/// a pass read before hits were kept (`files`: the profile's transcripts).
pub(crate) fn look_back_for_hits(
    account: &str,
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
        out.extend(
            claude_limits::limit_hits_in(path)
                .into_iter()
                .filter(|(_, sample)| now_ms.saturating_sub(sample.ts) <= HITS_LOOKBACK_MS)
                .map(|(kind, sample)| (account.to_owned(), kind, sample, UsageSource::ApiRetry)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_792_000_000_000;

    fn profile(dir: &Path, cache_at: i64) {
        std::fs::write(
            dir.join(".claude.json"),
            format!(
                r#"{{"oauthAccount":{{"accountUuid":"acct-a","organizationUuid":"org-a"}},
                "cachedUsageUtilization":{{"fetchedAtMs":{cache_at},"accountUuid":"acct-a",
                "utilization":{{"five_hour":{{"utilization":40,"resets_at":"2026-10-25T19:00:00+00:00"}},"seven_day":null}}}}}}"#
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
        let mut out = SourcedBatch::new();
        read_claude_records("acc", profile_dir.path(), &instances, &mut state, &mut out);
        assert_eq!(out.len(), 2, "the /usage reading and the meter's sample");
        assert!(out.iter().any(|row| row.3 == UsageSource::UsageCache));
        assert!(out.iter().any(|row| row.3 == UsageSource::DesktopApp));

        // Nothing changed: nothing is read again.
        out.clear();
        read_claude_records("acc", profile_dir.path(), &instances, &mut state, &mut out);
        assert!(out.is_empty());

        // The meter grows: only its new sample comes.
        instance(
            app.path(),
            &format!(
                r#"{{"org":"org-a","t":{T0},"u":{{"fh":12}}}},{{"org":"org-a","t":{},"u":{{"fh":15}}}}"#,
                T0 + 900_000
            ),
        );
        state.app.values_mut().for_each(|mark| mark.modified_ms = 0); // as if it changed
        read_claude_records("acc", profile_dir.path(), &instances, &mut state, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].2.used_pct, 15.0);
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
        let mut out = SourcedBatch::new();
        look_back_for_hits(
            "acc",
            std::slice::from_ref(&path),
            now,
            &mut state,
            &mut out,
        );
        assert_eq!(
            out.len(),
            1,
            "a hit older than a week describes nothing now"
        );
        assert_eq!(out[0].3, UsageSource::ApiRetry);
        out.clear();
        look_back_for_hits("acc", &[path], now, &mut state, &mut out);
        assert!(out.is_empty(), "once");
    }
}
