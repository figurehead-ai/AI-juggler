//! AJ-E1408: runtime overload alerts — the Mac under memory pressure,
//! swapping hard, busy for minutes, hot, running out of disk, or a session
//! working it hard — told once, calmly, while it's happening.
//!
//! Passive only (PROPOSAL §6.4): an alert is a notification and a Needs
//! you entry, nothing more — no stop, pause or launch, at any tier. Each
//! kind fires at most once per 30 minutes, after its condition has held
//! for its hold time, and clears when the condition clears. Like the other
//! immediate alerts (a quota wall), it is not held for quiet hours — those
//! delay the hourly digest only.

use std::collections::HashMap;

use crate::machine_health::{ThisMacInfo, ToolTreeInfo};

/// One alert, as the UI shows it.
#[derive(Clone, PartialEq, Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct OverloadAlertInfo {
    /// Stable key (`memory_pressure`, `swap`, `cpu`, `thermal`,
    /// `disk:<volume>`, `heavy:<session>`).
    pub key: String,
    /// `memory_pressure` | `swap` | `cpu` | `thermal` | `disk` | `heavy`.
    pub kind: String,
    /// The alert in words ("Your Mac is under memory pressure").
    pub title: String,
    /// The measured fact behind it ("swap is growing — 240 MB a minute").
    pub detail: String,
    /// The biggest contributor, when there is one: a session id …
    pub session_id: Option<String>,
    /// … or a tool AI Juggler didn't start (`desktop` | `terminal`).
    pub place: Option<String>,
    /// The contributor's memory, bytes.
    #[ts(type = "number | null")]
    pub subject_bytes: Option<u64>,
    /// Whether `subject_bytes` is an estimate (a tool AI Juggler didn't
    /// start: its helpers share memory the sum counts more than once).
    pub subject_estimate: bool,
    /// The contributor's CPU, percent of one core.
    pub subject_cpu: Option<f64>,
    /// When the condition started, epoch ms.
    #[ts(type = "number")]
    pub since_ms: i64,
}

/// One kind fires at most this often.
pub const ALERT_EVERY_MS: i64 = 30 * 60_000;
/// Memory pressure must hold this long before it's told.
const PRESSURE_HOLD_MS: i64 = 30_000;
/// Swap must keep growing this long (on top of its one-minute rate).
const SWAP_HOLD_MS: i64 = 60_000;
/// Swap growth that counts as thrashing, MB a minute.
const SWAP_THRASH_MB_PER_MIN: f64 = 200.0;
/// Whole-Mac CPU above this …
const CPU_BUSY_PCT: f64 = 90.0;
/// … for this long.
const CPU_WINDOW_MS: i64 = 120_000;

/// A condition, as evaluated now.
#[derive(Debug, Clone)]
struct Condition {
    key: String,
    kind: &'static str,
    hold_ms: i64,
    title: String,
    detail: String,
    subject: Option<ToolTreeInfo>,
}

#[derive(Debug, Clone, Default)]
struct Track {
    since: Option<i64>,
    active: bool,
    last_fired: Option<i64>,
}

/// The alert state machine (one per app, inside the machine watch).
#[derive(Debug, Default)]
pub struct AlertEngine {
    tracks: HashMap<String, Track>,
    active: Vec<OverloadAlertInfo>,
    fresh: Vec<OverloadAlertInfo>,
}

fn by_memory(mac: &ThisMacInfo) -> Option<ToolTreeInfo> {
    mac.top.first().cloned()
}

fn by_cpu(mac: &ThisMacInfo) -> Option<ToolTreeInfo> {
    mac.top
        .iter()
        .chain(mac.heavy.iter())
        .filter(|tree| tree.cpu_pct.is_some())
        .max_by(|a, b| {
            a.cpu_pct
                .unwrap_or_default()
                .total_cmp(&b.cpu_pct.unwrap_or_default())
        })
        .cloned()
}

/// Whole-Mac CPU above `limit` for every point of the last `window_ms`.
#[allow(clippy::cast_precision_loss)]
fn cpu_sustained(mac: &ThisMacInfo, limit: f64, window_ms: i64) -> Option<f64> {
    let series = &mac.machine.series;
    let newest = series.last()?;
    let window: Vec<_> = series
        .iter()
        .filter(|point| newest.at_ms - point.at_ms <= window_ms)
        .collect();
    let span = newest.at_ms - window.first()?.at_ms;
    if span < window_ms * 9 / 10 {
        return None;
    }
    let values: Vec<f64> = window.iter().filter_map(|point| point.cpu_pct).collect();
    (values.len() == window.len() && values.iter().all(|cpu| *cpu > limit))
        .then(|| values.iter().sum::<f64>() / values.len() as f64)
}

