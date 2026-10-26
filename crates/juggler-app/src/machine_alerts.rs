//! AJ-E1408: runtime overload alerts — the Mac under memory pressure,
//! swapping hard, busy for minutes, hot, running out of disk, or a session
//! working it hard — told once, calmly, while it's happening.
//!
//! Passive only (PROPOSAL §6.4): an alert is a notification and a Needs
//! you entry, nothing more — no stop, pause or launch, at any tier. An
//! alert turns on after its condition has held for its hold time, and
//! clears only after it has been gone for 45 s (a swap or CPU alert stays
//! on while a lower level holds), so a Mac hovering near a line doesn't
//! flap. Each key, and each kind, is told at most once per 30 minutes;
//! while it's on it stays in Needs you. Like the other
//! immediate alerts (a quota wall), it is not held for quiet hours — those
//! delay the hourly digest only.

use std::collections::HashMap;

use crate::machine_health::{ThisMacInfo, ToolTreeInfo};
use crate::machine_memory::{OTHER_APPS_NAMED, OtherAppInfo, tools_a_small_share};

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
    /// The contributor's memory, bytes — only when memory is what's high.
    #[ts(type = "number | null")]
    pub subject_bytes: Option<u64>,
    /// Whether `subject_bytes` is an estimate (a tool AI Juggler didn't
    /// start: its helpers share memory the sum counts more than once).
    pub subject_estimate: bool,
    /// The contributor's CPU, percent of one core — only when CPU is
    /// what's high.
    pub subject_cpu: Option<f64>,
    /// For a tool AI Juggler didn't start: how many windows (trees) the
    /// figure sums — "the Claude app is using about 16 GB across 3
    /// windows" (the 0.0.12 report: one window's figure read as the app's).
    pub subject_windows: Option<u32>,
    /// A memory alert while the AI tools hold under a third of the memory
    /// in use: the biggest other apps, by name, and no AI tool is named
    /// (the 0.1.4 review: the Claude app was blamed while other apps held
    /// most of it). Empty otherwise.
    pub other_apps: Vec<OtherAppInfo>,
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
/// Once on, a swap alert stays on above this, MB a minute.
const SWAP_STAY_MB_PER_MIN: f64 = 100.0;
/// Whole-Mac CPU above this …
const CPU_BUSY_PCT: f64 = 90.0;
/// … for this long.
const CPU_WINDOW_MS: i64 = 120_000;
/// Once on, a CPU alert stays on while the last 30 s average above this.
const CPU_STAY_PCT: f64 = 80.0;
/// The stay level's window.
const CPU_STAY_WINDOW_MS: i64 = 30_000;
/// An alert clears only after its condition has been gone this long.
const CLEAR_AFTER_MS: i64 = 45_000;
/// A tool AI Juggler didn't start sums memory its helpers share, so it's
/// blamed for memory only when it's still the largest at this share of
/// its sum.
const ESTIMATE_DISCOUNT: f64 = 0.5;

/// A condition, as evaluated now.
#[derive(Debug, Clone)]
struct Condition {
    key: String,
    kind: &'static str,
    hold_ms: i64,
    title: String,
    detail: String,
    subject: Option<ToolTreeInfo>,
    /// Which of the subject's figures to tell (a CPU-heavy session's
    /// memory isn't what's high).
    tell_cpu: bool,
    tell_memory: bool,
    /// The other apps to name instead of a subject (memory alerts only).
    other_apps: Vec<OtherAppInfo>,
}

#[derive(Debug, Clone, Default)]
struct Track {
    since: Option<i64>,
    active: bool,
    last_fired: Option<i64>,
    /// While on: when its condition went away (it clears 45 s later).
    gone_since: Option<i64>,
    /// While on: the alert as last seen, kept through the 45 s.
    last: Option<OverloadAlertInfo>,
}

/// The alert state machine (one per app, inside the machine watch).
#[derive(Debug, Default)]
pub struct AlertEngine {
    tracks: HashMap<String, Track>,
    /// When each kind was last told (one `heavy:` notice per 30 minutes,
    /// whichever session it is).
    kind_fired: HashMap<&'static str, i64>,
    active: Vec<OverloadAlertInfo>,
    fresh: Vec<OverloadAlertInfo>,
}

