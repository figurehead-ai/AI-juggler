//! The maintainer's third report: Claude accounts whose sessions all run
//! in the Claude app showed "Not measured yet" for both windows. Their
//! limits now come from Claude's own records (the app's meter, Claude
//! Code's last `/usage`, limit hits in transcripts), through one ingest
//! pass, named with their source and age; a stale reading is history; a
//! team seat with no week says so. The ignored `qa_real_readonly_*` test
//! runs the same reads on the real setup, read-only, and prints only each
//! profile's states and ages.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::commands;
use juggler_app::state::AppState;
use juggler_app::usage_pipeline;

const MIN: i64 = 60_000;
const HOUR: i64 = 60 * MIN;

fn now() -> i64 {
    juggler_core::model::Timestamp::now().millis()
}

fn iso(ms: i64) -> String {
    // Days since the epoch to a civil date (UTC), for transcript lines.
    let secs = ms.div_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(mo <= 2);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

struct Setup {
    state: Arc<AppState>,
    account: String,
    profile: PathBuf,
    instance: PathBuf,
}

/// A Claude profile used where it is, signed in to `acct-a` in `org-team`,
/// and a Claude app instance whose Code sessions are that sign-in's.
fn setup(root: &Path) -> Setup {
    let profile = root.join("ClaudeCodeProfiles/team-a");
    std::fs::create_dir_all(profile.join("projects/-w-p")).unwrap();
    std::fs::write(
        profile.join(".claude.json"),
        r#"{"oauthAccount":{"accountUuid":"acct-a","organizationUuid":"org-team","emailAddress":"planted@example.invalid"}}"#,
    )
    .unwrap();
    let data = root.join("app-data");
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(
        ClaudeCodeAdapter::with_bin("/bin/true").with_overlays_root(data.join("overlays")),
    ));
    let state = Arc::new(AppState::init_with_registry(data, registry).unwrap());
    let account = commands::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        profile.to_str().unwrap(),
        "Team A",
        commands::ImportRequest {
            in_place: true,
            ..commands::ImportRequest::default()
        },
    )
    .unwrap()
    .account
    .id
    .to_string();
    let instance = root.join("ClaudeProfiles/team-a");
    std::fs::create_dir_all(instance.join("claude-code-sessions/acct-a/org-team")).unwrap();
    // The pipeline looks in the instances it has seen.
    state
        .storage
        .lock()
        .unwrap()
        .app_settings()
        .set(
            "usage_ingest_state",
            &serde_json::json!({ "app_instances": [instance.to_string_lossy()] }).to_string(),
        )
        .unwrap();
    Setup {
        state,
        account,
        profile,
        instance,
    }
}