/// Pure: what holds right now.
fn conditions(mac: &ThisMacInfo) -> Vec<Condition> {
    let machine = &mac.machine;
    let mut out = Vec::new();
    if !machine.sampled {
        return out;
    }
    if let Some(level) = machine
        .pressure
        .as_deref()
        .filter(|level| matches!(*level, "warning" | "critical"))
    {
        out.push(Condition {
            key: "memory_pressure".into(),
            kind: "memory_pressure",
            hold_ms: PRESSURE_HOLD_MS,
            title: "Your Mac is under memory pressure".into(),
            detail: if level == "critical" {
                "macOS reports critical memory pressure".into()
            } else {
                "macOS reports memory pressure".into()
            },
            subject: by_memory(mac),
        });
    }
    if let Some(rate) = machine
        .swap_growth_mb_per_min
        .filter(|rate| *rate > SWAP_THRASH_MB_PER_MIN)
    {
        out.push(Condition {
            key: "swap".into(),
            kind: "swap",
            hold_ms: SWAP_HOLD_MS,
            title: "Your Mac is swapping hard".into(),
            detail: format!("swap is growing — {rate:.0} MB a minute"),
            subject: by_memory(mac),
        });
    }
    if let Some(mean) = cpu_sustained(mac, CPU_BUSY_PCT, CPU_WINDOW_MS) {
        out.push(Condition {
            key: "cpu".into(),
            kind: "cpu",
            hold_ms: 0,
            title: "The processor has been flat out for minutes".into(),
            detail: format!("{mean:.0}% busy for over two minutes"),
            subject: by_cpu(mac),
        });
    }
    if let Some(state) = machine
        .thermal
        .as_deref()
        .filter(|state| matches!(*state, "serious" | "critical"))
    {
        out.push(Condition {
            key: "thermal".into(),
            kind: "thermal",
            hold_ms: 0,
            title: "Your Mac is hot and slowing down".into(),
            detail: if state == "critical" {
                "macOS is heavily slowing it to cool off".into()
            } else {
                "macOS is slowing it to cool off".into()
            },
            subject: by_cpu(mac),
        });
    }
    // Only a disk that's really out of room (the pressure level, < 5 GB) is
    // an alert; "Getting tight" (< 20 GB) stays a calm word in This Mac —
    // a 256 GB Mac that lives under 20 GB free is never nagged.
    for reason in machine
        .summary
        .reasons
        .iter()
        .filter(|r| r.kind == "disk" && r.level == "pressure")
    {
        let volume = reason
            .words
            .split(" has only ")
            .next()
            .unwrap_or("A disk")
            .to_string();
        out.push(Condition {
            key: format!("disk:{volume}"),
            kind: "disk",
            hold_ms: 0,
            title: format!("{volume} is running out of space"),
            detail: reason.words.clone(),
            subject: None,
        });
    }
    for tree in &mac.heavy {
        let Some(session) = tree.session_id.as_deref() else {
            continue; // tools AI Juggler didn't start: This Mac shows them
        };
        out.push(Condition {
            key: format!("heavy:{session}"),
            kind: "heavy",
            hold_ms: 0, // the heavy rule is already a minute long
            title: "A session is working your Mac hard".into(),
            detail: tree.heavy_note.clone().unwrap_or_default(),
            subject: Some(tree.clone()),
        });
    }
    out
}

impl AlertEngine {
    /// One evaluation (the 5 s This Mac cadence): conditions that held
    /// for their hold time become active; a newly active kind is told
    /// unless it was told in the last 30 minutes; cleared ones go.
    pub fn evaluate(&mut self, mac: &ThisMacInfo, now_ms: i64) {
        let now: Vec<Condition> = conditions(mac);
        let holding: std::collections::HashSet<&str> =
            now.iter().map(|condition| condition.key.as_str()).collect();
        for (key, track) in &mut self.tracks {
            if !holding.contains(key.as_str()) {
                track.since = None;
                track.active = false;
            }
        }
        let mut active = Vec::new();
        for condition in now {
            let track = self.tracks.entry(condition.key.clone()).or_default();
            let since = *track.since.get_or_insert(now_ms);
            if now_ms - since < condition.hold_ms {
                continue;
            }
            let alert = OverloadAlertInfo {
                key: condition.key.clone(),
                kind: condition.kind.to_string(),
                title: condition.title,
                detail: condition.detail,
                session_id: condition
                    .subject
                    .as_ref()
                    .and_then(|s| s.session_id.clone()),
                place: condition.subject.as_ref().and_then(|s| s.place.clone()),
                subject_bytes: condition.subject.as_ref().map(|s| s.rss_bytes),
                subject_estimate: condition
                    .subject
                    .as_ref()
                    .is_some_and(|s| s.memory_estimate),
                subject_cpu: condition.subject.as_ref().and_then(|s| s.cpu_pct),
                since_ms: since,
            };
            if !track.active {
                track.active = true;
                let quiet = track
                    .last_fired
                    .is_some_and(|fired| now_ms - fired < ALERT_EVERY_MS);
                if !quiet {
                    track.last_fired = Some(now_ms);
                    self.fresh.push(alert.clone());
                }
            }
            active.push(alert);
        }
        self.active = active;
    }