/// The tree to name for memory: measured memory first. A tool AI Juggler
/// didn't start is named only when it's still the largest at half its
/// summed size.
#[allow(clippy::cast_precision_loss)]
fn by_memory(mac: &ThisMacInfo) -> Option<ToolTreeInfo> {
    let weight = |tree: &ToolTreeInfo| {
        let bytes = tree.rss_bytes as f64;
        if tree.memory_estimate {
            bytes * ESTIMATE_DISCOUNT
        } else {
            bytes
        }
    };
    candidates(mac)
        .into_iter()
        .max_by(|a, b| weight(a).total_cmp(&weight(b)))
}

/// Who a memory alert names: the AI tool holding the most, only while the
/// AI tools hold a third or more of what's in use; else the biggest other
/// apps, and no AI tool.
fn memory_cause(mac: &ThisMacInfo) -> (Option<ToolTreeInfo>, Vec<OtherAppInfo>) {
    if tools_a_small_share(mac) {
        let named = mac.other_apps.iter().take(OTHER_APPS_NAMED).cloned();
        (None, named.collect())
    } else {
        (by_memory(mac), Vec::new())
    }
}

fn by_cpu(mac: &ThisMacInfo) -> Option<ToolTreeInfo> {
    candidates(mac)
        .into_iter()
        .filter(|tree| tree.cpu_pct.is_some())
        .max_by(|a, b| {
            a.cpu_pct
                .unwrap_or_default()
                .total_cmp(&b.cpu_pct.unwrap_or_default())
        })
}

/// Who an alert may name: each session of ours, and each tool AI Juggler
/// didn't start as a whole ("The Claude app" is all its windows — the
/// figure its This Mac line and menu-bar line show, never one window
/// worded as the app).
fn candidates(mac: &ThisMacInfo) -> Vec<ToolTreeInfo> {
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<ToolTreeInfo> = mac
        .top
        .iter()
        .chain(mac.heavy.iter())
        .filter(|tree| tree.session_id.is_some())
        .filter(|tree| seen.insert(tree.session_id.clone()))
        .cloned()
        .collect();
    out.extend(
        mac.by_tool
            .iter()
            .filter(|group| !group.ours)
            .map(|group| ToolTreeInfo {
                root_pid: None,
                session_id: None,
                place: Some(group.tool.clone()),
                // The Claude tools AI Juggler didn't start (desktop | terminal).
                tool: Some(crate::machine_health::OBSERVED_TOOL.to_string()),
                group: group.tool.clone(),
                account_label: None,
                cpu_pct: group.cpu_one_core_pct,
                rss_bytes: group.totals.rss_bytes,
                memory_estimate: group.totals.memory_estimate,
                helpers: 0,
                heavy_note: None,
            }),
    );
    out
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

/// Whole-Mac CPU averaged over the last `window_ms`.
#[allow(clippy::cast_precision_loss)]
fn cpu_mean(mac: &ThisMacInfo, window_ms: i64) -> Option<f64> {
    let series = &mac.machine.series;
    let newest = series.last()?;
    let values: Vec<f64> = series
        .iter()
        .filter(|point| newest.at_ms - point.at_ms <= window_ms)
        .filter_map(|point| point.cpu_pct)
        .collect();
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

/// Pure: what holds right now. `on` is the keys already on: a swap or CPU
/// alert that's on stays on at a lower level.
fn conditions(mac: &ThisMacInfo, on: &std::collections::HashSet<String>) -> Vec<Condition> {
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
        let (subject, other_apps) = memory_cause(mac);
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
            subject,
            tell_cpu: false,
            tell_memory: true,
            other_apps,
        });
    }
    let swap_level = if on.contains("swap") {
        SWAP_STAY_MB_PER_MIN
    } else {
        SWAP_THRASH_MB_PER_MIN
    };
    if let Some(rate) = machine
        .swap_growth_mb_per_min
        .filter(|rate| *rate > swap_level)
    {
        let (subject, other_apps) = memory_cause(mac);
        out.push(Condition {
            key: "swap".into(),
            kind: "swap",
            hold_ms: SWAP_HOLD_MS,
            title: "Your Mac is swapping hard".into(),
            detail: format!("swap is growing — {rate:.0} MB a minute"),
            subject,
            tell_cpu: false,
            tell_memory: true,
            other_apps,
        });
    }
    let cpu = cpu_sustained(mac, CPU_BUSY_PCT, CPU_WINDOW_MS).or_else(|| {
        on.contains("cpu")
            .then(|| cpu_mean(mac, CPU_STAY_WINDOW_MS))
            .flatten()
            .filter(|mean| *mean > CPU_STAY_PCT)
    });
    if let Some(mean) = cpu {
        out.push(Condition {
            key: "cpu".into(),
            kind: "cpu",
            hold_ms: 0,
            title: "The processor has been flat out for minutes".into(),
            detail: format!("{mean:.0}% busy for over two minutes"),
            subject: by_cpu(mac),
            tell_cpu: true,
            tell_memory: false,
            other_apps: Vec::new(),
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
            tell_cpu: true,
            tell_memory: false,
            other_apps: Vec::new(),
        });
    }
    disk_conditions(mac, &mut out);
    heavy_conditions(mac, &mut out);
    out
}

