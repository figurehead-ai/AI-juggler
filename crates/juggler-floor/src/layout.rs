//! The office layout (SPEC §9.7, AJ-E3503): floors, cells, sizes, the
//! order Apps keep, and each area's desks.
//!
//! - A floor is a lobby strip and a grid of cells: two rows (back and
//!   front) either side of an aisle, and as many columns as half the
//!   floor's App limit (4–16 Apps, default 10). A column holds two areas.
//! - An App's size follows its sessions: S (up to 6, one cell), M (up to
//!   12, two cells side by side), L (up to 24, a 2 × 2 block across the
//!   aisle); beyond 24 an annex of the same App opens next in order.
//! - Order is the order the Floor first saw each App. Areas go first-fit,
//!   columns left to right, back row before front, on the first floor
//!   with room. So an area moves only when an App before it changes size.
//! - Grow at once, shrink after a day below the smaller size. A closed
//!   App's cells stay a lounge until the next reflow.
//! - Desk *i* belongs to session *i* while the area keeps its size; a
//!   crowded area gives up its nook, then uses pods, so every session has
//!   a desk.

use std::collections::BTreeMap;

use crate::family::{Family, Layout};
use crate::kit::{Choice, choose};
use crate::rng::Rng;

/// The lobby strip's width, metres.
pub const LOBBY_W: f32 = 8.4;
/// A column's width, metres.
pub const COL_W: f32 = 7.2;
/// A row's depth, metres.
pub const ROW_D: f32 = 9.0;
/// The aisle's width, metres.
pub const AISLE: f32 = 2.6;
/// A floor's depth, metres.
pub const DEPTH: f32 = 2.0 * ROW_D + AISLE;
/// From one floor to the next, metres (the building is cut away between
/// them, so floors read apart).
pub const STOREY: f32 = 15.0;
/// A wall's height, metres.
pub const WALL_H: f32 = 3.0;
/// The fewest and most Apps a floor holds (Settings › The Floor).
pub const PER_FLOOR: std::ops::RangeInclusive<usize> = 4..=16;
/// Apps a floor holds by default.
pub const DEFAULT_PER_FLOOR: usize = 10;
/// How long an App must fit a smaller size before it shrinks, ms.
pub const SHRINK_AFTER_MS: i64 = 24 * 60 * 60 * 1000;
/// The most desks one area holds; beyond it, annexes.
pub const AREA_MAX: usize = 24;

/// An area's size.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Size {
    /// Up to 6 desks, one cell.
    S,
    /// Up to 12 desks, two cells side by side.
    M,
    /// Up to 24 desks, a 2 × 2 block across the aisle.
    L,
}

impl Size {
    /// The size `desks` desks need (L for 13 and more).
    #[must_use]
    pub const fn for_desks(desks: usize) -> Self {
        if desks <= 6 {
            Self::S
        } else if desks <= 12 {
            Self::M
        } else {
            Self::L
        }
    }

    /// Its cells: (columns, rows).
    #[must_use]
    pub const fn cells(self) -> (usize, usize) {
        match self {
            Self::S => (1, 1),
            Self::M => (2, 1),
            Self::L => (2, 2),
        }
    }

    /// How many desks it holds.
    #[must_use]
    pub const fn capacity(self) -> usize {
        match self {
            Self::S => 6,
            Self::M => 12,
            Self::L => 24,
        }
    }
}

/// How many desks `sessions` sessions need, in whole areas: the main area
/// sized for up to 24, then annexes for the rest.
#[must_use]
pub fn desks_for(sessions: usize) -> usize {
    let mut left = sessions.max(1);
    let mut desks = 0;
    while left > AREA_MAX {
        desks += AREA_MAX;
        left -= AREA_MAX;
    }
    desks + Size::for_desks(left).capacity()
}

/// The areas `desks` desks make: the main one, then each annex.
#[must_use]
pub fn units(desks: usize) -> Vec<Size> {
    let mut left = desks.max(1);
    let mut out = Vec::new();
    while left > AREA_MAX {
        out.push(Size::L);
        left -= AREA_MAX;
    }
    out.push(Size::for_desks(left));
    out
}

