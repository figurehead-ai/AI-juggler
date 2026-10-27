//! Claude's own limit records, on synthetic files shaped like the real
//! ones (key paths and value types taken from a read-only look at a real
//! setup, 2026-10-25, Claude Code 2.1.302; every value is invented).

use super::*;

const HOUR: i64 = 3_600_000;
const T0: i64 = 1_792_000_000_000;
/// Planted where only the narrow shape may look: none may come out.
const PLANTED_EMAIL: &str = "PLANTED-EMAIL@example.invalid";
const PLANTED_KEY: &str = "PLANTED-KEY-0000";
const PLANTED_TEXT: &str = "PLANTED-MESSAGE-TEXT";

fn pair(account: &str, org: &str) -> SignIn {
    SignIn {
        account: account.to_owned(),
        org: org.to_owned(),
    }
}

fn history(samples: &[(&str, i64, Option<f64>, Option<f64>)]) -> String {
    let rows: Vec<String> = samples
        .iter()
        .map(|(org, t, fh, sd)| {
            let mut used = Vec::new();
            if let Some(fh) = fh {
                used.push(format!("\"fh\":{fh}"));
            }
            if let Some(sd) = sd {
                used.push(format!("\"sd\":{sd}"));
            }
            format!(
                "{{\"org\":\"{org}\",\"t\":{t},\"u\":{{{}}}}}",
                used.join(",")
            )
        })
        .collect();
    format!("{{\"version\":1,\"samples\":[{}]}}", rows.join(","))
}

#[test]
fn an_app_meter_sample_is_the_accounts_when_its_instance_names_only_that_account_in_the_org() {
    let mine = pair("acct-a", "org-team");
    let bytes = history(&[
        ("org-team", T0, Some(42.0), None),
        ("org-team", T0 + 15 * 60_000, Some(47.0), None),
        ("org-personal", T0 + 20 * 60_000, Some(5.0), Some(61.0)),
    ]);
    let seen = [mine.clone(), pair("acct-a", "org-personal")];
    let samples = app_usage_samples_from(bytes.as_bytes(), &seen, &mine, 0);
    assert_eq!(
        samples.len(),
        2,
        "the other organisation's samples aren't this sign-in's"
    );
    assert!(
        samples
            .iter()
            .all(|(kind, _)| *kind == WindowKind::FiveHour)
    );
    assert_eq!(samples[1].1.used_pct, 47.0);
    assert_eq!(
        samples[1].1.resets_at, None,
        "the meter has no reset time, and none is made up"
    );
    assert_eq!(samples[1].1.confidence, Confidence::Measured);

    // The personal organisation reports both windows.
    let personal =
        app_usage_samples_from(bytes.as_bytes(), &seen, &pair("acct-a", "org-personal"), 0);
    assert_eq!(personal.len(), 2);
    assert!(
        personal
            .iter()
            .any(|(kind, s)| *kind == WindowKind::Weekly { anchor_ms: None } && s.used_pct == 61.0)
    );
}

#[test]
fn two_accounts_of_one_org_in_one_instance_make_its_samples_nobodys() {
    let bytes = history(&[("org-team", T0, Some(42.0), None)]);
    let seen = [pair("acct-a", "org-team"), pair("acct-b", "org-team")];
    assert!(
        app_usage_samples_from(bytes.as_bytes(), &seen, &pair("acct-a", "org-team"), 0).is_empty()
    );
    // No Code sessions at all: whose meter it is can't be told either.
    assert!(
        app_usage_samples_from(bytes.as_bytes(), &[], &pair("acct-a", "org-team"), 0).is_empty()
    );
    // Another account's instance.
    let theirs = [pair("acct-b", "org-team")];
    assert!(
        app_usage_samples_from(bytes.as_bytes(), &theirs, &pair("acct-a", "org-team"), 0)
            .is_empty()
    );
}

