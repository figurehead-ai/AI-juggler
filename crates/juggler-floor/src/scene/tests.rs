//! The scene from a snapshot (AJ-E3506): every live session has a desk
//! and a door, the lobby's doors point where they say, the moving part
//! says when it moves, and the budgets hold.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use super::{Inputs, Live, Mark, Scene, Target, capacity, live_sessions, mark_letters, unused};
use crate::family::Family;
use crate::layout::Office;
use crate::seating::Seating;
use crate::snapshot::{
    FloorAccount, FloorApp, FloorAppKind, FloorMachine, FloorSession, FloorSnapshot,
};
use crate::state::FloorState;

const NOW: i64 = 1_800_000_000_000;
const DAY: i64 = 24 * 60 * 60 * 1000;

fn session(key: &str, state: FloorState, subagents: u32) -> FloorSession {
    FloorSession {
        key: key.into(),
        title: format!("Task {key}"),
        state,
        word: format!("{state:?}"),
        subagents,
        project: None,
        tasks: 0,
        cpu_pct: 5.0,
        started_ms: NOW - 1000,
        last_ms: NOW,
        go_to: None,
    }
}

fn app(id: &str, family: &str, sessions: Vec<FloorSession>) -> FloorApp {
    FloorApp {
        id: id.into(),
        name: format!("Tool {id} · Work"),
        tool: "tool".into(),
        host_app: None,
        logo: None,
        kind: if id.starts_with("app:") {
            FloorAppKind::Desktop
        } else {
            FloorAppKind::Cli
        },
        family: family.into(),
        colour_slot: 1,
        account_id: Some("acct:1".into()),
        initials: "WM".into(),
        open: sessions.iter().any(|s| s.state != FloorState::Ended),
        last_active_ms: Some(NOW),
        cpu_pct: 10.0,
        hot_since_ms: None,
        sessions,
    }
}

fn snapshot(apps: Vec<FloorApp>) -> FloorSnapshot {
    FloorSnapshot {
        at_ms: NOW,
        apps,
        accounts: vec![FloorAccount {
            id: "acct:1".into(),
            label: "Work Max".into(),
            left_pct: Some(15.0),
            resets_ms: None,
            last_used_ms: None,
            out: false,
        }],
        pools: Vec::new(),
        machine: FloorMachine::default(),
        notices: 0,
    }
}

/// The office, seated, as the render thread keeps it.
fn office_for(snap: &FloorSnapshot) -> (Office, BTreeMap<String, Vec<Option<String>>>) {
    let mut office = Office::new(7, 10);
    let open: Vec<(String, Family, usize)> = snap
        .apps
        .iter()
        .map(|a| {
            (
                a.id.clone(),
                Family::of(Some(&a.family)),
                live_sessions(a).len(),
            )
        })
        .collect();
    office.observe(&open, NOW);
    let mut seating = Seating::default();
    let seats = snap
        .apps
        .iter()
        .map(|a| {
            (
                a.id.clone(),
                seating.seat(&a.id, capacity(&office, &a.id), &live_sessions(a)),
            )
        })
        .collect();
    (office, seats)
}

fn build(snap: &FloorSnapshot) -> Scene {
    let (office, seats) = office_for(snap);
    Scene::build(Inputs {
        snapshot: snap,
        office: &office,
        seats: &seats,
        decay: true,
        covers: false,
        detail: false,
        machine: "This computer",
        now_ms: NOW,
        logos: &BTreeMap::new(),
    })
    .expect("it fits")
}

