//! AJ-E1102: usage statistics in the app (D-19, ADR-0015).
//!
//! The choice is stored per Mac as `usage_stats`
//! (`{mode, decided_at_ms, consent_version}`; never in a backup). At
//! launch the overrides are applied and, unless the result is off, the
//! recorder starts. Turning stats off deletes the whole `stats/` folder.
//! Built without the `stats` feature, the crate isn't in the binary and
//! every answer says "not included in this build".

// Tauri command signatures take State/String by value (as in commands.rs).
#![allow(clippy::needless_pass_by_value)]

use std::path::PathBuf;
use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// The setting key.
pub const SETTING_KEY: &str = "usage_stats";
/// The consent text's version (bump it to ask again — AJ-E1105).
pub const CONSENT_VERSION: u32 = 1;

/// The stored choice.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UsageStatsChoice {
    /// `off` | `local` | `share`.
    pub mode: String,
    /// When the user chose, epoch ms (`None`: never asked).
    pub decided_at_ms: Option<i64>,
    /// The consent version they answered.
    pub consent_version: u32,
}

/// What Settings ▸ Privacy shows.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageStatsInfo {
    /// This build includes usage statistics.
    pub included: bool,
    /// The user's choice: `off` | `local` | `share`.
    pub chosen: String,
    /// What applies after the overrides.
    pub effective: String,
    /// Reports may be uploaded.
    pub can_upload: bool,
    /// Why `effective` differs from `chosen`, in plain words.
    pub reason: Option<String>,
    /// When the user chose, epoch ms.
    #[ts(type = "number | null")]
    pub decided_at_ms: Option<i64>,
    /// The consent version they answered (0: never asked).
    pub consent_version: u32,
    /// The current consent version.
    pub current_consent_version: u32,
}

/// The result of changing the mode.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct UsageStatsSetResult {
    /// The new state.
    pub info: UsageStatsInfo,
    /// What happened, in plain words.
    pub message: String,
}

/// The app's `stats/` folder.
#[must_use]
pub fn stats_dir(state: &AppState) -> PathBuf {
    state.data_dir.join("stats")
}

/// The stored choice (off when unset or unreadable).
#[must_use]
pub fn choice(state: &Arc<AppState>) -> UsageStatsChoice {
    let stored = state
        .storage
        .lock()
        .ok()
        .and_then(|storage| storage.app_settings().get(SETTING_KEY).ok().flatten())
        .and_then(|raw| serde_json::from_str::<UsageStatsChoice>(&raw).ok());
    match stored {
        Some(choice) if matches!(choice.mode.as_str(), "off" | "local" | "share") => choice,
        _ => UsageStatsChoice {
            mode: "off".to_owned(),
            ..UsageStatsChoice::default()
        },
    }
}

fn save(state: &Arc<AppState>, choice: &UsageStatsChoice) -> AppResult<()> {
    let raw = serde_json::to_string(choice).map_err(|e| AppError::Io {
        context: "usage stats",
        message: e.to_string(),
    })?;
    let storage = state.storage.lock().map_err(|_| AppError::StatePoisoned)?;
    storage.app_settings().set(SETTING_KEY, &raw)?;
    Ok(())
}

/// The parts that exist only in builds with the `stats` feature.
#[cfg(feature = "stats")]
pub mod enabled {
    use super::{AppError, AppResult, AppState, Arc, UsageStatsChoice, UsageStatsInfo};
    use juggler_stats::cycle::{Context, Endpoints, Response, Transport};
    use juggler_stats::report::{Arch, Channel, Envelope, OsName};
    use juggler_stats::{Environment, Mode, effective};

    /// The stored mode string as a [`Mode`] (unknown → off).
    #[must_use]
    pub fn mode_of(raw: &str) -> Mode {
        match raw {
            "local" => Mode::Local,
            "share" => Mode::Share,
            _ => Mode::Off,
        }
    }

    /// What Settings shows for `choice` under `env`.
    #[must_use]
    pub fn info(choice: &UsageStatsChoice, env: &Environment) -> UsageStatsInfo {
        let applied = effective(mode_of(&choice.mode), env);
        UsageStatsInfo {
            included: true,
            chosen: choice.mode.clone(),
            effective: applied.mode.wire().to_owned(),
            can_upload: applied.can_upload,
            reason: applied.reason.map(|r| r.words().to_owned()),
            decided_at_ms: choice.decided_at_ms,
            consent_version: choice.consent_version,
            current_consent_version: super::CONSENT_VERSION,
        }
    }

