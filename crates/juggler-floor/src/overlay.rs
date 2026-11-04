//! The overlay, in device pixels over the pixelated frame: the floor
//! switcher, each area's chip when areas are small on screen (its name
//! and what needs you: `!N`, `?N`, `XN`, `OUT` or the usage left, `HOT`,
//! `UNUSED`), balloons and "+N" over desks, the hint line, the tooltip of
//! what's under the pointer, and the red-alert banner. Pure: quads the
//! renderer draws from the UI atlas.

use crate::camera::View;
use crate::driver::Chip;
use crate::math::{M4, V3};
use crate::models::rgba;
use crate::scene::{Mark, Scene};
use crate::state::FloorState;

/// One textured rectangle: where (device pixels), what part of the UI
/// atlas, and its tint.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Quad {
    /// `(x, y, w, h)` on screen.
    pub dst: [f32; 4],
    /// Its place in the atlas.
    pub src: [f32; 4],
    /// Its tint.
    pub tint: [f32; 4],
}

/// What the overlay needs besides the scene.
#[derive(Clone, Copy, Debug)]
pub struct Inputs<'a> {
    /// What's in view.
    pub view: &'a View,
    /// The frame's view and projection.
    pub vp: &'a M4,
    /// The window, device pixels.
    pub size: (u32, u32),
    /// Device pixels per point.
    pub scale: f32,
    /// The balloons and counts the moving part drew.
    pub marks: &'a [Mark],
    /// The pick under the pointer.
    pub hover: u32,
    /// The pointer, device pixels.
    pub mouse: Option<(f32, f32)>,
    /// The overload alert's words, when it's on.
    pub alert: Option<&'a str>,
    /// How to move the view, in this system's words.
    pub hint: &'a str,
}

const INK: [f32; 4] = [0.11, 0.1, 0.15, 0.92];
const PAPER: [f32; 4] = [0.98, 0.95, 0.88, 0.97];
const DARK: [f32; 4] = [0.12, 0.1, 0.16, 1.0];

struct Out<'a> {
    scene: &'a Scene,
    q: Vec<Quad>,
}

impl Out<'_> {
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: [f32; 4]) {
        self.q.push(Quad {
            dst: [x, y, w, h],
            src: self.scene.ui.white,
            tint: c,
        });
    }

    fn text_w(&self, s: &str, k: f32) -> f32 {
        let w: f32 = s
            .chars()
            .map(|c| {
                (self
                    .scene
                    .ui
                    .glyph
                    .get(&c.to_ascii_uppercase())
                    .map_or(3, |g| g.1)
                    + 1) as f32
                    * k
            })
            .sum();
        w - k
    }

    fn text(&mut self, x: f32, y: f32, s: &str, k: f32, c: [f32; 4]) {
        let mut cx = x;
        for ch in s.chars() {
            if let Some((r, gw)) = self.scene.ui.glyph.get(&ch.to_ascii_uppercase()) {
                self.q.push(Quad {
                    dst: [cx, y, *gw as f32 * k, 7.0 * k],
                    src: *r,
                    tint: c,
                });
                cx += (*gw + 1) as f32 * k;
            } else {
                cx += 4.0 * k;
            }
        }
    }
}

/// The overlay's unit: one pixel of the scene's pixel size.
#[must_use]
pub fn unit(scale: f32) -> f32 {
    (scale * 1.5).round().max(2.0)
}