#[test]
fn every_live_session_has_a_desk_and_a_door_to_it() {
    let snap = snapshot(vec![
        app(
            "cli:a",
            "workshop",
            vec![
                session("s:1", FloorState::Working, 2),
                session("s:2", FloorState::Asks, 0),
                session("s:3", FloorState::Ended, 0),
            ],
        ),
        app(
            "app:host:b",
            "studio",
            vec![session("c:x/9", FloorState::Done, 9)],
        ),
    ]);
    let scene = build(&snap);
    for key in ["s:1", "s:2", "c:x/9"] {
        let id = scene
            .pick_of_session(key)
            .unwrap_or_else(|| panic!("{key} has a desk"));
        assert_eq!(
            scene.pick_of(id).unwrap().target,
            Target::Session(key.into())
        );
        assert!(scene.anchor(id).is_some());
    }
    assert_eq!(
        scene.pick_of_session("s:3"),
        None,
        "an ended session has no desk"
    );
    let targets: Vec<&Target> = scene.picks.iter().map(|p| &p.target).collect();
    assert!(targets.contains(&&Target::App("cli:a".into())));
    assert!(targets.contains(&&Target::Account("acct:1".into())));
    assert!(targets.contains(&&Target::ThisMac));
    assert!(targets.contains(&&Target::Helpers("s:1".into())));
    assert!(scene.pick_of(0).is_none());
    assert!(targets.contains(&&Target::Cat));
    // AJ-E3516: the tower's floors are doors into them
    assert!(targets.contains(&&Target::Floor(0)));
    assert_eq!(scene.tower_floors.len(), scene.floors);
    assert_ne!(scene.stat.len(), 0);
}

#[test]
fn the_moving_part_shows_each_state_and_counts_subagents() {
    let snap = snapshot(vec![app(
        "cli:a",
        "industrial",
        vec![
            session("s:1", FloorState::Asks, 0),
            session("s:2", FloorState::Stuck, 0),
            session("s:3", FloorState::Crashed, 0),
            session("s:4", FloorState::Working, 9),
        ],
    )]);
    let scene = build(&snap);
    let mut out = Vec::new();
    let drawn = scene.frame(
        &Live {
            t: 1.0,
            ..Live::default()
        },
        &[0],
        &mut out,
    );
    assert!(drawn.moving);
    let balloons = drawn
        .marks
        .iter()
        .filter(|m| matches!(m, Mark::Balloon { .. }))
        .count();
    assert_eq!(balloons, 3, "asks, stuck and crashed each get one");
    assert!(
        drawn
            .marks
            .iter()
            .any(|m| matches!(m, Mark::Plus { n: 3, .. })),
        "9 subagents: 6 drawn, +3"
    );
    let none = scene.frame(&Live::default(), &[5], &mut out);
    assert!(
        out.is_empty() && none.marks.is_empty(),
        "nothing on a floor out of view"
    );
}

#[test]
fn an_unused_app_dims_by_how_long_and_decay_can_be_switched_off() {
    let mut idle = app("cli:a", "workshop", Vec::new());
    idle.open = false;
    // AJ-E3518: closed under an hour, to a day, to a week, to a month,
    // longer
    for (ago, level) in [
        (DAY / 48, 1),
        (DAY / 2, 2),
        (2 * DAY, 3),
        (10 * DAY, 4),
        (40 * DAY, 5),
    ] {
        idle.last_active_ms = Some(NOW - ago);
        assert_eq!(unused(&idle, NOW, true), level, "{ago} ms ago");
        assert_eq!(unused(&idle, NOW, false), 1, "decay off: only closed");
    }
    // the light falls with it, the logo and sign still readable at 25%
    let light: Vec<f32> = (0..=5).map(super::dimming).collect();
    assert!(light.windows(2).all(|w| w[1] < w[0]), "{light:?}");
    assert!((light[1] - 0.8).abs() < 1e-6 && (light[5] - 0.25).abs() < 1e-6);
    // dust from a day, thicker with time
    let dust: Vec<u32> = light.iter().map(|d| crate::models::dust_of(*d)).collect();
    assert_eq!(dust, vec![0, 0, 0, 1, 2, 3]);
    assert_eq!(mark_letters("Claude Code · Work Max"), "CC");
    assert_eq!(mark_letters("Codex"), "C");
}