/// What the Floor keeps per App (the `floor_areas` row).
#[derive(Clone, Debug, PartialEq)]
pub struct AreaState {
    /// Its place in the order the Floor first saw Apps.
    pub order: u32,
    /// Its tool's family when it was first seen.
    pub family: Family,
    /// Its look, chosen once.
    pub choice: Choice,
    /// The desks it holds now (6, 12, 24, then 24 more per annex step).
    pub desks: usize,
    /// Since when it has fitted fewer desks (epoch ms); `None` while it
    /// needs all it has.
    pub fits_smaller_since: Option<i64>,
}

/// The office: its seed, its floors' limit, every App it has seen, and
/// which Apps the last reflow placed.
#[derive(Clone, Debug, PartialEq)]
pub struct Office {
    /// The seed every App's first choice comes from (Re-create replaces it).
    pub seed: u32,
    /// Apps a floor holds.
    pub per_floor: usize,
    /// Every App seen, by id.
    pub apps: BTreeMap<String, AreaState>,
    /// The Apps placed at the last reflow, in order: a closed App stays
    /// here (its cells a lounge) until the next one.
    pub placed: Vec<String>,
}

/// What [`Office::observe`] did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Observed {
    /// Something to save changed.
    pub changed: bool,
    /// The areas must be placed again.
    pub reflow: bool,
}

impl Office {
    /// An empty office.
    #[must_use]
    pub fn new(seed: u32, per_floor: usize) -> Self {
        Self {
            seed,
            per_floor: per_floor.clamp(*PER_FLOOR.start(), *PER_FLOOR.end()),
            apps: BTreeMap::new(),
            placed: Vec::new(),
        }
    }

    /// Takes in the Apps open now, `(id, family, sessions)`, at `now`
    /// (epoch ms): a new App is placed after every other with its own
    /// look; one that needs more desks grows at once; one that has fitted
    /// fewer for a day shrinks. A reflow then places the Apps open now,
    /// in order; until one is needed, a closed App keeps its cells.
    pub fn observe(&mut self, open: &[(String, Family, usize)], now: i64) -> Observed {
        let mut seen = Observed::default();
        for (id, family, sessions) in open {
            let need = desks_for(*sessions);
            let next = u32::try_from(self.apps.len()).unwrap_or(u32::MAX);
            match self.apps.get_mut(id) {
                None => {
                    self.apps.insert(
                        id.clone(),
                        AreaState {
                            order: next,
                            family: *family,
                            choice: choose(*family, id, self.seed),
                            desks: need,
                            fits_smaller_since: None,
                        },
                    );
                    seen = Observed {
                        changed: true,
                        reflow: true,
                    };
                }
                Some(app) if need > app.desks => {
                    app.desks = need;
                    app.fits_smaller_since = None;
                    seen = Observed {
                        changed: true,
                        reflow: true,
                    };
                }
                Some(app) if need < app.desks => match app.fits_smaller_since {
                    None => {
                        app.fits_smaller_since = Some(now);
                        seen.changed = true;
                    }
                    Some(since) if now - since >= SHRINK_AFTER_MS => {
                        app.desks = need;
                        app.fits_smaller_since = None;
                        seen = Observed {
                            changed: true,
                            reflow: true,
                        };
                    }
                    Some(_) => {}
                },
                Some(app) => {
                    if app.fits_smaller_since.take().is_some() {
                        seen.changed = true;
                    }
                }
            }
        }
        let open_ids: Vec<&String> = open.iter().map(|(id, ..)| id).collect();
        let back = open_ids.iter().any(|id| !self.placed.contains(id));
        if seen.reflow || back || self.placed.is_empty() && !open.is_empty() {
            self.reflow_to(&open_ids);
            seen.reflow = true;
        }
        seen
    }

    /// Places the Apps `open` names, in order (closed ones leave).
    fn reflow_to(&mut self, open: &[&String]) {
        let mut ids: Vec<&String> = open
            .iter()
            .copied()
            .filter(|id| self.apps.contains_key(*id))
            .collect();
        ids.sort_by_key(|id| self.apps[*id].order);
        ids.dedup();
        self.placed = ids.into_iter().cloned().collect();
    }

