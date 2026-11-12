//! The overlay's controls: the floor switcher, the View button and the
//! Mac alert's chip, in one style (a dark chip, an underline).

use super::{Out, Quad};
use crate::camera::View;
use crate::driver::Chip;
use crate::flags::Need;
use crate::tex::Icon;

/// This Mac's alert as the overlay shows it: what kind (the alert's
/// `kind`: `memory_pressure`, `swap`, `cpu`, `thermal`, `disk`, `heavy`)
/// and whether it's critical.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MacAlert<'a> {
    /// The alert's kind.
    pub kind: &'a str,
    /// Critical (macOS's critical memory pressure or heat, a disk out of
    /// room): red; otherwise amber.
    pub critical: bool,
}

/// The Mac chip's words: what (`MAC MEMORY`) and its state (`HIGH`), and
/// its badge. Short, in the controls' voice, never an alarm.
#[must_use]
pub fn mac_chip_words(alert: MacAlert<'_>) -> (&'static str, &'static str, Icon) {
    match (alert.kind, alert.critical) {
        ("memory_pressure", false) => ("MAC MEMORY", "HIGH", Icon::Low),
        ("memory_pressure", true) => ("MAC MEMORY", "CRITICAL", Icon::Out),
        ("swap", _) => ("MAC MEMORY", "SWAPPING", Icon::Low),
        ("cpu", _) => ("MAC CPU", "BUSY", Icon::Hot),
        ("thermal", false) => ("MAC", "HOT", Icon::Hot),
        ("thermal", true) => ("MAC", "OVERHEATING", Icon::Hot),
        ("disk", _) => ("MAC DISK", "FULL", Icon::Out),
        ("heavy", _) => ("MAC", "WORKING HARD", Icon::Hot),
        _ => ("MAC", "CHECK", Icon::Low),
    }
}

/// The Mac chip: the floor tabs' and View's style (a dark chip, an
/// underline), its badge, "MAC MEMORY: HIGH" with the state in amber, or
/// red when critical; right under View. Still: no pulse, no flashing
/// (WCAG 2.3.1, Reduce motion). Its place.
pub(super) fn mac_chip(o: &mut Out<'_>, alert: MacAlert<'_>, u: f32, w: f32) -> [f32; 4] {
    let (what, state, icon) = mac_chip_words(alert);
    let what = format!("{what}: ");
    let tone = if alert.critical {
        Need::Crashed.colour()
    } else {
        Need::Low.colour()
    };
    let bw = 17.0 * u + o.text_w(&what, u) + o.text_w(state, u);
    let r = [(w - bw - 12.0 * u).round(), 30.0 * u, bw, 12.0 * u];
    o.rect(r[0], r[1], r[2], r[3], [0.11, 0.1, 0.15, 0.85]);
    o.rect(r[0], r[1] + r[3], r[2], u, tone);
    if let Some(src) = o.scene.ui.badge.get(&icon).copied() {
        o.q.push(Quad {
            dst: [r[0] + 3.0 * u, r[1] + 1.5 * u, 9.0 * u, 9.0 * u],
            src,
            tint: [1.0; 4],
        });
    }
    let tx = r[0] + 14.0 * u;
    o.text(tx, r[1] + 2.5 * u, &what, u, [0.85, 0.84, 0.92, 1.0]);
    o.text(tx + o.text_w(&what, u), r[1] + 2.5 * u, state, u, tone);
    r
}

pub(super) fn view_button(o: &mut Out<'_>, u: f32, w: f32) -> [f32; 4] {
    let label = "VIEW";
    let bw = o.text_w(label, u) + 22.0 * u;
    let r = [(w - bw - 12.0 * u).round(), 14.0 * u, bw, 12.0 * u];
    o.rect(r[0], r[1], r[2], r[3], [0.11, 0.1, 0.15, 0.85]);
    o.rect(r[0], r[1] + r[3], r[2], u, [0.53, 0.53, 0.96, 1.0]);
    o.text(
        r[0] + 5.0 * u,
        r[1] + 2.5 * u,
        label,
        u,
        [0.96, 0.94, 1.0, 1.0],
    );
    // a small arrow down: it opens a menu
    let (ax, ay) = (r[0] + r[2] - 12.0 * u, r[1] + 5.0 * u);
    for step in 0..3 {
        let st = step as f32;
        o.rect(
            ax + st * u,
            ay + st * u,
            (5.0 - 2.0 * st) * u,
            u,
            [0.85, 0.83, 0.96, 1.0],
        );
    }
    r
}

/// The floor switcher: Building, then each floor from the top.
pub(super) fn switcher(o: &mut Out<'_>, view: &View, u: f32) -> Vec<Chip> {
    let mut chips = Vec::new();
    let mut x = 16.0 * u;
    let mut items: Vec<(String, Option<usize>)> = vec![("BUILDING".into(), None)];
    items.extend(
        (0..o.scene.floors)
            .rev()
            .map(|f| (format!("FLOOR {}", f + 1), Some(f))),
    );
    for (s, floor) in items {
        let active = view.focus == floor;
        // AJ-E3516: each floor's waiting count on its tab
        let waiting = floor
            .and_then(|f| o.scene.tower_floors.get(f))
            .map_or(0, |t| t.waiting);
        let count = (waiting > 0).then(|| format!("!{waiting}"));
        let extra = count.as_ref().map_or(0.0, |c| o.text_w(c, u) + 4.0 * u);
        let r = [x, 14.0 * u, o.text_w(&s, u) + extra + 10.0 * u, 12.0 * u];
        o.rect(
            r[0],
            r[1],
            r[2],
            r[3],
            if active {
                [0.16, 0.14, 0.22, 0.95]
            } else {
                [0.11, 0.1, 0.15, 0.75]
            },
        );
        if active {
            o.rect(r[0], r[1] + r[3], r[2], u, [0.53, 0.53, 0.96, 1.0]);
        }
        o.text(
            r[0] + 5.0 * u,
            r[1] + 2.5 * u,
            &s,
            u,
            if active {
                [1.0; 4]
            } else {
                [0.7, 0.72, 0.8, 1.0]
            },
        );
        if let Some(c) = &count {
            o.text(
                r[0] + 9.0 * u + o.text_w(&s, u),
                r[1] + 2.5 * u,
                c,
                u,
                Need::Asks.colour(),
            );
        }
        chips.push((r, floor));
        x += r[2] + 4.0 * u;
    }
    chips
}