#[test]
fn only_newer_samples_come_and_odd_ones_are_skipped_never_guessed() {
    let mine = pair("acct-a", "org-team");
    let (t1, t2) = (T0 + HOUR, T0 + 2 * HOUR);
    let rows = [
        format!(r#"{{"org":"org-team","t":{T0},"u":{{"fh":10}}}}"#),
        format!(r#"{{"org":"org-team","t":{t1},"u":{{"fh":"high"}}}}"#),
        r#"{"org":"org-team","t":"later","u":{"fh":30}}"#.to_owned(),
        format!(r#"{{"org":"org-team","t":{t2},"u":{{"fh":140,"sd":-3}}}}"#),
    ];
    let bytes = format!(r#"{{"version":1,"samples":[{}]}}"#, rows.join(","));
    let samples = app_usage_samples_from(bytes.as_bytes(), std::slice::from_ref(&mine), &mine, T0);
    assert_eq!(
        samples.len(),
        2,
        "only the sample after the watermark, its two windows"
    );
    assert!(samples.iter().all(|(_, s)| s.ts == T0 + 2 * HOUR));
    assert!(
        samples.iter().any(|(_, s)| s.used_pct == 100.0),
        "clamped to 100"
    );
    assert!(
        samples.iter().any(|(_, s)| s.used_pct == 0.0),
        "clamped to 0"
    );
    assert!(app_usage_samples_from(b"not json", std::slice::from_ref(&mine), &mine, 0).is_empty());
    assert!(app_usage_samples_from(b"[1,2]", std::slice::from_ref(&mine), &mine, 0).is_empty());
}

#[test]
fn a_long_history_gives_only_its_newest_samples() {
    let mine = pair("acct-a", "org-team");
    let rows: Vec<(&str, i64, Option<f64>, Option<f64>)> = (0..2_100)
        .map(|i| ("org-team", T0 + i64::from(i) * 60_000, Some(1.0), None))
        .collect();
    let bytes = history(&rows);
    let samples = app_usage_samples_from(bytes.as_bytes(), std::slice::from_ref(&mine), &mine, 0);
    assert_eq!(samples.len(), APP_SAMPLES_MAX);
    assert_eq!(samples.last().unwrap().1.ts, T0 + 2_099 * 60_000);
}

#[test]
fn an_instance_names_its_sign_ins_by_folder_and_reads_its_history() {
    let dir = tempfile::tempdir().unwrap();
    let org = dir.path().join("claude-code-sessions/acct-a/org-team");
    std::fs::create_dir_all(&org).unwrap();
    std::fs::write(org.join("local_1.json"), "{}").unwrap();
    std::fs::write(dir.path().join("claude-code-sessions/stray.json"), "{}").unwrap();
    assert_eq!(
        instance_sign_ins(dir.path()),
        vec![pair("acct-a", "org-team")]
    );
    std::fs::write(
        dir.path().join(APP_USAGE_FILE),
        history(&[("org-team", T0, Some(42.0), None)]),
    )
    .unwrap();
    let samples = app_usage_samples(dir.path(), &pair("acct-a", "org-team"), 0);
    assert_eq!(samples.len(), 1);
    assert!(
        app_usage_samples(&dir.path().join("missing"), &pair("acct-a", "org-team"), 0).is_empty()
    );
}

fn state_file(cache_account: &str, five: &str, seven: &str, limits: &str) -> String {
    format!(
        r#"{{"numStartups":3,"primaryApiKey":"{PLANTED_KEY}",
            "oauthAccount":{{"accountUuid":"acct-a","organizationUuid":"org-team","emailAddress":"{PLANTED_EMAIL}","displayName":"Planted","billingType":"stripe_subscription"}},
            "projects":{{"/w/p":{{"lastCost":1.5}}}},
            "cachedUsageUtilization":{{"fetchedAtMs":{T0},"accountUuid":"{cache_account}","utilization":{{
                "five_hour":{five},"seven_day":{seven},"seven_day_opus":null,"iguana_necktie":null,
                "extra_usage":{{"is_enabled":false,"monthly_limit":null,"used_credits":null,"utilization":null}},
                "limits":{limits}}}}}}}"#
    )
}

#[test]
fn claude_codes_last_usage_reading_is_the_profiles_with_its_resets() {
    let text = state_file(
        "acct-a",
        r#"{"utilization":42.0,"resets_at":"2026-10-25T19:00:00.943648+00:00","limit_dollars":null,"used_dollars":null,"remaining_dollars":null,"locked_reason":null}"#,
        r#"{"utilization":61,"resets_at":"2026-10-30T04:00:00+00:00"}"#,
        "[]",
    );
    let read = profile_limits_from(text.as_bytes()).unwrap();
    assert_eq!(read.sign_in, Some(pair("acct-a", "org-team")));
    let cache = read.cache.unwrap();
    assert_eq!(cache.fetched_at, T0);
    assert_eq!(cache.samples.len(), 2);
    let (kind, five) = cache.samples[0];
    assert_eq!(kind, WindowKind::FiveHour);
    assert_eq!(five.ts, T0);
    assert_eq!(five.used_pct, 42.0);
    assert_eq!(
        five.resets_at,
        juggler_core::time::timestamp_ms("2026-10-25T19:00:00.943Z")
    );
    assert_eq!(cache.samples[1].1.used_pct, 61.0);
}

#[test]
fn a_team_plans_missing_week_gives_no_weekly_sample_and_limits_rows_fill_in() {
    // A team seat: `seven_day` is null, and the newer rows list only a
    // per-model week, which is not the whole week.
    let text = state_file(
        "acct-a",
        r#"{"utilization":7,"resets_at":"2026-10-25T19:00:00+00:00"}"#,
        "null",
        r#"[{"kind":"session","group":"session","percent":7,"resets_at":"2026-10-25T19:00:00+00:00","severity":"normal","is_active":true,"scope":null},
            {"kind":"weekly_scoped","group":"weekly","percent":0,"resets_at":"2026-10-30T04:00:00+00:00","severity":"normal","is_active":false,"scope":{"model":{"display_name":"Opus"},"surface":null}}]"#,
    );
    let cache = profile_limits_from(text.as_bytes()).unwrap().cache.unwrap();
    assert_eq!(cache.samples.len(), 1);
    assert_eq!(cache.samples[0].0, WindowKind::FiveHour);
    // The model's week is a limit of its own, never the whole week.
    assert_eq!(cache.model_weeks.len(), 1);
    assert_eq!(cache.model_weeks[0].0, "Opus");

    // Only the rows: they fill in, by kind.
    let rows_only = state_file(
        "acct-a",
        "null",
        "null",
        r#"[{"kind":"session","group":"session","percent":12,"resets_at":"2026-10-25T19:00:00+00:00"},
            {"kind":"weekly_all","group":"weekly","percent":33,"resets_at":"2026-10-30T04:00:00+00:00"}]"#,
    );
    let cache = profile_limits_from(rows_only.as_bytes())
        .unwrap()
        .cache
        .unwrap();
    assert_eq!(cache.samples.len(), 2);
    assert_eq!(cache.samples[1].0, WindowKind::Weekly { anchor_ms: None });
    assert_eq!(cache.samples[1].1.used_pct, 33.0);
}