    /// Re-create: a new office seed, a new look for every App, and the
    /// same order. Nothing outside the Floor changes.
    pub fn recreate(&mut self, seed: u32) {
        self.seed = seed;
        for (id, app) in &mut self.apps {
            app.choice = choose(app.family, id, seed);
        }
    }

    /// Sets how many Apps a floor holds (4–16).
    pub fn set_per_floor(&mut self, per_floor: usize) {
        self.per_floor = per_floor.clamp(*PER_FLOOR.start(), *PER_FLOOR.end());
    }
}

/// A rectangle on a floor, metres (x across, z deep).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    /// Left.
    pub x0: f32,
    /// Back.
    pub z0: f32,
    /// Right.
    pub x1: f32,
    /// Front.
    pub z1: f32,
}

impl Rect {
    /// Its width.
    #[must_use]
    pub fn w(&self) -> f32 {
        self.x1 - self.x0
    }
    /// Its depth.
    #[must_use]
    pub fn d(&self) -> f32 {
        self.z1 - self.z0
    }
    /// Its centre across.
    #[must_use]
    pub fn cx(&self) -> f32 {
        f32::midpoint(self.x0, self.x1)
    }
    /// Its centre in depth.
    #[must_use]
    pub fn cz(&self) -> f32 {
        f32::midpoint(self.z0, self.z1)
    }
    /// Shrunk by `m` on every side (grown for a negative `m`).
    #[must_use]
    pub fn inset(&self, m: f32) -> Self {
        Self {
            x0: self.x0 + m,
            z0: self.z0 + m,
            x1: self.x1 - m,
            z1: self.z1 - m,
        }
    }
    /// Whether it holds the point.
    #[must_use]
    pub fn contains(&self, x: f32, z: f32) -> bool {
        x >= self.x0 && x <= self.x1 && z >= self.z0 && z <= self.z1
    }
    /// Whether it overlaps `other`.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.x0 < other.x1 && other.x0 < self.x1 && self.z0 < other.z1 && other.z0 < self.z1
    }
}

/// One placed area: an App's, or one of its annexes.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    /// The App.
    pub id: String,
    /// 0 for its main area, then 1, 2… for its annexes.
    pub annex: usize,
    /// Its floor (0 is the first).
    pub floor: usize,
    /// Its first column.
    pub col: usize,
    /// Its first row (0 back, 1 front).
    pub row: usize,
    /// Its size.
    pub size: Size,
    /// Where it is.
    pub rect: Rect,
}

/// What a free cell is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Free {
    /// The floor's kitchen (its first free cell).
    Kitchen,
    /// A lounge.
    Lounge,
}

/// Where everything goes.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    /// Columns a floor has.
    pub cols: usize,
    /// A floor's width, metres.
    pub width: f32,
    /// How many floors.
    pub floors: usize,
    /// Every area, in order.
    pub areas: Vec<Placed>,
    /// Every free cell: `(floor, what, where)`.
    pub free: Vec<(usize, Free, Rect)>,
}

/// The rectangle of `(cw × ch)` cells from column `col`, row `row`.
#[must_use]
pub fn cell_rect(col: usize, row: usize, cw: usize, ch: usize) -> Rect {
    // Both edges from whole cells, so neighbours share an edge exactly.
    let edge = |c: usize| LOBBY_W + c as f32 * COL_W;
    let z0 = if row == 0 { 0.0 } else { ROW_D + AISLE };
    let z1 = if ch == 2 || row == 1 { DEPTH } else { ROW_D };
    Rect {
        x0: edge(col),
        z0,
        x1: edge(col + cw),
        z1,
    }
}

/// AJ-E3519: the office as the View menu's Arrange shows it, a view
/// only: a copy, never saved. Kept places: as it is. Packed: the active
/// Apps first (in their order), the others after them, so the active
/// ones sit together from the first cell.
#[must_use]
pub fn arranged(
    office: &Office,
    active: &std::collections::BTreeSet<String>,
    packed: bool,
) -> Office {
    let mut view = office.clone();
    if packed {
        let (on, off): (Vec<String>, Vec<String>) = office
            .placed
            .iter()
            .cloned()
            .partition(|id| active.contains(id));
        view.placed = on.into_iter().chain(off).collect();
    }
    view
}

