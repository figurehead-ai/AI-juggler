//! The menu bar's model, plan, ids and icon (AJ-E1411), and the fixture
//! Home and the menu both read.
#![expect(clippy::unwrap_used, reason = "a test fails loudly on a missing value")]

use super::*;
use crate::machine_groups::ToolGroupInfo;
use crate::machine_health::{MachineHealthInfo, RoomReasonInfo, RoomSummaryInfo, ToolTotalsInfo};

const G: u64 = 1_000_000_000;

fn calm() -> ThisMacInfo {
    ThisMacInfo {
        machine: MachineHealthInfo {
            sampled: true,
            cores: 10,
            cpu_pct: Some(34.0),
            memory_total: 32 * G,
            memory_used: Some(21 * G),
            memory_basis: Some("app_wired_compressed".into()),
            memory_available: 11 * G,
            pressure: Some("normal".into()),
            swap_total: 0,
            swap_used: 0,
            swap_growth_mb_per_min: Some(0.0),
            volumes: Vec::new(),
            power: None,
            thermal: Some("nominal".into()),
            summary: RoomSummaryInfo {
                level: "plenty".into(),
                words: "Plenty of room".into(),
                reasons: Vec::new(),
            },
            series: Vec::new(),
        },
        tools: ToolTotalsInfo {
            count: 4,
            memory_estimate: true,
            cpu_pct: Some(9.0),
            rss_bytes: 5_700_000_000,
        },
        ours: ToolTotalsInfo::default(),
        theirs: ToolTotalsInfo::default(),
        by_tool: vec![
            group("claude-code", "Claude Code", true, 2, 60.0, 4_100_000_000),
            group("desktop", "Claude app", false, 2, 14.0, 1_600_000_000),
        ],
        top: Vec::new(),
        heavy: Vec::new(),
        leftover_processes: 0,
        leftover_ports: 0,
        alerts: Vec::new(),
        recommendations: Vec::new(),
    }
}

/// `cpu` as Home's rows count it (one core); the all-cores share on a
/// 10-core Mac is a tenth of it.
fn group(tool: &str, label: &str, ours: bool, count: u32, cpu: f64, rss: u64) -> ToolGroupInfo {
    ToolGroupInfo {
        tool: tool.into(),
        label: label.into(),
        count_words: match (ours, count) {
            (true, 1) => "1 session".into(),
            (true, n) => format!("{n} sessions"),
            (false, 1) => "1 window".into(),
            (false, n) => format!("{n} windows"),
        },
        ours,
        totals: ToolTotalsInfo {
            count,
            memory_estimate: !ours,
            cpu_pct: Some(cpu / 10.0),
            rss_bytes: rss,
        },
        cpu_one_core_pct: Some(cpu),
    }
}

fn heavy(id: &str, note: &str) -> ToolTreeInfo {
    ToolTreeInfo {
        session_id: Some(id.into()),
        place: None,
        account_label: None,
        cpu_pct: Some(96.0),
        rss_bytes: 3_200_000_000,
        memory_estimate: false,
        helpers: 4,
        heavy_note: Some(note.into()),
    }
}

fn names(id: &str) -> Option<String> {
    match id {
        "s-1" => Some("feat/read-model".into()),
        "s-2" => Some("payments-api".into()),
        _ => None,
    }
}

fn texts(model: &TrayModel) -> Vec<&str> {
    model.items.iter().map(TrayItem::text).collect()
}

#[test]
fn a_calm_mac_is_one_quiet_icon_with_this_mac_behind_a_click() {
    let model = tray_model(&calm(), &names);
    assert_eq!(model.title, None, "no number while nothing needs a glance");
    assert!(!model.attention);
    assert_eq!(
        texts(&model),
        [
            "Plenty of room",
            "Memory: 21 GB of 32 GB in use · 34% of all CPU cores",
            "",
            // As Home's rows: memory, then CPU of one core.
            "Claude Code · 2 sessions · 4.1 GB · CPU 60%",
            "Claude app · 2 windows · about 1.6 GB · CPU 14%",
            "",
            "Open This Mac",
            "Quit AI Juggler",
        ]
    );
    assert_eq!(model.items[0].shape().1, Some(&TrayAction::ThisMac));
}

#[test]
fn nothing_running_says_so() {
    let mut info = calm();
    info.tools.count = 0;
    info.by_tool.clear();
    let model = tray_model(&info, &names);
    assert_eq!(model.items[0].text(), "Nothing running — your Mac has room");
    assert_eq!(
        texts(&model),
        [
            "Nothing running — your Mac has room",
            "Memory: 21 GB of 32 GB in use · 34% of all CPU cores",
            "",
            "",
            "Open This Mac",
            "Quit AI Juggler",
        ],
        "said once, not again as a tool line"
    );
}

