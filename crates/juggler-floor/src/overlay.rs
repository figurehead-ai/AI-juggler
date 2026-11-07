//! The overlay, in device pixels over the pixelated frame: the floor
//! switcher, labels by zoom (AJ-E3515, [`crate::flags`]: balloons up
//! close, flags over the desks that need you and a name flag per area in
//! the middle, a flag per App that needs you far out and in the
//! building), "+N" over desks, the hint line, the tooltip of what's under
//! the pointer, and the red-alert banner. Pure: quads the renderer draws
//! from the UI atlas.

use crate::camera::View;
use crate::driver::{CardButton, Chip};
use crate::flags::{Band, FlagAsk, Labels, Need, Slot, place_flags};
use crate::layout::{DEPTH, WALL_H};
use crate::math::{M4, V3};
use crate::models::rgba;
use crate::scene::{Mark, Scene, floor_y};
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
    /// AJ-E3517: the hover card open now.
    pub card: Option<&'a Card>,
}

/// AJ-E3517: a hover card: whose (their pick id), where it was asked for,
/// what they ask or did last (read when the card opened; never kept),
/// and whether the tool's own app can be brought forward to them.
#[derive(Clone, Debug, PartialEq)]
pub struct Card {
    /// The person's pick id.
    pub id: u32,
    /// Where it was asked for (the pointer), device pixels.
    pub at: (f32, f32),
    /// The pending question, else the latest step.
    pub said: Said,
    /// The tool's app can be brought forward (its adapter's Go to it),
    /// and its name ("Claude app").
    pub go_to: Option<String>,
}

/// What a card says they're on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Said {
    /// What they ask you (≤ 120 characters).
    Asks(String),
    /// What they did last (≤ 160 characters).
    Step(String),
    /// Nothing to show (idle; or last messages off, or none on this Mac).
    Nothing,
}