/// Only a disk that's really out of room (the pressure level, < 5 GB) is
/// an alert; "Getting tight" (< 20 GB) stays a calm word in This Mac — a
/// 256 GB Mac that lives under 20 GB free is never nagged.
fn disk_conditions(mac: &ThisMacInfo, out: &mut Vec<Condition>) {
    for reason in mac
        .machine
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
            tell_cpu: false,
            tell_memory: false,
            other_apps: Vec::new(),
        });
    }
}

/// A session AI Juggler started that's working the Mac hard.
fn heavy_conditions(mac: &ThisMacInfo, out: &mut Vec<Condition>) {
    for tree in &mac.heavy {
        let Some(session) = tree.session_id.as_deref() else {
            continue; // tools AI Juggler didn't start: This Mac shows them
        };
        let note = tree.heavy_note.as_deref().unwrap_or_default();
        out.push(Condition {
            key: format!("heavy:{session}"),
            kind: "heavy",
            hold_ms: 0, // the heavy rule is already a minute long
            title: "A session is working your Mac hard".into(),
            detail: note.to_string(),
            subject: Some(tree.clone()),
            // The note says what's high: "CPU 96%", "3.2 GB", or both.
            tell_cpu: note.split(" · ").any(|part| part.starts_with("CPU ")),
            tell_memory: note.split(" · ").any(|part| !part.starts_with("CPU ")),
            other_apps: Vec::new(),
        });
    }
}

