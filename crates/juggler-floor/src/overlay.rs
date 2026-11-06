//! The overlay, in device pixels over the pixelated frame: the floor
//! switcher, labels by zoom (AJ-E3515, [`crate::flags`]: balloons up
//! close, flags over the desks that need you and a name flag per area in
//! the middle, a flag per App that needs you far out and in the
//! building), "+N" over desks, the hint line, the tooltip of what's under
//! the pointer, and the red-alert banner. Pure: quads the renderer draws
//! from the UI atlas.

use crate::camera::View;
use crate::driver::Chip;
use crate::flags::{Band, FlagAsk, Labels, Need, Slot, place_flags};
use crate::layout::{DEPTH, WALL_H};
use crate::math::{M4, V3};
use crate::models::rgba;
use crate::scene::{Mark, Scene, floor_y};
use crate::state::FloorState;
use crate::tex::Icon;

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
    /// AJ-E3515: the zoom band ([`crate::flags::band`], kept by the
    /// renderer between frames for its hysteresis).
    pub band: Band,
    /// Which labels show.
    pub labels: Labels,
}

/// How wide a desk (1.4 m) is on screen at the view's target, in points
/// on a 640-point-wide view (device pixels over `scale`, scaled to that
/// width): what picks the zoom band, the same in every window size.
#[must_use]
pub fn desk_px(view: &View, vp: &M4, size: (u32, u32), scale: f32) -> f32 {
    let c = view.cam;
    let right = [c.yaw.cos(), 0.0, -c.yaw.sin()];
    let at = |p: V3| {
        let q = vp.project(p);
        (q[3] > 0.01).then(|| {
            (
                (q[0] / q[3] * 0.5 + 0.5) * size.0 as f32,
                (0.5 - q[1] / q[3] * 0.5) * size.1 as f32,
            )
        })
    };
    let half = 0.7;
    match (
        at([c.t[0] - right[0] * half, c.t[1], c.t[2] - right[2] * half]),
        at([c.t[0] + right[0] * half, c.t[1], c.t[2] + right[2] * half]),
    ) {
        (Some(a), Some(b)) => {
            let pt = (a.0 - b.0).hypot(a.1 - b.1) / scale.max(1.0);
            pt * 640.0 / (size.0 as f32 / scale.max(1.0)).max(1.0)
        }
        _ => 0.0,
    }
}

const INK: [f32; 4] = [0.11, 0.1, 0.15, 0.92];
const PAPER: [f32; 4] = [0.98, 0.95, 0.88, 0.97];
const DARK: [f32; 4] = [0.12, 0.1, 0.16, 1.0];

struct Out<'a> {
    scene: &'a Scene,
    q: Vec<Quad>,
    placed: Placed,
}

/// Where the overlay put the flags, for the tests: every flag's
/// rectangle, what they kept clear of, and what found no place.
#[derive(Clone, Debug, Default)]
pub struct Placed {
    /// Each drawn flag, `(x, y, w, h)` in device pixels.
    pub chips: Vec<[f32; 4]>,
    /// The switcher and hint line the flags keep clear of.
    pub kept_clear: Vec<[f32; 4]>,
    /// Name flags left out for want of room (they come back closer).
    pub left_out: usize,
    /// Counts that found no flag (never any: one with no room joins its
    /// neighbour's).
    pub left_out_urgent: usize,
    /// What needs you in view, counted, and what the drawn flags say in
    /// all: equal, so no count is lost.
    pub counts: (u32, u32),
    /// The band drawn.
    pub band: Option<Band>,
    /// In the building view, each drawn flag's floor, beside `chips`.
    pub floors: Vec<usize>,
    /// In the building view, each floor's outline on screen (slab to wall
    /// tops), by floor; a chip never touches another floor's.
    pub outlines: Vec<Poly>,
}

/// A convex outline on screen, device pixels.
pub type Poly = Vec<(f32, f32)>;

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
    let (q, chips, _) = build_placed(scene, i);
    (q, chips)
}