fn meter(instance: &Path, samples: &[(i64, f64, Option<f64>)]) {
    let rows: Vec<String> = samples
        .iter()
        .map(|(t, fh, sd)| match sd {
            Some(sd) => format!(r#"{{"org":"org-team","t":{t},"u":{{"fh":{fh},"sd":{sd}}}}}"#),
            None => format!(r#"{{"org":"org-team","t":{t},"u":{{"fh":{fh}}}}}"#),
        })
        .collect();
    std::fs::write(
        instance.join("plan-usage-history.json"),
        format!(r#"{{"version":1,"samples":[{}]}}"#, rows.join(",")),
    )
    .unwrap();
}

fn display(setup: &Setup) -> juggler_app::features::quota::QuotaDisplayInfo {
    let storage = setup.state.storage.lock().unwrap();
    let account = storage
        .accounts()
        .get(&juggler_core::model::AccountId::new(setup.account.as_str()))
        .unwrap();
    commands::quota_display_inner(&storage, &account, now())
}

#[test]
fn an_app_only_team_seat_is_measured_from_the_apps_meter_with_its_source_and_age() {
    let dir = tempfile::tempdir().unwrap();
    let setup = setup(dir.path());
    let at = now() - 20 * MIN;
    meter(
        &setup.instance,
        &[(at - 15 * MIN, 38.0, None), (at, 42.0, None)],
    );
    usage_pipeline::ingest_pass(&setup.state, 1);
    let info = display(&setup);
    let five = info.five_hour.expect("a 5-hour window");
    assert_eq!(five.percentage, Some(42.0));
    assert_eq!(five.measured_at_ms, Some(at));
    assert_eq!(five.source.as_deref(), Some("desktop_app"));
    assert_eq!(five.source_words.as_deref(), Some("the Claude app"));
    assert_eq!(
        five.resets_at_ms, None,
        "the meter has no reset, and none is made up"
    );
    // A team seat's meter has no week: said, never "not measured", and
    // never "no weekly limit": the meter can't see a model's own week.
    assert!(info.weekly.is_none());
    assert_eq!(
        info.weekly_omitted_reason.as_deref(),
        Some(
            "No all-models weekly limit in the Claude app; a weekly limit on one model shows once Claude Code checks its usage"
        )
    );
    assert!(info.model_weeks.is_empty());
}

/// The maintainer's report (0.1.6): "Claude runway 5 hour seems to be
/// showing, but not the 7 day one". A team seat's week is on one model
/// (`weekly_scoped`, "Fable"), with no all-models week: shown as that
/// model's week, with its source and reset, beside the 5 hours.
#[test]
fn a_team_seats_week_on_one_model_is_shown_as_that_models_week() {
    let dir = tempfile::tempdir().unwrap();
    let setup = setup(dir.path());
    let fetched = now() - 5 * MIN;
    let week_reset = now() + 3 * 24 * HOUR;
    std::fs::write(
        setup.profile.join(".claude.json"),
        format!(
            r#"{{"oauthAccount":{{"accountUuid":"acct-a","organizationUuid":"org-team"}},
            "cachedUsageUtilization":{{"fetchedAtMs":{fetched},"accountUuid":"acct-a","utilization":{{
            "five_hour":{{"utilization":7,"resets_at":"{}"}},"seven_day":null,
            "limits":[{{"kind":"session","group":"session","percent":7,"resets_at":"{}","scope":null,"is_active":true}},
              {{"kind":"weekly_scoped","group":"weekly","percent":0,"resets_at":"{}","severity":"normal","scope":{{"model":{{"id":null,"display_name":"Fable"}},"surface":null}},"is_active":false}}],
            "seven_day_breakdown":null,"weekly_scoped_shares":null}}}}}}"#,
            iso(now() + 2 * HOUR),
            iso(now() + 2 * HOUR),
            iso(week_reset)
        ),
    )
    .unwrap();
    // The app's meter read since, with no week (it can't see one).
    meter(&setup.instance, &[(now() - MIN, 9.0, None)]);
    usage_pipeline::ingest_pass(&setup.state, 1);
    let info = display(&setup);
    assert!(info.five_hour.is_some());
    assert!(info.weekly.is_none(), "no all-models week is made up");
    assert_eq!(
        info.weekly_omitted_reason.as_deref(),
        Some("Claude Code reports a weekly limit only for Fable")
    );
    assert_eq!(info.model_weeks.len(), 1);
    let fable = &info.model_weeks[0];
    assert_eq!(fable.model, "Fable");
    assert_eq!(
        fable.window.percentage,
        Some(0.0),
        "100% left, a real reading"
    );
    assert_eq!(fable.window.resets_at_ms, Some(week_reset / 1000 * 1000));
    assert_eq!(fable.window.source.as_deref(), Some("usage_cache"));
    assert_eq!(
        fable.window.source_words.as_deref(),
        Some("Claude Code's last usage check")
    );
    assert_eq!(fable.window.measured_at_ms, Some(fetched));
}

/// A personal plan has the whole week and a model's week beside it; the
/// model's week never stands in for the whole week.
#[test]
fn a_personal_plans_model_week_sits_beside_the_whole_week() {
    let dir = tempfile::tempdir().unwrap();
    let setup = setup(dir.path());
    let fetched = now() - 5 * MIN;
    let reset = iso(now() + 3 * 24 * HOUR);
    std::fs::write(
        setup.profile.join(".claude.json"),
        format!(
            r#"{{"oauthAccount":{{"accountUuid":"acct-a","organizationUuid":"org-team"}},
            "cachedUsageUtilization":{{"fetchedAtMs":{fetched},"accountUuid":"acct-a","utilization":{{
            "five_hour":{{"utilization":12,"resets_at":"{reset}"}},
            "seven_day":{{"utilization":40,"resets_at":"{reset}"}},
            "limits":[{{"kind":"weekly_all","percent":40,"resets_at":"{reset}"}},
              {{"kind":"weekly_scoped","percent":55,"resets_at":"{reset}","scope":{{"model":{{"display_name":"Fable"}}}}}}]}}}}}}"#
        ),
    )
    .unwrap();
    usage_pipeline::ingest_pass(&setup.state, 1);
    let info = display(&setup);
    assert_eq!(info.weekly.unwrap().percentage, Some(40.0));
    assert_eq!(info.weekly_omitted_reason, None);
    assert_eq!(info.model_weeks.len(), 1);
    assert_eq!(info.model_weeks[0].window.percentage, Some(55.0));
}