impl AlertEngine {
    /// One evaluation (the 5 s This Mac cadence): conditions that held
    /// for their hold time turn on; a newly on key is told unless it, or
    /// another of its kind, was told in the last 30 minutes; one that's
    /// been gone for 45 s clears.
    pub fn evaluate(&mut self, mac: &ThisMacInfo, now_ms: i64) {
        let on: std::collections::HashSet<String> = self
            .tracks
            .iter()
            .filter(|(_, track)| track.active)
            .map(|(key, _)| key.clone())
            .collect();
        let now: Vec<Condition> = conditions(mac, &on);
        let holding: std::collections::HashSet<&str> =
            now.iter().map(|condition| condition.key.as_str()).collect();
        let mut fading = Vec::new();
        for (key, track) in &mut self.tracks {
            if holding.contains(key.as_str()) {
                continue;
            }
            if track.active {
                let gone = *track.gone_since.get_or_insert(now_ms);
                if now_ms - gone < CLEAR_AFTER_MS {
                    fading.extend(track.last.clone());
                    continue;
                }
            }
            track.since = None;
            track.active = false;
            track.gone_since = None;
            track.last = None;
        }
        let mut active = Vec::new();
        for condition in now {
            let track = self.tracks.entry(condition.key.clone()).or_default();
            track.gone_since = None;
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
                subject_bytes: condition
                    .subject
                    .as_ref()
                    .filter(|_| condition.tell_memory)
                    .map(|s| s.rss_bytes),
                subject_estimate: condition
                    .subject
                    .as_ref()
                    .is_some_and(|s| condition.tell_memory && s.memory_estimate),
                subject_cpu: condition
                    .subject
                    .as_ref()
                    .filter(|_| condition.tell_cpu)
                    .and_then(|s| s.cpu_pct),
                subject_windows: condition
                    .subject
                    .as_ref()
                    .filter(|s| s.session_id.is_none())
                    .and_then(|s| s.place.as_deref())
                    .and_then(|place| {
                        mac.by_tool
                            .iter()
                            .find(|group| !group.ours && group.tool == place)
                    })
                    .map(|group| group.totals.count),
                other_apps: condition.other_apps,
                since_ms: since,
            };
            if !track.active {
                track.active = true;
                let recent = |fired: &i64| now_ms - fired < ALERT_EVERY_MS;
                let quiet = track.last_fired.as_ref().is_some_and(recent)
                    || self.kind_fired.get(condition.kind).is_some_and(recent);
                if !quiet {
                    track.last_fired = Some(now_ms);
                    self.kind_fired.insert(condition.kind, now_ms);
                    self.fresh.push(alert.clone());
                }
            }
            track.last = Some(alert.clone());
            active.push(alert);
        }
        fading.sort_by(|a, b| a.key.cmp(&b.key));
        active.extend(fading);
        self.active = active;
        // An ended session's `heavy:` key (and any other) is forgotten once
        // it's off and its 30 minutes have passed.
        self.tracks.retain(|_, track| {
            track.active
                || track.since.is_some()
                || track
                    .last_fired
                    .is_some_and(|fired| now_ms - fired < ALERT_EVERY_MS)
        });
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
        MachineHealthInfo, MachinePointInfo, RoomReasonInfo, RoomSummaryInfo, ToolGroupInfo,
        ToolTotalsInfo, VolumeInfo,
    };

    /// The Claude app as a whole: its line in This Mac and the menu bar.
    fn app(windows: u32, rss: u64, cpu: f64) -> ToolGroupInfo {
        ToolGroupInfo {
            tool: "desktop".into(),
            label: "Claude app".into(),
            ours: false,
            totals: ToolTotalsInfo {
                count: windows,
                memory_estimate: true,
                cpu_pct: Some(cpu / 10.0),
                rss_bytes: rss,
            },
            cpu_one_core_pct: Some(cpu),
        }
    }

    const G: u64 = 1_000_000_000;

    fn calm() -> ThisMacInfo {
        ThisMacInfo {
            machine: MachineHealthInfo {
                sampled: true,
                cores: 10,
                cpu_pct: Some(20.0),
                memory_total: 32 * G,
                memory_used: Some(20 * G),
                memory_compressed: Some(2 * G),
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
            // The AI tools hold most of what's in use (8 of 20 GB, over a
            // third): an alert may name one of them.
            tools: ToolTotalsInfo {
                count: 2,
                memory_estimate: true,
                cpu_pct: Some(17.0),
                rss_bytes: 8 * G,
            },
            ours: ToolTotalsInfo::default(),
            theirs: ToolTotalsInfo::default(),
            by_tool: vec![app(1, 2 * G, 140.0)],
            top: vec![
                tree(Some("s-big"), None, 4 * G, 30.0, None),
                tree(None, Some("desktop"), 2 * G, 140.0, None),
            ],
            others: Vec::new(),
            heavy: Vec::new(),
            other_apps: vec![
                other("next-server", 6_400_000_000, 1),
                other("Docker", 4 * G, 3),
            ],
            leftover_processes: 0,
            leftover_ports: 0,
            alerts: Vec::new(),
            recommendations: Vec::new(),
            awake: crate::features::app::awake::AwakeInfo::default(),
            own: crate::features::app::own::OwnInfo::default(),
        }
    }

    fn other(name: &str, rss: u64, processes: u32) -> OtherAppInfo {
        OtherAppInfo {
            name: name.into(),
            rss_bytes: rss,
            processes,
        }
    }

    /// The 0.1.4 review's Mac: 91 of 103 GB in use (47 GB of it kept
    /// compressed), the Claude app about 16 GB of it across 6 windows,
    /// other apps the rest.
    fn others_fill_it() -> ThisMacInfo {
        let mut mac = calm();
        mac.machine.memory_total = 103 * G;
        mac.machine.memory_used = Some(91 * G);
        mac.machine.memory_available = 12 * G;
        mac.machine.memory_compressed = Some(47 * G);
        mac.tools.rss_bytes = 16 * G;
        mac.top = vec![tree(None, Some("desktop"), 4 * G, 3.0, None)];
        mac.by_tool = vec![app(6, 16 * G, 9.0)];
        mac.other_apps = vec![
            other("next-server", 6_400_000_000, 1),
            other("Docker", 4 * G, 3),
            other("fseventsd", 1_500_000_000, 1),
            other("ffmpeg", 1_200_000_000, 2),
        ];
        mac
    }

    #[test]
    fn other_apps_are_named_when_the_ai_tools_are_a_small_share() {
        let mut mac = others_fill_it();
        mac.machine.pressure = Some("warning".into());
        mac.machine.swap_growth_mb_per_min = Some(3004.0);
        let told = run(&mut AlertEngine::default(), &mac, 0, 60);
        let kinds: Vec<&str> = told.iter().map(|alert| alert.kind.as_str()).collect();
        assert_eq!(kinds, ["memory_pressure", "swap"]);
        for alert in &told {
            assert_eq!(alert.place, None, "the Claude app isn't the cause");
            assert_eq!(alert.session_id, None);
            assert_eq!(alert.subject_bytes, None);
            assert_eq!(alert.subject_windows, None);
            let names: Vec<&str> = alert
                .other_apps
                .iter()
                .map(|app| app.name.as_str())
                .collect();
            assert_eq!(
                names,
                ["next-server", "Docker", "fseventsd"],
                "the three biggest"
            );
        }
        // The AI tools holding a third or more: the Claude app is named,
        // and no other app.
        mac.tools.rss_bytes = 40 * G;
        mac.by_tool = vec![app(6, 40 * G, 9.0)];
        let told = run(&mut AlertEngine::default(), &mac, 0, 60);
        assert_eq!(told[0].place.as_deref(), Some("desktop"));
        assert_eq!(told[0].subject_bytes, Some(40 * G));
        assert!(told[0].other_apps.is_empty());
    }

    #[test]
    fn without_a_reading_of_memory_in_use_the_biggest_ai_tool_is_named() {
        let mut mac = others_fill_it();
        mac.machine.pressure = Some("warning".into());
        mac.machine.memory_used = None;
        let told = run(&mut AlertEngine::default(), &mac, 0, 60);
        assert_eq!(told[0].place.as_deref(), Some("desktop"));
        assert!(told[0].other_apps.is_empty());
    }

    #[test]
    fn other_alerts_never_name_other_apps() {
        let mut mac = others_fill_it();
        for point in &mut mac.machine.series {
            point.cpu_pct = Some(97.0);
        }
        mac.machine.thermal = Some("serious".into());
        let told = run(&mut AlertEngine::default(), &mac, 0, 0);
        assert!(!told.is_empty());
        assert!(told.iter().all(|alert| alert.other_apps.is_empty()));
    }

    fn tree(
        session: Option<&str>,
        place: Option<&str>,
        rss: u64,
        cpu: f64,
        note: Option<&str>,
    ) -> ToolTreeInfo {
        ToolTreeInfo {
            root_pid: None,
            session_id: session.map(str::to_string),
            place: place.map(str::to_string),
            tool: None,
            group: place.unwrap_or("claude-code").to_string(),
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
    fn a_tool_we_did_not_start_is_blamed_only_when_clearly_the_largest() {
        let mut mac = calm();
        mac.machine.pressure = Some("warning".into());
        // The Claude app's 6 GB is summed over helpers that share pages:
        // our measured 4 GB session is named, not the app.
        mac.by_tool = vec![app(3, 6 * G, 20.0)];
        let told = run(&mut AlertEngine::default(), &mac, 0, 60);
        assert_eq!(told[0].session_id.as_deref(), Some("s-big"));
        // At 12 GB it's the largest even at half: named, as an estimate.
        mac.by_tool = vec![app(3, 12 * G, 20.0)];
        let told = run(&mut AlertEngine::default(), &mac, 0, 60);
        assert_eq!(told.len(), 1);
        assert_eq!(told[0].place.as_deref(), Some("desktop"));
        assert_eq!(told[0].subject_bytes, Some(12 * G));
        assert!(told[0].subject_estimate);
    }

    #[test]
    fn the_claude_app_is_named_by_all_its_windows_never_one() {
        // Six windows, the heaviest 4.9 GB: "The Claude app is using about
        // 12 GB" — the figure its own line shows — never 4.9 GB.
        let mut mac = calm();
        mac.machine.pressure = Some("warning".into());
        mac.top = vec![tree(None, Some("desktop"), 4_900_000_000, 3.0, None)];
        mac.by_tool = vec![app(6, 12 * G, 14.0)];
        let told = run(&mut AlertEngine::default(), &mac, 0, 60);
        assert_eq!(told[0].place.as_deref(), Some("desktop"));
        assert_eq!(told[0].subject_bytes, Some(12 * G));
        // CPU as Home's rows count it (one core), the app's windows summed.
        let mut busy = calm();
        for point in &mut busy.machine.series {
            point.cpu_pct = Some(97.0);
        }
        busy.top = vec![tree(None, Some("desktop"), 4_900_000_000, 3.0, None)];
        busy.by_tool = vec![app(6, 12 * G, 140.0)];
        let told = run(&mut AlertEngine::default(), &busy, 0, 0);
        assert_eq!(told[0].subject_cpu, Some(140.0));
    }

    /// The 0.0.12 report: the banner said 4.2 GB, Needs you 8.1 GB, This
    /// Mac three Claude app rows. The alert's subject is the app as a
    /// whole, and it says how many windows its figure sums.
    #[test]
    fn the_alert_says_how_many_windows_its_figure_sums() {
        let mut mac = calm();
        mac.machine.pressure = Some("warning".into());
        mac.top = vec![
            tree(None, Some("desktop"), 8_100_000_000, 3.0, None),
            tree(None, Some("desktop"), 4_200_000_000, 3.0, None),
            tree(None, Some("desktop"), 3_800_000_000, 3.0, None),
        ];
        mac.by_tool = vec![app(3, 16_100_000_000, 9.0)];
        let told = run(&mut AlertEngine::default(), &mac, 0, 60);
        assert_eq!(told[0].place.as_deref(), Some("desktop"));
        assert_eq!(told[0].subject_bytes, Some(16_100_000_000));
        assert_eq!(told[0].subject_windows, Some(3));
        // A session of ours is one tree: no window count.
        let mut ours = calm();
        ours.machine.pressure = Some("warning".into());
        ours.top = vec![tree(Some("s-big"), None, 4 * G, 30.0, None)];
        ours.by_tool = Vec::new();
        let told = run(&mut AlertEngine::default(), &ours, 0, 60);
        assert_eq!(told[0].session_id.as_deref(), Some("s-big"));
        assert_eq!(told[0].subject_windows, None);
    }

    #[test]
    fn a_flicker_neither_clears_nor_retells_it() {
        let mut pressured = calm();
        pressured.machine.pressure = Some("warning".into());
        let mut engine = AlertEngine::default();
        assert_eq!(run(&mut engine, &pressured, 0, 60).len(), 1);
        let since = engine.active()[0].since_ms;
        // One calm sample, then pressure again: still on, same start, not
        // told again.
        engine.evaluate(&calm(), 65_000);
        assert_eq!(engine.active().len(), 1, "still on through a flicker");
        assert!(run(&mut engine, &pressured, 70_000, 60).is_empty());
        assert_eq!(engine.active()[0].since_ms, since);
        // 45 s calm clears it.
        run(&mut engine, &calm(), 140_000, 45);
        assert!(engine.active().is_empty());
    }

    #[test]
    fn busy_cpu_stays_on_while_it_hovers_above_eighty() {
        let busy = |cpu: f64| {
            let mut mac = calm();
            for point in &mut mac.machine.series {
                point.cpu_pct = Some(cpu);
            }
            mac
        };
        let mut engine = AlertEngine::default();
        assert_eq!(run(&mut engine, &busy(97.0), 0, 0).len(), 1);
        // 85 % for minutes: on, not retold.
        assert!(run(&mut engine, &busy(85.0), 5_000, 300).is_empty());
        assert_eq!(engine.active().len(), 1);
        // Under 80 % for 45 s: off.
        run(&mut engine, &busy(40.0), 310_000, 50);
        assert!(engine.active().is_empty());
    }

    #[test]
    fn a_session_dipping_in_and_out_is_told_once_per_kind() {
        let heavy = |session: &str| {
            let mut mac = calm();
            mac.heavy = vec![tree(Some(session), None, G, 96.0, Some("CPU 96%"))];
            mac
        };
        let mut engine = AlertEngine::default();
        assert_eq!(run(&mut engine, &heavy("s-1"), 0, 0).len(), 1);
        run(&mut engine, &calm(), 5_000, 60);
        assert!(engine.active().is_empty());
        // Another session (or the same build in a new session) within 30
        // minutes: in Needs you, but no second notice.
        assert!(run(&mut engine, &heavy("s-2"), 70_000, 0).is_empty());
        assert_eq!(engine.active().len(), 1);
        run(&mut engine, &calm(), 75_000, 60);
        // The ended sessions' keys are forgotten after their 30 minutes.
        run(&mut engine, &calm(), 2_000_000, 0);
        assert!(engine.tracks.is_empty(), "{:?}", engine.tracks.keys());
    }

    #[test]
    fn it_clears_and_is_not_retold_within_thirty_minutes() {
        let mut pressured = calm();
        pressured.machine.pressure = Some("critical".into());
        let mut engine = AlertEngine::default();
        assert_eq!(run(&mut engine, &pressured, 0, 60).len(), 1);
        // Clears (45 s gone) …
        run(&mut engine, &calm(), 65_000, 45);
        assert!(engine.active().is_empty());
        // … comes back ten minutes later: active again, but not retold.
        let again = run(&mut engine, &pressured, 600_000, 60);
        assert!(again.is_empty());
        assert_eq!(engine.active().len(), 1, "still in Needs you");
        // After 30 minutes it may be told again.
        run(&mut engine, &calm(), 700_000, 45);
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
        assert_eq!(told[2].subject_cpu, Some(96.0));
        assert_eq!(told[2].subject_bytes, Some(3_200_000_000));
    }

    #[test]
    fn a_cpu_heavy_session_is_told_by_its_cpu_not_its_memory() {
        let mut mac = calm();
        mac.heavy = vec![tree(
            Some("s-cc"),
            None,
            1_100_000_000,
            96.0,
            Some("CPU 96%"),
        )];
        let told = run(&mut AlertEngine::default(), &mac, 0, 0);
        assert_eq!(told[0].subject_cpu, Some(96.0));
        assert_eq!(told[0].subject_bytes, None, "1.1 GB isn't what's high");
        mac.heavy = vec![tree(
            Some("s-mem"),
            None,
            3_200_000_000,
            20.0,
            Some("3.2 GB"),
        )];
        let told = run(&mut AlertEngine::default(), &mac, 0, 0);
        assert_eq!(told[0].subject_cpu, None);
        assert_eq!(told[0].subject_bytes, Some(3_200_000_000));
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