/// AJ-E3506's budget: building the scene for 20 Apps and 150 sessions ≤
/// 50 ms on a release build (a debug build gets ten times as long), and
/// a frame of the moving part well under that.
#[test]
fn building_twenty_apps_and_a_hundred_and_fifty_sessions_is_quick() {
    let scale = if cfg!(debug_assertions) { 10 } else { 1 };
    let families = [
        "workshop",
        "studio",
        "industrial",
        "maker-garage",
        "campus",
        "lab",
        "neutral",
    ];
    let apps: Vec<FloorApp> = (0..20)
        .map(|n| {
            let count = [2, 5, 9, 14, 7][n % 5] + usize::from(n == 0) * 2;
            let sessions = (0..count)
                .map(|k| session(&format!("s:{n}-{k}"), FloorState::Working, (k % 4) as u32))
                .collect();
            app(&format!("cli:{n}"), families[n % families.len()], sessions)
        })
        .collect();
    let snap = snapshot(apps);
    let (office, seats) = office_for(&snap);
    let started = std::time::Instant::now();
    let scene = Scene::build(Inputs {
        snapshot: &snap,
        office: &office,
        seats: &seats,
        decay: true,
        covers: false,
        detail: false,
        machine: "This computer",
        now_ms: NOW,
        logos: &BTreeMap::new(),
    })
    .expect("the atlas holds a big office");
    let took = started.elapsed();
    assert_eq!(
        scene
            .picks
            .iter()
            .filter(|p| matches!(p.target, Target::Session(_)))
            .count(),
        150
    );
    assert!(took.as_millis() <= 50 * scale, "building took {took:?}");
    let mut out = Vec::new();
    let floors: Vec<usize> = (0..scene.floors).collect();
    let started = std::time::Instant::now();
    let _ = scene.frame(
        &Live {
            t: 2.0,
            walkers: true,
            ..Live::default()
        },
        &floors,
        &mut out,
    );
    let frame = started.elapsed();
    assert!(frame.as_millis() <= 10 * scale, "a frame took {frame:?}");
    eprintln!(
        "build {took:?}, frame {frame:?}, {} still parts, {} moving",
        scene.stat.len(),
        out.len()
    );
}

#[test]
fn the_overlay_has_a_chip_per_floor_and_area_chips_from_afar() {
    use crate::camera::View;
    use crate::overlay::{Inputs as Over, build as overlay};
    let snap = snapshot(vec![app(
        "cli:a",
        "workshop",
        vec![session("s:1", FloorState::Asks, 0)],
    )]);
    let scene = build(&snap);
    let frame = scene.frame_size();
    let building = View {
        focus: None,
        cam: frame.home(None, 1.6),
    };
    let vp = building.cam.vp(1.6);
    let mut out = Vec::new();
    let drawn = scene.frame(&Live::default(), &building.floors(&frame), &mut out);
    let inputs = Over {
        view: &building,
        vp: &vp,
        size: (1600, 1000),
        scale: 2.0,
        marks: &drawn.marks,
        hover: 0,
        mouse: None,
        alert: Some("CPU 97% for 6 minutes"),
        hint: "Drag to pan",
        band: crate::flags::Band::Far,
        labels: crate::flags::Labels::Smart,
        card: None,
    };
    let (quads, chips, _) = overlay(&scene, &inputs);
    assert_eq!(chips.len(), scene.floors + 1);
    assert_eq!(chips[0].1, None, "the Building chip first");
    assert!(
        quads.len() > 40,
        "the flag of what needs you, the hint and the alert: {}",
        quads.len()
    );
    let near = View {
        focus: Some(0),
        cam: frame.home(Some(0), 1.6),
    };
    let (fewer, _, _) = overlay(
        &scene,
        &Over {
            view: &near,
            alert: None,
            band: crate::flags::Band::Close,
            ..inputs
        },
    );
    assert!(fewer.len() < quads.len());
}