    /// The alerts active now (Home's Needs you).
    #[must_use]
    pub fn active(&self) -> Vec<OverloadAlertInfo> {
        self.active.clone()
    }

    /// The alerts to tell since the last call (the notification region).
    pub fn take_fresh(&mut self) -> Vec<OverloadAlertInfo> {
        std::mem::take(&mut self.fresh)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::machine_health::{
        MachineHealthInfo, MachinePointInfo, RoomReasonInfo, RoomSummaryInfo, ToolTotalsInfo,
        VolumeInfo,
    };

    const G: u64 = 1_000_000_000;

    fn calm() -> ThisMacInfo {
        ThisMacInfo {
            machine: MachineHealthInfo {
                sampled: true,
                cores: 10,
                cpu_pct: Some(20.0),
                memory_total: 32 * G,
                memory_used: Some(20 * G),
                memory_basis: Some("app_wired_compressed".into()),
                memory_available: 12 * G,
                pressure: Some("normal".into()),
                swap_total: 4 * G,
                swap_used: 3 * G,
                swap_growth_mb_per_min: Some(0.0),
                volumes: vec![VolumeInfo {
                    name: "Macintosh HD".into(),
                    total_bytes: 1_000 * G,
                    available_bytes: 300 * G,
                }],
                power: None,
                thermal: Some("nominal".into()),
                summary: RoomSummaryInfo {
                    level: "plenty".into(),
                    words: "Plenty of room".into(),
                    reasons: Vec::new(),
                },
                series: (0..150)
                    .map(|i| MachinePointInfo {
                        at_ms: i * 2_000,
                        cpu_pct: Some(20.0),
                        memory_used: Some(20 * G),
                    })
                    .collect(),
            },
            tools: ToolTotalsInfo::default(),
            ours: ToolTotalsInfo::default(),
            theirs: ToolTotalsInfo::default(),
            top: vec![
                tree(Some("s-big"), None, 4 * G, 30.0, None),
                tree(None, Some("desktop"), 2 * G, 140.0, None),
            ],
            heavy: Vec::new(),
            leftover_processes: 0,
            leftover_ports: 0,
            alerts: Vec::new(),
            recommendations: Vec::new(),
        }
    }

    fn tree(
        session: Option<&str>,
        place: Option<&str>,
        rss: u64,
        cpu: f64,
        note: Option<&str>,
    ) -> ToolTreeInfo {
        ToolTreeInfo {
            session_id: session.map(str::to_string),
            place: place.map(str::to_string),
            account_label: None,
            cpu_pct: Some(cpu),
            rss_bytes: rss,
            memory_estimate: place.is_some(),
            helpers: 2,
            heavy_note: note.map(str::to_string),
        }
    }

    /// Runs the engine over `seconds` at the 5 s cadence; returns what it
    /// told and what is active at the end.
    fn run(
        engine: &mut AlertEngine,
        mac: &ThisMacInfo,
        from_ms: i64,
        seconds: i64,
    ) -> Vec<OverloadAlertInfo> {
        let mut told = Vec::new();
        for step in 0..=seconds / 5 {
            engine.evaluate(mac, from_ms + step * 5_000);
            told.extend(engine.take_fresh());
        }
        told
    }

    #[test]
    fn a_calm_mac_tells_nothing() {
        let mut engine = AlertEngine::default();
        assert!(run(&mut engine, &calm(), 0, 600).is_empty());
        assert!(engine.active().is_empty());
    }

    #[test]
    fn memory_pressure_is_told_once_after_thirty_seconds_naming_the_biggest() {
        let mut mac = calm();
        mac.machine.pressure = Some("warning".into());
        let mut engine = AlertEngine::default();
        assert!(
            run(&mut engine, &mac, 0, 25).is_empty(),
            "a 25 s blip is not told"
        );
        let told = run(&mut engine, &mac, 30_000, 300);
        assert_eq!(told.len(), 1, "once, not every 5 s");
        assert_eq!(told[0].title, "Your Mac is under memory pressure");
        assert_eq!(told[0].session_id.as_deref(), Some("s-big"));
        assert_eq!(told[0].subject_bytes, Some(4 * G));
        assert!(!told[0].subject_estimate, "our tree's memory is measured");
        assert_eq!(engine.active().len(), 1);
    }

    #[test]
    fn a_tool_we_did_not_start_is_named_with_its_memory_as_an_estimate() {
        let mut mac = calm();
        mac.machine.pressure = Some("warning".into());
        mac.top.reverse(); // the Claude app first
        let mut engine = AlertEngine::default();
        let told = run(&mut engine, &mac, 0, 60);
        assert_eq!(told.len(), 1);
        assert_eq!(told[0].place.as_deref(), Some("desktop"));
        assert_eq!(told[0].subject_bytes, Some(2 * G));
        assert!(told[0].subject_estimate);
    }

    #[test]
    fn it_clears_and_is_not_retold_within_thirty_minutes() {
        let mut pressured = calm();
        pressured.machine.pressure = Some("critical".into());
        let mut engine = AlertEngine::default();
        assert_eq!(run(&mut engine, &pressured, 0, 60).len(), 1);
        // Clears …
        run(&mut engine, &calm(), 65_000, 10);
        assert!(engine.active().is_empty());
        // … comes back ten minutes later: active again, but not retold.
        let again = run(&mut engine, &pressured, 600_000, 60);
        assert!(again.is_empty());
        assert_eq!(engine.active().len(), 1, "still in Needs you");
        // After 30 minutes it may be told again.
        run(&mut engine, &calm(), 700_000, 10);
        assert_eq!(run(&mut engine, &pressured, 2_000_000, 60).len(), 1);
    }

    #[test]
    fn swap_thrash_needs_a_sustained_rate() {
        let mut mac = calm();
        mac.machine.swap_growth_mb_per_min = Some(150.0);
        let mut engine = AlertEngine::default();
        assert!(
            run(&mut engine, &mac, 0, 300).is_empty(),
            "150 MB/min is not thrash"
        );
        mac.machine.swap_growth_mb_per_min = Some(240.0);
        let mut engine = AlertEngine::default();
        assert!(
            run(&mut engine, &mac, 0, 55).is_empty(),
            "needs a minute more"
        );
        let told = run(&mut engine, &mac, 60_000, 10);
        assert_eq!(told.len(), 1);
        assert_eq!(told[0].detail, "swap is growing — 240 MB a minute");
    }

    #[test]
    fn busy_cpu_needs_two_full_minutes_and_names_the_busiest() {
        let mut mac = calm();
        for point in mac.machine.series.iter_mut().skip(100) {
            point.cpu_pct = Some(97.0); // the last 100 s only
        }
        let mut engine = AlertEngine::default();
        assert!(run(&mut engine, &mac, 0, 10).is_empty());
        for point in &mut mac.machine.series {
            point.cpu_pct = Some(97.0);
        }
        let told = run(&mut engine, &mac, 20_000, 10);
        assert_eq!(told.len(), 1);
        assert_eq!(told[0].kind, "cpu");
        assert_eq!(
            told[0].place.as_deref(),
            Some("desktop"),
            "140 % beats 30 %"
        );
    }

    #[test]
    fn heat_disk_and_heavy_sessions_are_told_at_once() {
        let mut mac = calm();
        mac.machine.thermal = Some("serious".into());
        mac.machine.summary.reasons.push(RoomReasonInfo {
            kind: "disk".into(),
            level: "pressure".into(),
            words: "Macintosh HD has only 3.0 GB free".into(),
        });
        // A disk that's only getting tight is not an alert.
        mac.machine.summary.reasons.push(RoomReasonInfo {
            kind: "disk".into(),
            level: "tight".into(),
            words: "Work has only 14 GB free".into(),
        });
        mac.heavy = vec![
            tree(
                Some("s-hog"),
                None,
                3_200_000_000,
                96.0,
                Some("CPU 96% · 3.2 GB"),
            ),
            tree(None, Some("desktop"), 12 * G, 300.0, Some("12 GB")),
        ];
        let mut engine = AlertEngine::default();
        let told = run(&mut engine, &mac, 0, 0);
        let kinds: Vec<&str> = told.iter().map(|alert| alert.kind.as_str()).collect();
        assert_eq!(
            kinds,
            ["thermal", "disk", "heavy"],
            "observed tools are not alerted"
        );
        assert_eq!(told[1].title, "Macintosh HD is running out of space");
        assert_eq!(told[2].session_id.as_deref(), Some("s-hog"));
        assert_eq!(told[2].detail, "CPU 96% · 3.2 GB");
    }

    #[test]
    fn nothing_before_the_first_sample() {
        let mut mac = calm();
        mac.machine.sampled = false;
        mac.machine.pressure = Some("critical".into());
        let mut engine = AlertEngine::default();
        assert!(run(&mut engine, &mac, 0, 120).is_empty());
    }
}
