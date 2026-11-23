//! The overlay over the scene: the first-open line (AJ-E1548 #23) and
//! the Mac alert's chip (0.4.2), each kept clear and calm.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::Live;
use super::Target;
use super::tests::{app, build, session, snapshot};
use crate::state::FloorState;

/// AJ-E1548 #23: the first open says what the Floor's pieces are, in one
/// line over the hint line (the window's left, inside it, clear of the
/// hint, every glyph drawable), and the flags keep clear of it; without
/// it nothing is drawn there.
#[test]
fn the_first_open_line_says_what_the_floor_is_and_keeps_clear() {
    use crate::camera::View;
    use crate::overlay::{INTRO, Inputs as Over, build_placed, unit};
    let snap = snapshot(vec![app(
        "cli:a",
        "workshop",
        vec![session("s:1", FloorState::Asks, 0)],
    )]);
    let scene = build(&snap);
    let frame = scene.frame_size();
    for (size, focus) in [
        ((1600, 1000), Some(0)),
        ((800, 600), Some(0)),
        ((1600, 1000), None),
    ] {
        let view = View {
            focus,
            cam: frame.home(focus, size.0 as f32 / size.1 as f32),
        };
        let vp = view.cam.vp(size.0 as f32 / size.1 as f32);
        let mut out = Vec::new();
        let drawn = scene.frame(&Live::default(), &view.floors(&frame), &mut out);
        let over = Over {
            view: &view,
            vp: &vp,
            size,
            scale: 2.0,
            marks: &drawn.marks,
            hover: 0,
            mouse: None,
            alert: None,
            hint: "Drag to pan",
            band: crate::flags::Band::Middle,
            labels: crate::flags::Labels::Smart,
            card: None,
            opened: None,
            intro: true,
        };
        let (with, _, placed) = build_placed(&scene, &over);
        let r = placed.intro.expect("the first-open line");
        let (w, h) = (size.0 as f32, size.1 as f32);
        let hk = (unit(2.0) - 1.0).max(1.0);
        assert!(
            r[0] > 0.0 && r[1] > 0.0 && r[0] + r[2] < w,
            "{r:?} in {size:?}"
        );
        assert!(r[1] + r[3] <= h - 16.0 * hk, "{r:?} over the hint line");
        assert!(placed.kept_clear.contains(&r));
        for chip in &placed.chips {
            let apart = chip[0] >= r[0] + r[2]
                || chip[0] + chip[2] <= r[0]
                || chip[1] >= r[1] + r[3]
                || chip[1] + chip[3] <= r[1];
            assert!(apart, "a flag {chip:?} on the line {r:?}");
        }
        let (without, _, placed) = build_placed(
            &scene,
            &Over {
                intro: false,
                ..over
            },
        );
        assert!(placed.intro.is_none());
        assert!(with.len() > without.len());
    }
    // what it says: the area, the floor, a person's state, and how it goes
    let words = INTRO.join(" ");
    for part in [
        "AREA IS ONE APP",
        "FLOOR",
        "PERSON IS A SESSION",
        "WAVING NEEDS YOU",
        "HIDE",
    ] {
        assert!(words.contains(part), "{part}");
    }
    assert!(
        words.chars().all(|c| crate::tex::GLYPHS.contains(c)),
        "{words}"
    );
}

/// 0.4.2 (the maintainer: "we shouldn't overtake the whole screen in red
/// … why are all of the offices flashing red"): a Mac alert is one still
/// chip under View, amber or red by level, that opens This Mac; the rooms
/// draw exactly what they draw without it, and nothing ticks for it.
#[test]
fn a_mac_alert_is_one_calm_chip_never_a_change_in_the_rooms() {
    use crate::camera::View;
    use crate::flags::Need;
    use crate::overlay::{Inputs as Over, MacAlert, build_placed, mac_chip_words};
    let snap = snapshot(vec![app(
        "cli:a",
        "workshop",
        vec![session("s:1", FloorState::Working, 0)],
    )]);
    let scene = build(&snap);
    let frame = scene.frame_size();
    let view = View {
        focus: Some(0),
        cam: frame.home(Some(0), 1.6),
    };
    let vp = view.cam.vp(1.6);
    let mut out = Vec::new();
    let drawn = scene.frame(&Live::default(), &view.floors(&frame), &mut out);
    let calm = Over {
        view: &view,
        vp: &vp,
        size: (1600, 1000),
        scale: 2.0,
        marks: &drawn.marks,
        hover: 0,
        mouse: None,
        alert: None,
        hint: "Drag to pan",
        band: crate::flags::Band::Middle,
        labels: crate::flags::Labels::Smart,
        card: None,
        opened: None,
        intro: false,
    };
    let (without, _, placed) = build_placed(&scene, &calm);
    assert!(placed.banner.is_none());
    for critical in [false, true] {
        let alert = MacAlert {
            kind: "memory_pressure",
            critical,
        };
        let (with, _, placed) = build_placed(
            &scene,
            &Over {
                alert: Some(alert),
                ..calm
            },
        );
        let chip = placed.banner.expect("the Mac chip");
        // small, in the controls' row under View, on screen
        let view_r = placed.view.unwrap();
        assert!(chip[2] < 1600.0 / 3.0 && chip[3] <= view_r[3], "{chip:?}");
        assert!(
            chip[1] > view_r[1] && chip[0] + chip[2] <= 1600.0,
            "{chip:?}"
        );
        // it opens This Mac
        let mac = scene.pick_of_mac().unwrap();
        assert!(placed.doors.contains(&(chip, mac)));
        assert!(matches!(
            scene.pick_of(mac).map(|p| &p.target),
            Some(Target::ThisMac)
        ));
        // amber at warning, red only when critical
        let tone = if critical { Need::Crashed } else { Need::Low }.colour();
        let same = |a: [f32; 4]| a.iter().zip(tone).all(|(x, y)| (x - y).abs() < 1e-6);
        assert!(with.iter().any(|q| same(q.tint)));
        let words = mac_chip_words(alert);
        assert_eq!(words.0, "MAC MEMORY");
        assert_eq!(words.1, if critical { "CRITICAL" } else { "HIGH" });
        // nothing else in the overlay changed size or tint: only the
        // chip's quads were added, all inside it
        let extra: Vec<_> = with.iter().filter(|q| !without.contains(q)).collect();
        assert_ne!(extra.len(), 0);
        for q in extra {
            let d = q.dst;
            assert!(
                d[0] >= chip[0] - 0.01
                    && d[1] >= chip[1] - 0.01
                    && d[0] + d[2] <= chip[0] + chip[2] + 0.01
                    && d[1] + d[3] <= chip[1] + chip[3] + crate::overlay::unit(2.0) + 0.01, // its underline
                "{d:?} outside the chip {chip:?}"
            );
        }
    }
    // every kind has short words, and only the levels that mean danger
    // are red
    for kind in [
        "memory_pressure",
        "swap",
        "cpu",
        "thermal",
        "disk",
        "heavy",
        "new",
    ] {
        let (what, state, _) = mac_chip_words(MacAlert {
            kind,
            critical: false,
        });
        assert!(
            what.starts_with("MAC") && what.len() + state.len() <= 22,
            "{kind}"
        );
    }
}