/// The maintainer's report (0.1.6): a team seat read "Claude Code reports
/// no weekly limit", while Claude shows a week on one model. The shapes of
/// the real state files (2026-10-26, Claude Code 2.1.299, key paths and
/// types only; every value here is invented).
fn team_limits() -> &'static str {
    r#"[{"kind":"session","group":"session","percent":7,"severity":"normal","resets_at":"2026-10-25T19:00:00+00:00","scope":null,"is_active":true},
        {"kind":"weekly_scoped","group":"weekly","percent":0,"severity":"normal","resets_at":"2026-10-29T20:00:00+00:00","scope":{"model":{"id":null,"display_name":"Fable"},"surface":null},"is_active":false}]"#
}

#[test]
fn a_team_seats_only_week_is_its_models_own_week() {
    let text = state_file(
        "acct-a",
        r#"{"utilization":7,"resets_at":"2026-10-25T19:00:00+00:00","limit_dollars":null,"used_dollars":null,"remaining_dollars":null,"locked_reason":null}"#,
        "null",
        team_limits(),
    );
    let cache = profile_limits_from(text.as_bytes()).unwrap().cache.unwrap();
    assert_eq!(
        cache
            .samples
            .iter()
            .map(|(kind, _)| *kind)
            .collect::<Vec<_>>(),
        [WindowKind::FiveHour],
        "no all-models week is made up"
    );
    assert_eq!(cache.model_weeks.len(), 1);
    let (model, week) = &cache.model_weeks[0];
    assert_eq!(model, "Fable");
    assert_eq!(
        week.used_pct, 0.0,
        "0% used is a reading, not a missing one"
    );
    assert_eq!(week.ts, T0);
    assert_eq!(week.confidence, Confidence::Measured);
    assert_eq!(
        week.resets_at,
        juggler_core::time::timestamp_ms("2026-10-29T20:00:00Z")
    );
}

