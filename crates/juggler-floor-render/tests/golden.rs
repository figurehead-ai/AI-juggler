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
        go_to: None,
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
                resets_words: None,
            },
            FloorAccount {
                id: "a2".into(),
                label: "Home".into(),
                left_pct: Some(14.0),
                resets_ms: None,
                last_used_ms: None,
                out: false,
                resets_words: None,
            },
            FloorAccount {
                id: "a3".into(),
                label: "Out".into(),
                left_pct: Some(0.0),
                resets_ms: None,
                last_used_ms: None,
                out: true,
                resets_words: Some("9:31 PM".into()),
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
            room: None,
        },
        notices: 4,
        needs: 0,
    }
}

fn scene_of(snap: &FloorSnapshot, office: &mut Office, seating: &mut Seating) -> Scene {
    // as the app builds it: dust sheets with the decay setting (AJ-E3518)
    scene_with(snap, office, seating, true)
}

fn scene_with(
    snap: &FloorSnapshot,
    office: &mut Office,
    seating: &mut Seating,
    covers: bool,
) -> Scene {
    // the default look's people (D-86)
    scene_full(snap, office, seating, covers, true)
}

/// A scene with 0.4.1's dust sheets (`covers`) and second pass
/// (`detail`) chosen.
fn scene_full(
    snap: &FloorSnapshot,
    office: &mut Office,
    seating: &mut Seating,
    covers: bool,
    detail: bool,
) -> Scene {
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
        covers,
        detail,
        hide: &std::collections::BTreeSet::new(),
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
            card: None,
        };
        frames.push((
            shot.name,
            renderer
                .capture(&scene, &shot.view, &shot.live, SIZE, &extras)
                .unwrap(),
        ));
    }
    // D-80: Full resolution, the same scene at device pixels: the
    // building, a floor, its first area up close, a hover and the alert.
    renderer.set_full(true);
    for shot in shots(&scene).into_iter().filter(|shot| {
        ["building", "floor-1", "area-00", "hover", "red-alert"].contains(&shot.name.as_str())
    }) {
        let extras = Extras {
            hint: "Drag: pan  ·  Right-drag: turn and tilt  ·  Pinch: zoom  ·  H: reset",
            alert: shot.alert,
            mouse: shot.mouse,
            card: None,
        };
        frames.push((
            format!("full-{}", shot.name),
            renderer
                .capture(&scene, &shot.view, &shot.live, SIZE, &extras)
                .unwrap(),
        ));
    }
    // Picking per mode (D-80): under the hover shot's pointer both modes
    // find the same thing, one id per scene pixel (a device pixel in Full
    // resolution), after switching back and forth with no restart.
    let hover = shots(&scene)
        .into_iter()
        .find(|shot| shot.name == "hover")
        .unwrap();
    let (mx, my) = hover.mouse.unwrap();
    let extras = Extras {
        hint: "",
        alert: None,
        mouse: hover.mouse,
        card: None,
    };
    let mut picked = Vec::new();
    for full in [true, false, true, false] {
        renderer.set_full(full);
        renderer
            .capture(&scene, &hover.view, &hover.live, SIZE, &extras)
            .unwrap();
        picked.push(renderer.pick(mx, my, SIZE.2));
    }
    assert!(picked[0] != 0, "something is under the pointer");
    assert!(
        picked.iter().all(|id| *id == picked[0]),
        "both modes pick the same: {picked:?}"
    );
    // D-80's captures (opt-in, `AJ_D80_CAPTURE=<folder>`): both looks at
    // 1280 × 800 on a Retina-scaled window, for the gallery's notes.
    if let Some(dir) = std::env::var_os("AJ_D80_CAPTURE").map(PathBuf::from) {
        std::fs::create_dir_all(&dir).unwrap();
        let big = (1280, 800, 2.0);
        let aspect = big.0 as f32 / big.1 as f32;
        let frame = scene.frame_size();
        let views = [
            (
                "building",
                View {
                    focus: None,
                    cam: frame.home(None, aspect),
                },
            ),
            (
                "floor-1",
                View {
                    focus: Some(0),
                    cam: frame.home(Some(0), aspect),
                },
            ),
        ];
        let extras = Extras {
            hint: "Drag: pan  ·  Right-drag: turn and tilt  ·  Pinch: zoom  ·  H: reset",
            alert: None,
            mouse: None,
            card: None,
        };
        for full in [false, true] {
            renderer.set_full(full);
            for (name, view) in &views {
                let live = Live {
                    t: 2.0,
                    ..Live::default()
                };
                let img = renderer.capture(&scene, view, &live, big, &extras).unwrap();
                let look = if full { "full-resolution" } else { "pixel-art" };
                write_png(&dir.join(format!("{look}-{name}-1280x800.png")), &img);
            }
        }
        renderer.set_full(false);
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
        card: None,
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
    // the model sheet (AJ-E3506 R1)
    let sheet = model_sheet();
    renderer.load(&sheet);
    frames.push((
        "models".into(),
        renderer
            .capture(
                &sheet,
                &model_sheet_view(),
                &Live {
                    t: 0.6,
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

/// The area chips never print over each other, the floor switcher, the
/// hint line or a balloon, and never leave the window: in the usual
/// office and in a crowded one with long names, in the building view and
/// on each floor, at three window sizes (the D-80 review: the building's
/// App labels printed over each other and floor 1's balloons hid them).
#[test]
fn area_chips_never_overlap() {
    // AJ-E3515: flags by zoom: never over each other, the switcher or the
    // hint, inside the window, over their own floor in the building, and
    // every count that needs you on some flag (none lost)
    use juggler_floor::flags::{Labels, band};
    use juggler_floor::overlay;
    let mut crowded = company(false);
    let base = crowded.apps.clone();
    for n in 0..16 {
        let mut a = base[n % base.len()].clone();
        a.id = format!("{}-{n}", a.id);
        a.name = format!("A Rather Long App Name Number {n}");
        a.sessions = a
            .sessions
            .iter()
            .map(|s| FloorSession {
                key: format!("{}-{n}", s.key),
                ..s.clone()
            })
            .collect();
        crowded.apps.push(a);
    }
    let mut bands = std::collections::BTreeSet::new();
    // the reflowed office too: the first App grown, built on the usual
    // office as the gallery's `reflow-after` is
    let (mut office, mut seating) = (Office::new(42, 10), Seating::default());
    for (label, snap) in [
        ("usual", company(false)),
        ("reflowed", company(true)),
        ("crowded", crowded),
    ] {
        if label == "crowded" {
            (office, seating) = (Office::new(42, 10), Seating::default());
        }
        let scene = scene_of(&snap, &mut office, &mut seating);
        let frame = scene.frame_size();
        for size in [(1280u32, 800u32, 2.0f32), (1024, 700, 1.0), (640, 400, 1.0)] {
            let aspect = size.0 as f32 / size.1 as f32;
            let mut views: Vec<View> = std::iter::once(None)
                .chain((0..scene.floors).map(Some))
                .map(|focus| View {
                    focus,
                    cam: frame.home(focus, aspect),
                })
                .collect();
            // a floor at the middle and far zooms too
            for dist in [0.55f32, 1.6] {
                let home = frame.home(Some(0), aspect);
                views.push(View {
                    focus: Some(0),
                    cam: Cam {
                        dist: home.dist * dist,
                        ..home
                    },
                });
            }
            for view in views {
                // the red alert's banner is kept clear like the controls
                let alerts = [None, Some("CPU 98% for 6 minutes")];
                for (labels, alert) in [Labels::Smart, Labels::Attention, Labels::Names]
                    .into_iter()
                    .flat_map(|l| alerts.map(|a| (l, a)))
                {
                    let live = Live {
                        t: 2.0,
                        eye: view.cam.eye(),
                        ..Live::default()
                    };
                    let mut moving = Vec::new();
                    let drawn = scene.frame(&live, &view.floors(&frame), &mut moving);
                    let vp = view.cam.vp(aspect);
                    let b = band(
                        overlay::desk_px(&view, &vp, (size.0, size.1), size.2),
                        view.focus.is_none(),
                        None,
                    );
                    bands.insert(format!("{b:?}"));
                    let (_, _, placed) = overlay::build_placed(
                        &scene,
                        &overlay::Inputs {
                            view: &view,
                            vp: &vp,
                            size: (size.0, size.1),
                            scale: size.2,
                            marks: &drawn.marks,
                            hover: 0,
                            mouse: None,
                            alert,
                            hint: "Drag: pan",
                            band: b,
                            labels,
                            card: None,
                        },
                    );
                    let at = format!(
                        "{label}, {size:?}, floor {:?}, dist {:.0}, {b:?}, {labels:?}, alert {}",
                        view.focus,
                        view.cam.dist,
                        alert.is_some()
                    );
                    let meets = |a: &[f32; 4], b: &[f32; 4]| {
                        a[0] < b[0] + b[2]
                            && b[0] < a[0] + a[2]
                            && a[1] < b[1] + b[3]
                            && b[1] < a[1] + a[3]
                    };
                    for (n, a) in placed.chips.iter().enumerate() {
                        assert!(
                            a[0] >= 0.0
                                && a[1] >= 0.0
                                && a[0] + a[2] <= size.0 as f32
                                && a[1] + a[3] <= size.1 as f32,
                            "{at}: flag {a:?} leaves the window"
                        );
                        for b in &placed.chips[n + 1..] {
                            assert!(!meets(a, b), "{at}: flags {a:?} and {b:?} overlap");
                        }
                        for b in &placed.kept_clear {
                            assert!(!meets(a, b), "{at}: flag {a:?} covers {b:?}");
                        }
                        if let Some(banner) = &placed.banner {
                            assert!(!meets(a, banner), "{at}: flag {a:?} under the alert");
                        }
                        if let Some(lift) = &placed.lift {
                            assert!(!meets(a, lift), "{at}: flag {a:?} under the lift");
                        }
                    }
                    for (a, f) in placed.chips.iter().zip(&placed.floors) {
                        for (g, outline) in placed.outlines.iter().enumerate() {
                            assert!(
                                g == *f || !overlay::meets(a, outline),
                                "{at}: floor {f}'s flag {a:?} sits on floor {g}"
                            );
                        }
                    }
                    assert_eq!(placed.counts.0, placed.counts.1, "{at}: a count was lost");
                    assert_eq!(placed.banner.is_some(), alert.is_some(), "{at}: the banner");
                    assert_eq!(placed.left_out_urgent, 0, "{at}");
                    if b == juggler_floor::flags::Band::Close {
                        assert!(placed.chips.is_empty(), "{at}: no flags up close");
                    }
                    if labels == Labels::Attention {
                        assert_eq!(placed.left_out, 0, "{at}: no names asked for");
                    }
                }
            }
        }
    }
    assert!(
        bands.contains("Middle") && bands.contains("Far"),
        "the views cover the bands: {bands:?}"
    );
}

/// 0.4.1's design proposal (`docs/research/floor-04x.md` §10), opt-in:
/// `AJ_FLOOR_PROPOSAL=<folder> cargo test -p juggler-floor-render --test
/// golden -- --ignored proposal`. The same made-up company in 0.4.0's look
/// and the reworked one, by day, at the golden hour, at dusk and at night,
/// up close, unused, in both looks, and at three zooms without the
/// overlay, with `anchors.json` saying where each area and each desk that
/// needs you lands on screen (the proposal draws its labels there).
#[test]
#[ignore = "writes the design proposal's frames; opt-in"]
fn the_design_proposal_frames() {
    use juggler_floor::daylight::Sky;
    use std::fmt::Write as _;

    use juggler_floor::scene::Mark;
    use juggler_floor_render::Look;

    let Some(dir) = std::env::var_os("AJ_FLOOR_PROPOSAL").map(PathBuf::from) else {
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    let mut office = Office::new(42, 10);
    let mut seating = Seating::default();
    let scene = scene_full(&company(false), &mut office, &mut seating, false, false);
    let (mut office2, mut seating2) = (Office::new(42, 10), Seating::default());
    let covered = scene_full(&company(false), &mut office2, &mut seating2, true, false);
    let mut renderer = Renderer::headless(&scene).unwrap();
    let size = (1280, 800, 1.0);
    let aspect = size.0 as f32 / size.1 as f32;
    let frame = scene.frame_size();
    let home = frame.home(Some(0), aspect);
    let live = Live {
        t: 2.0,
        ..Live::default()
    };
    let extras = Extras {
        hint: "Drag: pan  ·  Right-drag: turn and tilt  ·  Pinch: zoom  ·  H: reset",
        alert: None,
        mouse: None,
        card: None,
    };
    let floor = View {
        focus: Some(0),
        cam: home,
    };
    let area_cam = |app: &str, dist: f32| {
        let l = scene.labels.iter().find(|l| l.app == app).unwrap();
        View {
            focus: Some(l.floor),
            cam: Cam {
                t: [l.at[0], l.at[1] - 2.5, l.at[2] + 1.0],
                yaw: 0.5,
                pitch: 0.85,
                dist,
            },
        }
    };
    let classic = Look::classic();
    let day = Look {
        sky: Sky::always_day(),
        rework: true,
        ..Look::classic()
    };
    let looks = [
        ("before", classic),
        ("after-day", day),
        (
            "after-golden",
            Look {
                sky: Sky::at(3.0, 250.0),
                rework: true,
                ..Look::classic()
            },
        ),
        (
            "after-dusk",
            Look {
                sky: Sky::at(-4.0, 262.0),
                rework: true,
                ..Look::classic()
            },
        ),
        (
            "after-night",
            Look {
                sky: Sky::always_night(),
                rework: true,
                ..Look::classic()
            },
        ),
    ];
    renderer.set_overlay(false);
    let shoot = |r: &mut Renderer, name: &str, view: &View| {
        let img = r.capture(&scene, view, &live, size, &extras).unwrap();
        write_png(&dir.join(format!("{name}.png")), &img);
    };
    for (name, look) in looks {
        renderer.set_look(look);
        shoot(&mut renderer, &format!("floor-{name}"), &floor);
    }
    for (name, look) in [("before", classic), ("after", day)] {
        renderer.set_look(look);
        shoot(
            &mut renderer,
            &format!("close-{name}"),
            &area_cam("cli:w", 13.0),
        );
        if look.rework {
            renderer.load(&covered);
            let img = renderer
                .capture(&covered, &area_cam("cli:n", 13.0), &live, size, &extras)
                .unwrap();
            write_png(&dir.join("unused-after.png"), &img);
            renderer.set_look(Look {
                sky: Sky::always_night(),
                rework: true,
                ..Look::classic()
            });
            let img = renderer
                .capture(&covered, &area_cam("cli:n", 13.0), &live, size, &extras)
                .unwrap();
            write_png(&dir.join("unused-after-night.png"), &img);
            renderer.set_look(look);
            renderer.load(&scene);
        } else {
            shoot(
                &mut renderer,
                &format!("unused-{name}"),
                &area_cam("cli:n", 13.0),
            );
        }
        renderer.set_full(true);
        shoot(
            &mut renderer,
            &format!("full-close-{name}"),
            &area_cam("cli:w", 13.0),
        );
        renderer.set_full(false);
    }
    // Three zooms, bare, for the labels by zoom; with where things land.
    renderer.set_look(day);
    let zooms = [
        ("zoom-close", area_cam("cli:w", 16.0)),
        (
            "zoom-mid",
            View {
                focus: Some(0),
                cam: home,
            },
        ),
        (
            "zoom-far",
            View {
                focus: Some(0),
                cam: Cam {
                    dist: home.dist * 1.6,
                    ..home
                },
            },
        ),
    ];
    let mut json = String::from("{");
    for (i, (name, view)) in zooms.iter().enumerate() {
        shoot(&mut renderer, name, view);
        let vp = view.cam.vp(aspect);
        let on_screen = |p: [f32; 3]| {
            let c = vp.project(p);
            (
                (c[0] / c[3] * 0.5 + 0.5) * size.0 as f32,
                (0.5 - c[1] / c[3] * 0.5) * size.1 as f32,
            )
        };
        let mut moving = Vec::new();
        let drawn = scene.frame(&live, &[0], &mut moving);
        let areas: Vec<String> = scene
            .labels
            .iter()
            .filter(|l| l.floor == 0)
            .filter_map(|l| {
                let app = scene.app(&l.app)?;
                let count = |s: FloorState| app.sessions.iter().filter(|x| x.state == s).count();
                let (x, y) = on_screen(l.at);
                Some(format!(
                    "{{\"app\":\"{}\",\"name\":\"{}\",\"slot\":{},\"x\":{x:.1},\"y\":{y:.1},\"asks\":{},\"stuck\":{},\"crashed\":{},\"done\":{},\"limit\":{},\"working\":{},\"unused\":{}}}",
                    app.id,
                    app.name,
                    app.colour_slot,
                    count(FloorState::Asks),
                    count(FloorState::Stuck),
                    count(FloorState::Crashed),
                    count(FloorState::Done),
                    count(FloorState::Limit),
                    count(FloorState::Working),
                    scene.unused_level(&app.id)
                ))
            })
            .collect();
        let desks: Vec<String> = drawn
            .marks
            .iter()
            .filter_map(|m| match m {
                Mark::Balloon { at, icon } => {
                    let (x, y) = on_screen(*at);
                    Some(format!(
                        "{{\"icon\":\"{icon:?}\",\"x\":{x:.1},\"y\":{y:.1}}}"
                    ))
                }
                Mark::Plus { .. } => None,
            })
            .collect();
        if i > 0 {
            json.push(',');
        }
        let _ = write!(
            json,
            "\"{name}\":{{\"areas\":[{}],\"desks\":[{}]}}",
            areas.join(","),
            desks.join(",")
        );
    }
    json.push('}');
    std::fs::write(dir.join("anchors.json"), json).unwrap();
    // Today's building and floor, with today's overlay, for the
    // navigation and labels' "before".
    renderer.set_overlay(true);
    renderer.set_look(classic);
    shoot(
        &mut renderer,
        "nav-building-before",
        &View {
            focus: None,
            cam: frame.home(None, aspect),
        },
    );
    shoot(&mut renderer, "labels-before-far", &zooms[2].1);
}

/// The second pass's material maps as swatches, each lit from the upper
/// left on a typical colour, for the design proposal (`AJ_FLOOR_PROPOSAL`
/// names the folder).
#[test]
#[ignore = "writes the design proposal's swatches; opt-in"]
fn the_material_swatches() {
    use juggler_floor::materials as m;
    let Some(dir) = std::env::var_os("AJ_FLOOR_PROPOSAL").map(PathBuf::from) else {
        return;
    };
    let dir = dir.join("swatches");
    std::fs::create_dir_all(&dir).unwrap();
    let swatches = [
        ("wood", m::WOOD, [0.72, 0.5, 0.32]),
        ("fabric", m::FABRIC, [0.3, 0.42, 0.62]),
        ("carpet", m::CARPET, [0.55, 0.36, 0.34]),
        ("metal", m::METAL, [0.62, 0.63, 0.66]),
        ("plastic", m::PLASTIC, [0.9, 0.9, 0.88]),
        ("leaf", m::LEAF, [0.2, 0.5, 0.25]),
        ("skin", m::SKIN, [0.9, 0.72, 0.6]),
        ("hair", m::HAIR, [0.42, 0.27, 0.16]),
        ("cloth", m::CLOTH, [0.78, 0.3, 0.24]),
        ("denim", m::DENIM, [0.2, 0.27, 0.42]),
        ("leather", m::LEATHER, [0.45, 0.28, 0.18]),
        ("ceramic", m::CERAMIC, [0.85, 0.83, 0.78]),
        ("plaster", m::PLASTER, [0.86, 0.84, 0.8]),
        ("rubber", m::RUBBER, [0.2, 0.2, 0.22]),
        ("paper", m::PAPER, [0.95, 0.93, 0.88]),
        ("soil", m::SOIL, [0.32, 0.22, 0.15]),
    ];
    let mut sheet = Img::new(4 * 260, 4 * 260);
    for (i, (name, mat, colour)) in swatches.into_iter().enumerate() {
        let img = m::preview(mat, colour);
        let k = i32::try_from(i).unwrap_or(0);
        sheet.blit(&img, (k % 4) * 260, (k / 4) * 260);
        write_png(&dir.join(format!("{name}.png")), &img);
    }
    write_png(&dir.join("sheet.png"), &sheet);
}

/// A part of `img`: `w` × `h` from `(x, y)`.
#[allow(clippy::many_single_char_names)] // a rectangle's own short names
fn crop(img: &Img, x: i32, y: i32, w: i32, h: i32) -> Img {
    let mut out = Img::new(w, h);
    for j in 0..h {
        for i in 0..w {
            let (sx, sy) = ((x + i).clamp(0, img.w - 1), (y + j).clamp(0, img.h - 1));
            let s = ((sy * img.w + sx) * 4) as usize;
            let o = ((j * w + i) * 4) as usize;
            out.px[o..o + 4].copy_from_slice(&img.px[s..s + 4]);
        }
    }
    out
}

/// The second pass's frames for the design proposal (0.4.1, research
/// §12): the first pass (v1) and the second (v2) side by side, up close
/// and at the normal zoom, in Pixel art (and its finer pixel) and Full
/// resolution, on a Retina-sized window; a portrait of a sitter in each
/// state; and what each costs, in `costs.json`.
#[test]
#[ignore = "writes the design proposal's second-pass frames; opt-in"]
#[allow(clippy::too_many_lines)] // one sheet of frames, shot by shot
fn the_second_pass_frames() {
    use juggler_floor::daylight::Sky;
    use juggler_floor_render::Look;
    use std::fmt::Write as _;
    use std::time::Instant;

    let Some(dir) = std::env::var_os("AJ_FLOOR_PROPOSAL").map(PathBuf::from) else {
        return;
    };
    let dir = dir.join("v2");
    std::fs::create_dir_all(&dir).unwrap();
    let only = std::env::var("AJ_SHOTS").unwrap_or_default();
    let want = |name: &str| only.is_empty() || only.split(',').any(|s| name.starts_with(s));
    let snap = company(false);
    let (mut o1, mut s1) = (Office::new(42, 10), Seating::default());
    let v1 = scene_full(&snap, &mut o1, &mut s1, false, false);
    let (mut o2, mut s2) = (Office::new(42, 10), Seating::default());
    let v2 = scene_full(&snap, &mut o2, &mut s2, false, true);
    let mut r = Renderer::headless(&v1).unwrap();
    r.set_overlay(false);
    let size = (2560, 1600, 2.0);
    let aspect = size.0 as f32 / size.1 as f32;
    let home = v1.frame_size().home(Some(0), aspect);
    let live = Live {
        t: 2.0,
        ..Live::default()
    };
    let extras = Extras {
        hint: "",
        alert: None,
        mouse: None,
        card: None,
    };
    let sky = Sky::always_day();
    let first = Look {
        sky,
        rework: true,
        ..Look::classic()
    };
    let second = Look {
        materials: true,
        ..first
    };
    let fine = Look {
        fine: true,
        ..second
    };
    let area = |app: &str, dist: f32, pitch: f32, yaw: f32| {
        let l = v1.labels.iter().find(|l| l.app == app).unwrap();
        View {
            focus: Some(l.floor),
            cam: Cam {
                t: [l.at[0] + 0.3, l.at[1] - 2.7, l.at[2] - 3.4],
                yaw,
                pitch,
                dist,
            },
        }
    };
    let floor = View {
        focus: Some(0),
        cam: home,
    };
    // (name, view, look, full, second pass)
    let mut shots: Vec<(String, View, Look, bool, bool)> = Vec::new();
    for (vw, view) in [("desk", area("cli:w", 5.2, 0.78, 0.5)), ("floor", floor)] {
        shots.push((format!("{vw}-v1-px"), view, first, false, false));
        shots.push((format!("{vw}-v2-px"), view, second, false, true));
        shots.push((format!("{vw}-v2-fine"), view, fine, false, true));
        shots.push((format!("{vw}-v1-full"), view, first, true, false));
        shots.push((format!("{vw}-v2-full"), view, second, true, true));
    }
    for (name, view, look, full, detail) in &shots {
        if !want(name) {
            continue;
        }
        let scene = if *detail { &v2 } else { &v1 };
        r.load(scene);
        r.set_look(*look);
        r.set_full(*full);
        let img = r.capture(scene, view, &live, size, &extras).unwrap();
        write_png(&dir.join(format!("{name}.png")), &img);
        if name.starts_with("desk") {
            write_png(
                &dir.join(format!("{name}-crop.png")),
                &crop(&img, 380, 40, 1100, 900),
            );
        }
    }
    // Portraits: one sitter in each state, turned to look at the camera.
    let mut states: Vec<(FloorState, u32)> = Vec::new();
    for l in &v1.labels {
        let Some(app) = v1.app(&l.app) else { continue };
        for s in &app.sessions {
            if states.iter().any(|(st, _)| *st == s.state) {
                continue;
            }
            if let Some(id) = v1.pick_of_session(&s.key) {
                states.push((s.state, id));
            }
        }
    }
    for (state, id) in &states {
        let Some((at, ahead)) = v1.sitter(*id) else {
            continue;
        };
        let stands = matches!(state, FloorState::Asks | FloorState::Crashed);
        let asleep = *state == FloorState::Limit;
        // from the sitter's side, a little in front, the head turned to us
        let across = [ahead[2], 0.0, -ahead[0]];
        let f = if stands {
            -1.0
        } else if asleep {
            0.6
        } else {
            0.9
        };
        let e = [
            ahead[0] * f + across[0] * 0.7,
            ahead[2] * f + across[2] * 0.7,
        ];
        let yaw = e[0].atan2(e[1]);
        let view = View {
            focus: Some(0),
            cam: Cam {
                t: if stands {
                    // beside the desk, where a standing sitter goes
                    [
                        at[0] - across[0] * 0.9 - ahead[0] * 0.13,
                        at[1] + 0.15,
                        at[2] - across[2] * 0.9 - ahead[2] * 0.13,
                    ]
                } else if asleep {
                    // the head down on the desk's edge
                    [at[0] + ahead[0] * 0.3, at[1] - 0.4, at[2] + ahead[2] * 0.3]
                } else {
                    [at[0], at[1] - 0.2, at[2]]
                },
                yaw,
                pitch: if asleep { 0.5 } else { 0.42 },
                dist: if asleep { 2.8 } else { 3.4 },
            },
        };
        let hov = Live {
            hover: *id,
            ..live.clone()
        };
        for (tag, scene, look, full) in [
            ("v1-full", &v1, first, true),
            ("v2-full", &v2, second, true),
            ("v1-px", &v1, first, false),
            ("v2-px", &v2, second, false),
        ] {
            let name = format!("person-{state:?}-{tag}").to_lowercase();
            if !want(&name) {
                continue;
            }
            r.load(scene);
            r.set_look(look);
            r.set_full(full);
            let img = r.capture(scene, &view, &hov, size, &extras).unwrap();
            write_png(
                &dir.join(format!("{name}.png")),
                &crop(&img, 880, 250, 800, 900),
            );
        }
    }
    // Costs: the maps, the scene, a frame (CPU), a frame (CPU and GPU).
    if !want("costs") {
        return;
    }
    let t0 = Instant::now();
    let maps = juggler_floor::materials::bake();
    let bake_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let mut builds = [0.0f64; 2];
    for (k, detail) in [false, true].into_iter().enumerate() {
        let t0 = Instant::now();
        for _ in 0..5 {
            let (mut o, mut s) = (Office::new(42, 10), Seating::default());
            let _ = scene_full(&snap, &mut o, &mut s, false, detail);
        }
        builds[k] = t0.elapsed().as_secs_f64() * 1000.0 / 5.0;
    }
    let mut frame_cpu = [0.0f64; 2];
    let mut parts = [0usize; 2];
    for (k, scene) in [&v1, &v2].into_iter().enumerate() {
        let mut moving = Vec::new();
        let t0 = Instant::now();
        for i in 0..200 {
            moving.clear();
            let l = Live {
                t: 2.0 + i as f32 / 60.0,
                ..live.clone()
            };
            let _ = scene.frame(&l, &[0], &mut moving);
        }
        frame_cpu[k] = t0.elapsed().as_secs_f64() * 1000.0 / 200.0;
        parts[k] = moving.len();
    }
    let mut json = format!(
        "{{\"bake_ms\":{bake_ms:.2},\"maps_mb\":{:.2},\"build_ms\":[{:.2},{:.2}],\"frame_cpu_ms\":[{:.3},{:.3}],\"moving_parts\":[{},{}],\"still_parts\":[{},{}],\"frames\":[",
        maps.data.len() as f64 / 1_048_576.0,
        builds[0],
        builds[1],
        frame_cpu[0],
        frame_cpu[1],
        parts[0],
        parts[1],
        v1.stat.len(),
        v2.stat.len()
    );
    let mut first_row = true;
    for (label, size) in [
        ("retina", (2560u32, 1600u32, 2.0f32)),
        ("wide", (3440, 1410, 1.0)),
    ] {
        for (mode, full) in [("px", false), ("fine", false), ("full", true)] {
            for (pass, scene, look) in [("v1", &v1, first), ("v2", &v2, second)] {
                if mode == "fine" && pass == "v1" {
                    continue;
                }
                let look = if mode == "fine" {
                    Look { fine: true, ..look }
                } else {
                    look
                };
                r.load(scene);
                r.set_look(look);
                r.set_full(full);
                let ms = r.bench(scene, &floor, &live, size, 120);
                if !first_row {
                    json.push(',');
                }
                first_row = false;
                let _ = write!(
                    json,
                    "{{\"window\":\"{label}\",\"mode\":\"{mode}\",\"pass\":\"{pass}\",\"ms\":{ms:.3}}}"
                );
            }
        }
    }
    json.push_str("]}");
    std::fs::write(dir.join("costs.json"), json).unwrap();
}

/// The process's resident memory, MB (`ps`).
fn resident_mb() -> f64 {
    let out = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse::<f64>()
        .unwrap_or(0.0)
        / 1024.0
}

/// D-79's "no growth across repeated opens and closes", for the default
/// look with its material maps (D-86): a renderer made, drawn and dropped
/// 30 times; the last 10 add almost nothing (`AJ_LEAK=1` prints them).
#[test]
#[ignore = "30 renderers on the GPU, about a minute; opt-in"]
fn opening_and_closing_does_not_grow() {
    use juggler_floor_render::Look;
    let mut office = Office::new(42, 10);
    let mut seating = Seating::default();
    let scene = scene_of(&company(false), &mut office, &mut seating);
    let floor = View {
        focus: Some(0),
        cam: scene.frame_size().home(Some(0), 1.6),
    };
    // the building view too (AJ-E3516's cross-section), every other time
    let building = View {
        focus: None,
        cam: scene.frame_size().home(None, 1.6),
    };
    let live = Live {
        t: 2.0,
        ..Live::default()
    };
    let extras = Extras {
        hint: "",
        alert: None,
        mouse: None,
        card: None,
    };
    let mut marks = Vec::new();
    for k in 0..30 {
        let mut r = Renderer::headless(&scene).unwrap();
        r.set_look(Look::default());
        r.set_full(k % 2 == 0);
        let _ = r
            .capture(
                &scene,
                if k % 4 < 2 { &floor } else { &building },
                &live,
                (1280, 800, 2.0),
                &extras,
            )
            .unwrap();
        drop(r);
        marks.push(resident_mb());
    }
    if std::env::var_os("AJ_LEAK").is_some() {
        eprintln!("resident MB after each close: {marks:.0?}");
    }
    let grew = marks[29] - marks[19];
    assert!(
        grew < 15.0,
        "the last 10 opens and closes added {grew:.1} MB: {marks:.0?}"
    );
}

/// AJ-E3515's evidence: the labels at each band (close, middle, far and
/// the building) in both looks, with the overlay (`AJ_FLOOR_EVIDENCE`
/// names the folder).
#[test]
#[ignore = "writes the labels' evidence captures; opt-in"]
fn the_label_bands() {
    let Some(dir) = std::env::var_os("AJ_FLOOR_EVIDENCE").map(PathBuf::from) else {
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    let mut office = Office::new(42, 10);
    let mut seating = Seating::default();
    let scene = scene_of(&company(false), &mut office, &mut seating);
    let mut r = Renderer::headless(&scene).unwrap();
    let size = (1280, 800, 2.0);
    let aspect = size.0 as f32 / size.1 as f32;
    let frame = scene.frame_size();
    let home = frame.home(Some(0), aspect);
    let live = Live {
        t: 2.0,
        ..Live::default()
    };
    let extras = Extras {
        hint: "Drag: pan  ·  Right-drag: turn and tilt  ·  Pinch: zoom  ·  H: reset",
        alert: None,
        mouse: None,
        card: None,
    };
    let views = [
        (
            "close",
            View {
                focus: Some(0),
                cam: Cam {
                    dist: home.dist * 0.3,
                    ..home
                },
            },
        ),
        (
            "middle",
            View {
                focus: Some(0),
                cam: home,
            },
        ),
        (
            "far",
            View {
                focus: Some(0),
                cam: Cam {
                    dist: home.dist * 1.6,
                    ..home
                },
            },
        ),
        (
            "building",
            View {
                focus: None,
                cam: frame.home(None, aspect),
            },
        ),
    ];
    for (mode, full) in [("pixel", false), ("full", true)] {
        r.set_full(full);
        for (name, view) in &views {
            let img = r.capture(&scene, view, &live, size, &extras).unwrap();
            write_png(&dir.join(format!("labels-{name}-{mode}.png")), &img);
        }
    }
}

#[test]
#[ignore = "prints desk sizes per zoom; a tuning aid"]
fn desk_sizes_by_zoom() {
    let mut office = Office::new(42, 10);
    let mut seating = Seating::default();
    let scene = scene_of(&company(false), &mut office, &mut seating);
    for size in [
        (1280u32, 800u32, 2.0f32),
        (2560, 1600, 2.0),
        (1024, 700, 1.0),
    ] {
        let aspect = size.0 as f32 / size.1 as f32;
        let home = scene.frame_size().home(Some(0), aspect);
        for k in [0.3f32, 0.6, 1.0, 1.3, 1.6] {
            let view = View {
                focus: Some(0),
                cam: Cam {
                    dist: home.dist * k,
                    ..home
                },
            };
            let vp = view.cam.vp(aspect);
            eprintln!(
                "{size:?} dist {k}× home: desk {:.1} pt",
                juggler_floor::overlay::desk_px(&view, &vp, (size.0, size.1), size.2)
            );
        }
    }
}

/// AJ-E3517: a hover card for a sitter in each state stays inside the
/// window wherever it's asked for (each corner and the middle), has its
/// Open button (and Open in its app only where Go to it can), and, with
/// `AJ_FLOOR_EVIDENCE`, is captured in both looks.
#[test]
fn hover_cards_stay_on_screen_with_their_buttons() {
    use juggler_floor::driver::CardButton;
    use juggler_floor::overlay::{Card, Said, build};
    let mut office = Office::new(42, 10);
    let mut seating = Seating::default();
    let scene = scene_of(&company(false), &mut office, &mut seating);
    let frame = scene.frame_size();
    let mut people: Vec<(FloorState, u32)> = Vec::new();
    for l in &scene.labels {
        let Some(app) = scene.app(&l.app) else {
            continue;
        };
        for s in &app.sessions {
            if !people.iter().any(|(st, _)| *st == s.state)
                && let Some(id) = scene.pick_of_session(&s.key)
            {
                people.push((s.state, id));
            }
        }
    }
    assert!(people.len() >= 5, "people in many states: {people:?}");
    for size in [(1280u32, 800u32, 2.0f32), (640, 400, 1.0)] {
        let aspect = size.0 as f32 / size.1 as f32;
        let view = View {
            focus: Some(0),
            cam: frame.home(Some(0), aspect),
        };
        let (vp, _) = view.sight(&frame, aspect);
        let (w, h) = (size.0 as f32, size.1 as f32);
        for (n, (state, id)) in people.iter().enumerate() {
            for at in [
                (5.0, 5.0),
                (w - 5.0, 5.0),
                (5.0, h - 5.0),
                (w - 5.0, h - 5.0),
                (w / 2.0, h / 2.0),
            ] {
                let card = Card {
                    id: *id,
                    at,
                    said: if n % 2 == 0 {
                        Said::Asks("Should I also update the snapshot test and the golden frames before I push this branch?".into())
                    } else {
                        Said::Step("ran \"pnpm test\"".into())
                    },
                    go_to: (n % 3 == 0).then(|| "Claude app".to_owned()),
                };
                let (_, _, out, _) = build(
                    &scene,
                    &juggler_floor::overlay::Inputs {
                        view: &view,
                        vp: &vp,
                        size: (size.0, size.1),
                        scale: size.2,
                        marks: &[],
                        hover: *id,
                        mouse: Some(at),
                        alert: None,
                        hint: "Drag: pan",
                        band: juggler_floor::flags::Band::Middle,
                        labels: juggler_floor::flags::Labels::Smart,
                        card: Some(&card),
                    },
                );
                let out = out.unwrap_or_else(|| panic!("{state:?}: a card"));
                let r = out.rect;
                assert!(
                    r[0] >= 0.0 && r[1] >= 0.0 && r[0] + r[2] <= w && r[1] + r[3] <= h,
                    "{state:?} at {at:?} in {size:?}: {r:?} leaves the window"
                );
                assert_eq!(out.buttons[0].1, CardButton::Open);
                assert_eq!(
                    out.buttons.iter().any(|(_, b)| *b == CardButton::GoTo),
                    card.go_to.is_some(),
                    "Open in its app only where Go to it can"
                );
                for (b, _) in &out.buttons {
                    assert!(
                        b[0] >= r[0] && b[0] + b[2] <= r[0] + r[2] + 0.5,
                        "{b:?} in {r:?}"
                    );
                }
            }
        }
    }
    // the evidence: a card per state, both looks
    let Some(dir) = std::env::var_os("AJ_FLOOR_EVIDENCE").map(PathBuf::from) else {
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    let mut r = Renderer::headless(&scene).unwrap();
    let size = (1280, 800, 2.0);
    let aspect = size.0 as f32 / size.1 as f32;
    for (state, id) in &people {
        let Some((at, ahead)) = scene.sitter(*id) else {
            continue;
        };
        let across = [ahead[2], 0.0, -ahead[0]];
        let e = [
            ahead[0] * 0.9 + across[0] * 0.7,
            ahead[2] * 0.9 + across[2] * 0.7,
        ];
        // their own floor in view
        let floor = (at[1] / juggler_floor::layout::STOREY).floor().max(0.0) as usize;
        let view = View {
            focus: Some(floor),
            cam: Cam {
                t: [at[0], at[1] - 0.6, at[2]],
                yaw: e[0].atan2(e[1]),
                pitch: 0.7,
                dist: 9.0,
            },
        };
        let (vp, _) = view.sight(&frame, aspect);
        let c = vp.project([at[0], at[1] + 0.2, at[2]]);
        let pt = (
            (c[0] / c[3] * 0.5 + 0.5) * size.0 as f32,
            (0.5 - c[1] / c[3] * 0.5) * size.1 as f32,
        );
        let said = match state {
            FloorState::Asks => {
                Said::Asks("Should I also update the snapshot test before I push?".into())
            }
            FloorState::Idle => Said::Nothing,
            _ => Said::Step("ran \"cargo test -p shop\"".into()),
        };
        let card = Card {
            id: *id,
            at: pt,
            said,
            go_to: Some("Claude app".into()),
        };
        let live = Live {
            t: 2.0,
            hover: *id,
            ..Live::default()
        };
        for (mode, full) in [("pixel", false), ("full", true)] {
            r.set_full(full);
            let img = r
                .capture(
                    &scene,
                    &view,
                    &live,
                    size,
                    &Extras {
                        hint: "Drag: pan  ·  Right-drag: turn and tilt  ·  Pinch: zoom  ·  H: reset",
                        alert: None,
                        mouse: Some(pt),
                        card: Some(&card),
                    },
                )
                .unwrap();
            write_png(
                &dir.join(format!("card-{state:?}-{mode}.png").to_lowercase()),
                &img,
            );
        }
    }
}

/// AJ-E3518 and AJ-E3520's evidence (`AJ_FLOOR_EVIDENCE`): an unused App
/// at each level (closed under an hour, to a day, a week, a month,
/// longer), by day and by night; and the floor at the four times of day
/// in both looks.
#[test]
#[ignore = "writes the decay and daylight evidence; opt-in"]
fn the_decay_and_daylight_captures() {
    const HOUR: i64 = 60 * 60 * 1000;
    use juggler_floor::daylight::Sky;
    use juggler_floor_render::Look;
    let Some(dir) = std::env::var_os("AJ_FLOOR_EVIDENCE").map(PathBuf::from) else {
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    let size = (1280, 800, 2.0);
    let aspect = size.0 as f32 / size.1 as f32;
    let extras = Extras {
        hint: "Drag: pan  ·  Right-drag: turn and tilt  ·  Pinch: zoom  ·  H: reset",
        alert: None,
        mouse: None,
        card: None,
    };
    let live = Live {
        t: 2.0,
        ..Live::default()
    };
    let mut renderer: Option<Renderer> = None;
    for (level, ago) in [
        (1, HOUR / 2),
        (2, 6 * HOUR),
        (3, 3 * DAY),
        (4, 12 * DAY),
        (5, 40 * DAY),
    ] {
        let mut snap = company(false);
        for a in &mut snap.apps {
            if a.id == "cli:n" {
                a.open = false;
                a.last_active_ms = Some(NOW - ago);
            }
        }
        let (mut office, mut seating) = (Office::new(42, 10), Seating::default());
        let scene = scene_of(&snap, &mut office, &mut seating);
        let r = renderer.get_or_insert_with(|| Renderer::headless(&scene).unwrap());
        r.load(&scene);
        let area = scene
            .plan
            .areas
            .iter()
            .find(|a| a.id == "cli:n" && a.annex == 0)
            .unwrap();
        let view = View {
            focus: Some(area.floor),
            cam: Cam {
                t: [
                    area.rect.cx() + 2.5,
                    juggler_floor::scene::floor_y(area.floor),
                    area.rect.cz() + 1.0,
                ],
                yaw: 0.3,
                pitch: 0.95,
                dist: 22.0,
            },
        };
        for (sky_name, sky) in [("day", Sky::always_day()), ("night", Sky::always_night())] {
            r.set_look(Look {
                sky,
                ..Look::default()
            });
            let img = r.capture(&scene, &view, &live, size, &extras).unwrap();
            write_png(
                &dir.join(format!("decay-level{level}-{sky_name}.png")),
                &img,
            );
        }
    }
    let mut office = Office::new(42, 10);
    let mut seating = Seating::default();
    let scene = scene_of(&company(false), &mut office, &mut seating);
    let r = renderer.as_mut().unwrap();
    r.load(&scene);
    let view = View {
        focus: Some(0),
        cam: scene.frame_size().home(Some(0), aspect),
    };
    for (name, sky) in [
        ("day", Sky::at(40.0, 180.0)),
        ("golden", Sky::at(3.0, 250.0)),
        ("dusk", Sky::at(-4.0, 262.0)),
        ("night", Sky::always_night()),
    ] {
        for (mode, full) in [("pixel", false), ("full", true)] {
            r.set_full(full);
            r.set_look(Look {
                sky,
                ..Look::default()
            });
            let img = r.capture(&scene, &view, &live, size, &extras).unwrap();
            write_png(&dir.join(format!("light-{name}-{mode}.png")), &img);
        }
    }
}

/// The maintainer's 2026-11-06 finding: on a quiet floor (nothing active)
/// after sunset, pale streaks over the lift and across a side wall: the
/// shadow map's edge stretched over what fell outside it. Draws that floor
/// by night, at dusk and by day, both looks, from the home view, turned
/// and panned to each end, into `AJ_FLOOR_EVIDENCE` (opt-in).
#[test]
#[ignore = "writes the quiet floor's captures; opt-in"]
fn the_quiet_floor_captures() {
    use juggler_floor::daylight::Sky;
    use juggler_floor_render::Look;
    let Some(dir) = std::env::var_os("AJ_FLOOR_EVIDENCE").map(PathBuf::from) else {
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    let size = (1280, 800, 2.0);
    let aspect = size.0 as f32 / size.1 as f32;
    let extras = Extras {
        hint: "",
        alert: None,
        mouse: None,
        card: None,
    };
    let live = Live {
        t: 2.0,
        ..Live::default()
    };
    let mut snap = company(false);
    for a in &mut snap.apps {
        a.open = false;
        a.last_active_ms = Some(NOW - 2 * 60 * 60 * 1000);
        a.sessions.clear();
    }
    let (mut office, mut seating) = (Office::new(42, 10), Seating::default());
    let scene = scene_of(&snap, &mut office, &mut seating);
    let mut r = Renderer::headless(&scene).unwrap();
    let home = scene.frame_size().home(Some(0), aspect);
    let width = scene.width;
    for (turn, yaw, x) in [
        ("home", 0.0f32, None),
        ("left", 0.6, None),
        ("right", -0.6, None),
        ("lift-end", 0.0, Some(6.0)),
        ("far-end", 0.0, Some(width - 6.0)),
    ] {
        let mut cam = Cam {
            yaw: home.yaw + yaw,
            ..home
        };
        if let Some(x) = x {
            cam.t[0] = x;
            cam.dist *= 0.6;
        }
        let view = View {
            focus: Some(0),
            cam,
        };
        // dusk from the west: its low light showed the streaks
        let sweep = [
            ("night", Sky::always_night()),
            ("dusk", Sky::at(-4.0, 270.0)),
            ("day", Sky::always_day()),
        ];
        for (sky_name, sky) in sweep {
            for (mode, full) in [("pixel", false), ("full", true)] {
                r.set_full(full);
                r.set_look(Look {
                    sky,
                    ..Look::default()
                });
                let img = r.capture(&scene, &view, &live, size, &extras).unwrap();
                write_png(
                    &dir.join(format!("quiet-{turn}-{sky_name}-{mode}.png")),
                    &img,
                );
            }
        }
    }
}

/// AJ-E3516's building view, the cross-section on its street: the
/// company by day, at dusk and at night, both looks, into
/// `AJ_FLOOR_EVIDENCE` (opt-in).
#[test]
#[ignore = "writes the building view's captures; opt-in"]
fn the_building_captures() {
    use juggler_floor::daylight::Sky;
    use juggler_floor_render::Look;
    let Some(dir) = std::env::var_os("AJ_FLOOR_EVIDENCE").map(PathBuf::from) else {
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    let size = (1280, 800, 2.0);
    let extras = Extras {
        hint: "Drag: pan  ·  Right-drag: turn and tilt  ·  Pinch: zoom  ·  H: reset",
        alert: None,
        mouse: None,
        card: None,
    };
    let live = Live {
        t: 2.0,
        ..Live::default()
    };
    let (mut office, mut seating) = (Office::new(42, 10), Seating::default());
    let scene = scene_of(&company(false), &mut office, &mut seating);
    let mut r = Renderer::headless(&scene).unwrap();
    let view = View {
        focus: None,
        cam: scene.frame_size().home(None, 1.6),
    };
    for (name, sky) in [
        ("day", Sky::at(40.0, 200.0)),
        ("dusk", Sky::at(-4.0, 262.0)),
        ("night", Sky::always_night()),
    ] {
        for (mode, full) in [("pixel", false), ("full", true)] {
            r.set_full(full);
            r.set_look(Look {
                sky,
                ..Look::default()
            });
            let img = r.capture(&scene, &view, &live, size, &extras).unwrap();
            write_png(&dir.join(format!("building-{name}-{mode}.png")), &img);
        }
    }
}

/// AJ-E3516 R6 (2026-11-06): the harness saw ~13 MB a cycle kept after
/// opening and closing with the building view showing. As the app does it:
/// a renderer per "open", a floor's frame, then the building view animating
/// (Motion on: the clouds drift) for many frames, then dropped. Prints the
/// resident memory after each close (opt-in; `AJ_LEAK_FRAMES` frames).
#[test]
#[ignore = "an open-and-close loop through the animating building view; opt-in"]
fn the_building_view_open_and_close_loop() {
    use juggler_floor_render::Look;
    let frames: usize = std::env::var("AJ_LEAK_FRAMES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(120);
    let mut office = Office::new(42, 10);
    let mut seating = Seating::default();
    let scene = scene_of(&company(false), &mut office, &mut seating);
    let floor = View {
        focus: Some(0),
        cam: scene.frame_size().home(Some(0), 1.6),
    };
    let building = View {
        focus: None,
        cam: scene.frame_size().home(None, 1.6),
    };
    let extras = Extras {
        hint: "Drag: pan",
        alert: None,
        mouse: None,
        card: None,
    };
    let mut marks = Vec::new();
    for _ in 0..12 {
        let mut r = Renderer::headless(&scene).unwrap();
        r.set_look(Look::default());
        let _ = r
            .capture(
                &scene,
                &floor,
                &Live {
                    t: 1.0,
                    ..Live::default()
                },
                (1280, 800, 2.0),
                &extras,
            )
            .unwrap();
        for k in 0..frames {
            let live = Live {
                t: 2.0 + k as f32 / 60.0,
                walkers: true,
                ..Live::default()
            };
            let _ = r
                .capture(&scene, &building, &live, (1280, 800, 2.0), &extras)
                .unwrap();
        }
        drop(r);
        marks.push(resident_mb());
    }
    eprintln!("resident MB after each close: {marks:.0?}");
}

/// Where the model sheet stands: beside the office, past its far end.
const MODELS_X0: f32 = 80.0;

/// The model sheet's scene (AJ-E3506 R1): every desk with its chair,
/// screens and props; every plant in every pot; people of every hair
/// style and top in every pose; on a plain floor beside an emptied,
/// one-floor office.
#[allow(clippy::too_many_lines)] // one sheet, row by row
fn model_sheet() -> Scene {
    use juggler_floor::colour::hex;
    use juggler_floor::kit::{ChairKind, DeskKind, Plant, Pot, Screens};
    use juggler_floor::math::T;
    use juggler_floor::mesh::Mesh;
    use juggler_floor::models::{
        B, Growth, Look, Pose, chair, desk, desk_props, person_as, plant, screens,
    };
    use juggler_floor::rng::Rng;
    const X0: f32 = MODELS_X0;
    // one floor (no lift), no one in: the sheet stands beside it
    let mut snap = company(false);
    snap.apps.truncate(2);
    for a in &mut snap.apps {
        a.sessions.clear();
    }
    let (mut office, mut seating) = (Office::new(42, 10), Seating::default());
    let mut scene = scene_of(&snap, &mut office, &mut seating);
    let mut items = Vec::new();
    {
        let mut b = B {
            out: &mut items,
            floor: 0,
            id: 0,
            dim: 1.0,
        };
        // the floor the models stand on
        b.put(
            Mesh::QuadUp,
            T::new().t(X0 + 9.0, 0.002, 7.0).s(34.0, 1.0, 26.0),
            hex("#d9d4c8"),
        );
        let desks = [
            (DeskKind::FourLeg, ChairKind::Task, Screens::Single),
            (DeskKind::Trestle, ChairKind::Mesh, Screens::Dual),
            (DeskKind::Pedestal, ChairKind::Stool, Screens::Wide),
            (DeskKind::Standing, ChairKind::Saddle, Screens::Laptop),
            (DeskKind::LShape, ChairKind::Ball, Screens::Dual),
            (DeskKind::Round, ChairKind::Task, Screens::Laptop),
        ];
        for (k, (kind, seat, screen)) in desks.into_iter().enumerate() {
            let at = T::new().t(X0 + k as f32 * 3.2, 0.0, 0.0);
            let top = desk(&mut b, at, kind, hex("#c8a070"), hex("#3a3b40"), 1.4);
            screens(&mut b, at, screen, top, [0.0, 0.0, 0.01, 0.01], 0.8, -1.0);
            let mut r = Rng(17 + k as u32);
            desk_props(&mut b, at, top, &mut r, hex("#4a7aff"), true, 1.4);
            chair(
                &mut b,
                at.t(0.0, 0.0, 0.75),
                seat,
                hex("#3a5a7a"),
                kind == DeskKind::Standing,
            );
        }
        let plants = [
            Plant::Monstera,
            Plant::Fern,
            Plant::Snake,
            Plant::Cactus,
            Plant::Fig,
            Plant::Palm,
            Plant::Herbs,
            Plant::Succulent,
        ];
        let pots = [
            Pot::Terracotta,
            Pot::Ceramic(hex("#e8e4dc")),
            Pot::Box(hex("#6b4a2f")),
            Pot::Basket,
            Pot::Concrete,
        ];
        for (k, kind) in plants.into_iter().enumerate() {
            let at = T::new().t(X0 + k as f32 * 2.2, 0.0, 5.0);
            let growth = Growth {
                s: 1.0,
                seed: 3 + k as u32,
                sway: 0.0,
                wilt: 0.0,
            };
            plant(&mut b, at, kind, pots[k % pots.len()], growth);
        }
        for (k, pot) in pots.into_iter().enumerate() {
            let at = T::new().t(X0 + 2.0 + k as f32 * 2.2, 0.0, 8.0);
            let growth = Growth {
                s: 0.8,
                seed: 11 + k as u32,
                sway: 0.0,
                wilt: 0.0,
            };
            plant(&mut b, at, Plant::Fern, pot, growth);
        }
        let poses = [
            Pose::Typing,
            Pose::Idle,
            Pose::Wave,
            Pose::Scratch,
            Pose::HandsOnHead,
            Pose::Stretch,
            Pose::Asleep,
            Pose::Helper,
            Pose::Walk,
            Pose::Stand,
        ];
        for (k, pose) in poses.into_iter().enumerate() {
            let mut r = Rng(101 + k as u32);
            let mut look = Look::random(&mut r, hex("#ff6b4a"));
            look.style = (k % 9) as u8;
            look.top_style = (k % 4) as u8;
            look.glasses = k % 3 == 0;
            look.beard = k % 4 == 1;
            // facing the camera
            let at = T::new()
                .t(X0 + k as f32 * 1.7, 0.0, 12.5)
                .ry(std::f32::consts::PI);
            let seat = if matches!(
                pose,
                Pose::Typing | Pose::Idle | Pose::Stretch | Pose::Asleep | Pose::Scratch
            ) {
                chair(&mut b, at, ChairKind::Task, hex("#3a5a7a"), false)
            } else {
                0.0
            };
            person_as(true, &mut b, at, &look, pose, 0.6, seat, 0.0);
        }
    }
    scene.stat = items;
    scene.props.clear();
    scene.lights.clear();
    scene.labels.clear();
    scene
}

/// The model sheet in one frame, for the gallery.
fn model_sheet_view() -> View {
    View {
        focus: Some(0),
        cam: Cam {
            t: [MODELS_X0 + 8.0, 0.8, 6.5],
            yaw: 0.0,
            pitch: 0.55,
            dist: 30.0,
        },
    }
}

/// AJ-E3506 R1: the model sheet: every desk with a chair, its screens and
/// its props; every plant in every pot; people of every hair style and
/// top in every pose, on a plain floor, both looks. The furniture of the
/// areas and the lobby is in the area and building frames.
#[test]
#[ignore = "writes the model sheet; opt-in (AJ_FLOOR_EVIDENCE)"]
#[allow(clippy::too_many_lines)] // one sheet, row by row
fn the_model_sheet() {
    use juggler_floor_render::Look as RenderLook;
    const X0: f32 = MODELS_X0;
    let Some(dir) = std::env::var_os("AJ_FLOOR_EVIDENCE").map(PathBuf::from) else {
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    let scene = model_sheet();
    let mut r = Renderer::headless(&scene).unwrap();
    let size = (1600, 1000, 2.0);
    let extras = Extras {
        hint: "",
        alert: None,
        mouse: None,
        card: None,
    };
    let live = Live {
        t: 0.6,
        ..Live::default()
    };
    for (name, t, pitch, dist) in [
        ("desks", [X0 + 8.0, 0.8, 0.6], 0.55, 27.0),
        ("plants", [X0 + 8.4, 0.6, 6.5], 0.55, 26.0),
        ("people", [X0 + 7.65, 0.9, 12.5], 0.35, 23.0),
    ] {
        let view = View {
            focus: Some(0),
            cam: Cam {
                t,
                yaw: 0.0,
                pitch,
                dist,
            },
        };
        for (mode, full) in [("pixel", false), ("full", true)] {
            r.set_full(full);
            r.set_look(RenderLook::default());
            let img = r.capture(&scene, &view, &live, size, &extras).unwrap();
            write_png(&dir.join(format!("models-{name}-{mode}.png")), &img);
        }
    }
}
