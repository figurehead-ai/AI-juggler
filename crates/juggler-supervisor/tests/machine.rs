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
        sampler.tick_with(i * 2_000, || vec![std::env::temp_dir()]);
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
    // Used and free on one basis: they add up to the Mac's memory.
    let used = latest.memory_used.expect("total − available off the host");
    assert_eq!(used + latest.memory_free(), latest.memory_total);
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
        sampler.tick_with(i * 2_000, || vec![std::env::temp_dir()]);
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
/// The budget is 2 ms p95 of this thread's CPU time (wall time on a busy
/// machine counts other programs' turns: 2.6 ms at load 21, where alone it
/// reads well under). Wall time is printed beside it. Under the lock the UI
/// also reads: its CPU p95 inside the same budget, and never the disk
/// listing (50–150 ms), which the wall-time maximum catches.
#[test]
fn tick_cost_stays_inside_the_budget() {
    use cpu_time::ThreadTime;
    let _serial = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut sampler = MachineSampler::new();
    // The first tick lists the disks (50–150 ms, once per 10 minutes);
    // the budget is the steady state.
    sampler.tick_with(0, || vec![std::env::temp_dir()]);
    let mut costs: Vec<u128> = Vec::new();
    let mut walls: Vec<u128> = Vec::new();
    let mut locked: Vec<u128> = Vec::new();
    let mut locked_wall: Vec<u128> = Vec::new();
    for i in 0..60 {
        let started = std::time::Instant::now();
        let cpu = ThreadTime::now();
        // What runs under the lock the UI also reads …
        let work = sampler.tick((i + 1) * 2_000);
        locked.push(cpu.elapsed().as_micros());
        locked_wall.push(started.elapsed().as_micros());
        // … and the disk half, done outside it.
        if let Some(mut work) = work {
            let volumes = work.volumes(&[std::env::temp_dir()]);
            sampler.finish_disks(work, volumes);
        }
        costs.push(cpu.elapsed().as_micros());
        walls.push(started.elapsed().as_micros());
    }
    costs.sort_unstable();
    walls.sort_unstable();
    locked_wall.sort_unstable();
    locked.sort_unstable();
    let p95 = costs[costs.len() * 95 / 100];
    let median = costs[costs.len() / 2];
    let locked_max = *locked_wall.last().unwrap();
    let locked_p95 = locked[locked.len() * 95 / 100];
    let wall_p95 = walls[walls.len() * 95 / 100];
    println!(
        "machine tick: CPU median {median} µs, p95 {p95} µs over 60 ticks (wall p95 {wall_p95} µs); under the lock p95 {locked_p95} µs, max {locked_max} µs"
    );
    assert!(p95 < 2_000, "p95 {p95} µs");
    // Never the disk listing (50–150 ms) under the lock; the max allows
    // scheduler jitter on a loaded machine.
    assert!(locked_p95 < 2_000, "lock held p95 {locked_p95} µs");
    assert!(locked_max < 20_000, "lock held {locked_max} µs");
}

/// Review of #67: a folder on a disk that isn't listed (here, a
/// `/Volumes/` path that isn't mounted) rebuilds the disk list early at
/// most every 5 minutes (150 ticks), never on every 30 s disk tick, and is
/// never credited to the startup disk.
#[test]
fn an_unknown_volume_rebuilds_the_list_at_most_every_five_minutes() {
    let _serial = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut sampler = MachineSampler::new();
    let unknown = std::path::PathBuf::from("/Volumes/E14-test-never-mounted/repo");
    for tick in 1..=160 {
        sampler.tick_with(i64::from(tick) * 2_000, || vec![unknown.clone()]);
    }
    assert_eq!(
        sampler.relists(),
        1,
        "only the first list in the first 5 minutes"
    );
    assert!(
        sampler.snapshot().volumes.is_empty(),
        "never credited to the startup disk"
    );
    for tick in 161..=170 {
        sampler.tick_with(i64::from(tick) * 2_000, || vec![unknown.clone()]);
    }
    assert_eq!(sampler.relists(), 2, "one early rebuild after 5 minutes");
}