#[test]
fn a_limit_hit_in_a_transcript_says_used_up_until_the_vendors_reset() {
    let dir = tempfile::tempdir().unwrap();
    let setup = setup(dir.path());
    let hit = now() - 30 * MIN;
    let reset_s = (now() + 2 * HOUR) / 1000;
    std::fs::write(
        setup.profile.join("projects/-w-p/s-1.jsonl"),
        format!(
            r#"{{"type":"assistant","sessionId":"s-1","timestamp":"{}","message":{{"content":[{{"type":"text","text":"PLANTED"}}]}},"error":"rate_limit","isApiErrorMessage":true,"quotaLimits":{{"status":"rejected","rateLimitType":"five_hour","resetsAt":{reset_s}}}}}"#,
            iso(hit)
        ) + "\n",
    )
    .unwrap();
    usage_pipeline::ingest_pass(&setup.state, 1);
    let five = display(&setup).five_hour.unwrap();
    assert_eq!(five.state, "exhausted");
    assert_eq!(five.resets_at_ms, Some(reset_s * 1000));
    assert_eq!(five.source.as_deref(), Some("api_retry"));
}

#[test]
fn claude_codes_last_usage_gives_both_windows_and_an_old_one_is_history() {
    let dir = tempfile::tempdir().unwrap();
    let setup = setup(dir.path());
    let fetched = now() - 13 * HOUR;
    std::fs::write(
        setup.profile.join(".claude.json"),
        format!(
            r#"{{"oauthAccount":{{"accountUuid":"acct-a","organizationUuid":"org-team"}},
            "cachedUsageUtilization":{{"fetchedAtMs":{fetched},"accountUuid":"acct-a","utilization":{{
            "five_hour":{{"utilization":40,"resets_at":"{}"}},
            "seven_day":{{"utilization":61,"resets_at":"{}"}}}}}}}}"#,
            iso(fetched + 2 * HOUR),
            iso(now() + 3 * 24 * HOUR)
        ),
    )
    .unwrap();
    usage_pipeline::ingest_pass(&setup.state, 1);
    let info = display(&setup);
    let five = info.five_hour.unwrap();
    assert_eq!(five.state, "reset", "13 hours old: history, not now");
    assert_eq!(five.percentage, None);
    assert_eq!(five.last_used_pct, Some(40.0));
    let week = info.weekly.unwrap();
    assert_eq!(week.percentage, Some(61.0), "a week stands until its reset");
    assert_eq!(week.source.as_deref(), Some("usage_cache"));
    assert!(week.resets_at_ms.is_some());
}

#[test]
fn reading_the_records_again_adds_nothing_and_writes_nothing_in_the_users_folders() {
    let dir = tempfile::tempdir().unwrap();
    let setup = setup(dir.path());
    meter(&setup.instance, &[(now() - 5 * MIN, 10.0, None)]);
    let before = |dir: &Path| {
        let mut out = Vec::new();
        for entry in walkdir(dir) {
            out.push((entry.clone(), std::fs::metadata(&entry).unwrap().len()));
        }
        out
    };
    let (profile, instance) = (before(&setup.profile), before(&setup.instance));
    usage_pipeline::ingest_pass(&setup.state, 1);
    usage_pipeline::ingest_pass(&setup.state, 15);
    let storage = setup.state.storage.lock().unwrap();
    let rows = storage
        .usage()
        .samples_for_account(&setup.account, juggler_core::quota::WindowKind::FiveHour)
        .unwrap();
    assert_eq!(rows.len(), 1);
    drop(storage);
    assert_eq!(
        before(&setup.profile),
        profile,
        "the profile folder is only read"
    );
    assert_eq!(
        before(&setup.instance),
        instance,
        "the app's folder is only read"
    );
}

fn walkdir(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walkdir(&path));
        } else {
            out.push(path);
        }
    }
    out.sort();
    out
}

