//! AJ-E3507: the renderer draws a real office offscreen, and a pick at a
//! desk's sitter is that desk's session. On a Mac there is always a GPU;
//! elsewhere (a CI box without one) the test says why it stops.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::collections::BTreeMap;

use juggler_floor::camera::{Cam, View};
use juggler_floor::family::Family;
use juggler_floor::layout::Office;
use juggler_floor::scene::{Inputs, Live, Scene, Target, capacity, live_sessions};
use juggler_floor::seating::Seating;
use juggler_floor::snapshot::{
    FloorAccount, FloorApp, FloorAppKind, FloorMachine, FloorSession, FloorSnapshot,
};
use juggler_floor::state::FloorState;
use juggler_floor_render::{Error, Extras, Renderer};

const NOW: i64 = 1_800_000_000_000;

fn snapshot() -> FloorSnapshot {
    let session = |key: &str, state| FloorSession {
        key: key.into(),
        title: key.into(),
        state,
        word: String::new(),
        subagents: 0,
        project: None,
        tasks: 0,
        cpu_pct: 1.0,
        started_ms: NOW,
        last_ms: NOW,
        go_to: None,
    };
    let app = FloorApp {
        id: "cli:a".into(),
        name: "Tool · Work".into(),
        tool: "tool".into(),
        host_app: None,
        logo: None,
        kind: FloorAppKind::Cli,
        family: "workshop".into(),
        colour_slot: 0,
        account_id: Some("acct".into()),
        initials: "W".into(),
        open: true,
        last_active_ms: Some(NOW),
        cpu_pct: 1.0,
        hot_since_ms: None,
        sessions: vec![
            session("s:1", FloorState::Working),
            session("s:2", FloorState::Asks),
        ],
    };
    FloorSnapshot {
        at_ms: NOW,
        apps: vec![app],
        accounts: vec![FloorAccount {
            id: "acct".into(),
            label: "Work".into(),
            left_pct: Some(60.0),
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

/// The sample office's scene.
fn scene(snap: &FloorSnapshot) -> Scene {
    let mut office = Office::new(3, 10);
    let open: Vec<_> = snap
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
    let seats: BTreeMap<_, _> = snap
        .apps
        .iter()
        .map(|a| {
            (
                a.id.clone(),
                seating.seat(&a.id, capacity(&office, &a.id), &live_sessions(a)),
            )
        })
        .collect();
    Scene::build(Inputs {
        snapshot: snap,
        office: &office,
        seats: &seats,
        decay: true,
        covers: false,
        detail: false,
        hide: &std::collections::BTreeSet::new(),
        machine: "This computer",
        now_ms: NOW,
        logos: &BTreeMap::new(),
    })
    .unwrap()
}

#[test]
fn it_draws_the_office_offscreen_and_picks_a_desk() {
    let snap = snapshot();
    let scene = scene(&snap);
    let mut renderer = match Renderer::headless(&scene) {
        Ok(r) => r,
        Err(Error::NoAdapter(why))
            if !cfg!(target_os = "macos") || std::env::var_os("CI").is_some() =>
        {
            eprintln!("no GPU here, the drawing test stops: {why}");
            return;
        }
        Err(e) => panic!("{e}"),
    };
    let (w, h, sf) = (800u32, 500u32, 2.0f32);
    let desk = scene.pick_of_session("s:1").unwrap();
    let at = scene.anchor(desk).unwrap();
    let view = View {
        focus: Some(0),
        cam: Cam {
            t: [at[0], 0.0, at[2]],
            yaw: 0.4,
            pitch: 0.9,
            dist: 9.0,
        },
    };
    let extras = Extras {
        hint: "Drag to pan",
        alert: None,
        mouse: None,
        card: None,
        opened: None,
    };
    let img = renderer
        .capture(
            &scene,
            &view,
            &Live {
                t: 1.0,
                ..Live::default()
            },
            (w, h, sf),
            &extras,
        )
        .unwrap();
    assert_eq!((img.w, img.h), (800, 500));
    // To look at it: FLOOR_DUMP=<file> writes the frame as raw RGBA.
    if let Ok(path) = std::env::var("FLOOR_DUMP") {
        std::fs::write(path, &img.px).unwrap();
    }
    let distinct: std::collections::BTreeSet<[u8; 4]> = img
        .px
        .chunks(4)
        .step_by(97)
        .map(|p| [p[0], p[1], p[2], p[3]])
        .collect();
    assert!(
        distinct.len() > 40,
        "a real picture, not a flat colour: {} colours",
        distinct.len()
    );
    let c = view.cam.vp(w as f32 / h as f32).project(at);
    let (sx, sy) = (
        (c[0] / c[3] * 0.5 + 0.5) * w as f32,
        (0.5 - c[1] / c[3] * 0.5) * h as f32,
    );
    let near: Vec<u32> = (-6..=6)
        .flat_map(|dx| (-6..=6).map(move |dy| (dx, dy)))
        .map(|(dx, dy)| renderer.pick(sx + dx as f32 * 2.0, sy + dy as f32 * 2.0, sf))
        .collect();
    assert!(
        near.iter().any(|id| scene
            .pick_of(*id)
            .is_some_and(|p| p.target == Target::Session("s:1".into()))),
        "the sitter at ({sx}, {sy}) picks its session: {near:?}"
    );
    assert_eq!(renderer.pick(-5.0, -5.0, sf), 0);
}
