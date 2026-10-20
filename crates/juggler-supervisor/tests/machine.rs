//! AJ-E1401: the machine sampler on the real OS — one `System` for its
//! lifetime, real memory, CPU measured from the second tick. Its own test
//! binary: the construction counter is process-global.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use juggler_supervisor::machine::{OsReadings, PowerState, PressureLevel, ThermalState};
use juggler_supervisor::{MachineSampler, system_construction_count};

/// The construction counter is process-global: tests that build a
/// sampler take this lock so the count is exact.
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct Fixed;

impl OsReadings for Fixed {
    fn pressure(&self) -> Option<PressureLevel> {
        Some(PressureLevel::Warning)
    }
    fn power(&self) -> Option<PowerState> {
        None
    }
    fn thermal(&self) -> Option<ThermalState> {
        Some(ThermalState::Nominal)
    }
}

#[test]
fn the_sampler_constructs_one_system_and_reads_real_memory() {
    let _serial = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let before = system_construction_count();
    let mut sampler = MachineSampler::with_readings(Box::new(Fixed));
    for i in 0..4 {
        sampler.tick(i * 2_000, || vec![std::env::temp_dir()]);
    }
    assert_eq!(
        system_construction_count() - before,
        1,
        "one System for the sampler's lifetime; ticks construct none"
    );
    let snapshot = sampler.snapshot();
    let latest = snapshot.latest.unwrap();
    assert!(latest.memory_total > 0, "real memory total");
    assert!(latest.memory_available <= latest.memory_total);
    assert_eq!(snapshot.ring.len(), 4);
    assert_eq!(snapshot.ring[0].cpu_pct, None, "first CPU is unmeasured");
    assert!(
        snapshot.ring[3]
            .cpu_pct
            .is_some_and(|cpu| (0.0..=100.0).contains(&cpu))
    );
    assert_eq!(latest.pressure, Some(PressureLevel::Warning));
    assert!(!snapshot.volumes.is_empty(), "the temp dir's volume");
    assert!(snapshot.cores >= 1);
}

/// Evidence helper (AJ-E1401): one live reading on this Mac, printed.
/// `cargo test -p juggler-supervisor --test machine -- --ignored --nocapture`
#[test]
#[ignore = "prints a live reading for the evidence manifest"]
fn live_reading() {
    let _serial = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut sampler = MachineSampler::new();
    for i in 0..3 {
        sampler.tick(i * 2_000, || vec![std::env::temp_dir()]);
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    let snapshot = sampler.snapshot();
    let summary =
        juggler_supervisor::summarize(snapshot, &juggler_supervisor::RoomThresholds::default());
    println!(
        "latest: {}",
        serde_json::to_string(&snapshot.latest).unwrap()
    );
    println!(
        "volumes: {}",
        serde_json::to_string(&snapshot.volumes).unwrap()
    );
    println!("power: {}", serde_json::to_string(&snapshot.power).unwrap());
    println!(
        "thermal: {}",
        serde_json::to_string(&snapshot.thermal).unwrap()
    );
    println!("summary: {}", serde_json::to_string(&summary).unwrap());
}

/// AJ-E1401 budget: the machine half of the tick (CPU + memory + the
/// pressure read every tick; disks every 15th, power/thermal every 30th).
/// The budget is 2 ms p95; the assertion leaves room for a loaded CI box
/// running an unoptimised build, and the evidence records the real p95.
#[test]
fn tick_cost_stays_inside_the_budget() {
    let _serial = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut sampler = MachineSampler::new();
    // The first tick lists the disks (50–150 ms, once per 10 minutes);
    // the budget is the steady state.
    sampler.tick(0, || vec![std::env::temp_dir()]);
    let mut costs: Vec<u128> = (0..60)
        .map(|i| {
            let started = std::time::Instant::now();
            sampler.tick((i + 1) * 2_000, || vec![std::env::temp_dir()]);
            started.elapsed().as_micros()
        })
        .collect();
    costs.sort_unstable();
    let p95 = costs[costs.len() * 95 / 100];
    let median = costs[costs.len() / 2];
    println!("machine tick: median {median} µs, p95 {p95} µs over 60 ticks");
    assert!(p95 < 2_000, "p95 {p95} µs");
}