#[test]
fn a_heavy_session_puts_a_number_up_and_comes_first() {
    let mut info = calm();
    info.heavy = vec![heavy("s-1", "CPU 96%"), heavy("s-2", "3.2 GB")];
    let model = tray_model(&info, &names);
    assert_eq!(model.title.as_deref(), Some("2"));
    assert!(model.attention);
    assert_eq!(
        model.items[0],
        TrayItem::Action {
            text: "feat/read-model is working your Mac hard — CPU 96%".into(),
            action: TrayAction::Session("s-1".into()),
        }
    );
    // The other heavy one is listed once, under its own heading.
    let rest = texts(&model);
    assert!(rest.contains(&"Working your Mac hard"));
    assert!(rest.contains(&"payments-api — 3.2 GB"));
    assert!(
        !rest[1..]
            .iter()
            .any(|line| line.starts_with("feat/read-model"))
    );
}

#[test]
fn the_mac_itself_outranks_a_session_and_a_heavy_alert_counts_once() {
    let mut info = calm();
    info.heavy = vec![heavy("s-1", "CPU 96%")];
    info.alerts = vec![
        alert(
            "heavy:s-1",
            "heavy",
            "A session is working your Mac hard",
            Some("s-1"),
            None,
        ),
        alert(
            "memory_pressure",
            "memory_pressure",
            "Your Mac is under memory pressure",
            None,
            Some("desktop"),
        ),
    ];
    info.alerts[1].subject_bytes = Some(12 * G);
    info.alerts[1].subject_estimate = true;
    info.alerts[1].subject_cpu = None; // a memory alert tells memory only
    let model = tray_model(&info, &names);
    assert_eq!(
        model.title.as_deref(),
        Some("2"),
        "the heavy session's alert counts once"
    );
    assert_eq!(
        model.items[0].text(),
        "Your Mac is under memory pressure — The Claude app is using about 12 GB"
    );
    assert_eq!(model.items[0].shape().1, Some(&TrayAction::ThisMac));
}

#[test]
fn a_tight_mac_stays_calm_in_shape_and_words() {
    let mut info = calm();
    info.machine.summary = RoomSummaryInfo {
        level: "tight".into(),
        words: "Getting tight".into(),
        reasons: vec![RoomReasonInfo {
            kind: "memory".into(),
            level: "tight".into(),
            words: "macOS reports memory pressure".into(),
        }],
    };
    let model = tray_model(&info, &names);
    assert_eq!(model.title, None);
    assert!(!model.attention, "tight is calm; pressure is not");
    // Home's calm words; what macOS reports belongs to the alert.
    assert_eq!(model.items[0].text(), "Getting tight");
    assert!(
        !texts(&model)
            .iter()
            .any(|line| line.contains("macOS reports"))
    );
    info.machine.summary.level = "pressure".into();
    assert!(tray_model(&info, &names).attention);
}

#[test]
fn before_the_first_reading_it_says_so() {
    let mut info = calm();
    info.machine.sampled = false;
    assert_eq!(
        tray_model(&info, &names).items[1].text(),
        "Measuring this Mac…"
    );
}

#[test]
fn a_long_name_is_cut() {
    let mut info = calm();
    info.heavy = vec![heavy("s-long", "CPU 96%")];
    let long = |_: &str| Some("x".repeat(60));
    let model = tray_model(&info, &long);
    assert!(
        model.items[0]
            .text()
            .starts_with(&format!("{}…", "x".repeat(39)))
    );
}

#[test]
fn the_icon_is_a_template_and_attention_is_a_shape() {
    let calm = icon_rgba(false);
    let busy = icon_rgba(true);
    assert_eq!(calm.len(), (ICON_PX * ICON_PX * 4) as usize);
    // Black with alpha only: macOS tints a template for the menu bar.
    assert!(
        calm.chunks(4)
            .all(|px| px[0] == 0 && px[1] == 0 && px[2] == 0)
    );
    let ink = |rgba: &[u8]| rgba.chunks(4).filter(|px| px[3] > 0).count();
    assert!(ink(&calm) > 150, "three balls");
    assert!(ink(&busy) > ink(&calm), "the attention dot adds ink");
    // The dot sits top right, where the calm icon is empty.
    let at = |rgba: &[u8], x: u32, y: u32| rgba[((y * ICON_PX + x) * 4 + 3) as usize];
    assert_eq!(at(&calm, 32, 4), 0);
    assert_eq!(at(&busy, 32, 4), 255);
    // A mark, not a fourth ball: smaller than the balls, and clear of
    // the arc.
    let dot = ink(&busy) - ink(&calm);
    assert!(
        dot * 2 < ink(&calm) / 3,
        "the dot is under half a ball's ink"
    );
}

