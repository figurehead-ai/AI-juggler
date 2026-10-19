use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::io::Read as _;

use super::*;
use crate::report::{Arch, Channel, OsName};
use crate::{StatEvent, effective};

const HOUR: i64 = HOUR_MS;
const DAY: i64 = DAY_MS;
/// 2026-W42 runs Monday 2026-10-12 to Monday 2026-10-19 (UTC).
const W42_START: i64 = 1_791_763_200_000;
const W42_END: i64 = W42_START + 7 * DAY;
// Weeks are assembled at the moment they close: a report's upload time
// is then 0–144 h later, never already due (a tick an hour later would
// find it due in ~0.7 % of runs — AJ-E806 ledger).

#[derive(Default)]
struct Recording {
    requests: RefCell<Vec<(&'static str, String, Vec<u8>)>>,
    responses: RefCell<VecDeque<Result<Response, String>>>,
}

impl Recording {
    fn respond(&self, status: u16, body: &str) {
        self.responses.borrow_mut().push_back(Ok(Response {
            status,
            retry_after: None,
            body: body.as_bytes().to_vec(),
        }));
    }
    fn respond_retry_after(&self, status: u16, retry_after: &str) {
        self.responses.borrow_mut().push_back(Ok(Response {
            status,
            retry_after: Some(retry_after.to_owned()),
            body: Vec::new(),
        }));
    }
    fn fail(&self) {
        self.responses
            .borrow_mut()
            .push_back(Err("connection refused".into()));
    }
    fn count(&self, method: &str) -> usize {
        self.requests
            .borrow()
            .iter()
            .filter(|r| r.0 == method)
            .count()
    }
    fn next(&self) -> Result<Response, String> {
        self.responses
            .borrow_mut()
            .pop_front()
            .unwrap_or_else(|| Err("no scripted response".into()))
    }
}

impl Transport for Recording {
    fn get(&self, url: &str) -> Result<Response, String> {
        self.requests
            .borrow_mut()
            .push(("GET", url.to_owned(), Vec::new()));
        self.next()
    }
    fn post_gzip_json(&self, url: &str, body: &[u8]) -> Result<Response, String> {
        self.requests
            .borrow_mut()
            .push(("POST", url.to_owned(), body.to_vec()));
        self.next()
    }
}

const CONFIG_ON: &str = r#"{"enabled":true,"sample_rate":1.0,"schemas":[1],"min_app_version":"0.1.0","max_app_version":null,"max_report_bytes":8192,"pow_bits":0,"known_versions":[]}"#;

fn envelope() -> Envelope {
    Envelope {
        app_version: "0.3.1".into(),
        channel: Channel::Beta,
        os: OsName::Macos,
        os_major: 15,
        arch: Arch::Arm64,
    }
}

fn endpoints() -> Endpoints {
    Endpoints::for_build(&Environment::default()).unwrap()
}

fn no_gauges() -> Vec<StatGauge> {
    Vec::new()
}

fn share() -> EffectiveMode {
    effective(Mode::Share, &Environment::default())
}

struct Fixture {
    _root: tempfile::TempDir,
    dir: PathBuf,
    envelope: Envelope,
    endpoints: Endpoints,
    net: Recording,
}

impl Fixture {
    /// Sharing since before W42, with a few W42 counters and active days.
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("stats");
        let store = Store::open(&dir).unwrap();
        store
            .set_meta("share_since", &(W42_START - DAY).to_string())
            .unwrap();
        store.set_meta("first_week", "2026-W40").unwrap();
        seed(&store, "2026-W42");
        Self {
            _root: root,
            dir,
            envelope: envelope(),
            endpoints: endpoints(),
            net: Recording::default(),
        }
    }

    fn ctx(&self, effective: EffectiveMode, now_ms: i64) -> Context<'_> {
        Context {
            effective,
            consent_current: true,
            consent_version: 1,
            envelope: &self.envelope,
            endpoints: Some(&self.endpoints),
            transport: &self.net,
            now_ms,
            gauges: &no_gauges,
            pow_budget: crate::pow::BUDGET,
        }
    }

    fn tick(&self, now_ms: i64) -> TickOutcome {
        tick(&self.dir, &self.ctx(share(), now_ms)).unwrap()
    }

    fn store(&self) -> Store {
        Store::open(&self.dir).unwrap()
    }

    fn upload_at(&self, week: &str) -> i64 {
        meta_i64(&self.store(), &format!("upload_at:{week}"))
            .unwrap()
            .unwrap()
    }
}