/// [`build`], also saying where the area chips went.
#[must_use]
pub fn build_placed(scene: &Scene, i: &Inputs<'_>) -> (Vec<Quad>, Vec<Chip>, Placed) {
    let mut o = Out {
        scene,
        q: Vec::new(),
        placed: Placed::default(),
    };
    let (w, h) = (i.size.0 as f32, i.size.1 as f32);
    let u = unit(i.scale);
    let project = |p: V3| -> Option<(f32, f32)> {
        let c = i.vp.project(p);
        (c[3] > 0.01).then(|| ((c[0] / c[3] * 0.5 + 0.5) * w, (0.5 - c[1] / c[3] * 0.5) * h))
    };
    // What the flags keep clear of: the floor switcher and the hint line
    // (and, up close, the balloons).
    let chips = switcher(&mut o, i.view, u);
    let mut taken: Vec<[f32; 4]> = chips
        .iter()
        .map(|(r, _)| [r[0], r[1], r[2], r[3] + u])
        .collect();
    let hk = (u - 1.0).max(1.0);
    taken.push([0.0, h - 16.0 * hk, w, 16.0 * hk]);
    let band = if i.view.focus.is_none() {
        Band::Far
    } else {
        i.band
    };
    let mut asks: Vec<(FlagAsk, FlagBody)> = Vec::new();
    marks(&mut o, i, (band, u), &project, &mut taken, &mut asks);
    if band != Band::Close {
        area_flags(&mut o, i, (band, hk), &project, &mut asks);
        flags(&mut o, i, &asks, &taken, hk);
    }
    o.placed.band = Some(band);
    o.placed.kept_clear = taken;
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
    (o.q, chips, o.placed)
}

/// The moving part's marks by band: up close each balloon (and "+N"
/// over subagents); in the middle a flag for each desk that needs you
/// (asked for in `asks`) and the "+N"; far out, neither.
fn marks(
    o: &mut Out<'_>,
    i: &Inputs<'_>,
    (band, u): (Band, f32),
    project: &impl Fn(V3) -> Option<(f32, f32)>,
    taken: &mut Vec<[f32; 4]>,
    asks: &mut Vec<(FlagAsk, FlagBody)>,
) {
    let hk = (u - 1.0).max(1.0);
    for m in i.marks {
        match m {
            Mark::Balloon { at, icon } => {
                let need = need_of(*icon);
                if band == Band::Middle
                    && let (Some(need), Some(foot)) = (need, project(*at))
                {
                    // over each desk that needs you: its badge and word
                    let word = need.word(false);
                    asks.push((
                        FlagAsk {
                            anchor: foot,
                            size: (o.text_w(word, hk) + 19.0 * hk, 11.0 * hk),
                            need: Some(need),
                            count: 1,
                            keep_out: Vec::new(),
                            hang: false,
                        },
                        FlagBody {
                            icon: Some(*icon),
                            words: word.to_owned(),
                            name: String::new(),
                            edge: need.colour(),
                            floor: i.view.focus.unwrap_or(0),
                        },
                    ));
                    continue;
                }
                if band != Band::Close && need.is_some() {
                    continue;
                }
                let (Some((sx, sy)), Some((r, iw, ih))) = (project(*at), o.scene.ui.icon.get(icon))
                else {
                    continue;
                };
                let (bw, bh) = (*iw as f32 * u, *ih as f32 * u);
                let dst = [(sx - bw / 2.0).round(), (sy - bh).round(), bw, bh];
                taken.push(dst);
                o.q.push(Quad {
                    dst,
                    src: *r,
                    tint: [1.0; 4],
                });
            }
            Mark::Plus { at, n } => {
                if band == Band::Far {
                    continue;
                }
                let Some((sx, sy)) = project(*at) else {
                    continue;
                };
                let s = format!("+{n}");
                let k = u - 1.0;
                let tw = o.text_w(&s, k);
                let r = [
                    (sx - tw / 2.0 - 3.0 * k).round(),
                    (sy - 11.0 * k).round(),
                    tw + 6.0 * k,
                    11.0 * k,
                ];
                taken.push(r);
                o.rect(r[0], r[1], r[2], r[3], [0.98, 0.95, 0.88, 0.95]);
                o.text((sx - tw / 2.0).round(), (sy - 9.0 * k).round(), &s, k, DARK);
            }
        }
    }
}

/// What a flag shows: its badge, its words, an App's name after them
/// (dimmer) and the colour on its edge.
struct FlagBody {
    icon: Option<Icon>,
    words: String,
    name: String,
    edge: [f32; 4],
    floor: usize,
}

