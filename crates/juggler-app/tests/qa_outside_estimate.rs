//! QA of AJ-E1538's size estimate on a real setup: how far
//! `outside_search_estimate` is from the index it predicts.
//!
//! Ignored, and a no-op unless `AIJ_QA_ESTIMATE_DATA` and
//! `AIJ_QA_ESTIMATE_OUT` are set. `AIJ_QA_ESTIMATE_DATA` must be a
//! THROWAWAY COPY of an AI Juggler Dev data folder that
//! `qa_real_readonly` filled (never the real AI Juggler's data, never the
//! Dev folder itself). The test reads the estimate, turns outside search
//! on in that copy, builds the whole index (reading the tools' files
//! read-only, as the app does), measures it, and turns it off again,
//! which deletes the index file. Delete the copy afterwards.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use juggler_app::features::outside_search::{estimate_impl, pass, set_impl, status_impl};
use juggler_app::state::AppState;

#[test]
#[ignore = "QA on a real setup; needs AIJ_QA_ESTIMATE_DATA (a throwaway copy) and AIJ_QA_ESTIMATE_OUT"]
fn outside_search_estimate_against_the_built_index() {
    let (Some(data), Some(out)) = (
        std::env::var_os("AIJ_QA_ESTIMATE_DATA").map(PathBuf::from),
        std::env::var_os("AIJ_QA_ESTIMATE_OUT").map(PathBuf::from),
    ) else {
        eprintln!("AIJ_QA_ESTIMATE_DATA / AIJ_QA_ESTIMATE_OUT not set: nothing to do");
        return;
    };
    let name = data.file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        name.contains("qa-estimate-copy"),
        "AIJ_QA_ESTIMATE_DATA must be a throwaway copy named *qa-estimate-copy*"
    );
    let state = Arc::new(AppState::init(data).unwrap());
    // The conversations the app lists (and the estimate draws from) come
    // from its conversations tick, as when the app runs.
    let mut listed = 0;
    for _ in 0..10 {
        juggler_app::features::conversations::tick(&state).unwrap();
        let now = state.conversations.all().unwrap().len();
        if now == listed {
            break;
        }
        listed = now;
    }
    assert!(listed > 0, "no conversations listed: nothing to measure");

    let started = Instant::now();
    let estimate = estimate_impl(&state).unwrap();
    let estimate_took = started.elapsed();

    set_impl(&state, true).unwrap();
    let started = Instant::now();
    let mut passes = 0u32;
    while pass(&state, Duration::from_secs(30)).unwrap() {
        passes += 1;
        assert!(
            started.elapsed() < Duration::from_secs(3 * 3600),
            "the build ran past 3 hours"
        );
    }
    let build_took = started.elapsed();
    let measured = status_impl(&state).unwrap();
    let off = set_impl(&state, false).unwrap();

    let size = measured.size_bytes.unwrap_or(0);
    let report = serde_json::json!({
        "conversations_listed": listed,
        "estimate": {"size_bytes": estimate.size_bytes, "sessions": estimate.sessions,
                     "took_s": estimate_took.as_secs_f64()},
        "measured": {"size_bytes": size, "sessions": measured.sessions, "indexed": measured.indexed,
                     "passes": passes + 1, "took_s": build_took.as_secs_f64()},
        "ratio_estimate_to_measured": if size > 0 { estimate.size_bytes as f64 / size as f64 } else { 0.0 },
        "index_after_off": {"enabled": off.enabled, "size_bytes": off.size_bytes},
    });
    std::fs::write(&out, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    eprintln!("wrote {}", out.display());
}