fn seed(store: &Store, week: &str) {
    let start = week::start_ms(week).unwrap();
    store.add(week, "stats.truncated", "{}", 2).unwrap();
    store
        .add(
            week,
            "switch.duration_s",
            r#"{"bucket":"15-60","kind":"provider"}"#,
            3,
        )
        .unwrap();
    store
        .add(
            week,
            "session.tokens",
            r#"{"direction":"input","provider":"codex"}"#,
            123_456,
        )
        .unwrap();
    store.mark_active(&day_label(start)).unwrap();
    store.mark_active(&day_label(start + 2 * DAY)).unwrap();
}

fn report_of(path: &Path) -> Report {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn a_closed_week_is_assembled_once_into_pending() {
    let f = Fixture::new();
    // Still open: nothing happens.
    assert!(f.tick(W42_END - 1).assembled.is_empty());
    let out = f.tick(W42_END);
    assert_eq!(out.assembled.len(), 1);
    let Placement::Pending { upload_at_ms } = out.assembled[0].1 else {
        panic!("{out:?}")
    };
    assert!((W42_END..=W42_END + UPLOAD_SPREAD_MS).contains(&upload_at_ms));
    let report = report_of(&f.dir.join("pending/2026-W42.json"));
    report::validate(&report).unwrap();
    assert_eq!(report.week, "2026-W42");
    // The two seeded days, plus the day of the tick that found it open.
    assert_eq!(report.days_active, 3);
    assert_eq!(report.install_age, "1-4w");
    let tokens = report
        .entries
        .iter()
        .find(|e| e.m == "session.tokens")
        .unwrap();
    assert_eq!(tokens.v, Some(120_000), "two significant figures");
    let switch = report
        .entries
        .iter()
        .find(|e| e.m == "switch.duration_s")
        .unwrap();
    assert_eq!(switch.h, Some(vec![0, 0, 3, 0]));
    // The week's rows are gone, so it is never assembled twice.
    assert!(f.store().counters("2026-W42").unwrap().is_empty());
    assert!(f.tick(W42_END + 1).assembled.is_empty());
    assert_eq!(f.net.requests.borrow().len(), 0, "nothing is due yet");
}

#[test]
fn weeks_without_full_consent_or_in_local_are_skipped() {
    let f = Fixture::new();
    f.store()
        .set_meta("share_since", &(W42_START + DAY).to_string())
        .unwrap();
    let out = f.tick(W42_END);
    assert_eq!(
        out.assembled[0].1,
        Placement::Skipped("partial_consent".into())
    );
    assert!(f.dir.join("skipped/2026-W42.json").exists());

    let g = Fixture::new();
    let local = effective(Mode::Local, &Environment::default());
    let out = tick(&g.dir, &g.ctx(local, W42_END)).unwrap();
    assert_eq!(out.assembled[0].1, Placement::Skipped("local".into()));
    assert_eq!(g.net.requests.borrow().len(), 0);
    // Local mode clears the consent mark: sharing again starts a new one.
    assert!(g.store().meta("share_since").unwrap().is_none());
}

#[test]
fn a_week_assembled_too_late_expires() {
    let f = Fixture::new();
    let out = f.tick(W42_END + EXPIRY_MS + HOUR);
    assert_eq!(out.assembled[0].1, Placement::Skipped("expired".into()));
}

#[test]
fn a_due_report_uploads_once_gzipped_and_moves_to_sent() {
    let f = Fixture::new();
    f.tick(W42_END);
    let due = f.upload_at("2026-W42") + 1;
    f.net.respond(200, CONFIG_ON);
    f.net.respond(202, r#"{"ok":true}"#);
    let out = f.tick(due);
    assert_eq!(
        out.uploads,
        vec![("2026-W42".to_owned(), UploadOutcome::Sent)]
    );
    let requests = f.net.requests.borrow();
    assert_eq!(requests[0].0, "GET");
    assert_eq!(requests[0].1, CONFIG_URL);
    assert_eq!(requests[1].0, "POST");
    assert_eq!(requests[1].1, REPORTS_URL);
    let mut json = Vec::new();
    flate2::read::GzDecoder::new(&requests[1].2[..])
        .read_to_end(&mut json)
        .unwrap();
    // Exactly the bytes kept locally.
    assert_eq!(
        json,
        std::fs::read(f.dir.join("sent/2026-W42.json")).unwrap()
    );
    assert!(!f.dir.join("pending/2026-W42.json").exists());
    let log = std::fs::read_to_string(f.dir.join("upload.log")).unwrap();
    assert!(log.contains(r#""status":202"#) && log.contains(r#""week":"2026-W42""#));
    drop(requests);
    // Nothing more to do.
    assert!(f.tick(due + HOUR).uploads.is_empty());
    assert_eq!(f.net.count("POST"), 1);
}

#[test]
fn several_weeks_go_oldest_first_one_per_tick() {
    let f = Fixture::new();
    seed(&f.store(), "2026-W43");
    let now = W42_END + 7 * DAY + HOUR; // both closed
    let out = f.tick(now);
    assert_eq!(out.assembled.len(), 2);
    for week in ["2026-W42", "2026-W43"] {
        f.store()
            .set_meta(&format!("upload_at:{week}"), &now.to_string())
            .unwrap();
    }
    let due = now + 1;
    f.net.respond(200, CONFIG_ON);
    f.net.respond(202, "{}");
    let out = f.tick(due);
    assert_eq!(
        out.uploads,
        vec![("2026-W42".to_owned(), UploadOutcome::Sent)]
    );
    let gets = f.net.count("GET");
    f.net.respond(202, "{}"); // the config is cached for a day
    let out = f.tick(due + HOUR);
    assert_eq!(
        out.uploads,
        vec![("2026-W43".to_owned(), UploadOutcome::Sent)]
    );
    assert_eq!(f.net.count("GET"), gets);
}

#[test]
fn retry_after_is_honoured_and_capped() {
    let f = Fixture::new();
    f.tick(W42_END);
    let due = f.upload_at("2026-W42") + 1;
    f.net.respond(200, CONFIG_ON);
    f.net.respond_retry_after(503, "86400");
    let out = f.tick(due);
    assert_eq!(
        out.uploads[0].1,
        UploadOutcome::Retry {
            at_ms: due + DAY,
            status: Some(503)
        }
    );
    // Not before then.
    assert!(f.tick(due + DAY - 1).uploads.is_empty());
    f.net.respond(200, CONFIG_ON); // the daily refresh
    f.net.respond_retry_after(429, "999999");
    let out = f.tick(due + DAY);
    assert_eq!(
        out.uploads[0].1,
        UploadOutcome::Retry {
            at_ms: due + DAY + RETRY_AFTER_CAP_MS,
            status: Some(429)
        }
    );
}

#[test]
fn network_errors_back_off_and_give_up_after_six_attempts() {
    let f = Fixture::new();
    f.tick(W42_END);
    let mut now = f.upload_at("2026-W42") + 1;
    // A config refresh is due a day after the last fetch.
    let script = |now: i64| {
        let fetched = meta_i64(&f.store(), "config_fetched_at").unwrap();
        if fetched.is_none_or(|at| now - at >= config::FRESH_MS) {
            f.net.respond(200, CONFIG_ON);
        }
        f.net.fail();
    };
    for (attempt, hours) in BACKOFF_HOURS.iter().enumerate() {
        script(now);
        let out = f.tick(now);
        let UploadOutcome::Retry {
            at_ms,
            status: None,
        } = out.uploads[0].1
        else {
            panic!("attempt {attempt}: {out:?}")
        };
        let delay = at_ms - now;
        let base = hours * HOUR;
        assert!(
            (base - base / 5..=base + base / 5).contains(&delay),
            "attempt {attempt}: {delay}"
        );
        now = at_ms;
    }
    script(now);
    let out = f.tick(now);
    assert_eq!(out.uploads[0].1, UploadOutcome::Skipped("gave_up".into()));
    assert!(f.dir.join("skipped/2026-W42.json").exists());
}

#[test]
fn rejected_reports_are_not_retried() {
    for status in [400, 404, 413] {
        let f = Fixture::new();
        f.tick(W42_END);
        let due = f.upload_at("2026-W42") + 1;
        f.net.respond(200, CONFIG_ON);
        f.net
            .respond(status, r#"{"error":"invalid","detail":"week"}"#);
        let out = f.tick(due);
        assert_eq!(
            out.uploads[0].1,
            UploadOutcome::Skipped(format!("rejected:{status}"))
        );
        assert!(f.tick(due + 7 * DAY).uploads.is_empty());
    }
}

#[test]
fn expiry_ends_retries() {
    let f = Fixture::new();
    f.tick(W42_END);
    let due = f.upload_at("2026-W42") + 1;
    f.net.respond(200, CONFIG_ON);
    f.net.respond_retry_after(503, "86400");
    f.tick(due);
    let out = f.tick(W42_END + EXPIRY_MS + 1);
    assert_eq!(out.uploads[0].1, UploadOutcome::Skipped("gave_up".into()));
    assert_eq!(f.net.count("POST"), 1);
}

#[test]
fn the_config_can_switch_off_narrow_and_sample() {
    let cases = [
        (r#"{"enabled":false}"#, "server_disabled"),
        (r#"{"enabled":true,"min_app_version":"9.0.0"}"#, "version"),
        (r#"{"enabled":true,"schemas":[2]}"#, "schema"),
        (r#"{"enabled":true,"sample_rate":0.000001}"#, "not_sampled"),
    ];
    for (config, reason) in cases {
        let f = Fixture::new();
        f.tick(W42_END);
        f.net.respond(200, config);
        let out = f.tick(f.upload_at("2026-W42") + 1);
        assert_eq!(
            out.uploads[0].1,
            UploadOutcome::Skipped(reason.into()),
            "{config}"
        );
        assert_eq!(f.net.count("POST"), 0, "{config}");
    }
}

#[test]
fn the_sample_rate_applied_is_in_the_report() {
    let f = Fixture::new();
    // A salt this week is selected at 0.5.
    let salt = (0u8..=255)
        .map(|b| vec![b; 32])
        .find(|salt| sampled(salt, "2026-W42", 0.5))
        .unwrap();
    f.store()
        .set_meta("salt", &crate::store::hex(&salt))
        .unwrap();
    f.tick(W42_END);
    f.net.respond(
        200,
        &CONFIG_ON.replace("\"sample_rate\":1.0", "\"sample_rate\":0.5"),
    );
    f.net.respond(202, "{}");
    f.tick(f.upload_at("2026-W42") + 1);
    let report = report_of(&f.dir.join("sent/2026-W42.json"));
    assert!((report.sample_rate - 0.5).abs() < f64::EPSILON);
}

#[test]
fn no_usable_config_means_no_upload_yet() {
    let f = Fixture::new();
    f.tick(W42_END);
    let due = f.upload_at("2026-W42") + 1;
    f.net.fail();
    assert!(f.tick(due).uploads.is_empty());
    assert_eq!(f.net.count("POST"), 0);
    // A stale copy (2 days) is reused when the refresh fails, but the
    // refresh is tried at most once a day.
    f.net.respond(200, CONFIG_ON);
    f.net.respond(503, "");
    f.tick(due + HOUR);
    let gets = f.net.count("GET");
    f.net.fail();
    f.net.respond_retry_after(503, "3600");
    f.tick(due + 2 * DAY);
    assert_eq!(f.net.count("GET"), gets + 1);
    f.net.respond_retry_after(503, "3600");
    f.tick(due + 2 * DAY + 2 * HOUR);
    assert_eq!(f.net.count("GET"), gets + 1, "no second fetch within a day");
}

#[test]
fn a_puzzle_is_solved_and_a_raised_one_refetches_the_config() {
    let f = Fixture::new();
    f.tick(W42_END);
    let due = f.upload_at("2026-W42") + 1;
    f.net.respond(200, CONFIG_ON);
    f.net.respond(403, r#"{"error":"pow","bits":8}"#);
    let out = f.tick(due);
    assert!(matches!(
        out.uploads[0].1,
        UploadOutcome::Retry {
            status: Some(403),
            ..
        }
    ));
    f.net
        .respond(200, &CONFIG_ON.replace("\"pow_bits\":0", "\"pow_bits\":8"));
    f.net.respond(202, "{}");
    let out = f.tick(due + HOUR);
    assert_eq!(out.uploads[0].1, UploadOutcome::Sent);
    assert_eq!(f.net.count("GET"), 2, "the 403 refetched the config");
    let report = report_of(&f.dir.join("sent/2026-W42.json"));
    let nonce = report.pow.unwrap();
    assert!(crate::pow::check(&report.report_id, "2026-W42", &nonce, 8));
}

#[test]
fn a_puzzle_above_28_bits_postpones_a_day() {
    let f = Fixture::new();
    f.tick(W42_END);
    let due = f.upload_at("2026-W42") + 1;
    f.net
        .respond(200, &CONFIG_ON.replace("\"pow_bits\":0", "\"pow_bits\":29"));
    let out = f.tick(due);
    assert_eq!(
        out.uploads[0].1,
        UploadOutcome::Retry {
            at_ms: due + DAY,
            status: None
        }
    );
    assert_eq!(f.net.count("POST"), 0);
}

#[test]
fn leaving_share_skips_pending_reports() {
    let f = Fixture::new();
    f.tick(W42_END);
    let local = effective(Mode::Local, &Environment::default());
    let out = tick(&f.dir, &f.ctx(local, W42_END + 2 * HOUR)).unwrap();
    assert_eq!(
        out.uploads[0].1,
        UploadOutcome::Skipped("mode_changed".into())
    );
    assert_eq!(f.net.requests.borrow().len(), 0);
}

#[test]
fn ci_debug_and_unanswered_consent_never_send() {
    let ci = effective(
        Mode::Share,
        &Environment {
            ci: Some("true".into()),
            ..Environment::default()
        },
    );
    let debug = effective(
        Mode::Share,
        &Environment {
            debug_build: true,
            ..Environment::default()
        },
    );
    for (name, effective_mode, consent) in [
        ("ci", ci, true),
        ("debug", debug, true),
        ("consent", share(), false),
    ] {
        let f = Fixture::new();
        let mut ctx = f.ctx(effective_mode, W42_END);
        ctx.consent_current = consent;
        tick(&f.dir, &ctx).unwrap();
        ctx.now_ms = W42_END + UPLOAD_SPREAD_MS + HOUR;
        let out = tick(&f.dir, &ctx).unwrap();
        assert!(out.uploads.is_empty(), "{name}");
        assert_eq!(f.net.requests.borrow().len(), 0, "{name}");
        assert!(
            f.dir.join("pending/2026-W42.json").exists(),
            "{name}: visible locally"
        );
    }
}

#[test]
fn release_builds_ignore_the_endpoint_variable() {
    let release = Endpoints::for_build(&Environment {
        endpoint: Some("http://localhost:8790/v1/reports".into()),
        ..Environment::default()
    })
    .unwrap();
    assert_eq!(release.reports, REPORTS_URL);
    let debug = Endpoints::for_build(&Environment {
        debug_build: true,
        endpoint: Some("http://localhost:8790/v1/reports".into()),
        ..Environment::default()
    })
    .unwrap();
    assert_eq!(debug.reports, "http://localhost:8790/v1/reports");
    assert_eq!(debug.config, "http://localhost:8790/v1/config.json");
    assert!(
        Endpoints::for_build(&Environment {
            debug_build: true,
            ..Environment::default()
        })
        .is_none()
    );
}

#[test]
fn off_does_nothing_and_creates_nothing() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("stats");
    let net = Recording::default();
    let envelope = envelope();
    let endpoints = endpoints();
    let ctx = Context {
        effective: effective(Mode::Off, &Environment::default()),
        consent_current: true,
        consent_version: 1,
        envelope: &envelope,
        endpoints: Some(&endpoints),
        transport: &net,
        now_ms: W42_END,
        gauges: &no_gauges,
        pow_budget: crate::pow::BUDGET,
    };
    assert_eq!(tick(&dir, &ctx).unwrap(), TickOutcome::default());
    assert!(!dir.exists());
    assert!(reports(&dir).unwrap().is_empty());
    assert!(!dir.exists());
    assert_eq!(net.requests.borrow().len(), 0);
}

#[test]
fn sampling_hits_its_rate_within_two_percent() {
    for rate in [0.1, 0.5] {
        let selected = (0u32..10_000)
            .filter(|i| sampled(&i.to_le_bytes(), "2026-W42", rate))
            .count();
        #[allow(clippy::cast_precision_loss)]
        let observed = selected as f64 / 10_000.0;
        println!("sampling: rate {rate} → {observed}");
        assert!((observed - rate).abs() <= 0.02, "rate {rate}: {observed}");
    }
    // Stable within a week, different across weeks.
    let salt = [7u8; 32];
    assert_eq!(
        sampled(&salt, "2026-W42", 0.5),
        sampled(&salt, "2026-W42", 0.5)
    );
    let weeks: Vec<bool> = (1..=40)
        .map(|w| sampled(&salt, &format!("2026-W{w:02}"), 0.5))
        .collect();
    assert!(weeks.contains(&true) && weeks.contains(&false));
}

#[test]
fn the_worst_case_week_stays_under_8_kb() {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(&root.path().join("stats")).unwrap();
    // Every metric with every value of every dimension (one at a time),
    // every histogram bucket, every gauge, all at their caps.
    for meta in crate::METRICS {
        let base: BTreeMap<String, String> = meta
            .dims
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v)[0].to_owned()))
            .collect();
        let mut variants = vec![base.clone()];
        for (k, values) in meta.dims {
            for v in *values {
                let mut d = base.clone();
                d.insert((*k).to_owned(), (*v).to_owned());
                variants.push(d);
            }
        }
        for d in variants {
            match meta.kind {
                "gauge" => {
                    let value = meta.values.last().copied().unwrap_or("other");
                    store
                        .set_gauge(
                            "2026-W42",
                            meta.name,
                            &serde_json::to_string(&d).unwrap(),
                            value,
                        )
                        .unwrap();
                }
                "histogram" => {
                    for bucket in meta.buckets {
                        let mut d = d.clone();
                        d.insert("bucket".into(), (*bucket).to_owned());
                        store
                            .add(
                                "2026-W42",
                                meta.name,
                                &serde_json::to_string(&d).unwrap(),
                                meta.max,
                            )
                            .unwrap();
                    }
                }
                _ => store
                    .add(
                        "2026-W42",
                        meta.name,
                        &serde_json::to_string(&d).unwrap(),
                        meta.max + 1,
                    )
                    .unwrap(),
            }
        }
    }
    for day in 0..7 {
        store
            .mark_active(&day_label(W42_START + day * DAY))
            .unwrap();
    }
    let (report, adjustments) = report::assemble(&store, "2026-W42", &envelope(), 1).unwrap();
    let bytes = report.to_bytes().len();
    println!(
        "worst case: {bytes} bytes, {} entries, {} truncated",
        report.entries.len(),
        adjustments.truncated
    );
    assert!(bytes <= report::MAX_REPORT_BYTES);
    assert!(adjustments.truncated > 0);
    report::validate(&report).unwrap();
    // ui.* goes first, then perf.*.
    assert!(!report.entries.iter().any(|e| e.m.starts_with("ui.")));
}

#[test]
fn assembly_is_fast() {
    // A typical week (the fixture) and the worst case; best of five, so a
    // busy test machine doesn't decide the result.
    let f = Fixture::new();
    let store = f.store();
    let mut best = Duration::MAX;
    for _ in 0..5 {
        let started = std::time::Instant::now();
        let (report, _) = report::assemble(&store, "2026-W42", &f.envelope, 1).unwrap();
        report::validate(&report).unwrap();
        best = best.min(started.elapsed());
    }
    println!("assembly: {} µs (best of 5)", best.as_micros());
    assert!(best < Duration::from_millis(50));
}

#[test]
fn reports_lists_every_file_with_its_state() {
    let f = Fixture::new();
    f.tick(W42_END);
    let listed = reports(&f.dir).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].status, "pending");
    assert!(listed[0].upload_at_ms.is_some());
    let json = read_report(&f.dir, "pending", "2026-W42").unwrap();
    assert!(json.starts_with(r#"{"schema":1"#));
    assert!(read_report(&f.dir, "pending", "../counters").is_err());
    assert!(read_report(&f.dir, "elsewhere", "2026-W42").is_err());
}

#[test]
fn old_sent_and_skipped_reports_are_pruned() {
    let f = Fixture::new();
    f.store()
        .set_meta("share_since", &(W42_START + DAY).to_string())
        .unwrap();
    f.tick(W42_END);
    assert!(f.dir.join("skipped/2026-W42.json").exists());
    f.tick(W42_START + KEEP_MS + DAY);
    assert!(!f.dir.join("skipped/2026-W42.json").exists());
}

#[test]
fn stats_events_record_into_weeks_the_cycle_closes() {
    // The recorder and the cycle share the store through SQLite (WAL).
    let f = Fixture::new();
    let stats = crate::Stats::start(&f.dir, share()).unwrap();
    stats.record_at(W42_START + HOUR, StatEvent::StatsTruncated, 5);
    stats.flush();
    let out = f.tick(W42_END);
    assert_eq!(out.assembled.len(), 1);
    let report = report_of(&f.dir.join("pending/2026-W42.json"));
    let truncated = report
        .entries
        .iter()
        .find(|e| e.m == "stats.truncated")
        .unwrap();
    assert_eq!(truncated.v, Some(7));
}