    fn io(e: impl std::fmt::Display) -> AppError {
        AppError::Io {
            context: "usage stats",
            message: e.to_string(),
        }
    }

    /// Starts (or stops) the recorder for the current choice, and notes
    /// the mode at once (a gap in sharing makes that week partial).
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Io`] when the store can't be opened.
    pub fn apply(state: &Arc<AppState>, env: &Environment) -> AppResult<()> {
        let applied = effective(mode_of(&super::choice(state).mode), env);
        let dir = super::stats_dir(state);
        if applied.mode == Mode::Off {
            juggler_stats::shutdown();
            return Ok(());
        }
        juggler_stats::install(juggler_stats::Stats::start(&dir, applied).map_err(io)?);
        juggler_stats::note_mode(&dir, applied).map_err(io)
    }

    /// What every report says about the app and the OS.
    #[must_use]
    pub fn envelope() -> Envelope {
        Envelope {
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            channel: if crate::updates::CHANNEL == "stable" {
                Channel::Stable
            } else {
                Channel::Beta
            },
            os: if cfg!(target_os = "macos") {
                OsName::Macos
            } else if cfg!(target_os = "windows") {
                OsName::Windows
            } else {
                OsName::Linux
            },
            os_major: sysinfo::System::os_version()
                .and_then(|v| v.split('.').next()?.trim().parse().ok())
                .unwrap_or(0),
            arch: if std::env::consts::ARCH == "aarch64" {
                Arch::Arm64
            } else {
                Arch::X86_64
            },
        }
    }

    /// The two stats requests over the account client's transport:
    /// `User-Agent: AIJuggler/<version>`, the platform trust store, no
    /// cookies, no redirects, 10 s timeout.
    pub struct StatsTransport(pub Arc<dyn juggler_account::Transport>);

    impl StatsTransport {
        /// The production transport.
        #[must_use]
        pub fn production() -> Self {
            Self(Arc::new(juggler_account::UreqTransport::with_timeout(
                env!("CARGO_PKG_VERSION"),
                std::time::Duration::from_secs(10),
            )))
        }

        fn send(&self, request: &juggler_account::HttpRequest) -> Result<Response, String> {
            self.0
                .send(request)
                .map(|r| Response {
                    status: r.status,
                    retry_after: r.header("retry-after").map(str::to_owned),
                    body: r.body,
                })
                .map_err(|e| e.0)
        }
    }

    impl Transport for StatsTransport {
        fn get(&self, url: &str) -> Result<Response, String> {
            self.send(&juggler_account::HttpRequest {
                method: "GET",
                url: url.to_owned(),
                bearer: None,
                content_type: "application/json",
                headers: Vec::new(),
                body: Vec::new(),
                long_transfer: false,
            })
        }

        fn post_gzip_json(&self, url: &str, body: &[u8]) -> Result<Response, String> {
            self.send(&juggler_account::HttpRequest {
                method: "POST",
                url: url.to_owned(),
                bearer: None,
                content_type: "application/json",
                headers: vec![("content-encoding", "gzip".to_owned())],
                body: body.to_vec(),
                long_transfer: false,
            })
        }
    }

    /// One tick of the weekly cycle (close weeks, upload at most one).
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Io`] on storage failures.
    pub fn tick(
        state: &Arc<AppState>,
        env: &Environment,
        envelope: &Envelope,
        transport: &dyn Transport,
        now_ms: i64,
    ) -> AppResult<juggler_stats::cycle::TickOutcome> {
        let choice = super::choice(state);
        let applied = effective(mode_of(&choice.mode), env);
        if let Some(recorder) = juggler_stats::current() {
            recorder.flush();
        }
        let endpoints = Endpoints::for_build(env);
        // AJ-E1103 supplies the environment gauges.
        let gauges = Vec::<juggler_stats::StatGauge>::new;
        let ctx = Context {
            effective: applied,
            consent_current: choice.consent_version == super::CONSENT_VERSION,
            consent_version: choice.consent_version,
            envelope,
            endpoints: endpoints.as_ref(),
            transport,
            now_ms,
            gauges: &gauges,
            pow_budget: juggler_stats::pow::BUDGET,
        };
        juggler_stats::cycle::tick(&super::stats_dir(state), &ctx).map_err(io)
    }

    /// The tick at launch, then hourly, on its own lowest-priority
    /// background thread (a puzzle, when the service asks for one, is
    /// solved there). In `off` each tick returns at once and touches
    /// nothing.
    pub fn spawn_ticks(state: Arc<AppState>) {
        let _ = std::thread::Builder::new()
            .name("usage-stats-tick".into())
            .spawn(move || {
                let _ = thread_priority::set_current_thread_priority(
                    thread_priority::ThreadPriority::Min,
                );
                let env = Environment::from_process();
                let transport = StatsTransport::production();
                let envelope = envelope();
                loop {
                    let now = juggler_core::model::Timestamp::now().millis();
                    if let Err(e) = tick(&state, &env, &envelope, &transport, now) {
                        eprintln!("ai-juggler: {e}");
                    }
                    std::thread::sleep(std::time::Duration::from_secs(3_600));
                }
            });
    }
}