/// Places the office: every placed App (and its annexes) first-fit, in
/// order, at most `per_floor` areas a floor; an annex on its App's floor
/// or a later one.
#[must_use]
pub fn pack(office: &Office) -> Plan {
    let cols = office.per_floor.div_ceil(2).max(2);
    let mut grids: Vec<(Vec<[bool; 2]>, usize)> = Vec::new();
    let mut areas = Vec::new();
    for id in &office.placed {
        let Some(app) = office.apps.get(id) else {
            continue;
        };
        // An annex never goes on a floor before its App's.
        let mut from = 0;
        for (annex, size) in units(app.desks).into_iter().enumerate() {
            let (cw, ch) = size.cells();
            let mut floor = from;
            loop {
                if floor == grids.len() {
                    grids.push((vec![[false; 2]; cols], 0));
                }
                let (grid, count) = &mut grids[floor];
                let spot = (*count < office.per_floor)
                    .then(|| first_fit(grid, cw, ch))
                    .flatten();
                if let Some((col, row)) = spot {
                    for cell in &mut grid[col..col + cw] {
                        cell[row..row + ch].fill(true);
                    }
                    *count += 1;
                    from = floor;
                    areas.push(Placed {
                        id: id.clone(),
                        annex,
                        floor,
                        col,
                        row,
                        size,
                        rect: cell_rect(col, row, cw, ch),
                    });
                    break;
                }
                floor += 1;
            }
        }
    }
    let mut free = Vec::new();
    for (floor, (grid, _)) in grids.iter().enumerate() {
        let mut first = true;
        for (col, rows) in grid.iter().enumerate() {
            for (row, used) in rows.iter().enumerate() {
                if !used {
                    let what = if first { Free::Kitchen } else { Free::Lounge };
                    first = false;
                    free.push((floor, what, cell_rect(col, row, 1, 1)));
                }
            }
        }
    }
    Plan {
        cols,
        width: LOBBY_W + cols as f32 * COL_W,
        floors: grids.len().max(1),
        areas,
        free,
    }
}

/// The first `(col, row)` where `cw × ch` cells are free: columns left to
/// right, the back row before the front.
fn first_fit(grid: &[[bool; 2]], cw: usize, ch: usize) -> Option<(usize, usize)> {
    let cols = grid.len();
    if cw > cols {
        return None;
    }
    for col in 0..=cols - cw {
        for row in 0..=2 - ch {
            if (col..col + cw).all(|c| (row..row + ch).all(|r| !grid[c][r])) {
                return Some((col, row));
            }
        }
    }
    None
}

/// A desk: where, which way its sitter faces (radians, 0 towards −z), its
/// group, and whether it's a seat at a shared table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    /// Across.
    pub x: f32,
    /// In depth.
    pub z: f32,
    /// The way its sitter faces.
    pub face: f32,
    /// Its group.
    pub group: usize,
    /// One seat at a shared table.
    pub shared: bool,
}

/// A group of desks round a centre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Group {
    /// Across.
    pub x: f32,
    /// In depth.
    pub z: f32,
    /// Its turn, radians.
    pub turn: f32,
    /// Its arrangement.
    pub kind: Layout,
    /// Its seats.
    pub seats: usize,
}

/// An area's desks and what's round them.
#[derive(Clone, Debug, PartialEq)]
pub struct AreaPlan {
    /// The layout used (a crowded area may fall back to pods).
    pub layout: Layout,
    /// Its desks: desk *i* is session *i*'s.
    pub spots: Vec<Spot>,
    /// Its desk groups.
    pub groups: Vec<Group>,
    /// Its nook, when it kept one.
    pub nook: Option<Rect>,
    /// Plant spots.
    pub plants: Vec<(f32, f32)>,
    /// Where the way in is.
    pub entrance: (f32, f32),
}

