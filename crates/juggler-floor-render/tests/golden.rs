//! AJ-E3513: the Floor's gallery of golden frames, drawn offscreen from a
//! neutral sample company (made-up names only, so the frames can be
//! shared): each floor, the building, a full turn, both tilt limits,
//! every state and family, hover, the red alert, and a reflow.
//!
//! - On Linux (CI's software Vulkan) each frame is compared with its
//!   baseline in `tests/golden/` under a tolerance; a changed frame is
//!   approved like a visual baseline, with a note
//!   (`AJ_WRITE_GOLDEN=1` writes them).
//! - On a Mac it is the Metal smoke render: every frame draws a real
//!   picture.
//! - With no GPU at all the test says why it stops.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use juggler_floor::camera::{Cam, PITCH_MAX, PITCH_MIN, View};
use juggler_floor::family::Family;
use juggler_floor::layout::Office;
use juggler_floor::paint::Img;
use juggler_floor::scene::{Inputs, Live, Scene, capacity, live_sessions};
use juggler_floor::seating::Seating;
use juggler_floor::snapshot::{
    FloorAccount, FloorApp, FloorAppKind, FloorMachine, FloorPool, FloorSession, FloorSnapshot,
};
use juggler_floor::state::FloorState;
use juggler_floor_render::{Error, Extras, Renderer};

const NOW: i64 = 1_800_000_000_000;
const DAY: i64 = 24 * 60 * 60 * 1000;
const SIZE: (u32, u32, f32) = (640, 400, 1.0);

fn session(key: &str, state: FloorState, subagents: u32) -> FloorSession {
    FloorSession {
        key: key.into(),
        title: format!("Task {key}"),
        state,
        word: String::new(),
        subagents,
        project: Some("shop".into()),
        tasks: 1,
        cpu_pct: 4.0,
        started_ms: NOW - 60_000,
        last_ms: NOW - 1000,
    }
}

/// The sample company: every family, every state, a hot App, unused ones,
/// low and empty Accounts, a pool, notices.
fn company(grown: bool) -> FloorSnapshot {
    use FloorState::{Asks, Crashed, Done, Idle, Limit, Stuck, Working};
    let app = |id: &str,
               name: &str,
               family: &str,
               slot: u8,
               account: &str,
               sessions: Vec<FloorSession>| FloorApp {
        id: id.into(),
        name: name.into(),
        tool: "tool".into(),
        host_app: None,
        logo: None,
        kind: if family == "studio" {
            FloorAppKind::Desktop
        } else {
            FloorAppKind::Cli
        },
        family: family.into(),
        colour_slot: slot,
        account_id: Some(account.into()),
        initials: if account == "a1" {
            "WK".into()
        } else {
            "HM".into()
        },
        open: !sessions.is_empty(),
        last_active_ms: Some(if sessions.is_empty() {
            NOW - 12 * DAY
        } else {
            NOW
        }),
        cpu_pct: 0.0,
        hot_since_ms: None,
        sessions,
    };
    let s = |prefix: &str, states: &[(FloorState, u32)]| -> Vec<FloorSession> {
        states
            .iter()
            .enumerate()
            .map(|(i, (st, sub))| session(&format!("s:{prefix}{i}"), *st, *sub))
            .collect()
    };
    let mut workshop = app(
        "cli:w",
        "Workshop Tool · Work",
        "workshop",
        0,
        "a1",
        s(
            "w",
            &[(Working, 3), (Asks, 0), (Working, 8), (Idle, 0), (Done, 0)],
        ),
    );
    if grown {
        workshop.sessions.extend(s("g", &[(Working, 0); 9]));
    }
    let mut industrial = app(
        "cli:i",
        "Industrial Tool · Home",
        "industrial",
        1,
        "a2",
        s("i", &[(Working, 0), (Crashed, 0), (Working, 2)]),
    );
    industrial.hot_since_ms = Some(NOW - 4 * 60_000);
    industrial.sessions[0].cpu_pct = 380.0;
    FloorSnapshot {
        at_ms: NOW,
        apps: vec![
            workshop,
            industrial,
            app(
                "app:s",
                "Studio App · Work",
                "studio",
                2,
                "a1",
                s("s", &[(Asks, 0), (Idle, 0)]),
            ),
            app(
                "cli:m",
                "Garage Tool · Lab",
                "maker-garage",
                3,
                "a2",
                s("m", &[(Stuck, 0), (Working, 1)]),
            ),
            app(
                "cli:c",
                "Campus Tool · Lab",
                "campus",
                4,
                "a1",
                s("c", &[(Working, 0), (Working, 0), (Done, 0)]),
            ),
            app(
                "cli:l",
                "Lab Tool · Out",
                "lab",
                5,
                "a3",
                s("l", &[(Limit, 0), (Limit, 0)]),
            ),
            app("cli:n", "Plain Tool · Old", "neutral", 6, "a1", Vec::new()),
            app(
                "cli:x",
                "Extra Tool · Spare",
                "workshop",
                7,
                "a2",
                s("x", &[(Working, 0)]),
            ),
            app(
                "cli:y",
                "Another Tool · Spare",
                "industrial",
                0,
                "a1",
                s("y", &[(Idle, 0)]),
            ),
            app(
                "cli:z",
                "Last Tool · Spare",
                "studio",
                1,
                "a2",
                s("z", &[(Working, 0), (Asks, 0)]),
            ),
            app(
                "cli:q",
                "Quiet Tool · Spare",
                "lab",
                2,
                "a1",
                s("q", &[(Working, 0)]),
            ),
        ],
        accounts: vec![
            FloorAccount {
                id: "a1".into(),
                label: "Work".into(),
                left_pct: Some(62.0),
                resets_ms: None,
                last_used_ms: None,
                out: false,
            },
            FloorAccount {
                id: "a2".into(),
                label: "Home".into(),
                left_pct: Some(14.0),
                resets_ms: None,
                last_used_ms: None,
                out: false,
            },
            FloorAccount {
                id: "a3".into(),
                label: "Out".into(),
                left_pct: Some(0.0),
                resets_ms: None,
                last_used_ms: None,
                out: true,
            },
        ],
        pools: vec![FloorPool {
            id: "pool-1".into(),
            name: "Shop".into(),
            sealed: false,
            sessions: vec!["s:w0".into(), "s:w1".into(), "s:w2".into()],
        }],
        machine: FloorMachine {
            load: 0.55,
            alert_since_ms: None,
            alert: None,
        },
        notices: 4,
    }
}

