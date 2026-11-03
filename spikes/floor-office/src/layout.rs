//! Where areas go and where desks go in them.
//!
//! A floor is a lobby strip and a grid of cells: as many columns as half the floor's App limit, two
//! rows (back and front) with an aisle between. An App needs 1 cell (up to 6 desks), 2 side by side
//! (up to 12), or a 2×2 block across the aisle (up to 24). Apps are placed first-fit, always in the
//! same order (the order the Floor first saw them), scanning columns left to right and back before
//! front, onto the first floor with room. So an area moves only when an App before it changes size,
//! and then everything after it moves forward in the same reading order. An App grows to a bigger
//! size at once and shrinks only after a day below, so areas don't flicker. Per App, the layout type
//! and seed are chosen once and saved (`OfficeState`); Re-create picks again with a new office seed.

use crate::kit::{choose, Choice, Layout, Tool};
use crate::paint::Rng;
use std::collections::BTreeMap;

pub const LOBBY_W: f32 = 8.4;
pub const COL_W: f32 = 7.2;
pub const ROW_D: f32 = 9.0;
pub const AISLE: f32 = 2.6;
pub const DEPTH: f32 = 2.0 * ROW_D + AISLE;
/// Storeys are drawn far apart, so the building view shows every floor without one hiding the next.
pub const STOREY: f32 = 15.0;
pub const WALL_H: f32 = 3.0;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, serde::Serialize, serde::Deserialize)]
pub enum Size { S, M, L }
impl Size {
    pub fn for_desks(n: usize) -> Size { if n <= 6 { Size::S } else if n <= 12 { Size::M } else { Size::L } }
    pub fn cells(self) -> (usize, usize) { match self { Size::S => (1, 1), Size::M => (2, 1), Size::L => (2, 2) } }
    pub fn capacity(self) -> usize { match self { Size::S => 6, Size::M => 12, Size::L => 24 } }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AppState { pub order: u32, pub tool: Tool, pub choice: Choice, pub size: Size }

/// What the Floor saves between launches.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct OfficeState { pub version: u32, pub office_seed: u32, pub max_per_floor: usize, pub apps: BTreeMap<String, AppState> }

impl OfficeState {
    pub fn new(seed: u32) -> Self { OfficeState { version: 1, office_seed: seed, max_per_floor: 10, apps: BTreeMap::new() } }
    pub fn load(path: &std::path::Path) -> Option<Self> { serde_json::from_slice(&std::fs::read(path).ok()?).ok() }
    pub fn save(&self, path: &std::path::Path) {
        if let Some(d) = path.parent() { let _ = std::fs::create_dir_all(d); }
        let _ = std::fs::write(path, serde_json::to_vec_pretty(self).unwrap_or_default());
    }
    /// Registers an App the first time it is seen, and grows its size when its desks need it.
    /// Returns whether anything changed (so the caller saves).
    pub fn see(&mut self, id: &str, tool: Tool, desks: usize) -> bool {
        let need = Size::for_desks(desks);
        let next = self.apps.len() as u32;
        let seed = self.office_seed;
        match self.apps.get_mut(id) {
            Some(a) if a.size >= need => false,
            Some(a) => { a.size = need; true }
            None => { self.apps.insert(id.to_string(), AppState { order: next, tool, choice: choose(tool, id, seed), size: need }); true }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Rect { pub x0: f32, pub z0: f32, pub x1: f32, pub z1: f32 }
impl Rect {
    pub fn w(&self) -> f32 { self.x1 - self.x0 }
    pub fn d(&self) -> f32 { self.z1 - self.z0 }
    pub fn cx(&self) -> f32 { (self.x0 + self.x1) / 2.0 }
    pub fn cz(&self) -> f32 { (self.z0 + self.z1) / 2.0 }
    pub fn inset(&self, m: f32) -> Rect { Rect { x0: self.x0 + m, z0: self.z0 + m, x1: self.x1 - m, z1: self.z1 - m } }
    pub fn contains(&self, x: f32, z: f32) -> bool { x >= self.x0 && x <= self.x1 && z >= self.z0 && z <= self.z1 }
    pub fn overlaps(&self, o: &Rect) -> bool { self.x0 < o.x1 && o.x0 < self.x1 && self.z0 < o.z1 && o.z0 < self.z1 }
}

#[derive(Clone, Debug)]
pub struct Placed { pub id: String, pub floor: usize, pub col: usize, pub row: usize, pub size: Size, pub rect: Rect }

pub struct Plan { pub cols: usize, pub width: f32, pub floors: usize, pub areas: Vec<Placed>, pub free: Vec<(usize, Rect)> }

pub fn cell_rect(col: usize, row: usize, cw: usize, ch: usize) -> Rect {
    let x0 = LOBBY_W + col as f32 * COL_W;
    let z0 = if row == 0 { 0.0 } else { ROW_D + AISLE };
    let z1 = if ch == 2 || row == 1 { DEPTH } else { ROW_D };
    Rect { x0, z0, x1: x0 + cw as f32 * COL_W, z1 }
}

/// First-fit in App order onto floors of `cols` columns × 2 rows, at most `max` Apps a floor.
pub fn pack(state: &OfficeState) -> Plan {
    let cols = state.max_per_floor.div_ceil(2).max(2);
    let mut order: Vec<(&String, &AppState)> = state.apps.iter().collect();
    order.sort_by_key(|(_, a)| a.order);
    let mut grids: Vec<(Vec<[bool; 2]>, usize)> = vec![];
    let mut areas = vec![];
    for (id, a) in order {
        let (cw, ch) = a.size.cells();
        let mut f = 0;
        loop {
            if f == grids.len() { grids.push((vec![[false; 2]; cols], 0)); }
            let (grid, count) = &mut grids[f];
            let mut spot = None;
            if *count < state.max_per_floor {
                'scan: for c in 0..=cols - cw {
                    for r in 0..=2 - ch {
                        if (c..c + cw).all(|cc| (r..r + ch).all(|rr| !grid[cc][rr])) { spot = Some((c, r)); break 'scan; }
                    }
                }
            }
            if let Some((c, r)) = spot {
                for cc in c..c + cw { for rr in r..r + ch { grid[cc][rr] = true; } }
                *count += 1;
                areas.push(Placed { id: id.clone(), floor: f, col: c, row: r, size: a.size, rect: cell_rect(c, r, cw, ch) });
                break;
            }
            f += 1;
        }
    }
    let mut free = vec![];
    for (f, (grid, _)) in grids.iter().enumerate() {
        for (c, rows) in grid.iter().enumerate() { for (r, used) in rows.iter().enumerate() { if !used { free.push((f, cell_rect(c, r, 1, 1))); } } }
    }
    Plan { cols, width: LOBBY_W + cols as f32 * COL_W, floors: grids.len().max(1), areas, free }
}

/// A desk: centre, the way its sitter faces (radians, 0 = towards -z), its group, and whether it is
/// one seat at a shared table.
#[derive(Clone, Copy, Debug)]
pub struct Spot { pub x: f32, pub z: f32, pub face: f32, pub group: usize, pub shared: bool }

/// A group of desks around a centre (for shared tables, the table itself).
#[derive(Clone, Copy, Debug)]
pub struct Group { pub x: f32, pub z: f32, pub turn: f32, pub kind: Layout, pub seats: usize }

pub struct AreaPlan { pub layout: Layout, pub spots: Vec<Spot>, pub groups: Vec<Group>, pub nook: Option<Rect>, pub plants: Vec<(f32, f32)>, pub entrance: (f32, f32) }

/// The desk layout for an area. Spot i always belongs to session i, so a session keeps its desk as
/// others come and go; the spots only change when the area changes size.
pub fn plan_area(rect: Rect, size: Size, layout: Layout, jitter: f32, seed: u32, row: usize) -> AreaPlan {
    let mut r = Rng(seed ^ 0x9e37_79b9);
    let aisle = Rect { x0: rect.x0, z0: ROW_D - 0.2, x1: rect.x1, z1: ROW_D + AISLE + 0.2 };
    let crosses = size == Size::L;
    // the nook takes a corner away from the aisle; small areas keep a plant corner instead
    let nook_w = if size == Size::S { 0.0 } else { 3.4 };
    let back = row == 0 || crosses;
    let nook = (nook_w > 0.0).then(|| {
        let right = r.chance(0.5);
        let (x0, x1) = if right { (rect.x1 - 0.4 - nook_w, rect.x1 - 0.4) } else { (rect.x0 + 0.4, rect.x0 + 0.4 + nook_w) };
        if back { Rect { x0, z0: rect.z0 + 0.5, x1, z1: rect.z0 + 0.5 + 3.2 } } else { Rect { x0, z0: rect.z1 - 3.7, x1, z1: rect.z1 - 0.5 } }
    });
    let usable = rect.inset(0.7);
    let aisle_z = if row == 0 && !crosses { rect.z1 } else if row == 1 { rect.z0 } else { ROW_D + AISLE / 2.0 };
    // Try the kit's layout with its nook, then without the nook, then compact pods: every session
    // must have a desk, so a crowded area gives up its nook before anyone goes without.
    let mut attempt = 0;
    let (layout, nook, seats, centres, base) = loop {
        let (lay, with_nook) = match attempt { 0 => (layout, true), 1 => (layout, false), _ => (Layout::Pods, false) };
        let nk = if with_nook { nook } else { None };
        let (seats, bw, bd, base, gap) = group_shape(lay);
        let need = size.capacity().div_ceil(seats.len());
        let mut avoid: Vec<Rect> = vec![];
        if crosses { avoid.push(aisle); }
        if let Some(n) = nk { avoid.push(n.inset(-0.3)); }
        let c = pack_groups(usable, &avoid, bw, bd, gap, need, aisle_z);
        if c.len() * seats.len() >= size.capacity() || attempt == 2 { break (lay, nk, seats, c, base); }
        attempt += 1;
    };
    let mut groups = vec![];
    let mut spots = vec![];
    for (gi, (x, z)) in centres.iter().enumerate() {
        let turn = base + if layout == Layout::Scatter { r.range(-0.45, 0.45) + (r.int(4) as f32) * std::f32::consts::FRAC_PI_2 } else { r.range(-jitter, jitter) };
        groups.push(Group { x: *x, z: *z, turn, kind: layout, seats: seats.len() });
        let (ct, st) = (turn.cos(), turn.sin());
        for (dx, dz, f) in &seats {
            spots.push(Spot { x: x + dx * ct + dz * st, z: z - dx * st + dz * ct, face: f + turn, group: gi, shared: layout == Layout::Tables });
        }
    }
    spots.truncate(size.capacity());
    // plants: corners and along the side walls, never in the way
    let mut plants = vec![];
    for (px, pz) in [(rect.x0 + 0.5, rect.z0 + 0.5), (rect.x1 - 0.5, rect.z0 + 0.5), (rect.x0 + 0.5, rect.z1 - 0.5), (rect.x1 - 0.5, rect.z1 - 0.5)] {
        if nook.map(|n| n.inset(-0.6).contains(px, pz)).unwrap_or(false) { continue; }
        if spots.iter().any(|s| (s.x - px).abs() < 1.1 && (s.z - pz).abs() < 1.1) { continue; }
        plants.push((px, pz));
    }
    let entrance = (rect.cx(), aisle_z);
    let _ = &mut r;
    AreaPlan { layout, spots, groups, nook, plants, entrance }
}

/// The seats of one desk group around its centre, (dx, dz, facing), the group's half box, its turn
/// and the gap it keeps from the next group.
fn group_shape(layout: Layout) -> (Vec<(f32, f32, f32)>, f32, f32, f32, f32) {
    use std::f32::consts::{FRAC_PI_4, PI, TAU};
    let (seats, hw, hd, base): (Vec<(f32, f32, f32)>, f32, f32, f32) = match layout {
        Layout::Pods => (vec![(-0.72, 0.36, 0.0), (0.72, 0.36, 0.0), (-0.72, -0.36, PI), (0.72, -0.36, PI)], 1.5, 1.6, 0.0),
        Layout::Bench | Layout::Diagonal => (vec![(-1.44, 0.36, 0.0), (0.0, 0.36, 0.0), (1.44, 0.36, 0.0), (-1.44, -0.36, PI), (0.0, -0.36, PI), (1.44, -0.36, PI)], 2.2, 1.6, if layout == Layout::Diagonal { FRAC_PI_4 } else { 0.0 }),
        Layout::Pinwheel => ((0..3).map(|i| { let a = i as f32 * TAU / 3.0; (0.62 * a.sin(), 0.62 * a.cos(), a) }).collect(), 1.55, 1.55, 0.0),
        Layout::Scatter => (vec![(0.0, 0.0, 0.0)], 1.0, 1.0, 0.0),
        Layout::Tables => (vec![(-1.3, 0.48, 0.0), (0.0, 0.48, 0.0), (1.3, 0.48, 0.0), (-1.3, -0.48, PI), (0.0, -0.48, PI), (1.3, -0.48, PI)], 2.2, 1.55, 0.0),
        Layout::Radial => ((0..6).map(|i| { let a = i as f32 * TAU / 6.0; (1.55 * a.sin(), 1.55 * a.cos(), a) }).collect(), 2.6, 2.6, 0.0),
    };
    let (c, s) = (base.cos().abs(), base.sin().abs());
    let gap = if layout == Layout::Scatter { 0.25 } else { 0.5 };
    (seats, hw * c + hd * s, hw * s + hd * c, base, gap)
}

/// Greedy packing: group boxes placed from the aisle inwards on a fine grid, never overlapping each
/// other or what must stay clear. Deterministic, so the same area always gets the same desks.
fn pack_groups(usable: Rect, avoid: &[Rect], bw: f32, bd: f32, gap: f32, need: usize, aisle_z: f32) -> Vec<(f32, f32)> {
    let step = 0.25;
    let mut cands = vec![];
    let mut z = usable.z0 + bd;
    while z <= usable.z1 - bd + 1e-3 {
        let mut x = usable.x0 + bw;
        while x <= usable.x1 - bw + 1e-3 { cands.push((x, z)); x += step; }
        z += step;
    }
    cands.sort_by(|a, b| ((a.1 - aisle_z).abs(), a.0).partial_cmp(&((b.1 - aisle_z).abs(), b.0)).unwrap());
    let mut acc: Vec<(f32, f32)> = vec![];
    for (x, z) in cands {
        let b = Rect { x0: x - bw, z0: z - bd, x1: x + bw, z1: z + bd };
        if avoid.iter().any(|a| b.overlaps(a)) { continue; }
        if acc.iter().any(|(ax, az)| (ax - x).abs() < 2.0 * bw + gap && (az - z).abs() < 2.0 * bd + gap) { continue; }
        acc.push((x, z));
        if acc.len() == need { break; }
    }
    acc
}
