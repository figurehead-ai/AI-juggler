//! The overlay, in device pixels over the pixelated frame: the floor
//! switcher, each area's chip when areas are small on screen (its name
//! and what needs you: `!N`, `?N`, `XN`, `OUT` or the usage left, `HOT`,
//! `UNUSED`), balloons and "+N" over desks, the hint line, the tooltip of
//! what's under the pointer, and the red-alert banner. Pure: quads the
//! renderer draws from the UI atlas.

use crate::camera::View;
use crate::driver::Chip;
use crate::layout::{DEPTH, WALL_H};
use crate::math::{M4, V3};
use crate::models::rgba;
use crate::scene::{Mark, Scene, floor_y};
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
    placed: Placed,
}

/// Where the overlay put the area chips, for the tests: every chip's
/// rectangle, what they kept clear of, and how many found no place.
#[derive(Clone, Debug, Default)]
pub struct Placed {
    /// Each drawn chip, `(x, y, w, h)` in device pixels.
    pub chips: Vec<[f32; 4]>,
    /// The switcher, hint line and balloons the chips keep clear of.
    pub kept_clear: Vec<[f32; 4]>,
    /// Chips left out for want of a free place.
    pub left_out: usize,
    /// In the building view, each drawn chip's floor, beside `chips`.
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
    let far = i.view.focus.is_none() || i.view.cam.dist > 52.0;
    // What the area chips must keep clear of: the floor switcher, the
    // hint line and the balloons over desks (D-80 review: labels printed
    // over each other and under the balloons).
    let chips = switcher(&mut o, i.view, u);
    let mut taken: Vec<[f32; 4]> = chips
        .iter()
        .map(|(r, _)| [r[0], r[1], r[2], r[3] + u])
        .collect();
    let hk = (u - 1.0).max(1.0);
    taken.push([0.0, h - 16.0 * hk, w, 16.0 * hk]);
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
                    let dst = [(sx - bw / 2.0).round(), (sy - bh).round(), bw, bh];
                    taken.push(dst);
                    o.q.push(Quad {
                        dst,
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
    if far {
        area_chips(&mut o, i, u, &project, &taken);
    }
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

/// Each area's chip: its name and what needs you, in words and colours.
/// Chips never cover each other or what's in `taken`: each takes the
/// first free place near its area (its own, then a row up or down, then
/// half a chip aside), stays on screen, and a moved one keeps a thin
/// stick to its area. A name longer than [`NAME_MAX`] is shortened.
fn area_chips(
    o: &mut Out<'_>,
    i: &Inputs<'_>,
    u: f32,
    project: &impl Fn(V3) -> Option<(f32, f32)>,
    taken: &[[f32; 4]],
) {
    let scene = o.scene;
    let k = (u - 1.0).max(1.0);
    let building = i.view.focus.is_none();
    // The building view: each floor's outline, so a chip stays over its
    // own floor (one moved up onto the next floor's slab read as that
    // floor's).
    let outlines = if building {
        floor_outlines(scene, project)
    } else {
        Vec::new()
    };
    let mut wants = Vec::new();
    for l in &scene.labels {
        if i.view.focus.is_some_and(|f| f != l.floor) {
            continue;
        }
        let (Some((sx, sy)), Some(app)) = (project(l.at), scene.app(&l.app)) else {
            continue;
        };
        let name = short_name(&app.name.to_uppercase());
        let tags = tags_of(scene, app);
        let tw = o.text_w(&name, k)
            + tags
                .iter()
                .map(|(s, _)| o.text_w(s, k) + 5.0 * k)
                .sum::<f32>();
        let accent = scene
            .kits
            .get(&app.id)
            .map_or([1.0; 4], |kit| rgba(kit.accent));
        wants.push(Want {
            ask: Ask {
                at: (sx, sy),
                size: (tw + 10.0 * k, 11.0 * k),
                urgent: !tags.is_empty(),
                keep_out: outlines
                    .iter()
                    .enumerate()
                    .filter(|(f, _)| *f != l.floor)
                    .map(|(_, p)| p.clone())
                    .collect(),
                down_first: building,
            },
            floor: l.floor,
            body: (name, tags, accent),
        });
    }
    let screen = (i.size.0 as f32, i.size.1 as f32);
    let spots = place(
        &wants.iter().map(|w| w.ask.clone()).collect::<Vec<_>>(),
        taken,
        screen,
        2.0 * k,
    );
    o.placed = Placed {
        chips: spots.iter().flatten().copied().collect(),
        floors: if building {
            wants
                .iter()
                .zip(&spots)
                .filter(|(_, s)| s.is_some())
                .map(|(w, _)| w.floor)
                .collect()
        } else {
            Vec::new()
        },
        outlines: outlines.clone(),
        kept_clear: taken.to_vec(),
        left_out: spots.iter().filter(|s| s.is_none()).count(),
    };
    // Sticks first, so no chip is crossed by another's.
    for (w, spot) in wants.iter().zip(&spots) {
        if let Some(r) = spot {
            stick(o, w.ask.at, r, k, w.body.2);
        }
    }
    for (w, spot) in wants.iter().zip(&spots) {
        let Some(r) = spot else { continue };
        let (name, tags, accent) = &w.body;
        let (x0, y0) = (r[0], r[1]);
        o.rect(x0, y0, r[2], r[3], INK);
        o.rect(x0, y0, 2.0 * k, r[3], *accent);
        o.text(x0 + 5.0 * k, y0 + 2.0 * k, name, k, [0.96, 0.94, 0.9, 1.0]);
        let mut tx = x0 + 5.0 * k + o.text_w(name, k) + 5.0 * k;
        for (s, c) in tags {
            o.text(tx, y0 + 2.0 * k, s, k, *c);
            tx += o.text_w(s, k) + 5.0 * k;
        }
    }
}

/// A moved chip's stick to its area (none when its area is just under
/// it): the area's colour, whole, one device pixel wide, down (or up) from
/// the chip, then along to the area when the chip moved aside.
fn stick(o: &mut Out<'_>, (sx, sy): (f32, f32), r: &[f32; 4], k: f32, colour: [f32; 4]) {
    let (top, below) = (r[1], r[1] + r[3]);
    let aside = sx < r[0] || sx > r[0] + r[2];
    if !(aside || sy > below + k || sy < top - k) {
        return;
    }
    let x = sx.clamp(r[0] + 2.0 * k, r[0] + r[2] - 2.0 * k).round();
    let (y0, y1) = if sy > below {
        (below, sy)
    } else if sy < top {
        (sy, top)
    } else {
        (sy, sy)
    };
    if y1 > y0 {
        o.rect(x, y0.round(), 1.0, (y1 - y0).round().max(1.0), colour);
    }
    if aside {
        let (x0, x1) = if sx < x { (sx, x) } else { (x, sx) };
        o.rect(
            x0.round(),
            sy.round(),
            (x1 - x0).round().max(1.0),
            1.0,
            colour,
        );
    }
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

/// What a chip says needs you, in words and colours.
fn tags_of(scene: &Scene, app: &crate::snapshot::FloorApp) -> Vec<Tag> {
    let count = |state: FloorState| app.sessions.iter().filter(|s| s.state == state).count();
    let mut tags: Vec<Tag> = Vec::new();
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
    tags
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

/// A chip that wants a place: its area on screen, its size, whether it
/// says something needs you (those are placed first), and what it shows.
struct Want {
    ask: Ask,
    floor: usize,
    body: (String, Vec<Tag>, [f32; 4]),
}

/// A chip's tag: its words and colour.
type Tag = (String, [f32; 4]);

/// A chip asking for a place.
#[derive(Clone, Debug, Default)]
pub struct Ask {
    /// Its area on screen; the chip sits centred just above it.
    pub at: (f32, f32),
    /// Its size, device pixels.
    pub size: (f32, f32),
    /// It says something needs you: placed first.
    pub urgent: bool,
    /// Outlines it never touches (in the building view, the other floors).
    pub keep_out: Vec<Poly>,
    /// Try down and aside before up (the building view, where up is the
    /// next floor).
    pub down_first: bool,
}

/// Where each chip goes (`None`: no free place near its area; it's left
/// out rather than drawn over another). Chips keep `gap` from each other,
/// from `taken` and from their `keep_out` outlines, and stay inside
/// `screen`; each takes the nearest free place by [`moves`]. Pure, for
/// the tests.
#[must_use]
pub fn place(
    wants: &[Ask],
    taken: &[[f32; 4]],
    screen: (f32, f32),
    gap: f32,
) -> Vec<Option<[f32; 4]>> {
    let down = wants.iter().any(|w| w.down_first);
    let mut order: Vec<usize> = (0..wants.len()).collect();
    // Urgent first; then the crowd stacks away from where it may move:
    // from the bottom up, or (down first) from the top down.
    order.sort_by(|&a, &b| {
        let by_y = wants[b].at.1.total_cmp(&wants[a].at.1);
        wants[b]
            .urgent
            .cmp(&wants[a].urgent)
            .then(if down { by_y.reverse() } else { by_y })
    });
    let mut placed: Vec<[f32; 4]> = Vec::new();
    let mut out = vec![None; wants.len()];
    let clear = |r: &[f32; 4], others: &[[f32; 4]]| {
        others.iter().all(|o| {
            r[0] + r[2] + gap <= o[0]
                || o[0] + o[2] + gap <= r[0]
                || r[1] + r[3] + gap <= o[1]
                || o[1] + o[3] + gap <= r[1]
        })
    };
    for n in order {
        let w = &wants[n];
        let ((sx, sy), (cw, ch)) = (w.at, w.size);
        let step = ch + gap;
        let found = moves(w.down_first).into_iter().find_map(|(dx, dy)| {
            let x = (sx - cw / 2.0 + dx * (cw + gap))
                .clamp(gap, (screen.0 - cw - gap).max(gap))
                .round();
            let y = (sy - ch + dy * step)
                .clamp(gap, (screen.1 - ch - gap).max(gap))
                .round();
            let r = [x, y, cw, ch];
            let grown = [x - gap, y - gap, cw + 2.0 * gap, ch + 2.0 * gap];
            (leader(w.at, &r) <= REACH * step
                && clear(&r, taken)
                && clear(&r, &placed)
                && !w.keep_out.iter().any(|p| meets(&grown, p)))
            .then_some(r)
        });
        if let Some(r) = found {
            placed.push(r);
        }
        out[n] = found;
    }
    out
}

/// How far a chip may sit from its area, in chip rows: farther, its
/// stick reads as pointing somewhere else (0.4.1 bug (c): leader lines
/// crossed the building to places far away), so it's left out instead.
pub const REACH: f32 = 3.0;

/// The length of a chip's stick: from its area's point to the nearest
/// edge of the chip.
#[must_use]
pub fn leader((sx, sy): (f32, f32), r: &[f32; 4]) -> f32 {
    let dx = (r[0] - sx).max(sx - (r[0] + r[2])).max(0.0);
    let dy = (r[1] - sy).max(sy - (r[1] + r[3])).max(0.0);
    dx.hypot(dy)
}

/// The moves a chip tries, nearest first: `(chip widths aside, rows down)`.
/// Up costs a little more than down in a floor's view; in the building
/// view up costs three times as much, so a chip moves down and aside first.
#[must_use]
pub fn moves(down_first: bool) -> Vec<(f32, f32)> {
    let mut all: Vec<(f32, f32, f32)> = Vec::new();
    for dx in [0.0f32, -0.5, 0.5, -1.0, 1.0, -1.5, 1.5, -2.0, 2.0] {
        for dy in -10..=10 {
            let dy = dy as f32;
            let up = if down_first { 3.0 } else { 1.0 };
            let cost = dx.abs() * 1.5 + if dy < 0.0 { -dy * up - 0.1 } else { dy };
            all.push((cost, dx, dy));
        }
    }
    all.sort_by(|a, b| a.0.total_cmp(&b.0));
    all.into_iter().map(|(_, dx, dy)| (dx, dy)).collect()
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
    use super::{Ask, NAME_MAX, REACH, hull, leader, meets, place, short_name};

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
    fn a_crowd_stacks_instead_of_overlapping() {
        // Five chips wanting the same spot, one urgent.
        let wants: Vec<_> = (0..5)
            .map(|n| Ask {
                at: (300.0, 200.0),
                size: (120.0, 11.0),
                urgent: n == 3,
                ..Ask::default()
            })
            .collect();
        let spots = place(&wants, &[[0.0, 0.0, 640.0, 20.0]], (640.0, 400.0), 2.0);
        let rects: Vec<_> = spots.iter().map(|s| s.unwrap()).collect();
        for (i, a) in rects.iter().enumerate() {
            assert!(a[1] >= 22.0, "keeps clear of what's taken: {a:?}");
            for b in &rects[i + 1..] {
                let apart = a[0] + a[2] + 2.0 <= b[0]
                    || b[0] + b[2] + 2.0 <= a[0]
                    || a[1] + a[3] + 2.0 <= b[1]
                    || b[1] + b[3] + 2.0 <= a[1];
                assert!(apart, "{a:?} and {b:?}");
            }
        }
        // The urgent one keeps its own place.
        assert_eq!(rects[3], [240.0, 189.0, 120.0, 11.0]);
    }

    #[test]
    fn down_first_keeps_clear_of_another_floor() {
        // The next floor's outline sits just above the area: a crowd of
        // three moves down and aside, never into it.
        let above = hull(&[(0.0, 100.0), (640.0, 100.0), (640.0, 185.0), (0.0, 185.0)]);
        let wants: Vec<_> = (0..3)
            .map(|_| Ask {
                at: (320.0, 200.0),
                size: (150.0, 11.0),
                keep_out: vec![above.clone()],
                down_first: true,
                ..Ask::default()
            })
            .collect();
        for r in place(&wants, &[], (640.0, 400.0), 2.0) {
            let r = r.unwrap();
            assert!(!meets(&r, &above), "{r:?} touches the floor above");
        }
        assert!(meets(&[10.0, 150.0, 20.0, 20.0], &above));
        assert!(!meets(&[10.0, 190.0, 20.0, 20.0], &above));
    }

    /// 0.4.1 bug (c): a chip with no room near its area is left out; it
    /// never hangs on a long stick far away.
    #[test]
    fn a_chip_never_strays_far_from_its_area() {
        let crowd: Vec<_> = (0..12)
            .map(|_| Ask {
                at: (320.0, 200.0),
                size: (150.0, 11.0),
                ..Ask::default()
            })
            .collect();
        let spots = place(&crowd, &[], (640.0, 400.0), 2.0);
        assert!(spots.iter().any(Option::is_none), "some are left out");
        for (r, w) in spots.iter().flatten().zip(&crowd) {
            assert!(leader(w.at, r) <= REACH * 13.0 + 0.01, "{r:?}");
        }
        assert!((leader((0.0, 0.0), &[10.0, 0.0, 5.0, 5.0]) - 10.0).abs() < 1e-4);
        assert!(leader((12.0, 2.0), &[10.0, 0.0, 5.0, 5.0]).abs() < 1e-4);
    }
}