/// Core: the current choice and what applies.
#[must_use]
pub fn usage_stats_info_impl(state: &Arc<AppState>) -> UsageStatsInfo {
    let choice = choice(state);
    #[cfg(feature = "stats")]
    {
        enabled::info(&choice, &juggler_stats::Environment::from_process())
    }
    #[cfg(not(feature = "stats"))]
    {
        UsageStatsInfo {
            included: false,
            chosen: choice.mode,
            effective: "off".to_owned(),
            can_upload: false,
            reason: Some("Not included in this build".to_owned()),
            decided_at_ms: choice.decided_at_ms,
            consent_version: choice.consent_version,
            current_consent_version: CONSENT_VERSION,
        }
    }
}

/// Core: records the user's choice and applies it. Choosing `off` stops
/// the recorder and deletes the whole `stats/` folder.
///
/// # Errors
///
/// Returns [`AppError::Validation`] for an unknown mode or a build
/// without usage statistics, and [`AppError::Io`] when the folder can't
/// be deleted or the store can't be opened.
pub fn usage_stats_set_impl(state: &Arc<AppState>, mode: &str) -> AppResult<UsageStatsSetResult> {
    if !matches!(mode, "off" | "local" | "share") {
        return Err(AppError::Validation {
            subject: "usage stats",
            message: format!("unknown mode {mode:?}"),
        });
    }
    if !cfg!(feature = "stats") && mode != "off" {
        return Err(AppError::Validation {
            subject: "usage stats",
            message: "usage statistics are not included in this build".to_owned(),
        });
    }
    save(
        state,
        &UsageStatsChoice {
            mode: mode.to_owned(),
            decided_at_ms: Some(juggler_core::model::Timestamp::now().millis()),
            consent_version: CONSENT_VERSION,
        },
    )?;
    let message = if mode == "off" {
        delete_local(state)?;
        "Usage stats are off. Local stats deleted.".to_owned()
    } else {
        #[cfg(feature = "stats")]
        enabled::apply(state, &juggler_stats::Environment::from_process())?;
        if mode == "local" {
            "Usage stats are counted on this Mac only.".to_owned()
        } else {
            "Usage stats are on. One small report a week will be sent.".to_owned()
        }
    };
    Ok(UsageStatsSetResult {
        info: usage_stats_info_impl(state),
        message,
    })
}

/// Stops the recorder and deletes the whole `stats/` folder.
///
/// # Errors
///
/// Returns [`AppError::Io`] when the folder can't be deleted.
pub fn delete_local(state: &Arc<AppState>) -> AppResult<()> {
    let dir = stats_dir(state);
    #[cfg(feature = "stats")]
    let deleted = juggler_stats::delete_all(&dir);
    #[cfg(not(feature = "stats"))]
    let deleted = match std::fs::remove_dir_all(&dir) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    };
    deleted.map_err(|e| AppError::Io {
        context: "usage stats",
        message: e.to_string(),
    })
}

/// At launch: start the recorder if the choice (after the overrides)
/// isn't off. Off creates nothing.
pub fn start(state: Arc<AppState>) {
    #[cfg(feature = "stats")]
    {
        if let Err(e) = enabled::apply(&state, &juggler_stats::Environment::from_process()) {
            eprintln!("ai-juggler: {e}");
        }
        enabled::spawn_ticks(state);
    }
    #[cfg(not(feature = "stats"))]
    let _ = state;
}