/// What a balloon says needs you (the cat's heart says nothing).
const fn need_of(icon: Icon) -> Option<Need> {
    match icon {
        Icon::Error => Some(Need::Crashed),
        Icon::Asks => Some(Need::Asks),
        Icon::Stuck => Some(Need::Stuck),
        Icon::Hot => Some(Need::Hot),
        Icon::Done => Some(Need::Done),
        Icon::Sleep => Some(Need::Out),
        Icon::Heart => None,
    }
}

/// A need's badge.
const fn icon_of(need: Need) -> Icon {
    match need {
        Need::Crashed => Icon::Error,
        Need::Asks => Icon::Asks,
        Need::Stuck => Icon::Stuck,
        Need::Hot => Icon::Hot,
        Need::Done => Icon::Done,
        Need::Out => Icon::Sleep,
    }
}

/// The areas' flags: in the middle band each area's name (unless only
/// what needs you shows); far out and in the building one flag per App
/// that needs you, with its count and name (and every App's name when all
/// names show).
fn area_flags(
    o: &mut Out<'_>,
    i: &Inputs<'_>,
    (band, k): (Band, f32),
    project: &impl Fn(V3) -> Option<(f32, f32)>,
    asks: &mut Vec<(FlagAsk, FlagBody)>,
) {
    let scene = o.scene;
    let building = i.view.focus.is_none();
    let outlines = if building {
        floor_outlines(scene, project)
    } else {
        Vec::new()
    };
    for l in &scene.labels {
        if i.view.focus.is_some_and(|f| f != l.floor) {
            continue;
        }
        let (Some(foot), Some(app)) = (project(l.at), scene.app(&l.app)) else {
            continue;
        };
        let name = short_name(&app.name.to_uppercase());
        let accent = scene
            .kits
            .get(&app.id)
            .map_or([1.0; 4], |kit| rgba(kit.accent));
        let keep_out: Vec<Poly> = outlines
            .iter()
            .enumerate()
            .filter(|(f, _)| *f != l.floor)
            .map(|(_, p)| p.clone())
            .collect();
        let needs = needs_of(scene, app);
        let top = needs.iter().map(|(n, _)| *n).min();
        let total: u32 = needs.iter().map(|(_, c)| c).sum();
        let (need, words, shown_name) = match (band, top) {
            (Band::Far, Some(top)) => {
                let words = if needs.len() == 1 {
                    format!("{total} {}", top.word(true))
                } else {
                    format!("{total} WAITING")
                };
                (Some(top), words, name)
            }
            (Band::Middle, _) | (Band::Far, None)
                if i.labels == Labels::Names
                    || (band == Band::Middle && i.labels == Labels::Smart) =>
            {
                (None, name, String::new())
            }
            _ => continue,
        };
        let width = o.text_w(&words, k)
            + if shown_name.is_empty() {
                0.0
            } else {
                o.text_w(&shown_name, k) + 5.0 * k
            }
            + if need.is_some() { 19.0 } else { 10.0 } * k;
        asks.push((
            FlagAsk {
                anchor: foot,
                size: (width, 11.0 * k),
                need,
                count: if need.is_some() { total } else { 0 },
                keep_out,
                hang: building,
            },
            FlagBody {
                icon: need.map(icon_of),
                words,
                name: shown_name,
                edge: accent,
                floor: l.floor,
            },
        ));
    }
    o.placed.outlines = outlines;
}