#[test]
fn a_personal_plan_has_the_whole_week_and_a_models_week_beside_it() {
    let text = format!(
        r#"{{"oauthAccount":{{"accountUuid":"acct-a","organizationUuid":"org-personal"}},
        "cachedUsageUtilization":{{"fetchedAtMs":{T0},"accountUuid":"acct-a","utilization":{{
            "five_hour":{{"utilization":12,"resets_at":"2026-10-25T19:00:00+00:00"}},
            "seven_day":{{"utilization":40,"resets_at":"2026-10-30T04:00:00+00:00"}},
            "limits":[{{"kind":"session","group":"session","percent":12,"resets_at":"2026-10-25T19:00:00+00:00","scope":null,"is_active":true}},
                {{"kind":"weekly_all","group":"weekly","percent":40,"resets_at":"2026-10-30T04:00:00+00:00","scope":null,"is_active":false}},
                {{"kind":"weekly_scoped","group":"weekly","percent":55,"resets_at":"2026-10-30T04:00:00+00:00","scope":{{"model":{{"id":null,"display_name":"Fable"}},"surface":null}},"is_active":false}}],
            "seven_day_breakdown":{{"as_of":"2026-10-25T12:00:00+00:00","window_started_at":"2026-10-23T04:00:00+00:00","rows":[{{"key":"k","display_name":"Fable","percent":30}}]}},
            "weekly_scoped_shares":[{{"limit_index":2,"used_percent_of_weekly":20,"allowance_percent_of_weekly":50}}]}}}}}}"#
    );
    let cache = profile_limits_from(text.as_bytes()).unwrap().cache.unwrap();
    assert_eq!(cache.samples.len(), 2);
    assert_eq!(cache.samples[1].0, WindowKind::Weekly { anchor_ms: None });
    assert_eq!(cache.samples[1].1.used_pct, 40.0);
    assert_eq!(cache.model_weeks.len(), 1);
    assert_eq!(cache.model_weeks[0].0, "Fable");
    assert_eq!(cache.model_weeks[0].1.used_pct, 55.0);
}

