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
            resets_words: None,
        }],
        pools: Vec::new(),
        machine: FloorMachine::default(),
        notices: 0,
        needs: 0,
    }
}

/// The office, seated, as the render thread keeps it.
fn office_for(snap: &FloorSnapshot) -> (Office, BTreeMap<String, Vec<Option<String>>>) {
    office_seeded(snap, 7)
}

fn office_seeded(
    snap: &FloorSnapshot,
    seed: u32,
) -> (Office, BTreeMap<String, Vec<Option<String>>>) {
    let mut office = Office::new(seed, 10);
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
    build_seeded(snap, 7)
}

fn build_seeded(snap: &FloorSnapshot, seed: u32) -> Scene {
    let (office, seats) = office_seeded(snap, seed);
    Scene::build(plain(snap, &office, &seats)).expect("it fits")
}

/// The plain inputs most tests build from: decay on, no covers or detail,
/// nothing hidden, no logos, on this computer at `NOW`.
fn plain<'a>(
    snap: &'a FloorSnapshot,
    office: &'a crate::layout::Office,
    seats: &'a BTreeMap<String, Vec<Option<String>>>,
) -> Inputs<'a> {
    const HIDE: &std::collections::BTreeSet<String> = &std::collections::BTreeSet::new();
    const LOGOS: &BTreeMap<String, crate::paint::Img> = &BTreeMap::new();
    Inputs {
        snapshot: snap,
        office,
        seats,
        decay: true,
        covers: false,
        detail: false,
        hide: HIDE,
        machine: "This computer",
        now_ms: NOW,
        logos: LOGOS,
    }
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
    // running again: lit at once, whatever its age
    idle.open = true;
    assert_eq!(unused(&idle, NOW, true), 0);
    // the light falls with it, the logo and sign still readable at 25%
    let light: Vec<f32> = (0..=5).map(super::dimming).collect();
    assert!(light.windows(2).all(|w| w[1] < w[0]), "{light:?}");
    assert!((light[1] - 0.8).abs() < 1e-6 && (light[5] - 0.25).abs() < 1e-6);
    // dust from a day, thicker with time
    let dust: Vec<u32> = light.iter().map(|d| crate::glow::dust_of(*d)).collect();
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
    let scene = Scene::build(plain(&snap, &office, &seats)).expect("the atlas holds a big office");
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
        opened: None,
    };
    let (quads, chips, _, view) = overlay(&scene, &inputs);
    assert!(view.is_some(), "the View button");
    // the tabs, then one summary line a floor (a click enters it)
    assert_eq!(chips.len(), 2 * scene.floors + 1);
    assert_eq!(chips[0].1, None, "the Building chip first");
    assert!((0..scene.floors).all(|f| chips[scene.floors + 1..].iter().any(|c| c.1 == Some(f))));
    assert!(
        quads.len() > 40,
        "the flag of what needs you, the hint and the alert: {}",
        quads.len()
    );
    let near = View {
        focus: Some(0),
        cam: frame.home(Some(0), 1.6),
    };
    let (fewer, near_chips, _, _) = overlay(
        &scene,
        &Over {
            view: &near,
            alert: None,
            band: crate::flags::Band::Close,
            ..inputs
        },
    );
    assert!(fewer.len() < quads.len());
    // one floor: no lift (a lone "1" at the edge said nothing, 2026-11-06)
    assert_eq!(scene.floors, 1);
    assert_eq!(near_chips.len(), scene.floors + 1, "the tabs only");
    assert!(
        !fewer.iter().any(|q| (q.tint[0] - 0.36).abs() < 1e-3
            && (q.tint[1] - 0.31).abs() < 1e-3
            && (q.tint[2] - 0.84).abs() < 1e-3),
        "no lift's floor number on a one-floor office"
    );
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
    assert_eq!(scene.summary(Some(0)), "Floor 1: 2 Apps · 1 asking here");
    assert_eq!(scene.summary(None), "The building: 2 Apps · 1 asking");
    assert_eq!(scene.floor_words(Some(0)), "2 Apps · 1 asking");
    assert_eq!(scene.area_stops().len(), 2);
    assert_eq!(scene.waiting_stops().len(), 1);
}

