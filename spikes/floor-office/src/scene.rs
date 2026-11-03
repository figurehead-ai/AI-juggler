//! The office as a scene: a static part built once (building, areas, furniture) and a dynamic part
//! rebuilt each drawn frame (people, screens, signals, the props you can play with). Every visual
//! signal comes from the data: sessions and their states, subagents, App load, unused Apps,
//! Accounts' remaining usage and This Mac's load. Everything you can click has a pick id.

use crate::kit::{kit_for, ChairKind, DeskKind, Divider, Kit, Layout, Nook, Plant, Pot, Screens, SignKind, Tool};
use crate::layout::{pack, plan_area, OfficeState, Plan, Rect, Size, AISLE, COL_W, DEPTH, LOBBY_W, ROW_D, STOREY, WALL_H};
use crate::math::{T, V3};
use crate::mesh::Mesh;
use crate::models::{self, chair, desk, desk_props, long_table, person, plant, Item, Look, Pose, B, CUTAWAY};
use crate::paint::{hash, hex, mix, ramp, Img, Rgba, Rng};
use crate::tex::{self, Atlas};
use std::f32::consts::{FRAC_PI_2, PI, TAU};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum St { Working, Idle, Asks, Stuck, Error, Done, Limit }
impl St {
    pub fn word(self) -> &'static str {
        match self { St::Working => "working", St::Idle => "idle", St::Asks => "asks you", St::Stuck => "stuck", St::Error => "crashed", St::Done => "done", St::Limit => "out of usage" }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Sess { pub st: St, pub helpers: usize }
pub struct AppDef { pub id: &'static str, pub name: &'static str, pub tool: Tool, pub slot: usize, pub account: usize, pub sessions: Vec<Sess>, pub decay: u8, pub overloaded: bool }
pub struct Account { pub name: &'static str, pub left: f32 }

/// The sample company: generic names only.
pub fn sample() -> (Vec<AppDef>, Vec<Account>) {
    use St::*;
    let s = |n: usize, marks: &[(usize, St)], helpers: &[(usize, usize)]| -> Vec<Sess> {
        (0..n).map(|i| Sess { st: marks.iter().find(|m| m.0 == i).map(|m| m.1).unwrap_or(Working), helpers: helpers.iter().find(|h| h.0 == i).map(|h| h.1).unwrap_or(0) }).collect()
    };
    let accounts = vec![
        Account { name: "Team plan", left: 0.58 },
        Account { name: "Pro · personal", left: 0.12 },
        Account { name: "Max · work", left: 0.83 },
        Account { name: "Plus · side", left: 0.0 },
    ];
    let apps = vec![
        AppDef { id: "app-1", name: "Claude Code · Work", tool: Tool::ClaudeCode, slot: 0, account: 2, sessions: s(14, &[(2, Asks), (5, Done), (7, Idle), (9, Stuck)], &[(0, 8), (3, 3), (10, 2)]), decay: 0, overloaded: false },
        AppDef { id: "app-2", name: "Codex · Main", tool: Tool::Codex, slot: 1, account: 0, sessions: s(9, &[(1, Error), (4, Idle)], &[(2, 4)]), decay: 0, overloaded: true },
        AppDef { id: "app-3", name: "Claude app · Work", tool: Tool::ClaudeApp, slot: 2, account: 2, sessions: s(5, &[(3, Asks), (1, Idle)], &[]), decay: 0, overloaded: false },
        AppDef { id: "app-4", name: "OpenCode · Lab", tool: Tool::OpenCode, slot: 3, account: 0, sessions: s(4, &[(2, Stuck)], &[(0, 2)]), decay: 0, overloaded: false },
        AppDef { id: "app-5", name: "Gemini CLI · Lab", tool: Tool::Gemini, slot: 4, account: 0, sessions: vec![], decay: 2, overloaded: false },
        AppDef { id: "app-6", name: "Cursor · Client", tool: Tool::Cursor, slot: 5, account: 0, sessions: s(6, &[(4, Done)], &[]), decay: 0, overloaded: false },
        AppDef { id: "app-7", name: "Claude Code · Side", tool: Tool::ClaudeCode, slot: 6, account: 1, sessions: s(8, &[(3, Asks)], &[(5, 5)]), decay: 0, overloaded: false },
        AppDef { id: "app-8", name: "Codex · Review", tool: Tool::Codex, slot: 7, account: 0, sessions: s(3, &[], &[]), decay: 0, overloaded: false },
        AppDef { id: "app-9", name: "Claude app · Personal", tool: Tool::ClaudeApp, slot: 0, account: 3, sessions: s(2, &[], &[]), decay: 0, overloaded: false },
        AppDef { id: "app-10", name: "Claude Code · Research", tool: Tool::ClaudeCode, slot: 1, account: 2, sessions: s(5, &[(1, Error)], &[(2, 6)]), decay: 0, overloaded: false },
        AppDef { id: "app-11", name: "OpenCode · Tools", tool: Tool::OpenCode, slot: 2, account: 0, sessions: s(7, &[(0, Idle)], &[]), decay: 0, overloaded: false },
        AppDef { id: "app-12", name: "Codex · Batch", tool: Tool::Codex, slot: 3, account: 0, sessions: s(11, &[], &[(6, 10)]), decay: 0, overloaded: false },
    ];
    (apps, accounts)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target { App(usize), Session(usize, usize), Helpers(usize, usize), Account(usize), ThisMac, Floor(usize), Prop(usize), Cat }
pub struct Pick { pub target: Target, pub label: String, pub detail: String }

/// At most this many helpers stand round one desk; more show as "+N".
pub const HELPERS_SHOWN: usize = 6;

struct DeskRec { app: usize, sess: usize, floor: i32, frame: T, seat: f32, top: f32, screens: Screens, look: Look, st: St, helpers: usize, id: u32, helper_id: u32, accent: Rgba, high: bool, shared: bool, width: f32 }
#[derive(Clone, Copy)]
pub enum PropKind { Plant { kind: Plant, pot: Pot, s: f32, seed: u32, wilt: f32 }, PingPong, Coffee, Beanbag(Rgba), Arcade }
pub struct PropRec { pub kind: PropKind, pub floor: i32, pub t: T, pub id: u32 }
pub struct Light { pub floor: i32, pub p: [f32; 4], pub c: [f32; 4], pub owner: u32 }
pub struct Label { pub floor: usize, pub at: V3, pub app: usize }
struct SignRec { floor: i32, at: T, account: usize }
struct FireRec { floor: i32, at: T }

pub struct Scene {
    pub apps: Vec<AppDef>,
    pub accounts: Vec<Account>,
    pub kits: Vec<Option<Kit>>,
    pub plan: Plan,
    pub area_of: Vec<Option<usize>>,
    pub stat: Vec<Item>,
    pub atlas: Img,
    pub ui: tex::Ui,
    pub picks: Vec<Pick>,
    pub lights: Vec<Light>,
    pub props: Vec<PropRec>,
    pub labels: Vec<Label>,
    pub floors: usize,
    pub width: f32,
    desks: Vec<DeskRec>,
    signs: Vec<SignRec>,
    fires: Vec<FireRec>,
    boards: Vec<(i32, T, Vec<u32>)>,
    racks: Vec<(i32, T)>,
    beacons: Vec<(i32, V3)>,
    tex: TexKeys,
}

struct TexKeys { code: Vec<[f32; 4]>, chat: [f32; 4], error: [f32; 4], frozen: [f32; 4], off: [f32; 4], done: [f32; 4] }

/// What the scene animates from outside: time, clicks on props, the hover, alerts and load.
pub struct Live { pub t: f32, pub clicked: std::collections::HashMap<u32, f32>, pub lamps_off: std::collections::HashSet<u32>, pub hover: u32, pub alert: bool, pub mac_load: f32, pub eye: V3 }

/// A screen-space marker the overlay draws: a balloon, a "+N" or a spotlight.
pub enum Mark { Balloon { at: V3, icon: &'static str }, Plus { at: V3, n: usize } }

pub fn floor_y(f: usize) -> f32 { f as f32 * STOREY }

impl Scene {
    fn pick(&mut self, target: Target, label: impl Into<String>, detail: impl Into<String>) -> u32 {
        self.picks.push(Pick { target, label: label.into(), detail: detail.into() });
        self.picks.len() as u32
    }
    pub fn pick_of(&self, id: u32) -> Option<&Pick> { if id == 0 { None } else { self.picks.get(id as usize - 1) } }

    /// Builds the office for the sample data. `grow` adds sessions to one App (the reflow demo).
    pub fn build(state: &mut OfficeState, grow: Option<(&str, usize)>) -> (Scene, bool) {
        let (mut apps, accounts) = sample();
        if let Some((id, n)) = grow {
            if let Some(a) = apps.iter_mut().find(|a| a.id == id) { while a.sessions.len() < n { a.sessions.push(Sess { st: St::Working, helpers: 0 }); } }
        }
        let mut changed = false;
        for a in &apps { changed |= state.see(a.id, a.tool, a.sessions.len()); }
        let plan = pack(state);
        let kits: Vec<Option<Kit>> = apps.iter().map(|a| state.apps.get(a.id).map(|s| kit_for(a.tool, s.choice, a.slot))).collect();
        let area_of: Vec<Option<usize>> = apps.iter().map(|a| plan.areas.iter().position(|p| p.id == a.id)).collect();
        let mut atlas = Atlas::new(2048, 2048);
        let tex = TexKeys {
            code: (0..4).map(|k| atlas.add(&format!("code{k}"), &tex::screen_img("code", hex("#8fb4ff"), 40 + k))).collect(),
            chat: atlas.add("chat", &tex::screen_img("chat", hex("#e8a07a"), 3)),
            error: atlas.add("error", &tex::screen_img("error", hex("#ffffff"), 5)),
            frozen: atlas.add("frozen", &tex::screen_img("frozen", hex("#ffffff"), 6)),
            off: atlas.add("off", &tex::screen_img("off", hex("#ffffff"), 7)),
            done: atlas.add("done", &tex::screen_img("done", hex("#ffffff"), 8)),
        };
        let floors = plan.floors.max(2);
        let width = plan.width;
        let mut sc = Scene { apps, accounts, kits, plan, area_of, stat: vec![], atlas: Img::new(1, 1), ui: tex::ui_atlas(), picks: vec![], lights: vec![], props: vec![], labels: vec![], floors, width, desks: vec![], signs: vec![], fires: vec![], boards: vec![], racks: vec![], beacons: vec![], tex };
        let mut out = std::mem::take(&mut sc.stat);
        sc.skyline(&mut out, &mut atlas);
        for f in 0..floors { sc.building_floor(&mut out, &mut atlas, f); }
        for ai in 0..sc.apps.len() { if sc.area_of[ai].is_some() { sc.area(&mut out, &mut atlas, ai); } }
        let free = sc.plan.free.clone();
        let mut seen_floor = vec![false; floors];
        for (f, r) in free {
            let commons = !seen_floor[f];
            seen_floor[f] = true;
            sc.free_cell(&mut out, f, r, commons);
        }
        sc.add_cat();
        sc.stat = out;
        sc.atlas = atlas.img;
        (sc, changed)
    }

    // ---------- the building ----------
    fn skyline(&mut self, out: &mut Vec<Item>, atlas: &mut Atlas) {
        let win = atlas.add("skyline", &tex::skyline_img());
        let mut b = B { out, floor: -1, id: 0, dim: 1.0 };
        let (cx, cz) = (self.width / 2.0, DEPTH / 2.0);
        b.put(Mesh::QuadUp, T::new().t(cx, -0.6, cz).s(900.0, 1.0, 900.0), hex("#1c1830"));
        let mut r = Rng(77);
        for k in 0..64 {
            let a = k as f32 / 64.0 * TAU + r.range(-0.03, 0.03);
            let d = r.range(85.0, 150.0);
            let (w, h, dd) = (r.range(9.0, 22.0), r.range(14.0, 70.0), r.range(9.0, 22.0));
            let c = hex(r.pick(&["#2c2a4a", "#33304f", "#282640", "#3a3558"]));
            let t = T::new().t(cx + d * a.sin(), -0.6, cz + d * a.cos()).ry(a);
            b.tex(Mesh::Box, t.s(w, h, dd), c, win, [w / 4.0, h / 8.0], 0.75, 0);
            if r.chance(0.3) { b.glow(Mesh::Box, t.t(0.0, h, 0.0).s(0.6, 3.0, 0.6), hex("#ff5a5a"), 1.0); }
        }
    }

    fn building_floor(&mut self, out: &mut Vec<Item>, atlas: &mut Atlas, f: usize) {
        let fy = floor_y(f);
        let fl = f as i32;
        let w = self.width;
        let lobby_floor = atlas.add("lobbyfloor", &tex::floor_img(&crate::kit::FloorMat::Concrete { tone: hex("#cdc6b8"), seam: 1.5 }, 3));
        let lobby_wall = atlas.add("lobbywall", &tex::wall_img(&crate::kit::WallFinish::Paint { c: hex("#e6e0d4"), dado: hex("#8a7a6a") }, 4, hex("#888888")));
        let floor_id = self.pick(Target::Floor(f), format!("Floor {}", f + 1), "the lift: go to another floor");
        // slab and the common floor (lobby and aisle)
        {
            let mut b = B { out: &mut *out, floor: fl, id: 0, dim: 1.0 };
            b.put(Mesh::Box, T::new().t(w / 2.0, fy - 0.42, DEPTH / 2.0).s(w + 0.3, 0.4, DEPTH + 0.3), hex("#4a4658"));
            b.put(Mesh::Box, T::new().t(w / 2.0, fy - 0.42, DEPTH + 0.13).s(w + 0.3, 0.06, 0.06), hex("#6a6680"));
            b.tex(Mesh::QuadUp, T::new().t(w / 2.0, fy, DEPTH / 2.0).s(w, 1.0, DEPTH), [255; 4], lobby_floor, [w / 4.0, DEPTH / 4.0], 0.0, 0);
        }
        // the back wall, with windows; each area's own finish behind it
        let mut x = 0.0;
        while x < w - 0.01 {
            let col = if x < LOBBY_W - 0.01 { None } else { Some(((x - LOBBY_W) / COL_W + 0.01).floor() as usize) };
            let seg_end = match col { None => LOBBY_W, Some(c) => LOBBY_W + (c + 1) as f32 * COL_W };
            let owner = col.and_then(|c| self.plan.areas.iter().position(|a| a.floor == f && a.row == 0 && c >= a.col && c < a.col + a.size.cells().0));
            let (texr, id, dim) = match owner.and_then(|pi| self.apps.iter().position(|ap| ap.id == self.plan.areas[pi].id)) {
                Some(ai) => {
                    let k = self.kits[ai].clone().unwrap();
                    (atlas.add(&format!("wall{ai}"), &tex::wall_img(&k.wall, hash(self.apps[ai].id), k.accent)), 0, if self.apps[ai].decay > 0 { 0.6 } else { 1.0 })
                }
                None => (lobby_wall, 0, 1.0),
            };
            let mut b = B { out: &mut *out, floor: fl, id, dim };
            wall_run(&mut b, T::new().t(0.0, fy, 0.0), x, seg_end, texr, true);
            x = seg_end;
        }
        // the side walls: glass on the lobby side, solid at the far end
        {
            let mut b = B { out: &mut *out, floor: fl, id: 0, dim: 1.0 };
            let left = T::new().t(0.0, fy, DEPTH).ry(FRAC_PI_2);
            for k in 0..((DEPTH / 2.4).ceil() as usize) {
                let z = k as f32 * 2.4;
                b.raw(Mesh::Box, left.t(z, 0.0, 0.0).s(0.08, WALL_H, 0.08), hex("#3a3848"), [0.0; 4], [1.0, 1.0], 0.0, CUTAWAY, false);
                b.glass(Mesh::QuadWall, left.t(z + 1.2, 0.05, 0.0).s(2.3, WALL_H - 0.1, 1.0), [170, 200, 230, 60], CUTAWAY);
            }
            b.raw(Mesh::Box, left.t(DEPTH / 2.0, WALL_H - 0.06, 0.0).s(DEPTH, 0.06, 0.1), hex("#3a3848"), [0.0; 4], [1.0, 1.0], 0.0, CUTAWAY, false);
            b.raw(Mesh::QuadWall, T::new().t(w, fy, DEPTH / 2.0).ry(-FRAC_PI_2).s(DEPTH, WALL_H, 1.0), [255; 4], lobby_wall, [DEPTH / 4.8, 1.0], 0.0, CUTAWAY | models::TEXTURED, false);
        }
        // the lobby: lift, directory, usage board, This Mac, a bench, plants
        let numeral = atlas.add(&format!("num{f}"), &tex::text_img(&[(&format!("{}", f + 1), hex("#ffd27a"))], hex("#1d1a24"), 2));
        {
            let mut b = B { out: &mut *out, floor: fl, id: floor_id, dim: 1.0 };
            let core = T::new().t(0.4, fy, 0.3);
            b.tex(Mesh::Box, core.t(1.4, 0.0, 1.35).s(2.8, WALL_H + 0.4, 2.7), [255; 4], lobby_wall, [0.6, 1.0], 0.0, 0);
            for s in [-1.0f32, 1.0] { b.put(Mesh::Box, core.t(1.4 + s * 0.31, 0.0, 2.71).s(0.6, 2.2, 0.04), hex("#b9bcc4")); }
            b.put(Mesh::Box, core.t(1.4, 2.2, 2.71).s(1.4, 0.12, 0.05), hex("#7a7c84"));
            b.tex(Mesh::QuadWall, core.t(1.4, 2.45, 2.72).s(0.5, 0.6, 1.0), [255; 4], numeral, [1.0, 1.0], 0.9, 0);
            b.glow(Mesh::Box, core.t(2.35, 1.1, 2.71).s(0.08, 0.14, 0.04), hex("#7fd6ff"), 1.0);
            self.beacons.push((fl, core.0.point([1.4, WALL_H + 0.5, 1.35])));
        }
        // the directory: this floor's areas, each with its colour
        let mut lines: Vec<(String, Rgba)> = vec![(format!("FLOOR {}", f + 1), hex("#ffd27a"))];
        let mut names: Vec<(usize, usize)> = self.plan.areas.iter().filter(|a| a.floor == f).map(|a| (a.col * 2 + a.row, self.apps.iter().position(|ap| ap.id == a.id).unwrap())).collect();
        names.sort();
        for (_, ai) in names { lines.push((self.apps[ai].name.to_uppercase(), self.kits[ai].as_ref().unwrap().accent)); }
        let dir_lines: Vec<(&str, Rgba)> = lines.iter().map(|(s, c)| (s.as_str(), *c)).collect();
        let dir = atlas.add(&format!("dir{f}"), &tex::text_img(&dir_lines, hex("#1d1a24"), 3));
        {
            let mut b = B { out: &mut *out, floor: fl, id: floor_id, dim: 1.0 };
            let t = T::new().t(4.2, fy, ROW_D + AISLE / 2.0 - 1.6).ry(0.35);
            b.put(Mesh::Box, t.s(0.5, 0.06, 0.4), hex("#2a2733"));
            b.put(Mesh::Box, t.t(0.0, 0.0, 0.0).s(0.08, 1.0, 0.08), hex("#2a2733"));
            let bh = 0.32 + 0.11 * lines.len() as f32;
            b.put(Mesh::Box, t.t(0.0, 1.0, 0.0).s(1.5, bh, 0.06), hex("#2a2733"));
            b.tex(Mesh::QuadWall, t.t(0.0, 1.02, 0.031).s(1.42, bh - 0.04, 1.0), [255; 4], dir, [1.0, 1.0], 0.6, 0);
            b.tex(Mesh::QuadWall, t.t(0.0, 1.02, -0.031).ry(PI).s(1.42, bh - 0.04, 1.0), [255; 4], dir, [1.0, 1.0], 0.6, 0);
        }
        // Accounts' usage board
        let mut ids = vec![];
        for (i, a) in self.accounts.iter().enumerate().map(|(i, a)| (i, a.name)).collect::<Vec<_>>() {
            ids.push(self.pick(Target::Account(i), a.to_string(), format!("{}% of usage left", (self.accounts[i].left * 100.0).round())));
        }
        let rows: Vec<(String, Rgba)> = std::iter::once(("USAGE LEFT".to_string(), hex("#ffd27a"))).chain(self.accounts.iter().map(|a| (a.name.to_uppercase(), hex("#e6e1d8")))).collect();
        let rows_ref: Vec<(&str, Rgba)> = rows.iter().map(|(s, c)| (s.as_str(), *c)).collect();
        let board = atlas.add("usage", &tex::text_img(&rows_ref, hex("#1d1a24"), 3));
        {
            let mut b = B { out: &mut *out, floor: fl, id: ids[0], dim: 1.0 };
            let t = T::new().t(6.2, fy, 1.0);
            let bh = 0.32 + 0.11 * rows.len() as f32;
            for s in [-1.0f32, 1.0] { b.put(Mesh::Box, t.t(s * 0.9, 0.0, 0.0).s(0.06, 1.0, 0.06), hex("#2a2733")); }
            b.put(Mesh::Box, t.t(0.0, 1.0, 0.0).s(2.1, bh, 0.06), hex("#2a2733"));
            b.tex(Mesh::QuadWall, t.t(-0.35, 1.02, 0.031).s(1.3, bh - 0.04, 1.0), [255; 4], board, [1.0, 1.0], 0.6, 0);
            self.boards.push((fl, t.t(0.0, 1.0, 0.04), ids));
        }
        // This Mac: a server rack whose lights show the load
        let mac = self.pick(Target::ThisMac, "This Mac", "load and memory");
        let rack = atlas.add("rack", &tex::rack_img());
        let label = atlas.add("thismac", &tex::text_img(&[("THIS MAC", hex("#7fd6ff"))], hex("#1d1a24"), 2));
        {
            let mut b = B { out: &mut *out, floor: fl, id: mac, dim: 1.0 };
            let t = T::new().t(1.0, fy, 6.4).ry(FRAC_PI_2);
            b.put(Mesh::Box, t.s(0.7, 2.0, 0.8), hex("#1c1d22"));
            b.tex(Mesh::QuadWall, t.t(0.0, 0.08, 0.401).s(0.6, 1.84, 1.0), [255; 4], rack, [1.0, 1.0], 0.4, 0);
            b.tex(Mesh::QuadWall, t.t(0.0, 2.05, 0.2).s(0.7, 0.2, 1.0), [255; 4], label, [1.0, 1.0], 0.8, 0);
            self.racks.push((fl, t.t(0.0, 0.0, 0.41)));
        }
        {
            let mut b = B { out: &mut *out, floor: fl, id: 0, dim: 1.0 };
            let bench = T::new().t(1.2, fy, 14.5).ry(FRAC_PI_2);
            b.cb(bench, [-1.0, 0.36, -0.25], [2.0, 0.1, 0.5], hex("#b5824f"));
            for s in [-1.0f32, 1.0] { b.bx(bench, [s * 0.85 - 0.04, 0.0, -0.22], [0.08, 0.36, 0.44], hex("#2a2733")); }
            let rug = atlas.add("lobbyrug", &tex::rug_img(hex("#4a4060"), hex("#ffd27a"), 9));
            b.tex(Mesh::Box, T::new().t(4.2, fy, 13.5).s(4.6, 0.012, 5.0), [255; 4], rug, [1.0, 1.0], 0.0, 0);
        }
        for (k, (x, z)) in [(0.9, 4.0), (0.9, 18.6), (7.6, 18.8), (3.6, 4.3)].iter().enumerate() {
            let id = self.pick(Target::Prop(self.props.len()), "A plant", "give it a shake");
            self.props.push(PropRec { kind: PropKind::Plant { kind: [Plant::Fig, Plant::Palm, Plant::Monstera, Plant::Snake][k], pot: Pot::Concrete, s: 1.3, seed: hash(&format!("lobby{f}{k}")), wilt: 0.0 }, floor: fl, t: T::new().t(*x, fy, *z), id });
        }
        if f == 0 {
            let id = self.pick(Target::Prop(self.props.len()), "The coffee machine", "make a coffee");
            let mut b = B { out: &mut *out, floor: fl, id, dim: 1.0 };
            let t = T::new().t(6.4, fy, DEPTH - 0.6).ry(PI);
            b.cb(t, [-0.9, 0.0, -0.3], [1.8, 0.9, 0.6], hex("#3a3848"));
            b.bx(t, [-0.95, 0.9, -0.33], [1.9, 0.04, 0.66], hex("#e6e1d8"));
            self.props.push(PropRec { kind: PropKind::Coffee, floor: fl, t: t.t(-0.4, 0.94, 0.0), id });
            for k in 0..4 { b.put(Mesh::Cyl8, t.t(0.3 + k as f32 * 0.12, 0.94, 0.05).s(0.08, 0.1, 0.08), hex(["#f4efe6", "#c86f4f", "#3a5a7a", "#e8b33a"][k])); }
        }
        // light along the aisle (the fittings are in the ceiling, which the cutaway leaves out)
        for k in 0..((w - LOBBY_W) / 4.0) as usize {
            let x = LOBBY_W + 2.0 + k as f32 * 4.0;
            self.lights.push(Light { floor: fl, p: [x, fy + WALL_H - 0.7, ROW_D + AISLE / 2.0, 5.0], c: [0.5, 0.42, 0.3, 1.0], owner: 0 });
        }
        if f == 0 { self.lights.push(Light { floor: fl, p: [4.0, fy + 2.6, 8.0, 7.0], c: [0.45, 0.4, 0.32, 1.0], owner: 0 }); }
    }

    fn free_cell(&mut self, out: &mut Vec<Item>, f: usize, r: Rect, commons: bool) {
        let fl = f as i32;
        let fy = floor_y(f);
        let id = self.pick(Target::Floor(f), if commons { "Kitchen and lounge" } else { "Free space" }, if commons { "where idle people go" } else { "room for a new App" });
        let mut b = B { out, floor: fl, id, dim: 1.0 };
        let wood = hex("#b5824f");
        let p = (r.cx(), r.cz());
        b.put(Mesh::QuadUp, T::new().t(p.0, fy + 0.006, p.1).s(r.w() - 0.4, 1.0, r.d() - 0.4), hex("#a07850"));
        if commons {
            // a kitchen counter with a fridge, and two round tables
            let k = T::new().t(r.x1 - 0.5, fy, r.cz()).ry(-FRAC_PI_2);
            b.cb(k, [-2.4, 0.0, -0.32], [4.8, 0.9, 0.64], hex("#e6e1d8"));
            b.bx(k, [-2.45, 0.9, -0.35], [4.9, 0.05, 0.7], hex("#3a3848"));
            b.cb(k, [2.5, 0.0, -0.35], [0.8, 1.9, 0.7], hex("#c9ccd4"));
            b.bx(k, [2.55, 0.95, 0.36], [0.05, 0.4, 0.02], hex("#8a8c94"));
            for (dx, dz) in [(-1.6, -2.4), (0.6, 2.0)] {
                let t = T::new().t(p.0 + dx, fy, p.1 + dz);
                desk(&mut b, t, DeskKind::Round, wood, hex("#2a2733"), 1.0);
                for k in 0..3 { chair(&mut b, t.ry(k as f32 * TAU / 3.0 + 0.3).t(0.0, 0.0, 0.8), ChairKind::Stool, hex("#e8b33a"), false); }
            }
        } else {
            b.cb(T::new().t(p.0 - 1.0, fy, p.1), [-1.0, 0.0, -0.45], [2.0, 0.45, 0.9], hex("#5a6a8a"));
            b.cb(T::new().t(p.0 - 1.0, fy, p.1 + 0.4), [-1.0, 0.45, -0.05], [2.0, 0.4, 0.2], hex("#5a6a8a"));
            let sign = T::new().t(p.0 + 1.2, fy, p.1);
            b.put(Mesh::Box, sign.s(0.06, 0.9, 0.06), hex("#2a2733"));
            b.put(Mesh::Box, sign.t(0.0, 0.9, 0.0).s(0.9, 0.5, 0.04), hex("#f4efe6"));
        }
        for (x, z) in [(r.x0 + 0.6, r.z0 + 0.6), (r.x0 + 0.6, r.z1 - 0.6)] {
            let pid = self.pick(Target::Prop(self.props.len()), "A plant", "give it a shake");
            self.props.push(PropRec { kind: PropKind::Plant { kind: Plant::Monstera, pot: Pot::Ceramic(hex("#e6e1d8")), s: 1.1, seed: hash(&format!("free{f}{x}{z}")), wilt: 0.0 }, floor: fl, t: T::new().t(x, fy, z), id: pid });
        }
        if commons {
            for k in 0..3 {
                let pid = self.pick(Target::Prop(self.props.len()), "A beanbag", "flop into it");
                self.props.push(PropRec { kind: PropKind::Beanbag(hex(["#d9644a", "#3f7fbf", "#e8b33a"][k])), floor: fl, t: T::new().t(r.x0 + 1.4 + k as f32 * 0.9, fy, r.z1 - 1.0), id: pid });
            }
        }
    }

    // ---------- an App's area ----------
    fn area(&mut self, out: &mut Vec<Item>, atlas: &mut Atlas, ai: usize) {
        let pa = self.plan.areas[self.area_of[ai].unwrap()].clone();
        let kit = self.kits[ai].clone().unwrap();
        let app_id = self.apps[ai].id;
        let seed = hash(app_id);
        let (f, fl, fy) = (pa.floor, pa.floor as i32, floor_y(pa.floor));
        let decay = self.apps[ai].decay;
        let dim = if decay > 0 { 0.62 } else { 1.0 };
        let n = self.apps[ai].sessions.len();
        let plan = plan_area(pa.rect, pa.size, kit.layout, kit.jitter, seed, pa.row);
        println!("{}", serde_json::json!({ "event": "area_plan", "app": self.apps[ai].name, "layout": format!("{:?} -> {:?}", kit.layout, plan.layout), "size": format!("{:?}", pa.size), "groups": plan.groups.len(), "spots": plan.spots.len(), "sessions": self.apps[ai].sessions.len(), "nook": plan.nook.is_some() }));
        let rect = pa.rect;
        let area_id = self.pick(Target::App(ai), self.apps[ai].name, format!("{} sessions · {} · {}", n, kit.family, self.accounts[self.apps[ai].account].name));
        let mut r = Rng(seed);
        let floor_tex = atlas.add(&format!("floor{ai}"), &tex::floor_img(&kit.floor, seed));
        let tile = tex::floor_tile(&kit.floor);
        // the area's floor, inset so the common floor shows as a border
        {
            let mut b = B { out: &mut *out, floor: fl, id: area_id, dim };
            let fr = rect.inset(0.12);
            b.raw(Mesh::QuadUp, T::new().t(fr.cx(), fy + 0.004, fr.cz()).s(fr.w(), 1.0, fr.d()), [255; 4], floor_tex, [fr.w() / tile, fr.d() / tile], 0.0, models::TEXTURED, false);
            if pa.size == Size::L {
                // the aisle runs through a big team: keep it common floor
                b.put(Mesh::QuadUp, T::new().t(rect.cx(), fy + 0.006, ROW_D + AISLE / 2.0).s(rect.w(), 1.0, AISLE), hex("#cfc8bb"));
            }
            if kit.rug != hex("#2a2d33") && !plan.groups.is_empty() {
                let rug = atlas.add(&format!("rug{ai}"), &tex::rug_img(kit.rug, kit.accent, seed));
                let g = plan.groups[0];
                b.tex(Mesh::Box, T::new().t(g.x, fy + 0.008, g.z).ry(g.turn).s(3.6, 0.01, 2.8), [255; 4], rug, [1.0, 1.0], 0.0, 0);
            }
        }
        // the divider on the lobby side or towards the area on the left
        {
            let mut b = B { out: &mut *out, floor: fl, id: area_id, dim };
            let (z0, z1) = (rect.z0 + 0.3, rect.z1 - 0.3);
            let aisle_band = (ROW_D - 0.2, ROW_D + AISLE + 0.2);
            let mut segs = vec![];
            if pa.size == Size::L { segs.push((z0, aisle_band.0 - 1.0)); segs.push((aisle_band.1 + 1.0, z1)); }
            else if pa.row == 0 { segs.push((z0, z1 - 1.3)); } else { segs.push((z0 + 1.3, z1)); }
            for (a, bz) in segs { divider(&mut b, &kit, rect.x0 + 0.06, fy, a, bz, &mut r, atlas); }
            // decor on the back wall
            if pa.row == 0 {
                let poster = atlas.add(&format!("poster{ai}"), &tex::poster_img(seed, kit.accent));
                let boardt = atlas.add(&format!("board{ai}"), &tex::board_img(seed));
                let mut x = rect.x0 + 1.4;
                while x < rect.x1 - 1.2 {
                    let t = T::new().t(x, fy, 0.03);
                    match r.int(3) {
                        0 => b.raw(Mesh::QuadWall, t.t(0.0, 1.35, 0.0).s(0.6, 0.8, 1.0), [255; 4], poster, [1.0, 1.0], 0.0, CUTAWAY | models::TEXTURED, false),
                        1 => b.raw(Mesh::QuadWall, t.t(0.0, 1.0, 0.0).s(1.4, 0.82, 1.0), [255; 4], boardt, [1.0, 1.0], 0.0, CUTAWAY | models::TEXTURED, false),
                        _ => {}
                    }
                    x += 2.4;
                }
            }
        }
        // the nook
        if let Some(nk) = plan.nook { self.nook(out, atlas, ai, fl, fy, nk, &kit, dim, area_id); }
        // desks: one per session, two spare (a dusty few when unused)
        let shown = if decay > 0 { 4.min(plan.spots.len()) } else { (n + 2).min(plan.spots.len()) };
        let wood = kit.desk_top;
        if plan.layout == Layout::Tables {
            let mut b = B { out: &mut *out, floor: fl, id: area_id, dim };
            for (gi, g) in plan.groups.iter().enumerate() {
                if plan.spots.iter().take(shown).any(|s| s.group == gi) { long_table(&mut b, T::new().t(g.x, fy, g.z).ry(g.turn), 4.2, wood, kit.desk_frame); }
            }
        }
        if plan.layout == Layout::Radial {
            for (gi, g) in plan.groups.iter().enumerate() {
                if !plan.spots.iter().take(shown).any(|s| s.group == gi) { continue; }
                let pid = self.pick(Target::Prop(self.props.len()), "A plant", "give it a shake");
                self.props.push(PropRec { kind: PropKind::Plant { kind: kit.plants[gi % kit.plants.len()], pot: kit.pot, s: 1.1, seed: seed ^ gi as u32, wilt: if decay > 0 { 0.7 } else { 0.0 } }, floor: fl, t: T::new().t(g.x, fy, g.z), id: pid });
            }
        }
        let fire_at = if self.apps[ai].overloaded { Some(1usize.min(n.saturating_sub(1))) } else { None };
        for (si, sp) in plan.spots.iter().take(shown).enumerate() {
            let occupied = si < n && decay == 0;
            let frame = T::new().t(sp.x, fy, sp.z).ry(sp.face);
            let (sid, hid) = if occupied {
                let s = self.apps[ai].sessions[si];
                let sid = self.pick(Target::Session(ai, si), format!("{} · desk {}", self.apps[ai].name, si + 1), s.st.word());
                let hid = self.pick(Target::Helpers(ai, si), format!("{} helpers at desk {}", s.helpers, si + 1), "subagents working for this session");
                (sid, hid)
            } else { (area_id, 0) };
            let mut b = B { out: &mut *out, floor: fl, id: sid, dim };
            let (top, w) = if sp.shared { (models::DESK_H, 1.2) } else {
                let w = if kit.desk == DeskKind::Round { 1.0 } else { 1.4 };
                (desk(&mut b, frame, kit.desk, wood, kit.desk_frame, w), w)
            };
            let high = kit.desk == DeskKind::Standing;
            let mut dr = Rng(seed ^ (si as u32).wrapping_mul(2654435761));
            if let Some(p) = desk_props(&mut b, frame.t(0.0, 0.0, if sp.shared { 0.1 } else { 0.0 }), top, &mut dr, kit.accent, kit.lamps && occupied && !sp.shared, w) {
                self.lights.push(Light { floor: fl, p: [p[0], p[1], p[2], 2.2], c: [0.55, 0.45, 0.28, 1.0], owner: sid });
            }
            if decay > 0 {
                // boxes, dust and a chair left askew
                b.cb(frame.t(0.2, top, -0.05).ry(0.3), [-0.25, 0.0, -0.2], [0.5, 0.32, 0.4], hex("#b58a5a"));
                for _ in 0..5 { b.put(Mesh::QuadUp, frame.t(dr.range(-0.6, 0.6), top + 0.002, dr.range(-0.3, 0.3)).s(0.08, 1.0, 0.06), hex("#b8b2a6")); }
                b.cb(frame.t(-1.0, 0.0, 0.9).ry(0.8), [-0.3, 0.0, -0.25], [0.6, 0.4, 0.5], hex("#b58a5a"));
            }
            let seat_t = if occupied { frame.t(0.0, 0.0, 0.62) } else { frame.t(dr.range(-0.3, 0.3), 0.0, 0.75 + dr.range(0.0, 0.3)).ry(dr.range(-0.6, 0.6)) };
            let seat = chair(&mut b, seat_t, kit.chair, kit.chair_c, high);
            if !occupied {
                let mut sb = B { out: &mut *out, floor: fl, id: sid, dim };
                models::screens(&mut sb, frame, kit.screens, top, self.tex.off, 0.0, -1.0);
                continue;
            }
            if Some(si) == fire_at { self.fires.push(FireRec { floor: fl, at: frame.t(0.45, top, -0.1) }); }
            let s = self.apps[ai].sessions[si];
            let st = if self.accounts[self.apps[ai].account].left <= 0.0 { St::Limit } else { s.st };
            let mut lr = Rng(seed ^ (si as u32 + 77).wrapping_mul(97));
            self.desks.push(DeskRec { app: ai, sess: si, floor: fl, frame, seat, top, screens: kit.screens, look: Look::random(&mut lr, kit.accent), st, helpers: s.helpers, id: sid, helper_id: hid, accent: kit.accent, high, shared: sp.shared, width: w });
        }
        // plants in the corners
        for (k, (x, z)) in plan.plants.iter().enumerate() {
            if k >= kit.greenery { break; }
            let pid = self.pick(Target::Prop(self.props.len()), "A plant", "give it a shake");
            self.props.push(PropRec { kind: PropKind::Plant { kind: kit.plants[k % kit.plants.len()], pot: kit.pot, s: r.range(1.0, 1.35), seed: seed ^ (k as u32 * 31), wilt: if decay > 0 { 0.7 } else { 0.0 } }, floor: fl, t: T::new().t(*x, fy, *z), id: pid });
        }
        // pendants over the desk groups
        if kit.pendants && decay == 0 {
            let mut b = B { out: &mut *out, floor: fl, id: area_id, dim };
            for g in plan.groups.iter().take(shown.div_ceil(plan.groups.first().map(|g| g.seats).unwrap_or(1))) {
                let t = T::new().t(g.x, fy + WALL_H - 0.7, g.z);
                b.put(Mesh::Box, t.t(0.0, 0.2, 0.0).s(0.01, 0.6, 0.01), hex("#2a2733"));
                if kit.tool == Tool::Codex { b.put(Mesh::Cyl8, t.s(0.12, 0.2, 0.12), hex("#1d1a24")); b.glow(Mesh::Ball, t.t(0.0, -0.16, 0.0).s(0.11, 0.16, 0.11), hex("#ffd38a"), 1.0); }
                else { b.put(Mesh::Cone8, t.s(0.5, 0.25, 0.5), kit.accent); b.glow(Mesh::Ball, t.t(0.0, -0.06, 0.0).s(0.1, 0.1, 0.1), hex("#fff1c8"), 1.0); }
                self.lights.push(Light { floor: fl, p: [g.x, fy + WALL_H - 1.0, g.z, 4.2], c: [0.5 * kit.warmth + 0.2, 0.42, 0.3 - 0.1 * kit.warmth, 1.0], owner: 0 });
            }
        }
        // the sign: one place for every area, over the way in, readable from both sides
        let (ex, ez) = plan.entrance;
        let ez = if pa.row == 0 && pa.size != Size::L { ez - 0.2 } else if pa.row == 1 { ez + 0.2 } else { ROW_D + 0.15 };
        let sign_style = match kit.sign { SignKind::Neon => "neon", SignKind::Banner => "light", _ => "dark" };
        let sign = atlas.add(&format!("sign{ai}"), &tex::sign_img(self.apps[ai].tool.monogram(), &self.apps[ai].name.to_uppercase(), kit.accent, sign_style));
        let (sw, sh) = ((crate::paint::text_width(&self.apps[ai].name.to_uppercase(), 1) + 22) as f32 * 0.024, 13.0 * 0.024);
        {
            let mut b = B { out: &mut *out, floor: fl, id: area_id, dim };
            let t = T::new().t(ex, fy + 2.3, ez).ry(if kit.sign == SignKind::Blade { 0.35 } else { 0.0 });
            let glow = if kit.sign == SignKind::Neon { 0.95 } else { 0.35 };
            for s in [-1.0f32, 1.0] { b.put(Mesh::Box, t.t(s * sw * 0.42, sh + 0.04, 0.0).s(0.012, WALL_H - 2.3 - sh, 0.012), hex("#2a2733")); }
            b.put(Mesh::Box, t.t(0.0, -0.03, 0.0).s(sw + 0.08, sh + 0.06, 0.05), match kit.sign { SignKind::Banner => kit.accent, SignKind::Neon => hex("#141420"), _ => hex("#2a2733") });
            b.tex(Mesh::QuadWall, t.t(0.0, 0.0, 0.026).s(sw, sh, 1.0), [255; 4], sign, [1.0, 1.0], glow, 0);
            b.tex(Mesh::QuadWall, t.t(0.0, 0.0, -0.026).ry(PI).s(sw, sh, 1.0), [255; 4], sign, [1.0, 1.0], glow, 0);
            if kit.sign == SignKind::Banner {
                b.put(Mesh::Box, t.t(0.0, -0.75, 0.0).s(sw * 0.3, 0.72, 0.02), kit.accent);
                b.put(Mesh::Cyl6, t.t(-sw * 0.2, -0.78, 0.0).rz(-FRAC_PI_2).s(0.03, sw * 0.4, 0.03), hex("#2a2733"));
            }
            if kit.sign == SignKind::Pennant {
                for k in 0..9 {
                    let x = ex - 2.6 + k as f32 * 0.65;
                    b.put(Mesh::Cone8, T::new().t(x, fy + 2.9, ez).rx(PI).s(0.3, 0.38, 0.03), hex(["#d9644a", "#3f7fbf", "#e8b33a"][k % 3]));
                }
                b.put(Mesh::Box, T::new().t(ex, fy + 2.9, ez).s(5.4, 0.01, 0.01), hex("#2a2733"));
            }
            if kit.sign == SignKind::Neon { self.lights.push(Light { floor: fl, p: [ex, fy + 2.3, ez, 3.0], c: [kit.accent[0] as f32 / 400.0, kit.accent[1] as f32 / 400.0, kit.accent[2] as f32 / 400.0, 1.0], owner: 0 }); }
            // the flag at the corner of the way in, in the App's colour, seen from anywhere
            let fx = rect.x0 + 0.45;
            let fz = if pa.row == 0 && pa.size != Size::L { rect.z1 - 0.45 } else { rect.z0 + 0.45 };
            b.put(Mesh::Cyl6, T::new().t(fx, fy, fz).s(0.05, 3.2, 0.05), hex("#d8d2c6"));
            b.put(Mesh::Ball, T::new().t(fx, fy + 3.18, fz).s(0.08, 0.08, 0.08), hex("#ffd27a"));
            b.glow(Mesh::Box, T::new().t(fx + 0.36, fy + 2.62, fz).s(0.7, 0.48, 0.02), kit.accent, 0.25);
            self.signs.push(SignRec { floor: fl, at: t.t(0.0, -0.13, 0.0), account: self.apps[ai].account });
            self.beacons.push((fl, [ex, fy + 2.9, ez]));
        }
        self.labels.push(Label { floor: f, at: [ex, fy + 3.3, ez], app: ai });
    }

    fn nook(&mut self, out: &mut Vec<Item>, atlas: &mut Atlas, ai: usize, fl: i32, fy: f32, nk: Rect, kit: &Kit, dim: f32, area_id: u32) {
        let (cx, cz) = (nk.cx(), nk.cz());
        let mut b = B { out: &mut *out, floor: fl, id: area_id, dim };
        let soft = mix(kit.chair_c, hex("#8a7a6a"), 0.35);
        match kit.nook {
            Nook::Sofa => {
                let t = T::new().t(cx, fy, cz - 0.6);
                b.cb(t, [-1.1, 0.0, -0.45], [2.2, 0.42, 0.9], soft);
                b.cb(t, [-1.1, 0.42, -0.45], [2.2, 0.45, 0.25], soft);
                for s in [-1.0f32, 1.0] { b.cb(t, [s * 1.0 - 0.15, 0.0, -0.45], [0.3, 0.62, 0.9], ramp(soft, -0.1)); }
                for k in 0..3 { b.cb(t, [-0.9 + k as f32 * 0.62, 0.42, -0.2], [0.56, 0.14, 0.55], ramp(soft, 0.12)); }
                b.put(Mesh::Cyl12, T::new().t(cx, fy + 0.4, cz + 0.6).s(0.9, 0.04, 0.6), kit.desk_top);
                b.put(Mesh::Cyl8, T::new().t(cx, fy, cz + 0.6).s(0.08, 0.4, 0.08), hex("#2a2733"));
                let lamp = T::new().t(cx + 1.4, fy, cz - 0.6);
                b.put(Mesh::Cyl8, lamp.s(0.3, 0.03, 0.3), hex("#2a2733"));
                b.put(Mesh::Cyl6, lamp.s(0.03, 1.5, 0.03), hex("#2a2733"));
                b.put(Mesh::Pot, lamp.t(0.0, 1.45, 0.0).s(0.42, 0.3, 0.42), hex("#f1e6d6"));
                self.lights.push(Light { floor: fl, p: [cx + 1.4, fy + 1.5, cz - 0.6, 3.5], c: [0.5, 0.38, 0.22, 1.0], owner: 0 });
            }
            Nook::Library => {
                let books = atlas.add(&format!("books{ai}"), &tex::books_img(hash(self.apps[ai].id)));
                for (k, dx) in [-0.8f32, 0.8].iter().enumerate() {
                    let t = T::new().t(cx + dx, fy, nk.z0 + 0.25);
                    b.put(Mesh::Box, t.s(1.5, 2.0, 0.4), hex("#6b4a32"));
                    for sh in 0..4 { b.tex(Mesh::QuadWall, t.t(0.0, 0.12 + sh as f32 * 0.47, 0.201).s(1.38, 0.38, 1.0), [255; 4], books, [1.0 + k as f32 * 0.3, 1.0], 0.0, 0); }
                }
                let ch = T::new().t(cx - 0.5, fy, cz + 0.6).ry(PI + 0.5);
                b.cb(ch, [-0.45, 0.0, -0.4], [0.9, 0.42, 0.8], soft);
                b.cb(ch, [-0.45, 0.42, 0.2], [0.9, 0.5, 0.2], soft);
                for s in [-1.0f32, 1.0] { b.cb(ch, [s * 0.42 - 0.1, 0.0, -0.4], [0.2, 0.62, 0.8], ramp(soft, -0.1)); }
                let rug = atlas.add(&format!("nookrug{ai}"), &tex::rug_img(kit.rug, kit.accent, 3));
                b.tex(Mesh::Box, T::new().t(cx, fy + 0.008, cz + 0.4).s(2.4, 0.01, 1.8), [255; 4], rug, [1.0, 1.0], 0.0, 0);
            }
            Nook::Huddle => {
                let t = T::new().t(cx, fy, cz);
                desk(&mut b, t, DeskKind::Round, kit.desk_top, kit.desk_frame, 1.1);
                for k in 0..4 { chair(&mut b, t.ry(k as f32 * FRAC_PI_2 + 0.4).t(0.0, 0.0, 0.85), kit.chair, kit.chair_c, false); }
                let wb = atlas.add(&format!("wb{ai}"), &tex::board_img(hash(self.apps[ai].id) ^ 5));
                let s = T::new().t(cx, fy, nk.z0 + 0.2);
                for x in [-0.7f32, 0.7] { b.put(Mesh::Box, s.t(x, 0.0, 0.0).s(0.04, 1.8, 0.04), hex("#9aa0a8")); }
                b.tex(Mesh::Box, s.t(0.0, 0.85, 0.0).s(1.5, 0.9, 0.03), [255; 4], wb, [1.0, 1.0], 0.0, 0);
            }
            Nook::Workbench => {
                let t = T::new().t(cx, fy, nk.z0 + 0.5);
                b.bx(t, [-1.3, 0.85, -0.4], [2.6, 0.08, 0.8], hex("#c9a46c"));
                for (x, z) in [(-1.2, -0.3), (1.2, -0.3), (-1.2, 0.3), (1.2, 0.3)] { b.bx(t, [x - 0.04, 0.0, z - 0.04], [0.08, 0.85, 0.08], hex("#3a3a3e")); }
                b.bx(t, [-1.25, 0.2, -0.35], [2.5, 0.04, 0.7], hex("#8a6a42"));
                let peg = atlas.add("peg", &tex::pegboard_img());
                b.tex(Mesh::Box, t.t(0.0, 0.93, -0.38).s(2.6, 1.1, 0.03), [255; 4], peg, [3.0, 1.4], 0.0, 0);
                for k in 0..6 { b.put(Mesh::Box, t.t(-1.0 + k as f32 * 0.4, 1.2, -0.35).s(0.05, 0.3, 0.03), hex(["#c84a3a", "#3a8fb0", "#e0a03a", "#2a2b30", "#9aa0a8", "#4a7a5a"][k])); }
                b.put(Mesh::Box, t.t(0.5, 0.93, 0.0).s(0.3, 0.2, 0.25), hex("#c84a3a"));
                b.put(Mesh::Cyl8, t.t(-0.5, 0.93, 0.1).s(0.15, 0.35, 0.15), hex("#3a8fb0"));
                // a bike against the bench
                let bike = T::new().t(cx + 0.4, fy, cz + 1.0).ry(0.3);
                for s in [-0.5f32, 0.5] { b.put(Mesh::Cyl12, bike.t(s, 0.33, 0.0).rx(FRAC_PI_2).s(0.66, 0.04, 0.66), hex("#1f1f24")); }
                b.put(Mesh::Box, bike.t(0.0, 0.35, 0.0).rz(-0.5).s(0.04, 0.6, 0.04), kit.accent);
                b.put(Mesh::Box, bike.t(-0.1, 0.62, 0.0).rz(FRAC_PI_2).s(0.04, 0.6, 0.04), kit.accent);
            }
            Nook::Beanbags => {
                for k in 0..3 {
                    let pid = self.pick(Target::Prop(self.props.len()), "A beanbag", "flop into it");
                    self.props.push(PropRec { kind: PropKind::Beanbag(hex(["#d9644a", "#3f7fbf", "#e8b33a"][k])), floor: fl, t: T::new().t(cx - 1.0 + k as f32 * 1.0, fy, cz + if k == 1 { -0.6 } else { 0.4 }), id: pid });
                }
                let mut b = B { out: &mut *out, floor: fl, id: area_id, dim };
                b.put(Mesh::Cyl12, T::new().t(cx, fy, cz + 0.1).s(0.6, 0.3, 0.6), hex("#e6e1d8"));
            }
            Nook::PingPong => {
                let pid = self.pick(Target::Prop(self.props.len()), "Ping-pong", "play a rally");
                self.props.push(PropRec { kind: PropKind::PingPong, floor: fl, t: T::new().t(cx, fy, cz), id: pid });
            }
            Nook::Arcade => {
                let pid = self.pick(Target::Prop(self.props.len()), "An arcade cabinet", "insert coin");
                self.props.push(PropRec { kind: PropKind::Arcade, floor: fl, t: T::new().t(cx - 0.6, fy, nk.z0 + 0.5), id: pid });
                let mut b = B { out: &mut *out, floor: fl, id: area_id, dim };
                b.cb(T::new().t(cx + 0.8, fy, cz + 0.4), [-0.6, 0.0, -0.4], [1.2, 0.4, 0.8], hex("#2a2b30"));
            }
        }
    }

    // ---------- the frame ----------
    /// Builds the moving part of the scene for the floors in view. Returns whether anything moves.
    pub fn frame(&self, live: &Live, floors: &[usize], out: &mut Vec<Item>, lights: &mut Vec<Light>, marks: &mut Vec<Mark>) -> bool {
        out.clear();
        lights.clear();
        marks.clear();
        let t = live.t;
        let on = |f: i32| f < 0 || floors.contains(&(f as usize));
        let mut moving = false;
        for l in &self.lights { if on(l.floor) && !(l.owner != 0 && live.lamps_off.contains(&l.owner)) { lights.push(Light { floor: l.floor, p: l.p, c: l.c, owner: l.owner }); } }
        // people at their desks, their screens and their helpers
        for d in self.desks.iter().filter(|d| on(d.floor)) {
            let mut b = B { out: &mut *out, floor: d.floor, id: d.id, dim: 1.0 };
            let ph = (d.app * 7 + d.sess * 3) as f32;
            let (pose, screen, glow, scroll) = match d.st {
                St::Working => (Pose::Typing, self.tex.code[(d.app + d.sess) % 4], 0.9, (t * 0.25 + ph * 0.13).fract()),
                St::Idle => (Pose::Idle, if self.apps[d.app].tool == Tool::ClaudeApp { self.tex.chat } else { self.tex.code[d.sess % 4] }, 0.6, -1.0),
                St::Asks => (Pose::Wave, self.tex.chat, 0.9, -1.0),
                St::Stuck => (Pose::Scratch, self.tex.frozen, 0.7, -1.0),
                St::Error => (Pose::HandsOnHead, self.tex.error, 1.0, -1.0),
                St::Done => (Pose::Stretch, self.tex.done, 0.9, -1.0),
                St::Limit => (Pose::Asleep, self.tex.off, 0.0, -1.0),
            };
            let screen_tex = if d.st == St::Working || d.st == St::Idle { if self.apps[d.app].tool == Tool::ClaudeApp { self.tex.chat } else { screen } } else { screen };
            let scroll = if self.apps[d.app].tool == Tool::ClaudeApp && d.st == St::Working { -1.0 } else { scroll };
            models::screens(&mut b, d.frame, d.screens, d.top, screen_tex, glow, scroll);
            // a hovered person looks up at you
            let turn = if live.hover == d.id { let e = live.eye; let p = d.frame.0.point([0.0, 1.2, 0.62]); let dir = [e[0] - p[0], e[2] - p[2]]; let face = d.frame.0.point([0.0, 0.0, -1.0]); let base = d.frame.0.point([0.0, 0.0, 0.0]); let f = [face[0] - base[0], face[2] - base[2]]; let ang = (f[0] * dir[1] - f[1] * dir[0]).atan2(f[0] * dir[0] + f[1] * dir[1]); -ang.clamp(-1.2, 1.2) } else { 0.0 };
            let stand_at = d.frame.t(if d.shared { 0.5 } else { 0.55 + d.width * 0.25 }, 0.0, 0.75).ry(PI + 0.4);
            let (root, seat) = if pose == Pose::Wave || pose == Pose::HandsOnHead { (stand_at, 0.0) } else { (d.frame.t(0.0, 0.0, 0.62), d.seat) };
            let anim_t = t + ph;
            let head = person(&mut b, root, &d.look, pose, anim_t, seat, turn);
            let icon = match d.st { St::Asks => Some("asks"), St::Stuck => Some("stuck"), St::Error => Some("error"), St::Done => Some("done"), St::Limit => Some("sleep"), _ => None };
            if let Some(i) = icon { marks.push(Mark::Balloon { at: [head[0], head[1] + 0.12 + 0.05 * (t * 3.0 + ph).sin(), head[2]], icon: i }); }
            if d.st == St::Asks {
                // a spotlight on the desk, seen from far
                let p = d.frame.0.point([0.0, 0.0, 0.3]);
                b.glass(Mesh::Cone8, T::new().t(p[0], p[1] + 3.4, p[2]).rx(PI).s(1.5, 3.4, 1.5), [255, 220, 120, 22], 0);
                lights.push(Light { floor: d.floor, p: [p[0], p[1] + 2.2, p[2], 4.5], c: [0.9, 0.75, 0.35, 1.0], owner: 0 });
            }
            if d.st == St::Stuck {
                let p = d.frame.0.point([0.0, d.top + 0.5, -0.2]);
                let k = 0.5 + 0.5 * (t * 2.5 + ph).sin();
                lights.push(Light { floor: d.floor, p: [p[0], p[1], p[2], 2.6], c: [0.9 * k, 0.5 * k, 0.1 * k, 1.0], owner: 0 });
            }
            if d.st == St::Error {
                for k in 0..4 {
                    let a = ((t * 0.6 + k as f32 * 0.25 + ph) % 1.0).max(0.0);
                    let p = d.frame.0.point([0.1 * (k as f32 - 1.5), d.top + 0.3 + a * 1.4, -0.25]);
                    b.put(Mesh::BallLo, T::new().t(p[0], p[1], p[2]).s(0.18 + a * 0.3, 0.18 + a * 0.3, 0.18 + a * 0.3), mix(hex("#5a5560"), hex("#9a96a0"), a));
                }
                let p = d.frame.0.point([0.0, d.top + 0.4, -0.2]);
                let k = 0.6 + 0.4 * (t * 9.0).sin();
                lights.push(Light { floor: d.floor, p: [p[0], p[1], p[2], 2.8], c: [0.9 * k, 0.12, 0.1, 1.0], owner: 0 });
            }
            // helpers stand round the desk, a few at most; the rest is a number
            if d.helpers > 0 {
                let mut hb = B { out: &mut *out, floor: d.floor, id: d.helper_id, dim: 1.0 };
                let shown = d.helpers.min(HELPERS_SHOWN);
                for k in 0..shown {
                    let a = -1.1 + k as f32 * (2.2 / (shown.max(2) - 1) as f32);
                    let r = 1.05 + 0.15 * (k % 2) as f32;
                    let at = d.frame.t(r * a.sin(), 0.0, 0.35 + r * a.cos() * 0.8).ry(a + PI);
                    let mut lr = Rng(hash(&format!("h{}{}{}", d.app, d.sess, k)));
                    let mut lk = Look::random(&mut lr, d.accent);
                    lk.height *= 0.9;
                    lk.top = mix(d.accent, hex("#f4efe6"), 0.45);
                    lk.top_style = 3;
                    let hh = person(&mut hb, at, &lk, Pose::Helper, t * 1.1 + k as f32, 0.0, 0.0);
                    if k == 0 && d.helpers > HELPERS_SHOWN { marks.push(Mark::Plus { at: [hh[0], hh[1] + 0.1, hh[2]], n: d.helpers - HELPERS_SHOWN }); }
                }
            }
            moving |= matches!(d.st, St::Working | St::Asks | St::Stuck | St::Error | St::Done | St::Limit | St::Idle);
        }
        // fires on overloaded Apps (never on people)
        for fr in self.fires.iter().filter(|f| on(f.floor)) {
            let mut b = B { out: &mut *out, floor: fr.floor, id: 0, dim: 1.0 };
            for k in 0..5 {
                let fl = (t * 7.0 + k as f32 * 1.7).sin() * 0.5 + 0.5;
                let s = 0.14 + 0.08 * fl - k as f32 * 0.015;
                b.glow(Mesh::Cone8, fr.at.t(0.07 * (k as f32 - 2.0), 0.0, 0.04 * ((k * 3) % 5) as f32 - 0.08).s(s, 0.28 + 0.2 * fl, s), if k % 2 == 0 { hex("#ff8a2a") } else { hex("#ffd24a") }, 1.0);
            }
            for k in 0..3 {
                let a = (t * 0.4 + k as f32 * 0.33) % 1.0;
                let p = fr.at.0.point([0.0, 0.5 + a * 1.6, 0.0]);
                b.put(Mesh::BallLo, T::new().t(p[0] + a * 0.3, p[1], p[2]).s(0.25 + a * 0.4, 0.25 + a * 0.4, 0.25 + a * 0.4), mix(hex("#4a4650"), hex("#8a8690"), a));
            }
            let p = fr.at.0.point([0.0, 0.4, 0.0]);
            let k = 0.75 + 0.25 * (t * 11.0).sin();
            lights.push(Light { floor: fr.floor, p: [p[0], p[1], p[2], 4.0], c: [1.0 * k, 0.45 * k, 0.1, 1.0], owner: 0 });
            moving = true;
        }
        // the props you can play with
        for (pi, p) in self.props.iter().enumerate().filter(|(_, p)| on(p.floor)) {
            let since = live.clicked.get(&p.id).map(|c| t - c).filter(|s| *s >= 0.0 && *s < 4.0);
            let mut b = B { out: &mut *out, floor: p.floor, id: p.id, dim: 1.0 };
            match p.kind {
                PropKind::Plant { kind, pot, s, seed, wilt } => {
                    let sway = since.map(|a| 0.35 * (a * 14.0).sin() * (1.0 - a / 1.6).max(0.0)).unwrap_or(0.0);
                    plant(&mut b, p.t, kind, pot, s, seed, sway, wilt);
                }
                PropKind::Beanbag(c) => {
                    let sq = since.map(|a| 1.0 - 0.35 * (1.0 - a / 1.2).max(0.0) * (a * 10.0).cos().abs()).unwrap_or(1.0);
                    b.put(Mesh::Ball, p.t.s(0.85, 0.55 * sq, 0.8), c);
                    b.put(Mesh::Ball, p.t.t(0.0, 0.32 * sq, 0.18).s(0.6, 0.3 * sq, 0.4), ramp(c, 0.1));
                }
                PropKind::Coffee => {
                    b.cb(p.t, [-0.2, 0.0, -0.2], [0.4, 0.5, 0.36], hex("#2a2b30"));
                    b.bx(p.t, [-0.12, 0.08, 0.12], [0.24, 0.06, 0.1], hex("#7a7c84"));
                    b.glow(Mesh::Box, p.t.t(0.1, 0.38, 0.17).s(0.04, 0.04, 0.02), hex("#5fd38d"), 1.0);
                    b.put(Mesh::Cyl8, p.t.t(0.0, 0.14, 0.17).s(0.08, 0.1, 0.08), hex("#f4efe6"));
                    if let Some(a) = since {
                        for k in 0..4 {
                            let u = ((a * 0.8 + k as f32 * 0.25) % 1.0).max(0.0);
                            b.put(Mesh::BallLo, p.t.t(0.03 * (u * 9.0).sin(), 0.26 + u * 0.6, 0.17).s(0.06 + u * 0.08, 0.06 + u * 0.08, 0.06 + u * 0.08), [235, 235, 240, 255]);
                        }
                        moving = true;
                    }
                }
                PropKind::PingPong => {
                    let t0 = p.t;
                    b.bx(t0, [-1.37, 0.72, -0.76], [2.74, 0.04, 1.52], hex("#2f6a4a"));
                    b.bx(t0, [-0.01, 0.76, -0.76], [0.02, 0.006, 1.52], hex("#f4efe6"));
                    b.bx(t0, [-1.37, 0.76, -0.01], [2.74, 0.006, 0.02], hex("#f4efe6"));
                    b.bx(t0, [-0.01, 0.76, -0.8], [0.02, 0.15, 1.6], hex("#e6e1d8"));
                    for (x, z) in [(-1.2, -0.6), (1.2, -0.6), (-1.2, 0.6), (1.2, 0.6)] { b.bx(t0, [x - 0.03, 0.0, z - 0.03], [0.06, 0.72, 0.06], hex("#2a2b30")); }
                    for s in [-1.0f32, 1.0] { b.put(Mesh::Cyl12, t0.t(s * 0.9, 0.77, s * 0.4).s(0.16, 0.012, 0.16), hex("#c84a3a")); }
                    if let Some(a) = since {
                        let u = (a * 1.4) % 2.0;
                        let x = if u < 1.0 { -1.2 + 2.4 * u } else { 1.2 - 2.4 * (u - 1.0) };
                        let y = 0.78 + 0.35 * (u.fract() * PI).sin();
                        b.glow(Mesh::Ball, t0.t(x, y, 0.1 * (a * 3.0).sin()).s(0.05, 0.05, 0.05), hex("#ffffff"), 0.5);
                        moving = true;
                    }
                }
                PropKind::Arcade => {
                    b.cb(p.t, [-0.35, 0.0, -0.3], [0.7, 1.75, 0.6], hex("#3a2a6a"));
                    b.bx(p.t, [-0.36, 1.45, -0.31], [0.72, 0.3, 0.62], hex("#ff5aa8"));
                    let c = if since.is_some() { mix(hex("#5aff9a"), hex("#ffe27a"), ((t * 6.0).sin() + 1.0) / 2.0) } else { hex("#3a5aff") };
                    b.glow(Mesh::Box, p.t.t(0.0, 1.0, 0.301).rx(-0.3).s(0.5, 0.38, 0.01), c, 1.0);
                    b.bx(p.t, [-0.32, 0.9, 0.22], [0.64, 0.05, 0.2], hex("#1d1a24"));
                    if since.is_some() { moving = true; }
                }
            }
            if since.is_some() { moving = true; }
            let _ = pi;
        }
        // the usage board's gauges and each area's battery for its Account
        for (fl, at, ids) in self.boards.iter().filter(|(f, ..)| on(*f)) {
            for (i, a) in self.accounts.iter().enumerate() {
                let mut b = B { out: &mut *out, floor: *fl, id: ids[i], dim: 1.0 };
                let c = gauge_colour(a.left, t);
                let y = 0.32 + 0.11 * (self.accounts.len() - i) as f32 - 0.2;
                b.put(Mesh::Box, at.t(0.65, y, 0.0).s(0.6, 0.06, 0.01), hex("#3a3848"));
                b.glow(Mesh::Box, at.t(0.35 + 0.3 * a.left.max(0.03), y + 0.005, 0.005).s(0.6 * a.left.max(0.03), 0.05, 0.01), c, 0.9);
            }
        }
        for s in self.signs.iter().filter(|s| on(s.floor)) {
            let a = &self.accounts[s.account];
            let mut b = B { out: &mut *out, floor: s.floor, id: 0, dim: 1.0 };
            let c = gauge_colour(a.left, t);
            let segs = (a.left * 4.0).ceil() as usize;
            for k in 0..4 {
                let lit = k < segs;
                b.glow(Mesh::Box, s.at.t(-0.17 + k as f32 * 0.115, -0.02, 0.0).s(0.1, 0.06, 0.06), if lit { c } else { hex("#3a3848") }, if lit { 0.9 } else { 0.0 });
            }
            if a.left < 0.2 { moving = true; }
        }
        // This Mac's rack: lights rise with the load
        for (fl, at) in self.racks.iter().filter(|(f, _)| on(*f)) {
            let mut b = B { out: &mut *out, floor: *fl, id: 0, dim: 1.0 };
            let lit = ((live.mac_load + 0.04 * (t * 3.0).sin()) * 10.0).round() as usize;
            for k in 0..10 {
                let c = if k < lit { if k >= 8 { hex("#ef5350") } else if k >= 6 { hex("#f0a640") } else { hex("#5fd38d") } } else { hex("#2a2c34") };
                b.glow(Mesh::Box, at.t(0.0, 0.25 + k as f32 * 0.16, 0.0).s(0.44, 0.08, 0.01), c, if k < lit { 1.0 } else { 0.0 });
            }
            moving = true;
        }
        // red alert: beacons turn at the lift and over every area
        if live.alert {
            for (fl, p) in self.beacons.iter().filter(|(f, _)| on(*f)) {
                let mut b = B { out: &mut *out, floor: *fl, id: 0, dim: 1.0 };
                b.put(Mesh::Cyl8, T::new().t(p[0], p[1], p[2]).s(0.18, 0.08, 0.18), hex("#2a2733"));
                b.glow(Mesh::Ball, T::new().t(p[0], p[1] + 0.06, p[2]).s(0.16, 0.18, 0.16), hex("#ff3b30"), 1.0);
                let a = t * 5.0;
                lights.push(Light { floor: *fl, p: [p[0] + 1.2 * a.cos(), p[1] - 0.4, p[2] + 1.2 * a.sin(), 7.0], c: [1.2, 0.08, 0.06, 1.0], owner: 0 });
            }
            moving = true;
        }
        // people walking the aisle, and the cat
        for f in floors {
            let fy = floor_y(*f);
            for k in 0..2 {
                let mut r = Rng(hash(&format!("walker{f}{k}")));
                let lk = Look::random(&mut r, hex("#c9c3d6"));
                let len = self.width - LOBBY_W - 2.0;
                let u = ((t * 0.9 / len) + k as f32 * 0.5 + *f as f32 * 0.21).fract() * 2.0;
                let (x, dir) = if u < 1.0 { (LOBBY_W + 1.0 + u * len, -FRAC_PI_2) } else { (LOBBY_W + 1.0 + (2.0 - u) * len, FRAC_PI_2) };
                let z = ROW_D + AISLE * (0.35 + 0.3 * k as f32);
                let mut b = B { out: &mut *out, floor: *f as i32, id: 0, dim: 1.0 };
                person(&mut b, T::new().t(x, fy, z).ry(dir), &lk, Pose::Walk, t * 1.3 + k as f32, 0.0, 0.0);
            }
            moving = true;
        }
        if floors.contains(&0) {
            let cat_id = self.picks.iter().position(|p| p.target == Target::Cat).map(|i| i as u32 + 1).unwrap_or(0);
            let sit = live.clicked.get(&cat_id).map(|c| t - c < 5.0).unwrap_or(false);
            let u = if sit { live.clicked[&cat_id] } else { t } * 0.12;
            let (cx, cz) = (4.0 + 2.2 * (u * TAU).cos(), 13.5 + 1.8 * (u * TAU).sin());
            let mut b = B { out: &mut *out, floor: 0, id: cat_id, dim: 1.0 };
            models::cat(&mut b, T::new().t(cx, 0.0, cz).ry(-(u * TAU) + PI), t * 9.0, sit);
            if sit { marks.push(Mark::Balloon { at: [cx, 0.75, cz], icon: "heart" }); }
        }
        moving
    }

    /// Adds the cat's pick (kept apart so a scene rebuild keeps ids stable for everything else).
    pub fn add_cat(&mut self) { self.pick(Target::Cat, "The office cat", "pet it"); }

    /// Where a desk's sitter is, for a pick id (to point at it).
    pub fn anchor(&self, id: u32) -> Option<V3> { self.desks.iter().find(|d| d.id == id).map(|d| d.frame.0.point([0.0, 1.2, 0.62])) }

    /// Counts per area for its chip: asks, stuck, crashed.
    pub fn counts(&self, ai: usize) -> (usize, usize, usize) {
        let s = &self.apps[ai].sessions;
        let out = self.accounts[self.apps[ai].account].left <= 0.0;
        if out { return (0, 0, 0); }
        (s.iter().filter(|x| x.st == St::Asks).count(), s.iter().filter(|x| x.st == St::Stuck).count(), s.iter().filter(|x| x.st == St::Error).count())
    }
}

fn gauge_colour(left: f32, t: f32) -> Rgba {
    if left <= 0.0 { hex("#ef5350") }
    else if left < 0.2 { if (t * 2.0).sin() > 0.0 { hex("#f0a640") } else { hex("#7a5a2a") } }
    else if left < 0.4 { hex("#e8c84a") } else { hex("#5fd38d") }
}

/// A run of the back wall from x0 to x1, with a window every 2.4 m.
fn wall_run(b: &mut B, t: T, x0: f32, x1: f32, tex: [f32; 4], windows: bool) {
    let piece = |b: &mut B, a: f32, c: f32, y0: f32, y1: f32| {
        if c - a < 0.01 { return; }
        b.raw(Mesh::QuadWall, t.t((a + c) / 2.0, y0, 0.0).s(c - a, y1 - y0, 1.0), [255; 4], tex, [(c - a) / 4.8, (y1 - y0) / 3.0], 0.0, CUTAWAY | models::TEXTURED, false);
        if let Some(last) = b.out.last_mut() { last.inst.off = [a / 4.8, (WALL_H - y1) / 3.0]; }
    };
    let (sill, lintel) = (0.95, 2.35);
    piece(b, x0, x1, 0.0, sill);
    piece(b, x0, x1, lintel, WALL_H);
    if !windows { piece(b, x0, x1, sill, lintel); return; }
    let mut x = x0;
    while x < x1 - 0.01 {
        let wx0 = x + 0.5;
        let wx1 = (x + 2.0).min(x1 - 0.4);
        if wx1 - wx0 < 0.6 { piece(b, x, x1, sill, lintel); break; }
        piece(b, x, wx0, sill, lintel);
        b.glass(Mesh::QuadWall, t.t((wx0 + wx1) / 2.0, sill, 0.0).s(wx1 - wx0, lintel - sill, 1.0), [150, 170, 220, 45], CUTAWAY);
        b.raw(Mesh::Box, t.t((wx0 + wx1) / 2.0, sill - 0.04, 0.03).s(wx1 - wx0 + 0.1, 0.05, 0.12), hex("#e6e1d8"), [0.0; 4], [1.0, 1.0], 0.0, CUTAWAY, false);
        b.raw(Mesh::Box, t.t((wx0 + wx1) / 2.0, sill, 0.0).s(0.04, lintel - sill, 0.05), hex("#3a3848"), [0.0; 4], [1.0, 1.0], 0.0, CUTAWAY, false);
        let next = (x + 2.4).min(x1);
        piece(b, wx1, next, sill, lintel);
        x = next;
    }
}

/// The edge between an area and its left neighbour (or the lobby), in the area's own style.
fn divider(b: &mut B, kit: &Kit, x: f32, fy: f32, z0: f32, z1: f32, r: &mut Rng, atlas: &mut Atlas) {
    if z1 - z0 < 0.5 { return; }
    let len = z1 - z0;
    let t = T::new().t(x, fy, (z0 + z1) / 2.0).ry(FRAC_PI_2);
    match kit.divider {
        Divider::Planters => {
            let n = (len / 1.0).floor().max(1.0) as usize;
            for k in 0..n {
                let p = t.t(-len / 2.0 + 0.5 + k as f32 * (len - 1.0) / (n.max(2) - 1) as f32, 0.0, 0.0);
                b.bx(p, [-0.45, 0.0, -0.22], [0.9, 0.42, 0.44], match kit.pot { Pot::Box(c) | Pot::Ceramic(c) => c, _ => hex("#6a6a68") });
                plant(b, p, if k % 2 == 0 { Plant::Snake } else { Plant::Fern }, Pot::Box(hex("#3a3a3a")), 0.75, r.int(1000) as u32, 0.0, 0.0);
            }
        }
        Divider::Shelves => {
            let books = atlas.add("divbooks", &tex::books_img(91));
            let n = (len / 1.2).floor().max(1.0) as usize;
            for k in 0..n {
                let p = t.t(-len / 2.0 + 0.6 + k as f32 * 1.2, 0.0, 0.0);
                b.put(Mesh::Box, p.s(1.15, 1.15, 0.36), hex("#6b4a32"));
                for s in 0..2 { for side in [0.0f32, PI] { b.tex(Mesh::QuadWall, p.ry(side).t(0.0, 0.1 + s as f32 * 0.52, 0.181).s(1.05, 0.42, 1.0), [255; 4], books, [1.0, 1.0], 0.0, 0); } }
            }
        }
        Divider::Glass => {
            let n = (len / 1.4).ceil() as usize;
            for k in 0..=n { b.put(Mesh::Box, t.t(-len / 2.0 + k as f32 * len / n as f32, 0.0, 0.0).s(0.05, 2.0, 0.05), hex("#9aa0a8")); }
            b.put(Mesh::Box, t.t(0.0, 1.98, 0.0).s(len, 0.04, 0.06), hex("#9aa0a8"));
            b.put(Mesh::Box, t.t(0.0, 0.0, 0.0).s(len, 0.08, 0.06), hex("#9aa0a8"));
            b.glass(Mesh::QuadWall, t.t(0.0, 0.08, 0.0).s(len, 1.9, 1.0), [200, 220, 235, 50], 0);
            b.glass(Mesh::QuadWall, t.t(0.0, 0.08, 0.0).ry(PI).s(len, 1.9, 1.0), [200, 220, 235, 50], 0);
            b.put(Mesh::Box, t.t(0.0, 1.0, 0.0).s(len, 0.12, 0.02), kit.accent);
        }
        Divider::LowWall => {
            b.put(Mesh::Box, t.s(len, 1.1, 0.2), match kit.wall { crate::kit::WallFinish::Brick { c, .. } => c, crate::kit::WallFinish::Tiles { c, .. } => c, _ => hex("#8a8f96") });
            b.put(Mesh::Box, t.t(0.0, 1.1, 0.0).s(len + 0.04, 0.06, 0.26), hex("#3a3d43"));
        }
        Divider::Lockers => {
            let n = (len / 0.42).floor() as usize;
            for k in 0..n {
                let p = t.t(-len / 2.0 + 0.21 + k as f32 * 0.42, 0.0, 0.0);
                let c = if k % 5 == 2 { kit.accent } else { hex(["#5a6a7a", "#6a7a8a"][k % 2]) };
                b.put(Mesh::Box, p.s(0.4, 1.7, 0.45), c);
                for side in [-1.0f32, 1.0] { b.put(Mesh::Box, p.t(0.12, 1.0, side * 0.23).s(0.03, 0.12, 0.02), hex("#c8c8cc")); }
            }
        }
        Divider::Pegboard => {
            let peg = atlas.add("peg", &tex::pegboard_img());
            let n = (len / 1.6).floor().max(1.0) as usize;
            for k in 0..n {
                let p = t.t(-len / 2.0 + 0.8 + k as f32 * 1.6, 0.0, 0.0);
                for s in [-0.7f32, 0.7] { b.put(Mesh::Box, p.t(s, 0.0, 0.0).s(0.05, 1.6, 0.3), hex("#3a3a3e")); }
                b.tex(Mesh::Box, p.t(0.0, 0.3, 0.0).s(1.4, 1.2, 0.03), [255; 4], peg, [2.0, 1.6], 0.0, 0);
                for j in 0..4 { b.put(Mesh::Box, p.t(-0.5 + j as f32 * 0.33, 0.9, 0.04).s(0.05, 0.28, 0.03), hex(["#c84a3a", "#3a8fb0", "#e0a03a", "#2a2b30"][j])); }
            }
        }
    }
}