#[test]
fn a_click_resolves_by_its_own_action_even_after_the_menu_changed() {
    let mut info = calm();
    info.heavy = vec![heavy("s-1", "CPU 96%"), heavy("s-2", "3.2 GB")];
    let open = tray_model(&info, &names);
    // The ids the open menu was built with.
    let ids: Vec<String> = (0_u64..)
        .zip(&open.items)
        .map(|(serial, item)| item_id(serial, item))
        .collect();
    let payments = open
        .items
        .iter()
        .position(|item| item.text() == "payments-api — 3.2 GB")
        .unwrap();
    // The heavy sessions calm down while the menu is open: the menu is
    // shorter, and Quit now sits where payments-api was.
    let calmer = tray_model(&calm(), &names);
    assert_ne!(
        calmer.items.get(payments).map(TrayItem::text),
        Some("payments-api — 3.2 GB"),
        "by position, that line is now something else"
    );
    // A click on the old line still means that session, never Quit.
    assert_eq!(
        action_of(&ids[payments]),
        Some(TrayAction::Session("s-2".into()))
    );
    assert_eq!(action_of(&ids[ids.len() - 1]), Some(TrayAction::Quit));
    assert_eq!(action_of(&ids[1]), None, "a figure does nothing");
    assert_eq!(action_of("tray:9:session:"), None);
    assert_eq!(action_of("something-else"), None, "not ours: a safe no-op");
}

/// Applies a plan to the old lines, as the native glue does.
fn apply(old: &[TrayItem], new: &[TrayItem], steps: &MenuPlan) -> Vec<TrayItem> {
    let mut lines = old.to_vec();
    for (position, words) in &steps.retext {
        lines[*position] = match &lines[*position] {
            TrayItem::Action { action, .. } => TrayItem::Action {
                text: words.clone(),
                action: action.clone(),
            },
            TrayItem::Label(_) => TrayItem::Label(words.clone()),
            TrayItem::Separator => TrayItem::Separator,
        };
    }
    lines.splice(
        steps.remove.clone(),
        new[steps.insert.clone()].iter().cloned(),
    );
    lines
}

#[test]
fn the_menu_changes_in_place_never_replaced() {
    let quiet = tray_model(&calm(), &names);
    // Figures only: every line keeps its place; only words change.
    let mut busier = calm();
    busier.machine.cpu_pct = Some(71.0);
    let next = tray_model(&busier, &names);
    let steps = plan(&quiet.items, &next.items);
    assert!(steps.remove.is_empty() && steps.insert.is_empty());
    assert_eq!(
        steps.retext,
        [(
            1,
            "Memory: 21 GB of 32 GB in use · 71% of all CPU cores".to_string()
        )]
    );
    assert_eq!(apply(&quiet.items, &next.items, &steps), next.items);
    // A heavy session appears: the start and the end stay; the run in
    // between is replaced (the first line and the new section).
    let mut heavy_mac = calm();
    heavy_mac.heavy = vec![heavy("s-1", "CPU 96%"), heavy("s-2", "3.2 GB")];
    let heavy_model = tray_model(&heavy_mac, &names);
    let steps = plan(&quiet.items, &heavy_model.items);
    assert_eq!(
        apply(&quiet.items, &heavy_model.items, &steps),
        heavy_model.items
    );
    assert!(
        steps.remove.end <= quiet.items.len() - 3,
        "Open This Mac, Quit and their divider stay put"
    );
    // And back.
    let steps = plan(&heavy_model.items, &quiet.items);
    assert_eq!(apply(&heavy_model.items, &quiet.items, &steps), quiet.items);
    // To and from "not measured".
    let steps = plan(&quiet.items, &unmeasured_model().items);
    assert_eq!(
        apply(&quiet.items, &unmeasured_model().items, &steps),
        unmeasured_model().items
    );
}

#[test]
fn voiceover_hears_what_the_number_counts_and_words_for_unknowns() {
    let mut info = calm();
    info.heavy = vec![heavy("s-1", "CPU 96%"), heavy("s-2", "3.2 GB")];
    let model = tray_model(&info, &names);
    assert!(model.tooltip.starts_with("AI Juggler — 2 need a glance: "));
    info.heavy.truncate(1);
    assert!(
        tray_model(&info, &names)
            .tooltip
            .starts_with("AI Juggler — 1 needs a glance: ")
    );
    let mut unknown = calm();
    unknown.machine.cpu_pct = None;
    assert_eq!(
        tray_model(&unknown, &names).items[1].text(),
        "Memory: 21 GB of 32 GB in use · CPU —"
    );
    let stale = unmeasured_model();
    assert_eq!(stale.items[0].text(), "Not measured right now");
    assert_eq!(stale.title, None);
}