fn scene_of(snap: &FloorSnapshot, office: &mut Office, seating: &mut Seating) -> Scene {
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
    let seats: BTreeMap<_, _> = snap
        .apps
        .iter()
        .map(|a| {
            (
                a.id.clone(),
                seating.seat(&a.id, capacity(office, &a.id), &live_sessions(a)),
            )
        })
        .collect();
    Scene::build(Inputs {
        snapshot: snap,
        office,
        seats: &seats,
        decay: true,
        machine: "This computer",
        now_ms: NOW,
        logos: &BTreeMap::new(),
    })
    .unwrap()
}

/// One frame of the gallery.
struct Shot {
    name: String,
    view: View,
    live: Live,
    alert: Option<&'static str>,
    mouse: Option<(f32, f32)>,
}

fn shots(scene: &Scene) -> Vec<Shot> {
    let frame = scene.frame_size();
    let aspect = SIZE.0 as f32 / SIZE.1 as f32;
    let live = Live {
        t: 2.0,
        ..Live::default()
    };
    let shot = |name: &str, view: View| Shot {
        name: name.into(),
        view,
        live: live.clone(),
        alert: None,
        mouse: None,
    };
    let mut out = vec![shot(
        "building",
        View {
            focus: None,
            cam: frame.home(None, aspect),
        },
    )];
    for f in 0..frame.floors {
        out.push(shot(
            &format!("floor-{}", f + 1),
            View {
                focus: Some(f),
                cam: frame.home(Some(f), aspect),
            },
        ));
    }
    let home = frame.home(Some(0), aspect);
    for k in 0..4 {
        out.push(shot(
            &format!("turn-{k}"),
            View {
                focus: Some(0),
                cam: Cam {
                    yaw: home.yaw + k as f32 * std::f32::consts::FRAC_PI_2,
                    ..home
                },
            },
        ));
    }
    out.push(shot(
        "tilt-low",
        View {
            focus: Some(0),
            cam: Cam {
                pitch: PITCH_MIN,
                ..home
            },
        },
    ));
    out.push(shot(
        "tilt-high",
        View {
            focus: Some(0),
            cam: Cam {
                pitch: PITCH_MAX,
                ..home
            },
        },
    ));
    for (i, area) in scene.area_stops().iter().enumerate() {
        let cam = Cam {
            t: [
                area.at[0],
                juggler_floor::scene::floor_y(area.floor),
                area.at[2] - 3.0,
            ],
            yaw: 0.5,
            pitch: 0.8,
            dist: 15.0,
        };
        out.push(shot(
            &format!("area-{i:02}"),
            View {
                focus: Some(area.floor),
                cam,
            },
        ));
    }
    if let Some(wait) = scene.waiting_stops().first() {
        let cam = Cam {
            t: [
                wait.at[0],
                juggler_floor::scene::floor_y(wait.floor),
                wait.at[2],
            ],
            yaw: 2.2,
            pitch: 1.05,
            dist: 9.0,
        };
        let view = View {
            focus: Some(wait.floor),
            cam,
        };
        let p = cam
            .vp(aspect)
            .project(scene.anchor(wait.id).unwrap_or(wait.at));
        let mouse = (
            (p[0] / p[3] * 0.5 + 0.5) * SIZE.0 as f32,
            (0.5 - p[1] / p[3] * 0.5) * SIZE.1 as f32,
        );
        out.push(Shot {
            name: "hover".into(),
            view,
            live: Live {
                hover: wait.id,
                ..live.clone()
            },
            alert: None,
            mouse: Some(mouse),
        });
    }
    out.push(Shot {
        name: "red-alert".into(),
        view: View {
            focus: Some(0),
            cam: home,
        },
        live: Live {
            alert: true,
            load: 0.98,
            ..live.clone()
        },
        alert: Some("CPU 98% for 6 minutes"),
        mouse: None,
    });
    out
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn write_png(path: &std::path::Path, img: &Img) {
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), img.w as u32, img.h as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&img.px)
        .unwrap();
}