/// The desks of an area: from the aisle inwards by its layout, from its
/// seed. Spot *i* always belongs to session *i*; the spots change only
/// when the area changes size. Every desk the size holds gets a spot: a
/// crowded area gives up its nook, then uses pods.
#[must_use]
pub fn plan_area(
    rect: Rect,
    size: Size,
    layout: Layout,
    jitter: f32,
    seed: u32,
    row: usize,
) -> AreaPlan {
    let mut r = Rng(seed ^ 0x9e37_79b9);
    let crosses = size == Size::L;
    let aisle = Rect {
        x0: rect.x0,
        z0: ROW_D - 0.2,
        x1: rect.x1,
        z1: ROW_D + AISLE + 0.2,
    };
    let nook = nook_for(rect, size, row == 0 || crosses, &mut r);
    let usable = rect.inset(0.7);
    let aisle_z = if row == 0 && !crosses {
        rect.z1
    } else if row == 1 {
        rect.z0
    } else {
        ROW_D + AISLE / 2.0
    };
    let mut attempt = 0;
    let (layout, nook, shape, centres) = loop {
        let (lay, with_nook) = match attempt {
            0 => (layout, true),
            1 => (layout, false),
            _ => (Layout::Pods, false),
        };
        let kept = if with_nook { nook } else { None };
        let shape = group_shape(lay);
        let need = size.capacity().div_ceil(shape.seats.len());
        let mut avoid = Vec::new();
        if crosses {
            avoid.push(aisle);
        }
        if let Some(n) = kept {
            avoid.push(n.inset(-0.3));
        }
        let centres = pack_groups(usable, &avoid, &shape, need, aisle_z);
        if centres.len() * shape.seats.len() >= size.capacity() || attempt == 2 {
            break (lay, kept, shape, centres);
        }
        attempt += 1;
    };
    let mut groups = Vec::new();
    let mut spots = Vec::new();
    for (gi, (x, z)) in centres.iter().enumerate() {
        let turn = shape.base
            + if layout == Layout::Scatter {
                r.range(-0.45, 0.45) + r.int(4) as f32 * std::f32::consts::FRAC_PI_2
            } else {
                r.range(-jitter, jitter)
            };
        groups.push(Group {
            x: *x,
            z: *z,
            turn,
            kind: layout,
            seats: shape.seats.len(),
        });
        let (ct, st) = (turn.cos(), turn.sin());
        for (dx, dz, face) in &shape.seats {
            spots.push(Spot {
                x: x + dx * ct + dz * st,
                z: z - dx * st + dz * ct,
                face: face + turn,
                group: gi,
                shared: layout == Layout::Tables,
            });
        }
    }
    spots.truncate(size.capacity());
    let plants = corner_plants(rect, nook, &spots);
    AreaPlan {
        layout,
        spots,
        groups,
        nook,
        plants,
        entrance: (rect.cx(), aisle_z),
    }
}

/// An M or L area's nook, at one end of its back (or, in the front row,
/// its front) wall; an S area has none.
fn nook_for(rect: Rect, size: Size, back: bool, r: &mut Rng) -> Option<Rect> {
    const W: f32 = 3.4;
    if size == Size::S {
        return None;
    }
    let (x0, x1) = if r.chance(0.5) {
        (rect.x1 - 0.4 - W, rect.x1 - 0.4)
    } else {
        (rect.x0 + 0.4, rect.x0 + 0.4 + W)
    };
    let (z0, z1) = if back {
        (rect.z0 + 0.5, rect.z0 + 3.7)
    } else {
        (rect.z1 - 3.7, rect.z1 - 0.5)
    };
    Some(Rect { x0, z0, x1, z1 })
}

/// A plant in each corner the nook and the desks leave free.
fn corner_plants(rect: Rect, nook: Option<Rect>, spots: &[Spot]) -> Vec<(f32, f32)> {
    [
        (rect.x0 + 0.5, rect.z0 + 0.5),
        (rect.x1 - 0.5, rect.z0 + 0.5),
        (rect.x0 + 0.5, rect.z1 - 0.5),
        (rect.x1 - 0.5, rect.z1 - 0.5),
    ]
    .into_iter()
    .filter(|(px, pz)| !nook.is_some_and(|n| n.inset(-0.6).contains(*px, *pz)))
    .filter(|(px, pz)| {
        !spots
            .iter()
            .any(|s| (s.x - px).abs() < 1.1 && (s.z - pz).abs() < 1.1)
    })
    .collect()
}