#[test]
fn a_window_with_no_figure_falls_back_to_its_limits_row() {
    // `seven_day` is there, but its `utilization` is null: the row says.
    let text = state_file(
        "acct-a",
        r#"{"utilization":null,"resets_at":null}"#,
        r#"{"utilization":null,"resets_at":null}"#,
        r#"[{"kind":"session","group":"session","percent":9,"resets_at":"2026-10-25T19:00:00+00:00"},
            {"kind":"weekly_all","group":"weekly","percent":21,"resets_at":"2026-10-30T04:00:00+00:00"}]"#,
    );
    let cache = profile_limits_from(text.as_bytes()).unwrap().cache.unwrap();
    let got: Vec<(WindowKind, f64, Option<i64>)> = cache
        .samples
        .iter()
        .map(|(kind, s)| (*kind, s.used_pct, s.resets_at))
        .collect();
    assert_eq!(
        got,
        [
            (
                WindowKind::FiveHour,
                9.0,
                juggler_core::time::timestamp_ms("2026-10-25T19:00:00Z")
            ),
            (
                WindowKind::Weekly { anchor_ms: None },
                21.0,
                juggler_core::time::timestamp_ms("2026-10-30T04:00:00Z")
            ),
        ]
    );
    // No figure anywhere: no reading, never 0%.
    let none = state_file(
        "acct-a",
        r#"{"utilization":null}"#,
        r#"{"utilization":null}"#,
        "[]",
    );
    let cache = profile_limits_from(none.as_bytes()).unwrap().cache.unwrap();
    assert!(cache.samples.is_empty());
}

#[test]
fn an_odd_limits_row_gives_nothing_and_spoils_nothing() {
    let long = "x".repeat(MODEL_NAME_MAX + 1);
    let rows = format!(
        r#"[{{"kind":7,"percent":3}},
            {{"kind":"weekly_scoped","percent":5,"scope":"Fable"}},
            {{"kind":"weekly_scoped","percent":5,"scope":{{"model":"Fable"}}}},
            {{"kind":"weekly_scoped","percent":5,"scope":{{"model":{{"display_name":"  "}}}}}},
            {{"kind":"weekly_scoped","percent":5,"scope":{{"model":{{"display_name":"{long}"}}}}}},
            {{"kind":"weekly_scoped","percent":5,"scope":{{"model":{{"display_name":"Bad\u0007"}}}}}},
            {{"kind":"weekly_scoped","percent":"high","scope":{{"model":{{"display_name":"Sonnet"}}}}}},
            {{"kind":"weekly_scoped","percent":250,"scope":{{"model":{{"display_name":" Opus "}}}}}},
            {{"kind":"weekly_scoped","percent":1,"scope":{{"model":{{"display_name":"Opus"}}}}}}]"#
    );
    let text = state_file(
        "acct-a",
        r#"{"utilization":42,"resets_at":null}"#,
        "null",
        &rows,
    );
    let cache = profile_limits_from(text.as_bytes()).unwrap().cache.unwrap();
    assert_eq!(cache.samples.len(), 1, "the 5-hour reading still comes");
    let weeks: Vec<(&str, f64)> = cache
        .model_weeks
        .iter()
        .map(|(model, s)| (model.as_str(), s.used_pct))
        .collect();
    assert_eq!(weeks, [("Opus", 100.0)], "named plainly, clamped, once");
}

#[test]
fn a_reading_for_another_account_is_not_the_profiles() {
    let text = state_file(
        "acct-other",
        r#"{"utilization":42.0,"resets_at":null}"#,
        "null",
        "[]",
    );
    let read = profile_limits_from(text.as_bytes()).unwrap();
    assert!(read.sign_in.is_some());
    assert_eq!(read.cache, None);
    // Signed out: no sign-in, so no reading is anyone's.
    let signed_out = format!(
        r#"{{"cachedUsageUtilization":{{"fetchedAtMs":{T0},"accountUuid":"acct-a","utilization":{{"five_hour":{{"utilization":1}}}}}}}}"#
    );
    assert_eq!(
        profile_limits_from(signed_out.as_bytes()).unwrap(),
        ProfileLimits::default()
    );
    assert_eq!(profile_limits_from(b"not json"), None);
}