/// On quit: write what was counted.
pub fn shutdown() {
    #[cfg(feature = "stats")]
    juggler_stats::shutdown();
}

/// AJ-E1102: the usage-stats choice and what applies.
#[tauri::command]
#[must_use]
pub fn usage_stats_info(state: tauri::State<'_, Arc<AppState>>) -> UsageStatsInfo {
    usage_stats_info_impl(state.inner())
}

/// AJ-E1102: sets the usage-stats mode (`off` deletes local stats).
///
/// # Errors
///
/// Per [`usage_stats_set_impl`].
#[tauri::command]
pub fn usage_stats_set(
    state: tauri::State<'_, Arc<AppState>>,
    mode: String,
) -> AppResult<UsageStatsSetResult> {
    usage_stats_set_impl(state.inner(), &mode)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn state() -> (tempfile::TempDir, Arc<AppState>) {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(AppState::init(dir.path().join("data")).unwrap());
        (dir, state)
    }

    #[test]
    fn default_is_off_and_creates_nothing() {
        let (_dir, state) = state();
        let info = usage_stats_info_impl(&state);
        assert_eq!(info.chosen, "off");
        assert_eq!(info.effective, "off");
        assert_eq!(info.consent_version, 0);
        start(Arc::clone(&state));
        assert!(!stats_dir(&state).exists());
    }

    #[cfg(feature = "stats")]
    #[test]
    fn choosing_off_deletes_the_folder_and_says_so() {
        let (_dir, state) = state();
        let on = usage_stats_set_impl(&state, "local").unwrap();
        assert_eq!(on.info.chosen, "local");
        assert_eq!(on.info.consent_version, CONSENT_VERSION);
        assert!(stats_dir(&state).join("counters.sqlite").exists());
        let off = usage_stats_set_impl(&state, "off").unwrap();
        assert_eq!(off.message, "Usage stats are off. Local stats deleted.");
        assert!(!stats_dir(&state).exists());
        assert!(usage_stats_set_impl(&state, "sometimes").is_err());
    }

    /// A local HTTP server answering `responses` in order; returns each
    /// raw request (head and body).
    #[cfg(feature = "stats")]
    fn serve(responses: Vec<&'static str>) -> (String, std::thread::JoinHandle<Vec<Vec<u8>>>) {
        use std::io::{Read as _, Write as _};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let mut seen = Vec::new();
            for response in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut raw = Vec::new();
                let mut buffer = [0_u8; 8192];
                let head_end = loop {
                    let read = stream.read(&mut buffer).unwrap();
                    raw.extend_from_slice(&buffer[..read]);
                    if let Some(at) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                        break at + 4;
                    }
                    assert!(read > 0, "connection closed mid-request");
                };
                let head = String::from_utf8_lossy(&raw[..head_end]).to_ascii_lowercase();
                let length: usize = head
                    .lines()
                    .find_map(|l| l.strip_prefix("content-length:"))
                    .map_or(0, |v| v.trim().parse().unwrap());
                while raw.len() < head_end + length {
                    let read = stream.read(&mut buffer).unwrap();
                    raw.extend_from_slice(&buffer[..read]);
                }
                stream.write_all(response.as_bytes()).unwrap();
                seen.push(raw);
            }
            seen
        });
        (base, handle)
    }

    #[cfg(feature = "stats")]
    fn http(status: &str, body: &str) -> &'static str {
        Box::leak(
            format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .into_boxed_str(),
        )
    }

    /// AJ-E1104/E1108: a debug build sends to the local endpoint only —
    /// config first, then the gzip report with the user agent and no
    /// cookies — and files it under `sent/`.
    #[cfg(feature = "stats")]
    #[test]
    fn a_debug_build_uploads_to_the_local_endpoint_with_the_contract_headers() {
        use std::io::Read as _;
        let (_dir, state) = state();
        usage_stats_set_impl(&state, "share").unwrap();
        let config = r#"{"enabled":true,"sample_rate":1.0,"schemas":[1],"min_app_version":"0.0.1","pow_bits":0}"#;
        let (base, server) = serve(vec![
            http("200 OK", config),
            http("202 Accepted", r#"{"ok":true}"#),
        ]);
        let env = juggler_stats::Environment {
            debug_build: true,
            endpoint: Some(format!("{base}/v1/reports")),
            ..juggler_stats::Environment::default()
        };
        // Sharing since before last week, which has a counter.
        let dir = stats_dir(&state);
        let now = juggler_core::model::Timestamp::now().millis();
        let last = juggler_stats::week::label(now - 7 * 86_400_000);
        let store = juggler_stats::Store::open(&dir).unwrap();
        store.set_meta("share_since", "0").unwrap();
        store.add(&last, "stats.truncated", "{}", 4).unwrap();
        let start = juggler_stats::week::start_ms(&last).unwrap();
        let (y, m, d) = juggler_stats::week::civil(start.div_euclid(86_400_000));
        store.mark_active(&format!("{y:04}-{m:02}-{d:02}")).unwrap();
        let transport = enabled::StatsTransport::production();
        // Just after the week closed: assembled, not yet due.
        let closed = juggler_stats::week::end_ms(&last).unwrap() + 1;
        let first = enabled::tick(&state, &env, &enabled::envelope(), &transport, closed).unwrap();
        assert!(first.uploads.is_empty(), "{first:?}");
        assert_eq!(first.assembled.len(), 1, "{first:?}");
        store
            .set_meta(&format!("upload_at:{last}"), &(now - 1).to_string())
            .unwrap();
        let second = enabled::tick(&state, &env, &enabled::envelope(), &transport, now).unwrap();
        assert_eq!(
            second.uploads,
            vec![(last.clone(), juggler_stats::cycle::UploadOutcome::Sent)]
        );
        let requests = server.join().unwrap();
        let get = String::from_utf8_lossy(&requests[0]).to_ascii_lowercase();
        assert!(get.starts_with("get /v1/config.json http/1.1"), "{get}");
        let post = &requests[1];
        let head_end = post.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
        let head = String::from_utf8_lossy(&post[..head_end]).to_ascii_lowercase();
        assert!(head.starts_with("post /v1/reports http/1.1"), "{head}");
        let agent = format!("user-agent: aijuggler/{}", env!("CARGO_PKG_VERSION"));
        assert!(head.contains(&agent), "{head}");
        assert!(get.contains(&agent), "{get}");
        assert!(head.contains("content-type: application/json"), "{head}");
        assert!(head.contains("content-encoding: gzip"), "{head}");
        assert!(!head.contains("cookie"), "{head}");
        let mut json = Vec::new();
        flate2::read::GzDecoder::new(&post[head_end..])
            .read_to_end(&mut json)
            .unwrap();
        assert_eq!(
            json,
            std::fs::read(dir.join("sent").join(format!("{last}.json"))).unwrap()
        );
        let report: serde_json::Value = serde_json::from_slice(&json).unwrap();
        assert_eq!(report["app"]["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(report["entries"][0]["m"], "stats.truncated");
        delete_local(&state).unwrap();
    }

    /// AJ-E1104 evidence against a running stats server — a local
    /// development server, never production from a test. Opt-in:
    /// `AIJ_E2E_STATS_URL=http://localhost:8790 cargo test -p juggler-app
    /// --lib usage_stats::tests::live -- --ignored --nocapture`.
    #[cfg(feature = "stats")]
    #[test]
    #[ignore = "needs a local stats server (AIJ_E2E_STATS_URL)"]
    fn live_stats_server_accepts_a_report() {
        use juggler_stats::cycle::Transport as _;
        let base = std::env::var("AIJ_E2E_STATS_URL").expect("AIJ_E2E_STATS_URL");
        assert!(
            base.contains("localhost") || base.contains("127.0.0.1"),
            "local servers only"
        );
        let (_dir, state) = state();
        usage_stats_set_impl(&state, "share").unwrap();
        let env = juggler_stats::Environment {
            debug_build: true,
            endpoint: Some(format!("{base}/v1/reports")),
            ..juggler_stats::Environment::default()
        };
        let dir = stats_dir(&state);
        let now = juggler_core::model::Timestamp::now().millis();
        let last = juggler_stats::week::label(now - 7 * 86_400_000);
        let store = juggler_stats::Store::open(&dir).unwrap();
        store.set_meta("share_since", "0").unwrap();
        let start = juggler_stats::week::start_ms(&last).unwrap();
        for day in 0..3 {
            let (y, m, d) = juggler_stats::week::civil(start.div_euclid(86_400_000) + day);
            store.mark_active(&format!("{y:04}-{m:02}-{d:02}")).unwrap();
        }
        store.add(&last, "app.launch", "{}", 3).unwrap();
        store
            .add(
                &last,
                "switch.duration_s",
                r#"{"bucket":"15-60","kind":"provider"}"#,
                2,
            )
            .unwrap();
        let transport = enabled::StatsTransport::production();
        // This workspace is 0.0.1, below the service's minimum; present a
        // release-shaped version (the app's own is skipped as `version`).
        let envelope = juggler_stats::report::Envelope {
            app_version: "0.1.0".to_owned(),
            ..enabled::envelope()
        };
        let closed = juggler_stats::week::end_ms(&last).unwrap() + 1;
        let first = enabled::tick(&state, &env, &envelope, &transport, closed).unwrap();
        println!("assembled: {:?}", first.assembled);
        store
            .set_meta(&format!("upload_at:{last}"), &(now - 1).to_string())
            .unwrap();
        let second = enabled::tick(&state, &env, &envelope, &transport, now).unwrap();
        println!("uploads: {:?}", second.uploads);
        assert_eq!(
            second.uploads,
            vec![(last.clone(), juggler_stats::cycle::UploadOutcome::Sent)]
        );
        let sent = std::fs::read(dir.join("sent").join(format!("{last}.json"))).unwrap();
        println!("report: {}", String::from_utf8_lossy(&sent));
        // The same report again is a duplicate, not a second report.
        let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut gzip, &sent).unwrap();
        let again = transport
            .post_gzip_json(&format!("{base}/v1/reports"), &gzip.finish().unwrap())
            .unwrap();
        println!(
            "again: {} {}",
            again.status,
            String::from_utf8_lossy(&again.body)
        );
        assert_eq!(again.status, 200);
        delete_local(&state).unwrap();
    }

    /// AJ-E1108: the tick thread (where a puzzle is solved) can drop to
    /// the lowest priority without privileges.
    #[cfg(feature = "stats")]
    #[test]
    fn the_tick_thread_runs_at_the_lowest_priority() {
        let lowered = std::thread::spawn(|| {
            thread_priority::set_current_thread_priority(thread_priority::ThreadPriority::Min)
        })
        .join()
        .unwrap();
        assert!(lowered.is_ok(), "{lowered:?}");
    }

    /// Off and local make zero stats requests, whatever is pending.
    #[cfg(feature = "stats")]
    #[test]
    fn off_and_local_send_nothing() {
        let (_dir, state) = state();
        let recording = Arc::new(juggler_account::RecordingTransport::default());
        let transport = enabled::StatsTransport(recording.clone());
        let release = juggler_stats::Environment::default();
        let now = juggler_core::model::Timestamp::now().millis();
        enabled::tick(&state, &release, &enabled::envelope(), &transport, now).unwrap();
        assert!(!stats_dir(&state).exists());
        usage_stats_set_impl(&state, "local").unwrap();
        let dir = stats_dir(&state);
        let last = juggler_stats::week::label(now - 7 * 86_400_000);
        juggler_stats::Store::open(&dir)
            .unwrap()
            .add(&last, "stats.truncated", "{}", 1)
            .unwrap();
        let out = enabled::tick(&state, &release, &enabled::envelope(), &transport, now).unwrap();
        assert_eq!(
            out.assembled[0].1,
            juggler_stats::cycle::Placement::Skipped("local".into())
        );
        assert_eq!(recording.count(), 0);
        delete_local(&state).unwrap();
    }

    #[cfg(feature = "stats")]
    #[test]
    fn overrides_show_their_reason() {
        let choice = UsageStatsChoice {
            mode: "share".to_owned(),
            decided_at_ms: Some(1),
            consent_version: 1,
        };
        let env = juggler_stats::Environment {
            do_not_track: Some("1".to_owned()),
            ..juggler_stats::Environment::default()
        };
        let info = enabled::info(&choice, &env);
        assert_eq!(info.effective, "off");
        assert_eq!(
            info.reason.as_deref(),
            Some("Off because DO_NOT_TRACK is set")
        );
    }
}