#[test]
fn the_title_says_needs_yous_one_count_beside_the_floors_sessions() {
    // AJ-E2502 and the persona walk's pilot: Home said "4 need you" while
    // the Floor said "1 waiting on you". The title now says Home's number,
    // then the sessions waiting on the floor in view.
    let mut snap = snapshot(vec![app(
        "cli:a",
        "workshop",
        vec![session("s:1", FloorState::Asks, 0)],
    )]);
    snap.needs = 4;
    let scene = build(&snap);
    assert_eq!(
        scene.summary(Some(0)),
        "Floor 1: 1 App · 4 need you · 1 asking here"
    );
    snap.needs = 1;
    snap.apps[0].sessions.clear();
    let scene = build(&snap);
    assert_eq!(scene.summary(Some(0)), "Floor 1: 1 App · 1 needs you");
}

#[test]
fn an_areas_words_lead_with_what_needs_you() {
    let one = |sessions| build(&snapshot(vec![app("cli:a", "workshop", sessions)]));
    let scene = one(vec![
        session("s:1", FloorState::Asks, 0),
        session("s:2", FloorState::Working, 1),
        session("s:3", FloorState::Working, 2),
    ]);
    let app_a = scene.app("cli:a").expect("the App");
    assert_eq!(
        scene.area_words(app_a),
        "1 asking · 2 working · 3 subagents · 15% left"
    );
    let scene = one(vec![session("s:1", FloorState::Idle, 0)]);
    assert_eq!(
        scene.area_words(scene.app("cli:a").expect("the App")),
        "1 session open, nothing waiting · 15% left"
    );
    let scene = one(vec![]);
    assert_eq!(
        scene.area_words(scene.app("cli:a").expect("the App")),
        "No sessions open · 15% left"
    );
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

/// AJ-E3516 (2026-11-06): the building view is a cross-section on its
/// street. Each App has an office whose click goes to its floor and whose
/// hover names it; an office that needs you has a flag with its count; the
/// street lamps light only at night; the sky is drawn only in the
/// building view, and its clouds never keep a still Floor drawing.
#[test]
fn the_building_is_a_cross_section_on_its_street() {
    use super::tower::TOWER;
    let mut snap = snapshot(vec![
        app(
            "cli:a",
            "workshop",
            vec![
                session("s:1", FloorState::Asks, 0),
                session("s:2", FloorState::Asks, 0),
            ],
        ),
        app(
            "cli:b",
            "studio",
            vec![session("s:3", FloorState::Working, 0)],
        ),
    ]);
    // room enough: no Low here (that's its own test)
    snap.accounts[0].left_pct = Some(60.0);
    let scene = build(&snap);
    for (id, f) in [("cli:a", 0usize), ("cli:b", 0)] {
        let name = &scene.app(id).unwrap().name;
        assert!(
            scene
                .picks
                .iter()
                .any(|p| &p.label == name && p.target == Target::Floor(f)),
            "{id}'s office goes to floor {f}"
        );
    }
    assert!(scene.picks.iter().any(|p| p.label == "The lobby"));
    let room = scene
        .tower_rooms
        .iter()
        .find(|r| r.app == "cli:a")
        .expect("cli:a waits on you");
    assert_eq!(room.count, 2);
    assert!(
        scene.tower_rooms.iter().all(|r| r.app != "cli:b"),
        "cli:b only works"
    );
    let night_only = scene
        .lights
        .iter()
        .filter(|l| l.floor == TOWER && l.c[3] < 0.5)
        .count();
    assert_eq!(night_only, 4, "four street lamps, lit only at night");
    // the sky: only in the building view; drifting only while animating
    let mut out = Vec::new();
    let still = scene.frame(&Live::default(), &[], &mut out);
    assert!(!still.moving, "a still building view draws once");
    assert!(out.iter().any(|i| i.floor == TOWER), "its sky");
    let moving = scene.frame(
        &Live {
            walkers: true,
            ..Live::default()
        },
        &[],
        &mut out,
    );
    assert!(moving.moving, "clouds drift while Motion is on");
    scene.frame(&Live::default(), &[0], &mut out);
    assert!(out.iter().all(|i| i.floor != TOWER), "no sky on a floor");
}

/// 0.4.1: the Floor says an Account's room as the app's chips do: OUT in
/// the danger red when it's spent, LOW in amber at 25% left or less, in
/// the same words; a crash still comes first.
#[test]
fn out_and_low_say_what_the_app_says() {
    use crate::flags::{LOW_LEFT, Need};
    let mut snap = snapshot(vec![app(
        "cli:a",
        "workshop",
        vec![session("s:1", FloorState::Working, 0)],
    )]);
    for (left, out, want) in [
        (Some(60.0), false, None),
        (Some(LOW_LEFT + 1.0), false, None),
        (Some(LOW_LEFT), false, Some((Need::Low, "LOW · 25% LEFT"))),
        (Some(12.4), false, Some((Need::Low, "LOW · 12% LEFT"))),
        (Some(0.0), true, Some((Need::Out, "OUT"))),
        (None, true, Some((Need::Out, "OUT"))),
        (None, false, None),
    ] {
        snap.accounts[0].resets_words = None;
        snap.accounts[0].left_pct = left;
        snap.accounts[0].out = out;
        let scene = build(&snap);
        let a = scene.app("cli:a").unwrap();
        let words = scene.room_words(a);
        assert_eq!(
            words.as_ref().map(|(n, w)| (*n, w.as_str())),
            want,
            "{left:?} out {out}"
        );
        assert_eq!(
            scene
                .needs_of(a)
                .iter()
                .any(|(n, _)| matches!(n, Need::Low | Need::Out)),
            want.is_some()
        );
    }
    // when it resets, in the user's words (the app fills them in)
    snap.accounts[0].out = true;
    snap.accounts[0].resets_words = Some("9:31 PM".into());
    let scene = build(&snap);
    assert_eq!(
        scene
            .room_words(scene.app("cli:a").unwrap())
            .map(|(_, w)| w),
        Some("OUT · RESETS 9:31 PM".to_owned())
    );
    // low is a state: it never adds to what waits, and gives way to it
    snap.accounts[0].left_pct = Some(10.0);
    snap.accounts[0].out = false;
    snap.apps[0].sessions[0].state = FloorState::Crashed;
    let scene = build(&snap);
    assert_eq!(
        scene.needs_of(scene.app("cli:a").unwrap()),
        vec![(Need::Crashed, 1)]
    );
    // the app's tones: danger red and warning amber, apart from a crash
    assert_eq!(Need::Out.colour(), [0.97, 0.44, 0.44, 1.0]);
    assert_eq!(Need::Low.colour(), [0.96, 0.62, 0.04, 1.0]);
    assert!(
        Need::Crashed < Need::Out && Need::Out < Need::Low,
        "urgency order"
    );
    // each has its badge (an empty gauge, a quarter-full one)
    let scene = build(&snap);
    for icon in [crate::tex::Icon::Out, crate::tex::Icon::Low] {
        assert!(scene.ui.badge.contains_key(&icon), "{icon:?}");
    }
}

/// A hash map's rows in a fixed order (the overlay's lookup tables).
fn sorted<K: std::fmt::Debug, V: std::fmt::Debug>(
    map: &mut std::collections::HashMap<K, V>,
) -> Vec<String> {
    let mut rows: Vec<String> = map.drain().map(|kv| format!("{kv:?}")).collect();
    rows.sort();
    rows
}

/// AJ-E3506 R4, AJ-E3513 R2: the same snapshot and seed give the same
/// scene, bit for bit (every mesh, light, pick, label and texture; floats
/// compared by their shortest exact form), and another seed another one.
#[test]
fn the_same_snapshot_and_seed_give_the_same_scene() {
    let snap = snapshot(vec![
        app("a", "claude", vec![session("1", FloorState::Working, 2)]),
        app("b", "codex", vec![session("2", FloorState::Asks, 0)]),
        app("c", "claude", vec![session("3", FloorState::Done, 0)]),
    ]);
    let print = |seed| {
        let mut scene = build_seeded(&snap, seed);
        let tables = [
            sorted(&mut scene.ui.glyph),
            sorted(&mut scene.ui.icon),
            sorted(&mut scene.ui.badge),
        ];
        format!("{scene:?} {tables:?}")
    };
    let first = print(7);
    assert_eq!(first, print(7), "seed 7 twice");
    assert_eq!(print(11), print(11), "seed 11 twice");
    assert_ne!(first, print(11), "another seed, another office");
}

/// AJ-E3508 R2: each App's logo is printed into its area's floor,
/// see-through (35 %), large (62 % of the short side, at most 6.5 m),
/// in the glass pass (blended, no pick id), just above the floor;
/// centred on the area, and in a large area that spans the aisle, on its
/// first row, so the walkway never cuts it.
#[test]
fn each_area_prints_its_logo_large_and_see_through() {
    use crate::layout::{ROW_D, Size};
    let apps = [("small", 3), ("middle", 9), ("large", 16)]
        .iter()
        .map(|(id, count)| {
            let sessions = (0..*count)
                .map(|k| session(&format!("s:{id}-{k}"), FloorState::Working, 0))
                .collect();
            app(id, "workshop", sessions)
        })
        .collect();
    let scene = build(&snapshot(apps));
    let mut sizes = Vec::new();
    for placed in scene.plan.areas.iter().filter(|p| p.annex == 0) {
        let rect = placed.rect;
        let logos: Vec<_> = scene
            .stat
            .iter()
            .filter(|i| {
                i.glass
                    && i.mesh == crate::mesh::Mesh::QuadUp
                    && (i.inst.col[3] - super::LOGO_OPACITY).abs() < 0.01
                    && (i.inst.m[3][0] - rect.cx()).abs() < 1e-3
            })
            .collect();
        assert_eq!(logos.len(), 1, "{}: one logo", placed.id);
        let m = logos[0].inst.m;
        let side = (rect.w().min(rect.d()) * 0.62).min(6.5);
        assert!((m[0][0] - side).abs() < 1e-3 && (m[2][2] - side).abs() < 1e-3);
        let z = if placed.size == Size::L && placed.row == 0 {
            rect.z0 + ROW_D * 0.5
        } else {
            rect.cz()
        };
        assert!((m[3][2] - z).abs() < 1e-3, "{}: centred", placed.id);
        assert!(m[3][1] > 0.0 && m[3][1] < 0.02, "just above the floor");
        sizes.push((placed.size, placed.row));
    }
    sizes.sort_by_key(|(s, _)| s.capacity());
    // the large area starts on row 0: the first-row rule is the one used
    let sizes: Vec<Size> = sizes
        .iter()
        .map(|&(size, row)| {
            assert!(size != Size::L || row == 0, "the large one on row 0");
            size
        })
        .collect();
    assert_eq!(sizes, [Size::S, Size::M, Size::L], "all three sizes seen");
}

/// AJ-E3509's table test: every state to its pose, balloon, the need it
/// shows and the flag's word, and the App's flag counting it (SPEC §9.7).
#[test]
fn every_state_has_its_pose_balloon_and_flag() {
    use super::{balloon_of, pose_of};
    use crate::flags::Need;
    use crate::models::Pose;
    use crate::tex::Icon;
    let table = [
        (FloorState::Working, Pose::Typing, None, None, None),
        (FloorState::Idle, Pose::Idle, None, None, None),
        (
            FloorState::Asks,
            Pose::Wave,
            Some(Icon::Asks),
            Some(Need::Asks),
            Some("1 ASKS"),
        ),
        (
            FloorState::Stuck,
            Pose::Scratch,
            Some(Icon::Stuck),
            Some(Need::Stuck),
            Some("1 STUCK"),
        ),
        (
            FloorState::Crashed,
            Pose::HandsOnHead,
            Some(Icon::Error),
            Some(Need::Crashed),
            Some("1 CRASHED"),
        ),
        (
            FloorState::Done,
            Pose::Stretch,
            Some(Icon::Done),
            Some(Need::Done),
            Some("1 DONE"),
        ),
        // asleep, a red Z; its flag is its Account's OUT (`out_and_low_say_what_the_app_says`)
        (
            FloorState::Limit,
            Pose::Asleep,
            Some(Icon::Sleep),
            Some(Need::Out),
            None,
        ),
        (FloorState::Ended, Pose::Idle, None, None, None),
    ];
    for (state, pose, balloon, need, flag) in table {
        assert_eq!(pose_of(state), pose, "{state:?}: pose");
        assert_eq!(balloon_of(state), balloon, "{state:?}: balloon");
        assert_eq!(Need::of_state(state), need, "{state:?}: need");
        let snap = snapshot(vec![app("a", "workshop", vec![session("s:1", state, 0)])]);
        let scene = build(&snap);
        let counted = scene.needs_of(&snap.apps[0]);
        // the sessions' needs (the fixture's Account is low: its LOW aside)
        let words = counted
            .iter()
            .find(|(need, _)| !matches!(need, Need::Low | Need::Out | Need::Hot))
            .map(|(need, n)| format!("{n} {}", need.word(false)));
        assert_eq!(words.as_deref(), flag, "{state:?}: the App's flag");
        // a desk drawn in that state, posed and ballooned as the table says
        if state != FloorState::Ended {
            assert_eq!(scene.desks.len(), 1, "{state:?}: one desk");
            assert_eq!(scene.desks[0].state, state);
        }
    }
}

/// SPEC §9.7's limits: done for 15 minutes, then idle; smoke for 10;
/// each is news to the feed when it passes (`time_limits`).
#[test]
fn done_rests_after_15_minutes_and_smoke_stops_after_10() {
    use super::{DONE_FOR_MS, SMOKE_FOR_MS, drawn_state, smoking, time_limits};
    let done = session("s:d", FloorState::Done, 0);
    assert_eq!(drawn_state(&done, NOW + DONE_FOR_MS - 1), FloorState::Done);
    assert_eq!(drawn_state(&done, NOW + DONE_FOR_MS), FloorState::Idle);
    let crashed = session("s:c", FloorState::Crashed, 0);
    assert!(smoking(&crashed, NOW + SMOKE_FOR_MS - 1));
    assert!(!smoking(&crashed, NOW + SMOKE_FOR_MS));
    assert_eq!(
        drawn_state(&crashed, NOW + DAY),
        FloorState::Crashed,
        "X stays"
    );
    let mut snap = snapshot(vec![app("a", "workshop", vec![done, crashed])]);
    snap.at_ms = NOW + 60_000;
    let before = time_limits(&snap);
    snap.at_ms = NOW + SMOKE_FOR_MS + 60_000;
    let smoke_out = time_limits(&snap);
    assert_ne!(before, smoke_out, "smoke out is news");
    snap.at_ms = NOW + DONE_FOR_MS + 60_000;
    assert_ne!(smoke_out, time_limits(&snap), "done to idle is news");
    // drawn so: the desk of a done session an hour old sits idle, no tick
    let mut scene_snap = snapshot(vec![app(
        "a",
        "workshop",
        vec![session("s:d", FloorState::Done, 0)],
    )]);
    scene_snap.apps[0].sessions[0].last_ms = NOW - 60 * 60 * 1000;
    let scene = build(&scene_snap);
    assert_eq!(scene.desks[0].state, FloorState::Idle);
    assert!(
        scene
            .needs_of(&scene_snap.apps[0])
            .iter()
            .all(|(need, _)| *need != crate::flags::Need::Done),
        "no DONE flag"
    );
}

/// SPEC §9.7: an App over its CPU share for 2 minutes gets one fire, on
/// its busiest desk, never on a person; a cool App none.
#[test]
fn a_hot_app_gets_one_fire_beside_its_busiest_desk() {
    let mut hot = app(
        "hot",
        "workshop",
        (0..3)
            .map(|k| session(&format!("s:h{k}"), FloorState::Working, 0))
            .collect(),
    );
    hot.sessions[1].cpu_pct = 90.0;
    hot.hot_since_ms = Some(NOW - 3 * 60 * 1000);
    let cool = app(
        "cool",
        "studio",
        vec![session("s:c", FloorState::Working, 0)],
    );
    let scene = build(&snapshot(vec![hot, cool]));
    assert_eq!(
        scene.fires.len(),
        1,
        "one fire an App, none for the cool one"
    );
    let fire = scene.fires[0].1.0.point([0.0; 3]);
    let busiest = scene
        .desks
        .iter()
        .find(|d| d.session.key == "s:h1")
        .expect("the busiest desk");
    let desk = busiest.frame.0.point([0.0; 3]);
    let flat = |a: crate::math::V3, b: crate::math::V3| {
        ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    };
    assert!(flat(fire, desk) < 1.0, "on the busiest desk");
    for d in &scene.desks {
        let person = d.frame.0.point([0.0, 0.0, 0.62]);
        assert!(
            flat(fire, person) > 0.6,
            "{}: never on a person",
            d.session.key
        );
    }
}

/// SPEC §9.7: a sign's 4-segment battery yellow under 40 %, blinking
/// amber under 20 %, red when out; the rack's lights rise with the load,
/// green to red; the whiteboard counts the App's open tasks.
#[test]
fn the_battery_the_rack_and_the_whiteboard_say_their_numbers() {
    use super::area::tasks_words;
    use super::frame::{RACK_LIGHTS, battery_lit, gauge, rack_light, rack_lit};
    use crate::colour::hex;
    assert_eq!(gauge(0.8, 0.0), hex("#5fd38d"), "green");
    assert_eq!(gauge(0.3, 0.0), hex("#e8c84a"), "yellow under 40 %");
    assert_ne!(gauge(0.1, 0.5), gauge(0.1, 2.0), "blinking under 20 %");
    assert_eq!(gauge(0.1, 0.5), hex("#f0a640"), "amber");
    assert_eq!(gauge(0.0, 0.0), hex("#ef5350"), "out");
    assert_eq!(
        [1.0, 0.3, 0.1, 0.0].map(battery_lit),
        [4, 2, 1, 0],
        "segments lit"
    );
    assert_eq!(
        [0.0, 0.5, 1.0].map(|load| rack_lit(load, 0.0)),
        [0, 5, RACK_LIGHTS]
    );
    let lights: Vec<_> = (0..RACK_LIGHTS)
        .map(|k| rack_light(k, RACK_LIGHTS))
        .collect();
    assert_eq!(lights[0].0, hex("#5fd38d"), "green at the bottom");
    assert_eq!(lights[6].0, hex("#f0a640"), "amber");
    assert_eq!(lights[9].0, hex("#ef5350"), "red at the top");
    assert!(rack_light(5, 5).1 < f32::EPSILON, "unlit above the load");
    let mut a = app(
        "a",
        "workshop",
        vec![
            session("s:1", FloorState::Working, 0),
            session("s:2", FloorState::Idle, 0),
        ],
    );
    a.sessions[0].tasks = 2;
    a.sessions[1].tasks = 1;
    assert_eq!(tasks_words(&a), (3, "3 TASKS".to_owned()));
    a.sessions[0].tasks = 0;
    assert_eq!(tasks_words(&a).1, "1 TASK");
}

/// AJ-E3510 R2: an App's whiteboard is its own door, to the App's tasks.
#[test]
fn the_whiteboard_opens_the_apps_tasks() {
    let mut a = app(
        "cli:a",
        "workshop",
        vec![session("s:1", FloorState::Working, 0)],
    );
    a.sessions[0].tasks = 2;
    let scene = build(&snapshot(vec![a]));
    let board = scene
        .picks
        .iter()
        .find(|p| p.target == Target::Tasks("cli:a".into()))
        .expect("the whiteboard's door");
    assert_eq!(board.detail, "2 tasks");
    assert!(board.label.ends_with("· tasks"), "{}", board.label);
}

/// AJ-E3506 R1: every plant stands in every pot, and the props on 24
/// desks of one area (one seed, each desk its own draw, as an area seeds
/// them) are never the same set twice.
#[test]
fn every_pot_holds_a_plant_and_no_two_desks_share_their_props() {
    use crate::colour::hex;
    use crate::kit::{Plant, Pot};
    use crate::math::T;
    use crate::models::{B, Growth, plant};
    use crate::rng::Rng;
    let pots = [
        Pot::Terracotta,
        Pot::Ceramic(hex("#e8e4dc")),
        Pot::Box(hex("#6b4a2f")),
        Pot::Basket,
        Pot::Concrete,
    ];
    for pot in pots {
        for kind in [Plant::Monstera, Plant::Fern, Plant::Cactus, Plant::Palm] {
            let mut out = Vec::new();
            let mut b = B {
                out: &mut out,
                floor: 0,
                id: 1,
                dim: 1.0,
            };
            let growth = Growth {
                s: 1.0,
                seed: 3,
                sway: 0.1,
                wilt: 0.0,
            };
            plant(&mut b, T::new(), kind, pot, growth);
            assert!(out.len() > 2, "{kind:?} in {pot:?}: a pot and its plant");
        }
    }
    let seed: u32 = 0x5eed;
    let mut drawn_sets = std::collections::HashSet::new();
    let mut sets = std::collections::BTreeSet::new();
    for si in 0..24u32 {
        let mut out = Vec::new();
        let mut b = B {
            out: &mut out,
            floor: 0,
            id: 1,
            dim: 1.0,
        };
        let mut r = Rng(seed ^ si.wrapping_mul(2_654_435_761));
        super::area::props_unlike(
            &mut b,
            (T::new(), 0.74, 1.4),
            &mut r,
            hex("#4a7aff"),
            si % 3 == 0,
            &mut drawn_sets,
        );
        // what's on the desk and in which colours, wherever it stands
        let set: Vec<String> = out
            .iter()
            .map(|item| format!("{:?}{:?}", item.mesh, item.inst.col))
            .collect();
        sets.insert(set);
    }
    assert_eq!(sets.len(), 24, "never the same set twice");
}

/// A person's hover line says its state's word and its App; without a
/// word, just the App (no stray separator).
#[test]
fn a_persons_hover_line_has_no_stray_separator() {
    let mut quiet = session("s:q", FloorState::Working, 0);
    quiet.word = String::new();
    let snap = snapshot(vec![app(
        "a",
        "workshop",
        vec![session("s:w", FloorState::Working, 0), quiet],
    )]);
    let scene = build(&snap);
    let detail = |key: &str| {
        scene
            .picks
            .iter()
            .find(|p| p.target == Target::Session(key.into()))
            .map(|p| p.detail.clone())
    };
    assert_eq!(detail("s:w").as_deref(), Some("Working · Tool a · Work"));
    assert_eq!(detail("s:q").as_deref(), Some("Tool a · Work"));
}

#[test]
fn running_dry_says_how_much_is_left_and_when_it_comes_back() {
    // AJ-E3514 F4: "which Account is about to run dry?" — the flag and the
    // area say the room left and the reset, in the app's words.
    let mut snap = snapshot(vec![app("cli:a", "workshop", vec![])]);
    for account in &mut snap.accounts {
        account.left_pct = Some(12.0);
        account.resets_words = Some("9:31 PM".to_owned());
    }
    let scene = build(&snap);
    let a = scene.app("cli:a").expect("the App");
    assert_eq!(
        scene.room_words(a).map(|(_, words)| words).as_deref(),
        Some("LOW · 12% LEFT · RESETS 9:31 PM")
    );
    assert_eq!(
        scene.area_words(a),
        "No sessions open · 12% left · resets 9:31 PM"
    );
}

#[test]
fn the_rack_says_this_macs_room_and_load_in_the_apps_words() {
    let mut machine = crate::snapshot::FloorMachine {
        load: 0.234,
        ..Default::default()
    };
    assert_eq!(super::mac_words(&machine), "CPU 23%");
    machine.room = Some("Plenty of room".to_owned());
    assert_eq!(super::mac_words(&machine), "Plenty of room · CPU 23%");
    machine.alert = Some("CPU 96% for 6 minutes".to_owned());
    assert_eq!(
        super::mac_words(&machine),
        "Plenty of room · CPU 96% for 6 minutes"
    );
}

#[test]
fn a_floor_says_in_one_line_what_needs_you_most_urgent_first() {
    // The maintainer, 0.4.1: one line a floor ("FLOOR 2 · 2 RUNNING LOW
    // · 1 CRASHED …"), not a flag an App.
    let mut snap = snapshot(vec![
        app(
            "cli:a",
            "workshop",
            vec![
                session("s:1", FloorState::Crashed, 0),
                session("s:2", FloorState::Asks, 1),
            ],
        ),
        app("cli:b", "studio", vec![session("s:3", FloorState::Asks, 0)]),
    ]);
    for account in &mut snap.accounts {
        account.left_pct = Some(8.0);
        account.resets_words = Some("2:24 PM".to_owned());
    }
    let scene = build(&snap);
    let words: Vec<String> = scene.floor_summary(0).into_iter().map(|(_, w)| w).collect();
    assert_eq!(words, ["2 WAITING", "1 CRASHED", "2 LOW"]);
    let detail = scene.floor_detail(0);
    assert_eq!(detail.len(), 2);
    assert!(
        detail[0].ends_with("1 asking · 1 crashed · 8% left · resets 2:24 PM")
            || detail[0].contains("8% left · resets 2:24 PM"),
        "{detail:?}"
    );
    let calm = build(&snapshot(vec![app("cli:c", "workshop", vec![])]));
    assert!(
        calm.floor_summary(0).is_empty()
            || calm
                .floor_summary(0)
                .iter()
                .all(|(n, _)| *n == crate::flags::Need::Low)
    );
}

#[test]
fn an_areas_words_say_how_many_helpers_its_sessions_run() {
    let mut busy = session("s:1", FloorState::Working, 0);
    busy.subagents = 3;
    let scene = build(&snapshot(vec![app("cli:a", "workshop", vec![busy])]));
    let words = scene.area_words(scene.app("cli:a").expect("the App"));
    assert!(words.starts_with("1 working · 3 subagents"), "{words}");
}

/// 0.4.1's offices pass: a lit office's ceiling strip glows and an unused
/// one's hangs dark; the unused one lies under a shade.
#[test]
fn a_lit_offices_strip_glows_and_an_unused_ones_is_dark() {
    let mut idle = app("cli:b", "workshop", Vec::new());
    idle.open = false;
    idle.last_active_ms = Some(NOW - 10 * DAY);
    let busy = app(
        "cli:a",
        "workshop",
        vec![session("s:a", FloorState::Working, 0)],
    );
    let snap = snapshot(vec![busy, idle]);
    let scene = build(&snap);
    let strip_y = super::floor_y(0) + crate::layout::WALL_H - 0.2;
    let near = |m: &[[f32; 4]; 4], y: f32| (m[3][1] - y).abs() < 0.05;
    let of = |id: &str| {
        let at = scene
            .picks
            .iter()
            .position(|p| p.target == Target::App(id.into()));
        u32::try_from(at.expect("drawn") + 1).expect("few picks")
    };
    let glowing = |id: u32| {
        scene
            .stat
            .iter()
            .filter(|i| i.inst.id == id && near(&i.inst.m, strip_y) && i.inst.misc[2] >= 0.5)
            .count()
    };
    assert!(glowing(of("cli:a")) >= 1, "the lit office's strip glows");
    assert_eq!(glowing(of("cli:b")), 0, "the unused office's is dark");
    let shaded = |id: u32| {
        scene.stat.iter().any(|i| {
            i.inst.id == id && i.glass && i.mesh == crate::mesh::Mesh::QuadUp && i.inst.col[0] < 0.2
        })
    };
    assert!(shaded(of("cli:b")) && !shaded(of("cli:a")));
}

/// FL1 (rc31): a flag opens what it names: an OUT or LOW badge its
/// App's Account (the walk's OUT badge opened nothing), any other badge
/// its desk, an area's name its App.
#[test]
fn a_flag_opens_what_it_names() {
    use crate::flags::Need;
    let snap = snapshot(vec![app(
        "a",
        "claude",
        vec![
            session("1", FloorState::Limit, 0),
            session("2", FloorState::Crashed, 0),
        ],
    )]);
    let scene = build(&snap);
    let desk = scene.pick_of_session("1").expect("a desk");
    let account = scene.pick_of_account("acct:1").expect("its sign");
    assert_eq!(
        scene.pick_of(account).map(|p| &p.target),
        Some(&Target::Account("acct:1".into()))
    );
    assert_eq!(scene.door_of_flag(desk, Some(Need::Out)), account);
    assert_eq!(scene.door_of_flag(desk, Some(Need::Low)), account);
    let crashed = scene.pick_of_session("2").expect("a desk");
    assert_eq!(scene.door_of_flag(crashed, Some(Need::Crashed)), crashed);
    let area = scene
        .labels
        .iter()
        .find(|l| l.app == "a")
        .expect("a label")
        .id;
    assert_eq!(scene.door_of_flag(area, None), area);
    assert_eq!(scene.door_of_flag(area, Some(Need::Out)), account);
    // every balloon says whose desk it's over
    let frame = scene.frame(&Live::default(), &[0], &mut Vec::new());
    assert!(
        frame
            .marks
            .iter()
            .any(|m| matches!(m, Mark::Balloon { id, .. } if *id == desk)),
        "{:?}",
        frame.marks
    );
}