#[test]
fn nothing_but_the_ids_and_the_numbers_is_kept() {
    let text = state_file(
        "acct-a",
        r#"{"utilization":42.0,"resets_at":null}"#,
        "null",
        "[]",
    );
    let kept = format!("{:?}", profile_limits_from(text.as_bytes()).unwrap());
    for planted in [PLANTED_EMAIL, PLANTED_KEY, "Planted", "/w/p", "stripe"] {
        assert!(!kept.contains(planted), "{planted} was kept: {kept}");
    }
}

#[test]
fn a_profile_folders_state_file_is_read_through_the_narrow_shape() {
    let dir = tempfile::tempdir().unwrap();
    let text = state_file(
        "acct-a",
        r#"{"utilization":42.0,"resets_at":null}"#,
        "null",
        "[]",
    );
    std::fs::write(dir.path().join(".claude.json"), text).unwrap();
    let read = profile_limits(dir.path()).unwrap();
    assert_eq!(read.cache.unwrap().samples.len(), 1);
    assert_eq!(profile_limits(&dir.path().join("missing")), None);
}

fn hit_line(kind: &str, status: &str, resets_s: i64) -> String {
    format!(
        r#"{{"parentUuid":"p1","isSidechain":false,"userType":"external","cwd":"/w/p","sessionId":"s-1","version":"2.1.302","gitBranch":"main","entrypoint":"claude-desktop","type":"assistant","uuid":"u1","timestamp":"2026-10-24T18:02:11.511Z",
        "message":{{"id":"m1","model":"<synthetic>","role":"assistant","stop_reason":"stop_sequence","type":"message","usage":{{"input_tokens":0,"output_tokens":0,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"content":[{{"type":"text","text":"{PLANTED_TEXT}"}}]}},
        "requestId":"r1","error":"rate_limit","isApiErrorMessage":true,"apiErrorStatus":429,"apiError":"limit",
        "apiErrorParams":{{"rate_limit_info":{{"status":"{status}","rateLimitType":"{kind}","resetsAt":{resets_s},"isUsingOverage":false,"overageStatus":"rejected","overageDisabledReason":"org_level_disabled"}}}},
        "quotaLimits":{{"status":"{status}","rateLimitType":"{kind}","resetsAt":{resets_s},"isUsingOverage":false,"overageStatus":"rejected","overageDisabledReason":"org_level_disabled","unifiedRateLimitFallbackAvailable":false}}}}"#
    )
    // One JSONL line, as Claude Code writes it.
    .replace('\n', "")
}

fn value(line: &str) -> serde_json::Value {
    serde_json::from_str(line).unwrap()
}

#[test]
fn a_limit_hit_is_its_window_used_up_until_the_vendors_reset() {
    let at = juggler_core::time::timestamp_ms("2026-10-24T18:02:11.511Z").unwrap();
    let reset_s = at / 1000 + 3 * 3600;
    let (kind, sample) =
        limit_hit_sample(&value(&hit_line("five_hour", "rejected", reset_s))).unwrap();
    assert_eq!(kind, WindowKind::FiveHour);
    assert_eq!(sample.ts, at);
    assert_eq!(sample.used_pct, 100.0);
    assert_eq!(
        sample.resets_at,
        Some(reset_s * 1000),
        "epoch seconds, as Claude Code writes them"
    );
    let (weekly, _) =
        limit_hit_sample(&value(&hit_line("seven_day", "rejected", reset_s))).unwrap();
    assert_eq!(weekly, WindowKind::Weekly { anchor_ms: None });
}

#[test]
fn a_per_model_week_a_warning_or_another_error_is_no_hit() {
    let reset_s = 1_792_900_000;
    for kind in [
        "seven_day_opus",
        "seven_day_sonnet",
        "seven_day_overage_included",
        "overage",
    ] {
        assert_eq!(
            limit_hit_sample(&value(&hit_line(kind, "rejected", reset_s))),
            None,
            "{kind}"
        );
    }
    assert_eq!(
        limit_hit_sample(&value(&hit_line("five_hour", "allowed_warning", reset_s))),
        None
    );
    let other = hit_line("five_hour", "rejected", reset_s)
        .replace("\"error\":\"rate_limit\"", "\"error\":\"overloaded\"");
    assert_eq!(limit_hit_sample(&value(&other)), None);
    let no_reset = hit_line("five_hour", "rejected", reset_s)
        .replace(&format!("\"resetsAt\":{reset_s}"), "\"resetsAt\":null");
    assert_eq!(
        limit_hit_sample(&value(&no_reset)),
        None,
        "no reset, no hit: a reset is never guessed"
    );
}