fn read_png(path: &std::path::Path) -> Option<Vec<u8>> {
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).ok()?));
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    Some(buf)
}

/// The share of pixels that differ by more than a little.
fn changed(a: &[u8], b: &[u8]) -> f32 {
    if a.len() != b.len() {
        return 1.0;
    }
    let differ = a
        .chunks(4)
        .zip(b.chunks(4))
        .filter(|(p, q)| (0..3).any(|k| p[k].abs_diff(q[k]) > 24))
        .count();
    differ as f32 / (a.len() / 4).max(1) as f32
}

#[test]
fn the_gallery_of_golden_frames() {
    let mut office = Office::new(42, 10);
    let mut seating = Seating::default();
    let scene = scene_of(&company(false), &mut office, &mut seating);
    let mut renderer = match Renderer::headless(&scene) {
        Ok(r) => r,
        Err(Error::NoAdapter(why))
            if !cfg!(target_os = "macos") || std::env::var_os("CI").is_some() =>
        {
            eprintln!("no GPU here, the gallery stops: {why}");
            return;
        }
        Err(e) => panic!("{e}"),
    };
    let mut frames: Vec<(String, Img)> = Vec::new();
    for shot in shots(&scene) {
        let extras = Extras {
            hint: "Drag: pan  ·  Right-drag: turn and tilt  ·  Pinch: zoom  ·  H: reset",
            alert: shot.alert,
            mouse: shot.mouse,
        };
        frames.push((
            shot.name,
            renderer
                .capture(&scene, &shot.view, &shot.live, SIZE, &extras)
                .unwrap(),
        ));
    }
    // A reflow: the first App grows from M to L; the areas after it move on.
    let grown = scene_of(&company(true), &mut office, &mut seating);
    renderer.load(&grown);
    let home = grown
        .frame_size()
        .home(Some(0), SIZE.0 as f32 / SIZE.1 as f32);
    let extras = Extras {
        hint: "",
        alert: None,
        mouse: None,
    };
    frames.push((
        "reflow-after".into(),
        renderer
            .capture(
                &grown,
                &View {
                    focus: Some(0),
                    cam: home,
                },
                &Live {
                    t: 2.0,
                    ..Live::default()
                },
                SIZE,
                &extras,
            )
            .unwrap(),
    ));

    for (name, img) in &frames {
        let colours: std::collections::BTreeSet<[u8; 3]> = img
            .px
            .chunks(4)
            .step_by(53)
            .map(|p| [p[0], p[1], p[2]])
            .collect();
        assert!(
            colours.len() > 30,
            "{name}: a real picture ({} colours)",
            colours.len()
        );
    }
    let compare = cfg!(target_os = "linux");
    let write = std::env::var_os("AJ_WRITE_GOLDEN").is_some();
    let dump = std::env::var_os("AJ_DUMP_GOLDEN").map(PathBuf::from);
    if let Some(dir) = &dump {
        std::fs::create_dir_all(dir).unwrap();
    }
    let mut failures = Vec::new();
    for (name, img) in &frames {
        let path = golden_dir().join(format!("{name}.png"));
        if let Some(dir) = &dump {
            write_png(&dir.join(format!("{name}.png")), img);
        }
        if write && compare {
            std::fs::create_dir_all(golden_dir()).unwrap();
            write_png(&path, img);
            continue;
        }
        if compare {
            match read_png(&path) {
                Some(base) => {
                    let share = changed(&base, &img.px);
                    if share > 0.02 {
                        failures.push(format!("{name}: {:.1}% of pixels changed", share * 100.0));
                    }
                }
                None => failures.push(format!("{name}: no baseline (AJ_WRITE_GOLDEN=1 writes it)")),
            }
        }
    }
    assert!(
        failures.is_empty(),
        "the gallery changed:\n{}",
        failures.join("\n")
    );
}