/// One desk group's shape: its seats `(dx, dz, facing)`, its half box,
/// its base turn and the gap it keeps.
struct Shape {
    seats: Vec<(f32, f32, f32)>,
    half_w: f32,
    half_d: f32,
    base: f32,
    gap: f32,
}

fn group_shape(layout: Layout) -> Shape {
    use std::f32::consts::{FRAC_PI_4, PI, TAU};
    let (seats, hw, hd, base): (Vec<(f32, f32, f32)>, f32, f32, f32) = match layout {
        Layout::Pods => (
            vec![
                (-0.72, 0.36, 0.0),
                (0.72, 0.36, 0.0),
                (-0.72, -0.36, PI),
                (0.72, -0.36, PI),
            ],
            1.5,
            1.6,
            0.0,
        ),
        Layout::Bench | Layout::Diagonal => (
            vec![
                (-1.44, 0.36, 0.0),
                (0.0, 0.36, 0.0),
                (1.44, 0.36, 0.0),
                (-1.44, -0.36, PI),
                (0.0, -0.36, PI),
                (1.44, -0.36, PI),
            ],
            2.2,
            1.6,
            if layout == Layout::Diagonal {
                FRAC_PI_4
            } else {
                0.0
            },
        ),
        Layout::Pinwheel => (
            (0..3)
                .map(|i| {
                    let a = i as f32 * TAU / 3.0;
                    (0.62 * a.sin(), 0.62 * a.cos(), a)
                })
                .collect(),
            1.55,
            1.55,
            0.0,
        ),
        Layout::Scatter => (vec![(0.0, 0.0, 0.0)], 1.0, 1.0, 0.0),
        Layout::Tables => (
            vec![
                (-1.3, 0.48, 0.0),
                (0.0, 0.48, 0.0),
                (1.3, 0.48, 0.0),
                (-1.3, -0.48, PI),
                (0.0, -0.48, PI),
                (1.3, -0.48, PI),
            ],
            2.2,
            1.55,
            0.0,
        ),
        Layout::Radial => (
            (0..6)
                .map(|i| {
                    let a = i as f32 * TAU / 6.0;
                    (1.55 * a.sin(), 1.55 * a.cos(), a)
                })
                .collect(),
            2.6,
            2.6,
            0.0,
        ),
    };
    let (c, s) = (base.cos().abs(), base.sin().abs());
    Shape {
        seats,
        half_w: hw * c + hd * s,
        half_d: hw * s + hd * c,
        base,
        gap: if layout == Layout::Scatter { 0.25 } else { 0.5 },
    }
}

/// Group centres on a fine grid from the aisle inwards, never overlapping
/// each other or what must stay clear. Deterministic.
fn pack_groups(
    usable: Rect,
    avoid: &[Rect],
    shape: &Shape,
    need: usize,
    aisle_z: f32,
) -> Vec<(f32, f32)> {
    let (bw, bd, gap) = (shape.half_w, shape.half_d, shape.gap);
    let step = 0.25;
    let mut candidates = Vec::new();
    let mut z = usable.z0 + bd;
    while z <= usable.z1 - bd + 1e-3 {
        let mut x = usable.x0 + bw;
        while x <= usable.x1 - bw + 1e-3 {
            candidates.push((x, z));
            x += step;
        }
        z += step;
    }
    candidates.sort_by(|a, b| {
        (a.1 - aisle_z)
            .abs()
            .total_cmp(&(b.1 - aisle_z).abs())
            .then(a.0.total_cmp(&b.0))
    });
    let mut placed: Vec<(f32, f32)> = Vec::new();
    for (x, z) in candidates {
        let bounds = Rect {
            x0: x - bw,
            z0: z - bd,
            x1: x + bw,
            z1: z + bd,
        };
        if avoid.iter().any(|a| bounds.overlaps(a)) {
            continue;
        }
        if placed
            .iter()
            .any(|(px, pz)| (px - x).abs() < 2.0 * bw + gap && (pz - z).abs() < 2.0 * bd + gap)
        {
            continue;
        }
        placed.push((x, z));
        if placed.len() == need {
            break;
        }
    }
    placed
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