#[test]
fn an_older_line_with_only_the_api_error_params_still_reads() {
    let reset_s = 1_792_900_000;
    let mut line = value(&hit_line("five_hour", "rejected", reset_s));
    line.as_object_mut().unwrap().remove("quotaLimits");
    assert!(limit_hit_sample(&line).is_some());
}

#[test]
fn a_transcripts_hits_are_read_line_by_line_and_its_words_never_kept() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("s-1.jsonl");
    let reset_s = 1_792_900_000;
    let body = [
        r#"{"type":"user","message":{"role":"user","content":"rate_limit is a word here"}}"#
            .to_owned(),
        hit_line("five_hour", "rejected", reset_s),
        "not json".to_owned(),
        hit_line("seven_day", "rejected", reset_s + 86_400),
    ]
    .join("\n");
    std::fs::write(&path, body).unwrap();
    let hits = limit_hits_in(&path);
    assert_eq!(hits.len(), 2);
    let kept = format!("{hits:?}");
    assert!(!kept.contains(PLANTED_TEXT) && !kept.contains("/w/p"));
    assert!(limit_hits_in(&dir.path().join("missing.jsonl")).is_empty());
}

#[test]
fn the_transcript_reader_keeps_hits_as_samples_beside_its_tokens() {
    let reset_s = 1_792_900_000;
    let usage = r#"{"type":"assistant","timestamp":"2026-10-24T17:00:00Z","sessionId":"s-1","requestId":"r0","message":{"id":"m0","model":"claude-x","usage":{"input_tokens":10,"output_tokens":5}}}"#;
    let slice = format!("{usage}\n{}\n", hit_line("five_hour", "rejected", reset_s));
    let outcome = crate::quota_ingest::transcript_events("acc", slice.as_bytes());
    assert_eq!(outcome.events.len(), 1, "the hit's empty usage is no event");
    assert_eq!(outcome.samples.len(), 1);
    assert_eq!(outcome.samples[0].1.used_pct, 100.0);
}

#[test]
fn the_usage_probe_is_claude_codes_own_usage_command_headless() {
    let args = usage_probe_args();
    assert_eq!(&args[..2], ["-p", "/usage"]);
    for flag in ["--no-session-persistence", "--strict-mcp-config"] {
        assert!(args.iter().any(|arg| arg == flag), "{flag}");
    }
    let settings = args.iter().position(|arg| arg == "--settings").unwrap();
    let hooks: serde_json::Value = serde_json::from_str(&args[settings + 1]).unwrap();
    assert_eq!(hooks["disableAllHooks"], true);
    assert!(
        !args
            .iter()
            .any(|arg| arg == "--model" || arg.contains("prompt")),
        "no model turn is asked for"
    );
}

#[test]
fn the_limit_readers_log_nothing() {
    // What is read is never logged: no logging or printing in the readers
    // (this module, and the app's records and probe modules).
    let sources = [
        include_str!("../claude_limits.rs"),
        include_str!("../../../juggler-app/src/features/quota/records.rs"),
        include_str!("../../../juggler-app/src/features/quota/probe.rs"),
    ];
    for source in sources {
        let product = source.split("#[cfg(test)]").next().unwrap();
        for logger in [
            "tracing::",
            "log::",
            "println!",
            "eprintln!",
            "dbg!",
            "print!(",
        ] {
            assert!(!product.contains(logger), "{logger} in a limit reader");
        }
    }
}