#[test]
fn asking_claude_code_is_on_by_default_and_off_everywhere_when_turned_off() {
    use juggler_app::features::quota::probe;
    let dir = tempfile::tempdir().unwrap();
    let setup = setup(dir.path());
    // D-39 (SETTLED, option A): on by default for every profile.
    let status = probe::status_impl(&setup.state, &setup.account).unwrap();
    assert!(status.offered, "D-39: offered by default");
    assert_eq!(status.state, "on", "D-39: each profile is on by default");
    // Turned off everywhere: nothing is offered and nothing runs.
    setup
        .state
        .storage
        .lock()
        .unwrap()
        .app_settings()
        .set(probe::ALLOWED_KEY, "off")
        .unwrap();
    let status = probe::status_impl(&setup.state, &setup.account).unwrap();
    assert!(!status.offered, "off everywhere when turned off");
    assert!(probe::turn_on_impl(&setup.state, &setup.account).is_err());
    assert!(probe::run_now_impl(&setup.state, &setup.account).is_err());
}

#[test]
fn once_ruled_on_the_probe_runs_claude_codes_usage_in_the_profile_and_keeps_no_output() {
    use juggler_app::features::quota::probe;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    // A stand-in `claude` that records its arguments and its config folder.
    let record = root.join("record.txt");
    let bin = root.join("claude");
    std::fs::write(
        &bin,
        format!(
            "#!/bin/sh\necho \"$* | $CLAUDE_CONFIG_DIR\" >> '{}'\necho SECRET-OUTPUT\n",
            record.display()
        ),
    )
    .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let profile = root.join("ClaudeCodeProfiles/team-a");
    std::fs::create_dir_all(profile.join("projects")).unwrap();
    let data = root.join("app-data");
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(
        ClaudeCodeAdapter::with_bin(bin.to_str().unwrap())
            .with_overlays_root(data.join("overlays")),
    ));
    let state = Arc::new(AppState::init_with_registry(data, registry).unwrap());
    let account = commands::account_import_mode_impl(
        &state,
        CLAUDE_CODE_ID,
        profile.to_str().unwrap(),
        "Team A",
        commands::ImportRequest {
            in_place: true,
            ..commands::ImportRequest::default()
        },
    )
    .unwrap()
    .account
    .id
    .to_string();
    state
        .storage
        .lock()
        .unwrap()
        .app_settings()
        .set(probe::ALLOWED_KEY, "on")
        .unwrap();
    let status = probe::turn_on_impl(&state, &account).unwrap();
    assert_eq!(status.state, "on");
    assert_eq!(status.last_outcome.as_deref(), Some("ran"));
    let ran = std::fs::read_to_string(&record).unwrap();
    assert!(
        ran.starts_with("-p /usage --no-session-persistence"),
        "{ran}"
    );
    assert!(
        ran.trim_end().ends_with(profile.to_str().unwrap()),
        "in the profile: {ran}"
    );
    // Off: nothing more runs.
    probe::turn_off_impl(&state, &account).unwrap();
    assert!(probe::run_now_impl(&state, &account).is_err());
}

// ---------------------------------------------------------------------
// The real setup, read-only (run by hand: `--ignored --nocapture`).