/// Builds the overlay; the floor switcher's chips, for clicks.
#[must_use]
pub fn build(scene: &Scene, i: &Inputs<'_>) -> (Vec<Quad>, Vec<Chip>) {
    let mut o = Out {
        scene,
        q: Vec::new(),
    };
    let (w, h) = (i.size.0 as f32, i.size.1 as f32);
    let u = unit(i.scale);
    let project = |p: V3| -> Option<(f32, f32)> {
        let c = i.vp.project(p);
        (c[3] > 0.01).then(|| ((c[0] / c[3] * 0.5 + 0.5) * w, (0.5 - c[1] / c[3] * 0.5) * h))
    };
    let far = i.view.focus.is_none() || i.view.cam.dist > 52.0;
    if far {
        area_chips(&mut o, i, u, &project);
    }
    if !far || i.view.cam.dist < 70.0 {
        for m in i.marks {
            match m {
                Mark::Balloon { at, icon } => {
                    let (Some((sx, sy)), Some((r, iw, ih))) =
                        (project(*at), scene.ui.icon.get(icon))
                    else {
                        continue;
                    };
                    let (bw, bh) = (*iw as f32 * u, *ih as f32 * u);
                    o.q.push(Quad {
                        dst: [(sx - bw / 2.0).round(), (sy - bh).round(), bw, bh],
                        src: *r,
                        tint: [1.0; 4],
                    });
                }
                Mark::Plus { at, n } => {
                    let Some((sx, sy)) = project(*at) else {
                        continue;
                    };
                    let s = format!("+{n}");
                    let k = u - 1.0;
                    let tw = o.text_w(&s, k);
                    o.rect(
                        (sx - tw / 2.0 - 3.0 * k).round(),
                        (sy - 11.0 * k).round(),
                        tw + 6.0 * k,
                        11.0 * k,
                        [0.98, 0.95, 0.88, 0.95],
                    );
                    o.text((sx - tw / 2.0).round(), (sy - 9.0 * k).round(), &s, k, DARK);
                }
            }
        }
    }
    let chips = switcher(&mut o, i.view, u);
    let hk = (u - 1.0).max(1.0);
    o.text(
        16.0 * u,
        h - 14.0 * hk,
        &i.hint.to_uppercase(),
        hk,
        [0.85, 0.83, 0.9, 0.7],
    );
    if let (Some(p), Some((mx, my))) = (scene.pick_of(i.hover), i.mouse) {
        let k = u - 1.0;
        let (l1, l2) = (p.label.to_uppercase(), p.detail.to_uppercase());
        let tw = o.text_w(&l1, k).max(o.text_w(&l2, k));
        let (x0, y0) = (
            (mx + 14.0 * k).min(w - tw - 14.0 * k).round(),
            (my + 14.0 * k).min(h - 26.0 * k).round(),
        );
        o.rect(x0, y0, tw + 10.0 * k, 22.0 * k, PAPER);
        o.text(x0 + 5.0 * k, y0 + 3.0 * k, &l1, k, DARK);
        o.text(x0 + 5.0 * k, y0 + 12.0 * k, &l2, k, [0.38, 0.33, 0.46, 1.0]);
    }
    if let Some(words) = i.alert {
        let s = format!("RED ALERT · {}", words.to_uppercase());
        let tw = o.text_w(&s, u);
        let x0 = ((w - tw) / 2.0 - 6.0 * u).round();
        o.rect(
            x0,
            30.0 * u,
            tw + 12.0 * u,
            12.0 * u,
            [0.55, 0.06, 0.08, 0.95],
        );
        o.text(x0 + 6.0 * u, 32.5 * u, &s, u, [1.0, 0.9, 0.88, 1.0]);
    }
    (o.q, chips)
}

/// Each area's chip: its name and what needs you, in words and colours.
fn area_chips(
    o: &mut Out<'_>,
    i: &Inputs<'_>,
    u: f32,
    project: &impl Fn(V3) -> Option<(f32, f32)>,
) {
    let scene = o.scene;
    for l in &scene.labels {
        if i.view.focus.is_some_and(|f| f != l.floor) {
            continue;
        }
        let (Some((sx, sy)), Some(app)) = (project(l.at), scene.app(&l.app)) else {
            continue;
        };
        let k = (u - 1.0).max(1.0);
        let name = app.name.to_uppercase();
        let count = |state: FloorState| app.sessions.iter().filter(|s| s.state == state).count();
        let mut tags: Vec<(String, [f32; 4])> = Vec::new();
        for (state, mark, c) in [
            (FloorState::Asks, '!', [1.0, 0.83, 0.3, 1.0]),
            (FloorState::Stuck, '?', [0.94, 0.65, 0.25, 1.0]),
            (FloorState::Crashed, 'X', [0.94, 0.33, 0.31, 1.0]),
        ] {
            let n = count(state);
            if n > 0 {
                tags.push((format!("{mark}{n}"), c));
            }
        }
        if let Some(account) = scene.account(app.account_id.as_deref()) {
            if account.out {
                tags.push(("OUT".into(), [0.94, 0.33, 0.31, 1.0]));
            } else if let Some(left) = account.left_pct.filter(|l| *l < 20.0) {
                tags.push((format!("{}%", left.round()), [0.94, 0.65, 0.25, 1.0]));
            }
        }
        if app.hot_since_ms.is_some() {
            tags.push(("HOT".into(), [1.0, 0.55, 0.2, 1.0]));
        }
        if scene.unused_level(&app.id) > 0 {
            tags.push(("UNUSED".into(), [0.7, 0.7, 0.75, 1.0]));
        }
        let tw = o.text_w(&name, k)
            + tags
                .iter()
                .map(|(s, _)| o.text_w(s, k) + 5.0 * k)
                .sum::<f32>();
        let (cw, ch) = (tw + 10.0 * k, 11.0 * k);
        let (x0, y0) = ((sx - cw / 2.0).round(), (sy - ch).round());
        o.rect(x0, y0, cw, ch, INK);
        let accent = scene
            .kits
            .get(&app.id)
            .map_or([1.0; 4], |kit| rgba(kit.accent));
        o.rect(x0, y0, 2.0 * k, ch, accent);
        o.text(x0 + 5.0 * k, y0 + 2.0 * k, &name, k, [0.96, 0.94, 0.9, 1.0]);
        let mut tx = x0 + 5.0 * k + o.text_w(&name, k) + 5.0 * k;
        for (s, c) in &tags {
            o.text(tx, y0 + 2.0 * k, s, k, *c);
            tx += o.text_w(s, k) + 5.0 * k;
        }
    }
}

/// The floor switcher: Building, then each floor from the top.
fn switcher(o: &mut Out<'_>, view: &View, u: f32) -> Vec<Chip> {
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
        let r = [x, 14.0 * u, o.text_w(&s, u) + 10.0 * u, 12.0 * u];
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
        chips.push((r, floor));
        x += r[2] + 4.0 * u;
    }
    chips
}