/// Where the card went and its buttons.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CardOut {
    /// The card, `(x, y, w, h)`.
    pub rect: [f32; 4],
    /// Its buttons.
    pub buttons: Vec<([f32; 4], CardButton)>,
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
    /// AJ-E3517: the hover card, when one is open.
    pub card: Option<CardOut>,
    /// AJ-E3519: the View button, top right.
    pub view: Option<[f32; 4]>,
    /// The red alert's banner, when there is one (kept clear).
    pub banner: Option<[f32; 4]>,
    /// The lift's whole stack, this floor's number too (kept clear).
    pub lift: Option<[f32; 4]>,
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
pub fn build(
    scene: &Scene,
    i: &Inputs<'_>,
) -> (Vec<Quad>, Vec<Chip>, Option<CardOut>, Option<[f32; 4]>) {
    let (q, chips, placed) = build_placed(scene, i);
    (q, chips, placed.card, placed.view)
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
    // (and, up close, the balloons). The controls are drawn after the
    // balloons and flags, so nothing in the office ever covers them.
    let controls_from = o.q.len();
    let mut chips = switcher(&mut o, i.view, u);
    let mut stack = None;
    if let Some(f) = i.view.focus {
        let (buttons, whole) = lift(&mut o, f, u, (w, h));
        chips.extend(buttons);
        stack = whole;
    }
    o.placed.lift = stack;
    let mut taken: Vec<[f32; 4]> = chips
        .iter()
        .map(|(r, _)| [r[0], r[1], r[2], r[3] + u])
        .chain(stack.map(|r| [r[0], r[1], r[2], r[3] + u]))
        .collect();
    let hk = (u - 1.0).max(1.0);
    taken.push([0.0, h - 16.0 * hk, w, 16.0 * hk]);
    let view = view_button(&mut o, u, w);
    taken.push(view);
    o.placed.view = Some(view);
    let controls: Vec<_> = o.q.drain(controls_from..).collect();
    // the red alert's banner, kept clear too: no flag hides behind it
    let alert = i.alert.map(|words| {
        let s = format!("RED ALERT · {}", words.to_uppercase());
        let tw = o.text_w(&s, u);
        let x0 = ((w - tw) / 2.0 - 6.0 * u).round();
        let banner = [x0, 30.0 * u, tw + 12.0 * u, 12.0 * u];
        taken.push([banner[0], banner[1], banner[2], banner[3] + u]);
        o.placed.banner = Some(banner);
        (s, banner)
    });
    let band = if i.view.focus.is_none() {
        Band::Far
    } else {
        i.band
    };
    let mut asks: Vec<(FlagAsk, FlagBody)> = Vec::new();
    marks(&mut o, i, (band, u), &project, &mut taken, &mut asks);
    if i.view.focus.is_none() {
        tower(&mut o, u, &project, &mut taken);
        tower_flags(&mut o, hk, &project, &mut asks);
        flags(&mut o, i, &asks, &taken, hk);
    } else if band != Band::Close {
        area_flags(&mut o, i, (band, hk), &project, &mut asks);
        flags(&mut o, i, &asks, &taken, hk);
    }
    o.placed.band = Some(band);
    o.placed.kept_clear = taken;
    o.q.extend(controls);
    // the hint on a dark backing: readable over a day sky too
    let hint = i.hint.to_uppercase();
    let hw = o.text_w(&hint, hk);
    o.rect(
        12.0 * u,
        h - 16.0 * hk,
        hw + 8.0 * u,
        11.0 * hk,
        [0.08, 0.07, 0.12, 0.55],
    );
    o.text(16.0 * u, h - 14.0 * hk, &hint, hk, [0.85, 0.83, 0.9, 0.85]);
    if let Some(card) = i.card {
        o.placed.card = draw_card(&mut o, card, (u, (w, h)));
    }
    if let (Some(p), Some((mx, my))) = (scene.pick_of(i.hover), i.mouse)
        && i.card.is_none_or(|c| c.id != i.hover)
    {
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
    if let Some((s, [x0, y0, bw, bh])) = alert {
        o.rect(x0, y0, bw, bh, [0.55, 0.06, 0.08, 0.95]);
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
        Icon::Sleep | Icon::Out => Some(Need::Out),
        Icon::Low => Some(Need::Low),
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
        Need::Out => Icon::Out,
        Need::Low => Icon::Low,
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
        let needs = scene.needs_of(app);
        let top = needs.iter().map(|(n, _)| *n).min();
        let total: u32 = needs.iter().map(|(_, c)| c).sum();
        let (need, words, shown_name) = match (band, top) {
            (Band::Far, Some(top)) => {
                let words = match (needs.len(), scene.room_words(app)) {
                    // only its room: the app's own words ("LOW · 12% LEFT")
                    (1, Some((need, words))) if need == top => words,
                    (1, _) => format!("{total} {}", top.word(true)),
                    _ => format!("{total} WAITING"),
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

/// The longest App name a chip shows whole; longer ones are cut ("..").
/// Most App names ("Claude Code · Work Max") fit.
pub const NAME_MAX: usize = 28;

fn short_name(name: &str) -> String {
    // A part the next one repeats goes first ("Claude Code · Work · Work
    // Max", a profile named for its Account, reads "Claude Code · Work Max";
    // "Claude Code · Work · work", the same word in another case, reads
    // "Claude Code · work": the Floor writes in capitals, so it would read
    // "WORK · WORK").
    let parts: Vec<&str> = name.split(" · ").map(str::trim).collect();
    let kept: Vec<&str> = parts
        .iter()
        .enumerate()
        .filter(|(n, p)| {
            parts.get(n + 1).is_none_or(|next| {
                let (p, next) = (p.to_lowercase(), next.to_lowercase());
                !(next == p || next.starts_with(&format!("{p} ")))
            })
        })
        .map(|(_, p)| *p)
        .collect();
    let name = kept.join(" · ");
    if name.chars().count() <= NAME_MAX {
        return name;
    }
    // A profile that's one long word keeps its end, where profiles of one
    // tool differ (2026-11-06, the lab: "claude-client-acme" and
    // "claude-client-globex" both read "CLAUDE CODE.."):
    // "CLAUDE CODE · ..GLOBEX".
    if let [head @ .., last] = kept.as_slice()
        && !head.is_empty()
        && !last.contains(' ')
    {
        let head = head.join(" · ");
        let room = NAME_MAX.saturating_sub(head.chars().count() + 3 + 2);
        if room >= 6 {
            let tail: String = last
                .chars()
                .skip(last.chars().count().saturating_sub(room))
                .collect();
            // whole pieces: from the first hyphen in it when one is there
            let tail = match tail.find(['-', '_', '.']) {
                Some(i) if tail.len() - i > 4 => tail[i + 1..].to_owned(),
                _ => tail,
            };
            return format!("{head} · ..{tail}");
        }
    }
    // Then cut at a word, never inside one, and never on the separator:
    // "INDUSTRIAL TOOL ·.." says nothing.
    let cut: String = name.chars().take(NAME_MAX - 2).collect();
    let at_word = match cut.rfind(' ') {
        Some(i) if name.chars().nth(NAME_MAX - 2) != Some(' ') => &cut[..i],
        _ => cut.as_str(),
    };
    let at_word = if at_word.trim().is_empty() {
        cut.as_str()
    } else {
        at_word
    };
    format!("{}..", at_word.trim_end_matches([' ', '·', '.']))
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

/// AJ-E3519: the View button, top right beside the tabs' row: its
/// place.
fn view_button(o: &mut Out<'_>, u: f32, w: f32) -> [f32; 4] {
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

/// AJ-E3516: the lift on a floor's right edge, vertically centred: the
/// floor above (with its waiting count) over this floor's number over
/// the floor below; the buttons are hidden at the top and the bottom.
/// Their rectangles, as floor chips for clicks.
fn lift(o: &mut Out<'_>, f: usize, u: f32, (w, h): (f32, f32)) -> (Vec<Chip>, Option<[f32; 4]>) {
    let floors = o.scene.floors;
    if floors < 2 {
        // one floor: nowhere to ride, and a lone "1" says nothing
        return (Vec::new(), None);
    }
    let k = (u - 1.0).max(1.0);
    let label = |n: usize| {
        let waiting = o.scene.tower_floors.get(n).map_or(0, |t| t.waiting);
        (format!("FLOOR {}", n + 1), waiting)
    };
    let mut rows: Vec<(Option<usize>, String, usize, i8)> = Vec::new();
    if f + 1 < floors {
        let (s, n) = label(f + 1);
        rows.push((Some(f + 1), s, n, 1));
    }
    rows.push((None, format!("{}", f + 1), 0, 0));
    if f > 0 {
        let (s, n) = label(f - 1);
        rows.push((Some(f - 1), s, n, -1));
    }
    let row_h = 14.0 * k;
    let gap = 3.0 * k;
    let total = rows.len() as f32 * (row_h + gap) + 6.0 * k;
    let mut y = ((h - total) / 2.0).round();
    let top = y;
    let mut chips = Vec::new();
    // one width for the stack: the widest row's
    let bw = rows
        .iter()
        .map(|(to, s, waiting, _)| {
            let tk = if to.is_none() { k * 1.6 } else { k };
            o.text_w(s, tk)
                + if *waiting > 0 {
                    o.text_w(&format!("!{waiting}"), k) + 4.0 * k
                } else {
                    0.0
                }
                + 22.0 * k
        })
        .fold(0.0f32, f32::max);
    for (to, s, waiting, dir) in rows {
        let count = (waiting > 0).then(|| format!("!{waiting}"));
        let big = to.is_none();
        let tk = if big { k * 1.6 } else { k };
        let bh = if big { row_h + 6.0 * k } else { row_h };
        let r = [(w - bw - 12.0 * u).round(), y, bw, bh];
        o.rect(
            r[0],
            r[1],
            r[2],
            r[3],
            if big {
                [0.36, 0.31, 0.84, 0.95]
            } else {
                [0.08, 0.07, 0.12, 0.9]
            },
        );
        let mut tx = r[0] + 7.0 * k;
        if dir != 0 {
            // an arrow: a little stepped triangle
            let (ax, ay) = (tx, r[1] + bh / 2.0);
            for step in 0..4 {
                let s = step as f32;
                let yy = if dir > 0 {
                    ay - 2.0 * k + s * k
                } else {
                    ay + 2.0 * k - s * k
                };
                o.rect(
                    ax + (3.0 - s) * k,
                    yy,
                    (2.0 * s + 1.0) * k,
                    k,
                    [0.9, 0.88, 0.96, 1.0],
                );
            }
            tx += 10.0 * k;
        }
        let ty = r[1] + (bh - 7.0 * tk) / 2.0;
        if big {
            // this floor's number, centred
            tx = r[0] + (bw - o.text_w(&s, tk)) / 2.0;
        }
        o.text(tx.round(), ty.round(), &s, tk, [0.96, 0.94, 1.0, 1.0]);
        if let Some(c) = &count {
            o.text(
                (tx + o.text_w(&s, tk) + 4.0 * k).round(),
                (r[1] + (bh - 7.0 * k) / 2.0).round(),
                c,
                k,
                Need::Asks.colour(),
            );
        }
        if let Some(to) = to {
            chips.push((r, Some(to)));
        }
        y += bh + gap;
    }
    // the whole stack, this floor's number too: kept clear of flags
    let stack = [(w - bw - 12.0 * u).round(), top, bw, y - gap - top];
    (chips, Some(stack))
}

/// AJ-E3516: the building view's words: each floor's name at its left,
/// with how many wait there; the floors' names are kept clear of flags.
fn tower(
    o: &mut Out<'_>,
    u: f32,
    project: &impl Fn(V3) -> Option<(f32, f32)>,
    taken: &mut Vec<[f32; 4]>,
) {
    let k = (u - 1.0).max(1.0);
    for t in o.scene.tower_floors.clone() {
        let Some((lx, ly)) = project(t.ends.0) else {
            continue;
        };
        let name = format!("FLOOR {}", t.floor + 1);
        let count = (t.waiting > 0).then(|| format!("!{}", t.waiting));
        let tw = o.text_w(&name, k) + count.as_ref().map_or(0.0, |c| o.text_w(c, k) + 4.0 * k);
        let r = [
            (lx - tw - 12.0 * k).round(),
            (ly - 6.0 * k).round(),
            tw + 8.0 * k,
            12.0 * k,
        ];
        o.rect(r[0], r[1], r[2], r[3], [0.12, 0.1, 0.18, 0.9]);
        o.text(
            r[0] + 4.0 * k,
            r[1] + 2.5 * k,
            &name,
            k,
            [0.95, 0.93, 1.0, 1.0],
        );
        if let Some(c) = count {
            o.text(
                r[0] + 8.0 * k + o.text_w(&name, k),
                r[1] + 2.5 * k,
                &c,
                k,
                Need::Asks.colour(),
            );
        }
        taken.push(r);
    }
}

/// AJ-E3516: a flag on each office that needs you, as the far band's on a
/// floor: what, how many and whose, in the App's colour (placed with the
/// others: never over each other, no count lost).
fn tower_flags(
    o: &mut Out<'_>,
    k: f32,
    project: &impl Fn(V3) -> Option<(f32, f32)>,
    asks: &mut Vec<(FlagAsk, FlagBody)>,
) {
    let scene = o.scene;
    for room in &scene.tower_rooms {
        let (Some(foot), Some(app)) = (project(room.at), scene.app(&room.app)) else {
            continue;
        };
        let name = short_name(&app.name.to_uppercase());
        // as the far band's flags on a floor say it
        let words = match (scene.needs_of(app).len(), scene.room_words(app)) {
            (1, Some((need, words))) if need == room.need => words,
            (1, _) => format!("{} {}", room.count, room.need.word(room.count > 1)),
            _ => format!("{} WAITING", room.count),
        };
        let accent = scene
            .kits
            .get(&app.id)
            .map_or([1.0; 4], |kit| rgba(kit.accent));
        let width = o.text_w(&words, k) + o.text_w(&name, k) + 5.0 * k + 19.0 * k;
        asks.push((
            FlagAsk {
                anchor: foot,
                size: (width, 11.0 * k),
                need: Some(room.need),
                count: room.count,
                keep_out: Vec::new(),
                hang: true,
            },
            FlagBody {
                icon: Some(icon_of(room.need)),
                words,
                name,
                edge: accent,
                floor: room.floor,
            },
        ));
    }
}

/// Words wrapped to lines `width` wide as drawn at `k`, at most `rows`
/// lines (the last ends in ".." when cut).
fn wrap(o: &Out<'_>, text: &str, (width, k): (f32, f32), rows: usize) -> Vec<String> {
    let fits = |s: &str| o.text_w(s, k) <= width;
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let mut word = word.to_owned();
        while !fits(&word) && word.chars().count() > 1 {
            word.pop();
        }
        let joined = if line.is_empty() {
            word.clone()
        } else {
            format!("{line} {word}")
        };
        if fits(&joined) {
            line = joined;
        } else {
            lines.push(std::mem::replace(&mut line, word));
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    if lines.len() > rows {
        lines.truncate(rows);
        if let Some(last) = lines.last_mut() {
            while !fits(&format!("{last}..")) && !last.is_empty() {
                last.pop();
            }
            *last = format!("{}..", last.trim_end());
        }
    }
    lines
}

/// AJ-E3517: the hover card, a speech bubble near its person: the state
/// (badge and word), the session's title, what they ask or did last (or
/// how long they've been idle), the App, project, subagents and tasks;
/// then Open in AI Juggler and, where the tool's app can be brought
/// forward, Open in it. Kept inside the window.
#[allow(clippy::too_many_lines)] // one card, line by line: its bubble, words and buttons
fn draw_card(o: &mut Out<'_>, card: &Card, (u, (w, h)): (f32, (f32, f32))) -> Option<CardOut> {
    let (app, session) = o.scene.person(card.id)?;
    let (app, session) = (app.clone(), session.clone());
    let k = (u - 1.0).max(1.0);
    let pad = 6.0 * k;
    let buttons: Vec<(String, CardButton)> =
        std::iter::once(("OPEN IN AI JUGGLER".to_owned(), CardButton::Open))
            .chain(
                card.go_to
                    .as_ref()
                    .map(|app| (format!("OPEN IN {}", app.to_uppercase()), CardButton::GoTo)),
            )
            .collect();
    let buttons_w: f32 = buttons.iter().map(|(l, _)| o.text_w(l, k) + 14.0 * k).sum();
    let cw = (210.0 * k).max(buttons_w + 2.0 * pad).min(w - 16.0 * u);
    let room = (cw - 2.0 * pad, k);
    let line_h = 10.0 * k;
    let need = Need::of_state(session.state);
    let title = wrap(o, &session.title.to_uppercase(), room, 2);
    let said: Vec<String> = match &card.said {
        Said::Asks(q) => wrap(o, &format!("ASKS: {}", q.to_uppercase()), room, 4),
        Said::Step(st) => wrap(o, &format!("LATEST: {}", st.to_uppercase()), room, 4),
        Said::Nothing => {
            let mins = (o.scene.now_ms() - session.last_ms).max(0) / 60_000;
            let ago = match mins {
                0 => "ACTIVE JUST NOW".to_owned(),
                m if m < 60 => format!("LAST ACTIVE {m} MIN AGO"),
                m if m < 48 * 60 => format!("LAST ACTIVE {} H AGO", m / 60),
                m => format!("LAST ACTIVE {} DAYS AGO", m / (24 * 60)),
            };
            vec![ago]
        }
    };
    let mut meta = vec![short_name(&app.name.to_uppercase())];
    if let Some(p) = &session.project {
        meta.push(p.to_uppercase());
    }
    if session.subagents > 0 {
        meta.push(format!("{} SUBAGENTS", session.subagents));
    }
    if session.tasks > 0 {
        meta.push(format!("{} TASKS", session.tasks));
    }
    let meta = wrap(o, &meta.join(" · "), room, 2);
    let button_h = 13.0 * k;
    let ch = pad * 2.0
        + line_h * (1 + title.len() + said.len() + meta.len()) as f32
        + 2.0 * k
        + button_h
        + 4.0 * k;
    // above and to the right of its person; below when there's no room
    let (ax, ay) = card.at;
    let x = (ax + 10.0 * k)
        .clamp(8.0 * u, (w - cw - 8.0 * u).max(8.0 * u))
        .round();
    let above = ay - ch - 14.0 * k;
    let y = if above >= 40.0 * u {
        above
    } else {
        ay + 18.0 * k
    };
    let y = y.clamp(8.0 * u, (h - ch - 8.0 * u).max(8.0 * u)).round();
    let rect = [x, y, cw, ch];
    let ink = [0.11, 0.1, 0.17, 1.0];
    // the bubble: its border, its face, and its tail towards the person
    o.rect(x - 2.0 * k, y - 2.0 * k, cw + 4.0 * k, ch + 4.0 * k, ink);
    o.rect(x, y, cw, ch, [1.0, 0.99, 0.96, 1.0]);
    let tail_x = ax.clamp(x + 8.0 * k, x + cw - 8.0 * k);
    let tail_y = if y > ay {
        y - 2.0 * k
    } else {
        y + ch + 2.0 * k
    };
    for step in 0..3 {
        let st = step as f32;
        let ty = if y > ay {
            tail_y - (st + 1.0) * 2.0 * k
        } else {
            tail_y + st * 2.0 * k
        };
        o.rect(
            tail_x - (3.0 - st) * k,
            ty,
            (6.0 - 2.0 * st) * k,
            2.0 * k,
            ink,
        );
    }
    let mut ty = y + pad;
    let mut tx = x + pad;
    let state_ink = need.map_or([0.33, 0.31, 0.42, 1.0], |n| match n {
        Need::Asks | Need::Done => [0.48, 0.36, 0.0, 1.0],
        other => {
            let c = other.colour();
            [c[0] * 0.75, c[1] * 0.6, c[2] * 0.6, 1.0]
        }
    });
    if let Some(r) = need.and_then(|n| o.scene.ui.badge.get(&icon_of(n)).copied()) {
        o.q.push(Quad {
            dst: [tx, ty, 9.0 * k, 9.0 * k],
            src: r,
            tint: [1.0; 4],
        });
        tx += 12.0 * k;
    }
    let word = if session.word.is_empty() {
        need.map_or("WORKING", |n| n.word(false)).to_owned()
    } else {
        session.word.to_uppercase()
    };
    o.text(tx, ty + k, &word, k, state_ink);
    ty += line_h;
    for l in &title {
        o.text(x + pad, ty + k, l, k, ink);
        ty += line_h;
    }
    if !said.is_empty() {
        o.rect(
            x + pad - 2.0 * k,
            ty - k,
            cw - 2.0 * pad + 4.0 * k,
            line_h * said.len() as f32 + k,
            [0.95, 0.93, 0.86, 1.0],
        );
    }
    for l in &said {
        o.text(x + pad, ty + k, l, k, [0.2, 0.18, 0.26, 1.0]);
        ty += line_h;
    }
    for l in &meta {
        o.text(x + pad, ty + k, l, k, [0.42, 0.4, 0.5, 1.0]);
        ty += line_h;
    }
    ty += 2.0 * k;
    let mut bx = x + pad;
    let mut out = CardOut {
        rect,
        buttons: Vec::new(),
    };
    for (n, (label, button)) in buttons.iter().enumerate() {
        let bw = o.text_w(label, k) + 10.0 * k;
        let r = [bx, ty, bw.min(x + cw - pad - bx), button_h];
        if n == 0 {
            o.rect(r[0], r[1], r[2], r[3], ink);
            o.text(r[0] + 5.0 * k, r[1] + 3.0 * k, label, k, [1.0; 4]);
        } else {
            o.rect(r[0], r[1], r[2], r[3], ink);
            o.rect(
                r[0] + k,
                r[1] + k,
                r[2] - 2.0 * k,
                r[3] - 2.0 * k,
                [1.0, 0.99, 0.96, 1.0],
            );
            o.text(r[0] + 5.0 * k, r[1] + 3.0 * k, label, k, ink);
        }
        out.buttons.push((r, *button));
        bx += r[2] + 4.0 * k;
    }
    Some(out)
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
        // a profile named as its Account says it once (the lab, 2026-11-06:
        // "Claude Code · Work · work" read "WORK · WORK"); a longer one
        // keeps the longer
        assert_eq!(
            short_name("CLAUDE CODE · WORK · WORK"),
            "CLAUDE CODE · WORK"
        );
        assert_eq!(
            short_name("CLAUDE CODE · WORK · WORK MAX"),
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
        assert_eq!(
            short_name("ABCDEFGHIJKLMNOPQRSTUVWXYZABCD"),
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ.."
        );
        assert!(!short_name("ABCDEFGHIJKLMNOPQRSTUV · LONGER").contains("·.."));
        // 2026-11-06, the lab: a profile named for its Account says it once,
        // and a long name is cut at a word
        assert_eq!(
            short_name("CLAUDE CODE · WORK · WORK MAX"),
            "CLAUDE CODE · WORK MAX"
        );
        assert_eq!(
            short_name("CODEX · PERSONAL · DEFAULT"),
            "CODEX · PERSONAL · DEFAULT"
        );
        assert_eq!(
            short_name("CLAUDE CODE · PERSONAL · PERSONAL PRO"),
            "CLAUDE CODE · PERSONAL PRO"
        );
        // one tool's profiles keep what tells them apart
        assert_eq!(
            short_name("CLAUDE CODE · CLAUDE-CLIENT-GLOBEX"),
            "CLAUDE CODE · ..GLOBEX"
        );
        assert_ne!(
            short_name("CLAUDE CODE · CLAUDE-CLIENT-ACME-LONGER"),
            short_name("CLAUDE CODE · CLAUDE-CLIENT-GLOBEX")
        );
    }

    #[test]
    fn a_rectangle_meets_an_outline_only_when_they_touch() {
        let square = hull(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
        assert!(meets(&[5.0, 5.0, 10.0, 10.0], &square));
        assert!(!meets(&[11.0, 0.0, 5.0, 5.0], &square));
    }
}