/// Every Claude profile on this Mac and the Claude app instances, read as
/// the pipeline reads them, evaluated as the display evaluates them. It
/// writes nothing anywhere and prints only each profile's states, ages
/// and sources: never an id, a value from a file other than the window
/// percentages, or any text.
#[test]
#[ignore = "reads the real setup; run by hand"]
fn qa_real_readonly_claude_limits_per_profile() {
    use juggler_adapters::claude_limits;
    use juggler_core::quota::{QuotaCap, QuotaSample, UsageSource, WindowKind, evaluate_window};
    let home = dirs::home_dir().unwrap();
    let support = home.join("Library/Application Support");
    let mut profiles: Vec<(String, PathBuf)> = vec![("Default".into(), home.join(".claude"))];
    if let Ok(entries) = std::fs::read_dir(support.join("ClaudeCodeProfiles")) {
        let mut more: Vec<(String, PathBuf)> = entries
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
            .collect();
        more.sort();
        profiles.extend(more);
    }
    // Instances as the pipeline finds them: the app's default folder and
    // the running ones.
    let mut instances = vec![support.join("Claude")];
    let registry = juggler_app::state::default_registry();
    let apps: Vec<_> = registry
        .ids()
        .iter()
        .filter_map(|id| registry.get(id))
        .flat_map(|adapter| adapter.host_apps())
        .collect();
    let hosts = juggler_app::features::conversations::places::HostRegistry::new(apps);
    let table = juggler_app::features::conversations::procs::table();
    for (_, dir) in
        juggler_app::features::conversations::places::running_instances(&table, &hosts, &home)
    {
        if !instances.contains(&dir) {
            instances.push(dir);
        }
    }
    let now = now();
    println!("instances found: {}", instances.len());
    for (label, dir) in profiles {
        let mut five: Vec<(QuotaSample, Option<UsageSource>)> = Vec::new();
        let mut week: Vec<(QuotaSample, Option<UsageSource>)> = Vec::new();
        let mut push = |kind: WindowKind, sample: QuotaSample, source: UsageSource| match kind {
            WindowKind::FiveHour => five.push((sample, Some(source))),
            WindowKind::Weekly { .. } => week.push((sample, Some(source))),
        };
        let limits = claude_limits::profile_limits(&dir).unwrap_or_default();
        let mut counts = [0usize; 3];
        if let Some(cache) = &limits.cache {
            for (kind, sample) in &cache.samples {
                counts[0] += 1;
                push(*kind, *sample, UsageSource::UsageCache);
            }
        }
        if let Some(sign_in) = &limits.sign_in {
            for instance in &instances {
                for (kind, sample) in claude_limits::app_usage_samples(instance, sign_in, 0) {
                    counts[1] += 1;
                    push(kind, sample, UsageSource::DesktopApp);
                }
            }
        }
        for file in walk_jsonl(&dir.join("projects")) {
            let recent = std::fs::metadata(&file)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|m| m.elapsed().ok())
                .is_some_and(|age| age.as_secs() < 7 * 86_400);
            if !recent {
                continue;
            }
            for (kind, sample) in claude_limits::limit_hits_in(&file) {
                counts[2] += 1;
                push(kind, sample, UsageSource::ApiRetry);
            }
        }
        five.sort_by_key(|(s, _)| s.ts);
        week.sort_by_key(|(s, _)| s.ts);
        let describe = |name: &str,
                        rows: &[(QuotaSample, Option<UsageSource>)],
                        kind: WindowKind| {
            let samples: Vec<QuotaSample> = rows.iter().map(|(s, _)| *s).collect();
            let reading = evaluate_window(&[], &samples, "acc", kind, QuotaCap::default(), now);
            let source =
                juggler_app::features::quota::sources::reading_source(rows, reading.measured_at)
                    .map_or("-", UsageSource::as_str);
            let age = reading
                .measured_at
                .map_or("-".to_owned(), |at| format!("{} min", (now - at) / MIN));
            let left = reading.percentage.map_or("-".to_owned(), |pct| {
                format!("{}% left", (100.0 - pct).round())
            });
            let resets = reading.resets_at.map_or("no reset known".to_owned(), |at| {
                format!("resets in {} min", (at - now) / MIN)
            });
            let history = reading.last_used_pct.map_or(String::new(), |pct| {
                format!(", last reading {}% used", pct.round())
            });
            format!(
                "{name}: {:?} {left}, {resets}, from {source}, {age} old{history}",
                reading.state
            )
        };
        let no_week = juggler_app::features::quota::sources::week_not_reported(&five, &week, now);
        println!(
            "{label}: signed in: {}; readings: usage cache {}, app meter {}, limit hits {}",
            limits.sign_in.is_some(),
            counts[0],
            counts[1],
            counts[2]
        );
        println!("    {}", describe("5-hour", &five, WindowKind::FiveHour));
        // Each model's own week, by the name Claude shows (never an id).
        for (model, sample) in limits.cache.iter().flat_map(|cache| &cache.model_weeks) {
            let rows = [(*sample, Some(UsageSource::UsageCache))];
            println!(
                "    {}",
                describe(
                    &format!("week, {model} only"),
                    &rows,
                    WindowKind::Weekly { anchor_ms: None }
                )
            );
        }
        if no_week {
            println!("    week: the plan's own records leave the all-models week out");
        } else {
            println!(
                "    {}",
                describe("week", &week, WindowKind::Weekly { anchor_ms: None })
            );
        }
    }
}

fn walk_jsonl(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk_jsonl(&path));
        } else if path.extension().is_some_and(|ext| ext == "jsonl") {
            out.push(path);
        }
    }
    out
}