/// One tree for the shared fixture.
fn input(
    session: Option<&str>,
    place: Option<&'static str>,
    cpu: Option<f64>,
    rss: u64,
) -> crate::machine_health::TreeInput {
    crate::machine_health::TreeInput {
        session_id: session.map(str::to_string),
        place,
        provider: session.map(|_| "claude-code".to_string()),
        account_label: Some("Work Max".into()),
        stats: juggler_supervisor::LiveProcessStats {
            child_count: 2,
            cpu_pct: cpu,
            rss_bytes: rss,
        },
        // A minute and more at this load: heavy when it's over the line.
        ring: (0..32)
            .map(|i| juggler_supervisor::TreeSample {
                at_ms: i * 2_000,
                cpu_pct: cpu,
                rss_bytes: rss,
            })
            .collect(),
        hog_rules: Vec::new(),
    }
}

/// The fixture Home and the menu both read
/// (`ui/src/lib/__fixtures__/menu-bar.json`): a heavy session of ours,
/// the Claude app in two windows, `claude` in a terminal not measured
/// yet, and memory pressure held long enough to be told.
#[test]
fn the_menu_and_home_read_one_fixture() {
    use crate::machine_health::assemble;
    let mut machine = calm().machine;
    machine.pressure = Some("warning".into());
    machine.summary = RoomSummaryInfo {
        level: "tight".into(),
        words: "Getting tight".into(),
        reasons: vec![RoomReasonInfo {
            kind: "memory".into(),
            level: "tight".into(),
            words: "macOS reports memory pressure".into(),
        }],
    };
    let session = input(Some("s-1"), None, Some(96.0), 1_100_000_000);
    let window_a = input(None, Some("desktop"), Some(10.0), 4_900_000_000);
    let window_b = input(None, Some("desktop"), Some(4.0), 7_100_000_000);
    let terminal = input(None, Some("terminal"), None, 300_000_000);
    let trees = [session.clone(), window_a, window_b, terminal.clone()];
    let mut info = assemble(machine.clone(), &trees, (0, 0));
    let mut engine = crate::machine_alerts::AlertEngine::default();
    for step in 0..=12 {
        engine.evaluate(&info, step * 5_000);
    }
    info.alerts = engine.active();
    let menu = tray_model(&info, &names);
    let lines: Vec<&str> = texts(&menu);

    // The Claude app is named by all its windows: the alert's figure
    // is its line's figure.
    assert_eq!(
        lines[0],
        "Your Mac is under memory pressure — The Claude app is using about 12 GB"
    );
    assert!(lines.contains(&"Claude app · 2 windows · about 12 GB · CPU 14%"));
    // Tools with one tree read as that tree's row on Home.
    assert!(lines.contains(&"Claude Code · 1 session · 1.1 GB · CPU 96%"));
    assert!(lines.contains(&"claude in a terminal · 1 running · about 300 MB · CPU —"));

    // Each one-tree tool's row as Home draws it (its own This Mac).
    let single = |tree: &crate::machine_health::TreeInput, line: &str| {
        serde_json::json!({
            "line": line,
            "tree": assemble(machine.clone(), std::slice::from_ref(tree), (0, 0)).top[0],
        })
    };
    let fixture = serde_json::json!({
        "about": "Generated by tray::tests::the_menu_and_home_read_one_fixture; AIJ_UPDATE_FIXTURES=1 rewrites it.",
        "info": info,
        "names": { "s-1": "feat/read-model" },
        "menu": lines,
        "single": [
            single(&session, "Claude Code · 1 session · 1.1 GB · CPU 96%"),
            single(&terminal, "claude in a terminal · 1 running · about 300 MB · CPU —"),
        ],
    });
    let generated = format!("{}\n", serde_json::to_string_pretty(&fixture).unwrap());
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../ui/src/lib/__fixtures__/menu-bar.json");
    if std::env::var_os("AIJ_UPDATE_FIXTURES").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &generated).unwrap();
    }
    // Compared as values: a workspace build turns on serde_json's
    // `preserve_order`, which changes key order but never content.
    let committed: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    assert_eq!(
        committed, fixture,
        "the shared menu-bar fixture is stale: run with AIJ_UPDATE_FIXTURES=1"
    );
}

fn alert(
    key: &str,
    kind: &str,
    title: &str,
    session: Option<&str>,
    place: Option<&str>,
) -> OverloadAlertInfo {
    OverloadAlertInfo {
        key: key.into(),
        kind: kind.into(),
        title: title.into(),
        detail: "detail".into(),
        session_id: session.map(str::to_string),
        place: place.map(str::to_string),
        subject_name: place.map(|p| crate::running_setups::place_words(p).subject.to_string()),
        subject_bytes: None,
        subject_estimate: false,
        subject_cpu: Some(96.0),
        since_ms: 0,
    }
}