#[test]
fn a_pool_gets_a_meeting_room_and_the_lobby_a_notice_board() {
    use crate::snapshot::FloorPool;
    let mut snap = snapshot(vec![app(
        "cli:a",
        "workshop",
        vec![
            session("s:1", FloorState::Working, 0),
            session("s:2", FloorState::Idle, 0),
            session("s:3", FloorState::Working, 0),
        ],
    )]);
    snap.pools = vec![
        FloorPool {
            id: "pool-1".into(),
            name: "Shop".into(),
            sealed: false,
            sessions: vec!["s:1".into(), "s:2".into()],
        },
        FloorPool {
            id: "pool-2".into(),
            name: "Alone".into(),
            sealed: false,
            sessions: vec!["s:3".into()],
        },
    ];
    snap.notices = 12;
    let scene = build(&snap);
    let targets: Vec<&Target> = scene.picks.iter().map(|p| &p.target).collect();
    assert!(
        targets.contains(&&Target::Pool("pool-1".into())),
        "two members: a room"
    );
    assert!(
        !targets.contains(&&Target::Pool("pool-2".into())),
        "one member: no room"
    );
    let notices = scene
        .picks
        .iter()
        .find(|p| p.target == Target::Notices)
        .expect("the notice board");
    assert_eq!(notices.detail, "12 unread");
}

#[test]
fn the_floor_opens_where_an_app_needs_you_most() {
    let mut apps: Vec<FloorApp> = (0..12)
        .map(|n| {
            app(
                &format!("cli:{n}"),
                "neutral",
                vec![session(&format!("s:{n}"), FloorState::Working, 0)],
            )
        })
        .collect();
    apps[11].sessions = vec![
        session("s:a", FloorState::Asks, 0),
        session("s:b", FloorState::Stuck, 0),
    ];
    let scene = build(&snapshot(apps));
    assert!(scene.floors >= 2);
    assert_eq!(
        scene.neediest_floor(),
        Some(1),
        "the last App sits on floor 2"
    );
    let calm = build(&snapshot(vec![app(
        "cli:a",
        "neutral",
        vec![session("s:1", FloorState::Working, 0)],
    )]));
    assert_eq!(calm.neediest_floor(), None);
}

#[test]
fn the_window_says_the_floor_in_view_in_words() {
    let scene = build(&snapshot(vec![
        app(
            "cli:a",
            "workshop",
            vec![session("s:1", FloorState::Asks, 0)],
        ),
        app(
            "cli:b",
            "studio",
            vec![session("s:2", FloorState::Working, 0)],
        ),
    ]));
    assert_eq!(scene.summary(Some(0)), "Floor 1: 2 Apps, 1 waiting on you");
    assert_eq!(
        scene.summary(None),
        "The building: 2 Apps, 1 waiting on you"
    );
    assert_eq!(scene.area_stops().len(), 2);
    assert_eq!(scene.waiting_stops().len(), 1);
}

#[test]
fn the_atlas_keeps_only_its_rows_in_use() {
    // D-79 (Full resolution's memory): a big office (20 Apps, every
    // family, 150 sessions) still leaves most of the 2048² atlas unused
    let families = [
        "claude-code",
        "claude-app",
        "codex",
        "opencode",
        "gemini",
        "cursor",
        "other",
    ];
    let apps: Vec<FloorApp> = (0..20)
        .map(|i| {
            let sessions = (0..if i < 10 { 8 } else { 7 })
                .map(|k| session(&format!("s:{i}-{k}"), FloorState::Working, 0))
                .collect();
            app(&format!("cli:{i}"), families[i % families.len()], sessions)
        })
        .collect();
    let scene = build(&snapshot(apps));
    assert!(scene.atlas.h <= 1024, "{} rows kept", scene.atlas.h);
    assert_eq!(
        scene.atlas.px.len(),
        (scene.atlas.w * scene.atlas.h * 4) as usize
    );
    assert!((scene.atlas_v * scene.atlas.h as f32 - 2048.0).abs() < 1e-3);
    eprintln!("atlas rows kept: {} (of 2048)", scene.atlas.h);
}
