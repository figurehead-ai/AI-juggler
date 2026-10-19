use super::*;

fn share() -> EffectiveMode {
    effective(Mode::Share, &Environment::default())
}

fn off() -> EffectiveMode {
    effective(Mode::Off, &Environment::default())
}

/// 2026-10-14 12:00 UTC (a Wednesday of 2026-W42).
const WEDNESDAY: i64 = 1_791_979_200_000;

#[test]
fn off_creates_no_folder_and_records_nothing() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("stats");
    let stats = Stats::start(&dir, off()).unwrap();
    stats.record(StatEvent::StatsDropped);
    stats.set_gauge(StatGauge::examples()[0]);
    stats.flush();
    stats.stop();
    assert!(!dir.exists(), "off must not create the stats folder");
}

#[test]
fn counters_gauges_histograms_and_days_reach_the_store() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("stats");
    let stats = Stats::start(&dir, share()).unwrap();
    for _ in 0..3 {
        stats.record_at(WEDNESDAY, StatEvent::StatsTruncated, 1);
    }
    stats.record_at(WEDNESDAY, StatEvent::StatsTruncated, 4);
    let histogram = StatEvent::examples()
        .into_iter()
        .find(|e| e.metric() == "switch.duration_s")
        .unwrap();
    let StatEvent::SwitchDurationS { kind, .. } = histogram else {
        unreachable!()
    };
    stats.record_at(
        WEDNESDAY,
        StatEvent::SwitchDurationS { kind, value: 22.0 },
        1,
    );
    let gauge = StatGauge::examples()[0];
    stats.set_gauge_at(WEDNESDAY, gauge);
    stats.flush();
    stats.stop();

    let store = Store::open(&dir).unwrap();
    let counters = store.counters("2026-W42").unwrap();
    assert!(counters.contains(&("stats.truncated".into(), "{}".into(), 7)));
    let switch = counters
        .iter()
        .find(|c| c.0 == "switch.duration_s")
        .unwrap();
    assert_eq!(
        switch.1,
        format!("{{\"bucket\":\"15-60\",\"kind\":\"{}\"}}", kind.wire())
    );
    let gauges = store.gauges("2026-W42").unwrap();
    assert_eq!(gauges.len(), 1);
    assert_eq!(gauges[0].0, gauge.metric());
    assert_eq!(gauges[0].2, gauge.value());
    assert_eq!(store.active_days("2026-10-12", "2026-10-18").unwrap(), 1);
    assert_eq!(
        store.salt().unwrap().len(),
        32,
        "a salt exists once stats are on"
    );
}

#[cfg(unix)]
#[test]
fn the_folder_is_private() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("stats");
    drop(Stats::start(&dir, share()).unwrap());
    let mode = std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o700);
}

#[test]
fn a_full_channel_drops_and_counts() {
    let root = tempfile::tempdir().unwrap();
    // No worker drains this channel, so it fills after two.
    let (tx, _rx) = std::sync::mpsc::sync_channel(2);
    let stats = Stats {
        effective: share(),
        dir: root.path().to_path_buf(),
        tx: Some(tx),
        dropped: Arc::new(AtomicU64::new(0)),
        worker: Mutex::new(None),
    };
    for _ in 0..5 {
        stats.record(StatEvent::StatsTruncated);
    }
    assert_eq!(stats.dropped.load(Ordering::Relaxed), 3);
}

#[test]
fn drops_become_the_stats_dropped_counter() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("stats");
    let stats = Stats::start(&dir, share()).unwrap();
    stats.dropped.store(7, Ordering::Relaxed);
    stats.flush();
    stats.stop();
    let store = Store::open(&dir).unwrap();
    let week = week::label(now_ms());
    assert!(
        store
            .counters(&week)
            .unwrap()
            .contains(&("stats.dropped".into(), "{}".into(), 7))
    );
}

#[test]
fn delete_all_removes_everything() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("stats");
    install(Stats::start(&dir, share()).unwrap());
    record(StatEvent::StatsTruncated);
    current().unwrap().flush();
    assert!(dir.join("counters.sqlite").exists());
    delete_all(&dir).unwrap();
    assert!(!dir.exists());
    assert!(current().is_none());
    // Recording after delete is a silent no-op.
    record(StatEvent::StatsTruncated);
    assert!(!dir.exists());
    delete_all(&dir).unwrap();
}

#[test]
fn record_is_under_a_microsecond() {
    let root = tempfile::tempdir().unwrap();
    let stats = Stats::start(&root.path().join("stats"), share()).unwrap();
    let mut samples = Vec::with_capacity(2000);
    for _ in 0..2000 {
        let started = Instant::now();
        for _ in 0..10 {
            stats.record(StatEvent::StatsTruncated);
        }
        samples.push(started.elapsed().as_nanos() / 10);
    }
    samples.sort_unstable();
    let median = samples[samples.len() / 2];
    let off = Stats::disabled(root.path(), off());
    let started = Instant::now();
    for _ in 0..10_000 {
        off.record(StatEvent::StatsTruncated);
    }
    let off_ns = started.elapsed().as_nanos() / 10_000;
    println!("record(): median {median} ns per event (share); {off_ns} ns per event (off)");
    assert!(median < 1_000, "record() median {median} ns ≥ 1 µs");
}

#[test]
fn dims_keys_are_canonical_json() {
    assert_eq!(dims_key(&[]), "{}");
    assert_eq!(
        dims_key(&[("provider", "codex"), ("kind", "a\"b")]),
        r#"{"kind":"a\"b","provider":"codex"}"#
    );
}

#[test]
fn utc_days() {
    assert_eq!(utc_day(WEDNESDAY), "2026-10-14");
    assert_eq!(utc_day(0), "1970-01-01");
}

#[test]
fn a_week_of_counters_flushes_in_under_5_ms() {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(&root.path().join("stats")).unwrap();
    // Every typed metric on each of the 7 days, plus each histogram's
    // buckets: more distinct rows than a real week has. The budget is
    // what the flush costs the app — its thread's CPU time. Wall time is
    // printed too; it adds the disk's write latency, which another
    // program saturating the disk can stretch.
    let mut best_cpu = Duration::MAX;
    let mut best_wall = Duration::MAX;
    let mut rows = 0;
    for _ in 0..5 {
        let mut pending = Pending::default();
        for day in 0..7 {
            let at_ms = WEDNESDAY + (day - 2) * 86_400_000;
            for event in StatEvent::examples() {
                pending.take(Message::Event {
                    at_ms,
                    event,
                    by: 3,
                });
            }
            for gauge in StatGauge::examples() {
                pending.take(Message::Gauge { at_ms, gauge });
            }
        }
        rows = pending.counters.len() + pending.gauges.len();
        let (cpu, wall) = (thread_cpu(), Instant::now());
        pending.write(&store).unwrap();
        best_wall = best_wall.min(wall.elapsed());
        best_cpu = best_cpu.min(thread_cpu().saturating_sub(cpu));
        assert!(pending.is_empty());
    }
    println!(
        "flush: {rows} rows in {} µs CPU, {} µs wall (best of 5)",
        best_cpu.as_micros(),
        best_wall.as_micros()
    );
    assert!(
        best_cpu < Duration::from_millis(5),
        "flush took {best_cpu:?} CPU"
    );
}

/// This thread's CPU time.
fn thread_cpu() -> Duration {
    cpu_time::ThreadTime::now().as_duration()
}