/// Places the flags ([`place_flags`]) and draws them: each on its pole,
/// its edge in its colour, its badge, words and name; one others joined
/// says how many wait there in all.
fn flags(
    o: &mut Out<'_>,
    i: &Inputs<'_>,
    asks: &[(FlagAsk, FlagBody)],
    taken: &[[f32; 4]],
    k: f32,
) {
    let screen = (i.size.0 as f32, i.size.1 as f32);
    let plain: Vec<FlagAsk> = asks.iter().map(|(a, _)| a.clone()).collect();
    let slots = place_flags(&plain, taken, screen, 2.0 * k, 6.0 * k);
    let asked: u32 = plain
        .iter()
        .filter(|a| a.need.is_some())
        .map(|a| a.count)
        .sum();
    let mut shown = 0;
    // poles first, so no flag is crossed by another's pole
    for ((a, _), slot) in asks.iter().zip(&slots) {
        if let Slot::Placed { rect, .. } = slot {
            let x = a.anchor.0.clamp(rect[0] + k, rect[0] + rect[2] - k).round();
            // up from its place to the flag, or down to one hanging below
            let (from, to) = if rect[1] >= a.anchor.1 {
                (a.anchor.1, rect[1])
            } else {
                (rect[1] + rect[3], a.anchor.1)
            };
            let len = (to - from).max(k);
            o.rect(
                x,
                from.round(),
                k.max(1.0),
                len.round(),
                [0.91, 0.89, 0.83, 0.95],
            );
            o.rect(
                x + k.max(1.0),
                from.round(),
                1.0,
                len.round(),
                [0.0, 0.0, 0.0, 0.3],
            );
        }
    }
    for ((a, body), slot) in asks.iter().zip(&slots) {
        match slot {
            Slot::Placed {
                rect,
                total,
                joined,
            } => {
                if a.need.is_some() {
                    shown += total;
                }
                o.placed.chips.push(*rect);
                if i.view.focus.is_none() {
                    o.placed.floors.push(body.floor);
                }
                draw_flag(o, (a, body), (rect, *total, *joined), k);
            }
            Slot::Dropped => o.placed.left_out += 1,
            Slot::Joined(_) => {}
        }
    }
    o.placed.counts = (asked, shown);
    o.placed.left_out_urgent = usize::try_from(asked.saturating_sub(shown)).unwrap_or(0);
}

/// Each floor's outline on screen, slab to wall tops, by floor.
fn floor_outlines(scene: &Scene, project: &impl Fn(V3) -> Option<(f32, f32)>) -> Vec<Poly> {
    let w = scene.frame_size().width;
    (0..scene.floors)
        .map(|f| {
            let fy = floor_y(f);
            let mut corners = Vec::new();
            for x in [0.0, w] {
                for y in [fy - 0.6, fy + WALL_H] {
                    for z in [0.0, DEPTH] {
                        corners.extend(project([x, y, z]));
                    }
                }
            }
            hull(&corners)
        })
        .collect()
}

/// One placed flag: its edge, badge, words and name (or, when others
/// joined it, how many wait there and from how many places).
fn draw_flag(
    o: &mut Out<'_>,
    (a, body): (&FlagAsk, &FlagBody),
    (rect, total, joined): (&[f32; 4], u32, u32),
    k: f32,
) {
    let (x0, y0) = (rect[0], rect[1]);
    let (words, name, icon) = if joined > 0 {
        let w = format!("{total} WAITING");
        let from = format!("{} PLACES", joined + 1);
        let size = o.text_w(&w, k) + o.text_w(&from, k) + 24.0 * k;
        o.rect(
            x0,
            y0,
            size.min(rect[2]).max(o.text_w(&w, k) + 19.0 * k),
            rect[3],
            INK,
        );
        (
            w,
            if size <= rect[2] { from } else { String::new() },
            body.icon,
        )
    } else {
        o.rect(x0, y0, rect[2], rect[3], INK);
        (body.words.clone(), body.name.clone(), body.icon)
    };
    o.rect(x0, y0, 2.0 * k, rect[3], body.edge);
    let mut tx = x0 + 5.0 * k;
    if let Some(r) = icon.and_then(|ic| o.scene.ui.badge.get(&ic).copied()) {
        o.q.push(Quad {
            dst: [tx, y0 + k, 9.0 * k, 9.0 * k],
            src: r,
            tint: [1.0; 4],
        });
        tx += 12.0 * k;
    }
    let plain = [0.96, 0.94, 0.9, 1.0];
    let ink = a.need.map_or(plain, |n| {
        if joined > 0 {
            Need::Asks.colour()
        } else {
            n.colour()
        }
    });
    o.text(tx, y0 + 2.0 * k, &words, k, ink);
    if !name.is_empty() {
        tx += o.text_w(&words, k) + 5.0 * k;
        o.text(tx, y0 + 2.0 * k, &name, k, [0.86, 0.84, 0.9, 0.75]);
    }
}

/// What needs you in an App: each need and how many (sessions, or one
/// for its Account out of usage or the machine hot).
fn needs_of(scene: &Scene, app: &crate::snapshot::FloorApp) -> Vec<(Need, u32)> {
    let count = |state: FloorState| {
        u32::try_from(app.sessions.iter().filter(|s| s.state == state).count()).unwrap_or(0)
    };
    let mut needs: Vec<(Need, u32)> = [
        (Need::Crashed, count(FloorState::Crashed)),
        (Need::Asks, count(FloorState::Asks)),
        (Need::Stuck, count(FloorState::Stuck)),
        (Need::Done, count(FloorState::Done)),
    ]
    .into_iter()
    .filter(|(_, n)| *n > 0)
    .collect();
    if app.hot_since_ms.is_some() {
        needs.push((Need::Hot, 1));
    }
    if scene
        .account(app.account_id.as_deref())
        .is_some_and(|a| a.out)
    {
        needs.push((Need::Out, 1));
    }
    needs.sort();
    needs
}

/// The longest App name a chip shows whole; longer ones end in "..".
/// Most App names ("Claude Code · Work Max") fit.
pub const NAME_MAX: usize = 24;

fn short_name(name: &str) -> String {
    if name.chars().count() <= NAME_MAX {
        return name.to_string();
    }
    let cut: String = name.chars().take(NAME_MAX - 2).collect();
    // Never end on the separator: "INDUSTRIAL TOOL ·.." says nothing.
    format!("{}..", cut.trim_end_matches([' ', '·', '.']))
}

/// The convex hull of points on screen (none for fewer than three).
#[must_use]
pub fn hull(points: &[(f32, f32)]) -> Poly {
    let mut p = points.to_vec();
    if p.len() < 3 {
        return Vec::new();
    }
    p.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let cross = |o: (f32, f32), a: (f32, f32), b: (f32, f32)| {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };
    let mut h: Vec<(f32, f32)> = Vec::new();
    for pass in 0..2 {
        let start = h.len();
        let iter: Box<dyn Iterator<Item = &(f32, f32)>> = if pass == 0 {
            Box::new(p.iter())
        } else {
            Box::new(p.iter().rev())
        };
        for &q in iter {
            while h.len() >= start + 2 && cross(h[h.len() - 2], h[h.len() - 1], q) <= 0.0 {
                h.pop();
            }
            h.push(q);
        }
        h.pop();
    }
    h
}

/// Whether a rectangle `(x, y, w, h)` and a convex outline meet
/// (separating axes: the rectangle's two and each edge's normal).
#[must_use]
pub fn meets(r: &[f32; 4], poly: &[(f32, f32)]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let corners = [
        (r[0], r[1]),
        (r[0] + r[2], r[1]),
        (r[0] + r[2], r[1] + r[3]),
        (r[0], r[1] + r[3]),
    ];
    let span = |pts: &[(f32, f32)], ax: (f32, f32)| {
        pts.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| {
            let d = p.0 * ax.0 + p.1 * ax.1;
            (lo.min(d), hi.max(d))
        })
    };
    let mut axes = vec![(1.0, 0.0), (0.0, 1.0)];
    for n in 0..poly.len() {
        let (a, b) = (poly[n], poly[(n + 1) % poly.len()]);
        axes.push((b.1 - a.1, a.0 - b.0));
    }
    axes.into_iter().all(|ax| {
        let (a, b) = (span(&corners, ax), span(poly, ax));
        a.1 > b.0 && b.1 > a.0
    })
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{NAME_MAX, hull, meets, short_name};

    #[test]
    fn long_names_end_cleanly() {
        assert_eq!(
            short_name("CLAUDE CODE · WORK MAX"),
            "CLAUDE CODE · WORK MAX"
        );
        let long = short_name("A RATHER LONG APP NAME NUMBER 12");
        assert!(
            long.ends_with("..") && long.chars().count() <= NAME_MAX,
            "{long}"
        );
        assert_eq!(
            short_name("INDUSTRIAL TOOL · HOME OFFICE PLUS"),
            "INDUSTRIAL TOOL · HOME.."
        );
        assert!(!short_name("ABCDEFGHIJKLMNOPQRSTUV · LONGER").contains("·.."));
    }

    #[test]
    fn a_rectangle_meets_an_outline_only_when_they_touch() {
        let square = hull(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
        assert!(meets(&[5.0, 5.0, 10.0, 10.0], &square));
        assert!(!meets(&[11.0, 0.0, 5.0, 5.0], &square));
    }
}
