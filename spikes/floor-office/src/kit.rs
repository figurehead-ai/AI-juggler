//! Style kits: how one App's area of the office looks.
//!
//! The App's tool picks a design family, so every Codex App, say, reads as industrial. A persisted
//! choice picks the variant inside the family: the layout type and a seed for everything else
//! (floor, wall, desk model, chairs, plants, divider, nook, sign). The choice is made once, the
//! first time the Floor sees the App, and kept, so an area never changes its look by itself.
//! Big things vary between kits (palette, floor, wall, furniture family, layout); small things
//! (props, wear, people) vary per desk in the scene builder. The App's colour is its mark colour
//! (MARK_COLOURS in ui/src/lib/mark-hues.ts) and appears on its sign, rug and team badges.

use crate::paint::{hash, hex, oklch, Rgba, Rng};

/// AI Juggler's App colours, slot by slot (OKLCH from mark-hues.ts).
pub const MARK_COLOURS: [(f32, f32, f32); 8] = [
    (0.627, 0.16, 255.0),
    (0.549, 0.143, 130.0),
    (0.629, 0.16, 345.0),
    (0.562, 0.16, 320.0),
    (0.571, 0.16, 280.0),
    (0.619, 0.136, 115.0),
    (0.645, 0.16, 325.0),
    (0.564, 0.16, 345.0),
];
pub fn mark_colour(slot: usize) -> Rgba {
    let (l, c, h) = MARK_COLOURS[slot % MARK_COLOURS.len()];
    oklch(l, c, h)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Tool { ClaudeCode, ClaudeApp, Codex, OpenCode, Gemini, Cursor }
impl Tool {
    /// A two-letter monogram for the sign. The real build shows the tool's own icon, read at run time.
    pub fn monogram(self) -> &'static str {
        match self { Tool::ClaudeCode => "CC", Tool::ClaudeApp => "CA", Tool::Codex => "CX", Tool::OpenCode => "OC", Tool::Gemini => "GM", Tool::Cursor => "CU" }
    }
    pub fn family(self) -> &'static str {
        match self { Tool::ClaudeCode => "Workshop", Tool::ClaudeApp => "Studio", Tool::Codex => "Industrial", Tool::OpenCode => "Maker garage", Tool::Gemini => "Campus", Tool::Cursor => "Lab" }
    }
    /// The layout types this family uses; the App's persisted choice is one of them.
    pub fn layouts(self) -> &'static [Layout] {
        match self {
            Tool::ClaudeCode => &[Layout::Pods, Layout::Pinwheel, Layout::Scatter],
            Tool::ClaudeApp => &[Layout::Tables, Layout::Scatter, Layout::Radial],
            Tool::Codex => &[Layout::Bench, Layout::Diagonal, Layout::Pods],
            Tool::OpenCode => &[Layout::Scatter, Layout::Pinwheel],
            Tool::Gemini => &[Layout::Pinwheel, Layout::Radial, Layout::Tables],
            Tool::Cursor => &[Layout::Pods, Layout::Diagonal],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum FloorMat {
    Planks { wood: Rgba, width: f32 },
    Parquet { wood: Rgba, block: f32 },
    Carpet { a: Rgba, b: Rgba, tile: f32 },
    Concrete { tone: Rgba, seam: f32 },
    Terrazzo { base: Rgba, chips: [Rgba; 3] },
    Rubber { c: Rgba },
}
#[derive(Clone, Copy, Debug)]
pub enum WallFinish {
    Paint { c: Rgba, dado: Rgba },
    Brick { c: Rgba, mortar: Rgba },
    Slats { wood: Rgba, back: Rgba },
    Tiles { c: Rgba, grout: Rgba },
    Stripes { a: Rgba, b: Rgba },
    Acoustic { a: Rgba, b: Rgba },
}
/// How desks are arranged in the area.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Layout {
    /// Clusters of four facing desks.
    Pods,
    /// Long rows of desks, two facing rows per bench.
    Bench,
    /// Three desks at 120° around a shared centre, staggered.
    Pinwheel,
    /// Freestanding desks at loose angles, placed with clearance.
    Scatter,
    /// Long shared tables with laptops both sides.
    Tables,
    /// Rows turned 45° to the room.
    Diagonal,
    /// Desks around a central planter, facing out.
    Radial,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeskKind { FourLeg, Trestle, Pedestal, Standing, LShape, Round }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChairKind { Task, Mesh, Stool, Saddle, Ball }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screens { Single, Dual, Laptop, Wide }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Plant { Monstera, Fern, Snake, Cactus, Fig, Palm, Herbs, Succulent }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pot { Terracotta, Ceramic(Rgba), Box(Rgba), Basket, Concrete }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Divider { Planters, Shelves, Glass, LowWall, Lockers, Pegboard }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Nook { Sofa, Beanbags, Huddle, Library, Workbench, PingPong, Arcade }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SignKind { Plate, Neon, Banner, Pennant, Blade }

/// What the Floor keeps per App, so its look survives restarts and new kit versions.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Choice { pub layout: Layout, pub seed: u32 }

/// The first-time choice for an App, from the office seed (changed by Re-create) and the App's id.
pub fn choose(tool: Tool, app_id: &str, office_seed: u32) -> Choice {
    let mut r = Rng::of(&format!("choice|{office_seed}|{tool:?}|{app_id}"));
    Choice { layout: *r.pick(tool.layouts()), seed: hash(&format!("seed|{office_seed}|{app_id}")) }
}

#[derive(Clone, Debug)]
pub struct Kit {
    pub tool: Tool,
    pub family: &'static str,
    pub accent: Rgba,
    pub layout: Layout,
    /// Up to this much random turn per desk group, in radians: 0 for strict families.
    pub jitter: f32,
    pub floor: FloorMat,
    pub wall: WallFinish,
    pub desk: DeskKind,
    pub desk_top: Rgba,
    pub desk_frame: Rgba,
    pub chair: ChairKind,
    pub chair_c: Rgba,
    pub screens: Screens,
    pub plants: Vec<Plant>,
    pub pot: Pot,
    pub greenery: usize,
    pub divider: Divider,
    pub nook: Nook,
    pub warmth: f32,
    pub sign: SignKind,
    pub rug: Rgba,
    pub lamps: bool,
    pub pendants: bool,
}

impl Kit {
    pub fn variant(&self) -> String {
        let f = match self.floor { FloorMat::Planks { .. } => "planks", FloorMat::Parquet { .. } => "parquet", FloorMat::Carpet { .. } => "carpet", FloorMat::Concrete { .. } => "concrete", FloorMat::Terrazzo { .. } => "terrazzo", FloorMat::Rubber { .. } => "rubber" };
        let w = match self.wall { WallFinish::Paint { .. } => "paint", WallFinish::Brick { .. } => "brick", WallFinish::Slats { .. } => "slats", WallFinish::Tiles { .. } => "tiles", WallFinish::Stripes { .. } => "stripes", WallFinish::Acoustic { .. } => "acoustic" };
        format!("{:?} layout, {f}, {w}, {:?} desks, {:?} chairs, {:?} divider, {:?} nook", self.layout, self.desk, self.chair, self.divider, self.nook)
    }
}

/// The kit for an App: family from its tool, everything else from its persisted choice.
pub fn kit_for(tool: Tool, choice: Choice, slot: usize) -> Kit {
    let mut r = Rng(choice.seed | 1);
    let accent = mark_colour(slot);
    let pick = |r: &mut Rng, xs: &[&str]| hex(r.pick(xs));
    let layout = choice.layout;
    match tool {
        Tool::ClaudeCode => {
            // Workshop: warm timber, terracotta and brick, craft lamps, a reading nook, lots of green.
            let wood = pick(&mut r, &["#b07a4a", "#9a6238", "#c08b57", "#7d5434"]);
            let floor = if r.chance(0.5) { FloorMat::Parquet { wood, block: 0.6 } } else { FloorMat::Planks { wood, width: 0.24 } };
            let wall = match r.int(3) {
                0 => WallFinish::Brick { c: pick(&mut r, &["#a4543a", "#b5654a", "#8f4a35"]), mortar: hex("#d8c7b4") },
                1 => WallFinish::Paint { c: pick(&mut r, &["#c86f4f", "#8a9a6e", "#c9925c"]), dado: hex("#6b4a32") },
                _ => WallFinish::Slats { wood: pick(&mut r, &["#8a5a36", "#a06a40"]), back: hex("#3b2a22") },
            };
            Kit { tool, family: tool.family(), accent, layout, jitter: 0.12, floor, wall,
                desk: *r.pick(&[DeskKind::Trestle, DeskKind::FourLeg, DeskKind::Pedestal]), desk_top: pick(&mut r, &["#c99764", "#b5824f", "#d4a873"]), desk_frame: hex("#4a3324"),
                chair: ChairKind::Task, chair_c: pick(&mut r, &["#a3502f", "#5c6b3b", "#2f4a6b", "#7a3b4a"]),
                screens: *r.pick(&[Screens::Single, Screens::Dual]),
                plants: vec![Plant::Monstera, Plant::Fern, Plant::Fig], pot: *r.pick(&[Pot::Terracotta, Pot::Basket]), greenery: 4 + r.int(3),
                divider: *r.pick(&[Divider::Shelves, Divider::Planters]), nook: *r.pick(&[Nook::Library, Nook::Sofa]),
                warmth: 0.9, sign: *r.pick(&[SignKind::Plate, SignKind::Banner]), rug: pick(&mut r, &["#7a3b2e", "#4a5a3a", "#3a4a6a"]), lamps: true, pendants: r.chance(0.5) }
        }
        Tool::ClaudeApp => {
            // Studio: pale oak, soft pastel walls, white tables, ceramics, posters.
            let floor = if r.chance(0.6) { FloorMat::Planks { wood: pick(&mut r, &["#e1c9a2", "#d8bd94", "#e8d3b0"]), width: 0.24 } } else { FloorMat::Carpet { a: hex("#c9c3d6"), b: hex("#bdb6cc"), tile: 0.5 } };
            let wall = if r.chance(0.6) { WallFinish::Paint { c: pick(&mut r, &["#e8c4c0", "#c3dccd", "#c4d4e8", "#e6dcc0"]), dado: hex("#f4efe6") } } else { WallFinish::Stripes { a: hex("#f1e6d6"), b: pick(&mut r, &["#e8b9b0", "#b9d8c6", "#b8c8e6"]) } };
            Kit { tool, family: tool.family(), accent, layout, jitter: 0.08, floor, wall,
                desk: *r.pick(&[DeskKind::FourLeg, DeskKind::Round]), desk_top: hex("#f2f0ea"), desk_frame: hex("#c9c5bb"),
                chair: *r.pick(&[ChairKind::Stool, ChairKind::Task]), chair_c: pick(&mut r, &["#e6a99a", "#9cc7b1", "#a8b9de", "#e8cf8e"]),
                screens: Screens::Laptop,
                plants: vec![Plant::Fig, Plant::Succulent, Plant::Monstera], pot: Pot::Ceramic(pick(&mut r, &["#f4efe6", "#e9c9b8", "#cfe0d6"])), greenery: 3 + r.int(2),
                divider: *r.pick(&[Divider::Glass, Divider::Planters]), nook: *r.pick(&[Nook::Sofa, Nook::Huddle]),
                warmth: 0.65, sign: SignKind::Banner, rug: pick(&mut r, &["#e9d7c4", "#d6e3dc", "#e3d6e6"]), lamps: false, pendants: true }
        }
        Tool::Codex => {
            // Industrial: polished concrete, brick or concrete panels, black steel, Edison pendants.
            let floor = FloorMat::Concrete { tone: pick(&mut r, &["#8d8f94", "#7a7c82", "#9a948a"]), seam: if r.chance(0.5) { 3.0 } else { 2.0 } };
            let wall = if r.chance(0.55) { WallFinish::Brick { c: pick(&mut r, &["#7e3b2f", "#d9d4cc", "#6b6560"]), mortar: hex("#4a4440") } } else { WallFinish::Tiles { c: pick(&mut r, &["#8a8f96", "#a59f95"]), grout: hex("#5a5e64") } };
            Kit { tool, family: tool.family(), accent, layout, jitter: 0.0, floor, wall,
                desk: *r.pick(&[DeskKind::FourLeg, DeskKind::Pedestal]), desk_top: pick(&mut r, &["#2d2f34", "#6b4c33", "#3a3d43"]), desk_frame: hex("#1c1d21"),
                chair: ChairKind::Mesh, chair_c: hex("#25262b"),
                screens: *r.pick(&[Screens::Dual, Screens::Wide]),
                plants: vec![Plant::Snake, Plant::Cactus], pot: Pot::Concrete, greenery: 2 + r.int(2),
                divider: *r.pick(&[Divider::Lockers, Divider::LowWall]), nook: *r.pick(&[Nook::Huddle, Nook::Arcade]),
                warmth: 0.55, sign: *r.pick(&[SignKind::Neon, SignKind::Blade]), rug: hex("#3a3d43"), lamps: false, pendants: true }
        }
        Tool::OpenCode => {
            // Maker garage: rubber floor, plywood and pegboard, standing desks, bikes, neon.
            let floor = FloorMat::Rubber { c: pick(&mut r, &["#3f5a4a", "#3d4a5c", "#55524c"]) };
            let wall = if r.chance(0.6) { WallFinish::Slats { wood: hex("#c9a46c"), back: hex("#6b5638") } } else { WallFinish::Paint { c: pick(&mut r, &["#2f4a3f", "#3a3f55"]), dado: hex("#c9a46c") } };
            Kit { tool, family: tool.family(), accent, layout, jitter: 0.3, floor, wall,
                desk: DeskKind::Standing, desk_top: hex("#d2b07a"), desk_frame: hex("#3a3a3e"),
                chair: *r.pick(&[ChairKind::Stool, ChairKind::Saddle]), chair_c: pick(&mut r, &["#e0a03a", "#3a8fb0"]),
                screens: *r.pick(&[Screens::Laptop, Screens::Dual]),
                plants: vec![Plant::Herbs, Plant::Cactus, Plant::Snake], pot: Pot::Box(hex("#c9a46c")), greenery: 2 + r.int(2),
                divider: Divider::Pegboard, nook: *r.pick(&[Nook::Workbench, Nook::PingPong]),
                warmth: 0.6, sign: SignKind::Neon, rug: hex("#2f3a44"), lamps: true, pendants: false }
        }
        Tool::Gemini => {
            // Campus: terrazzo or bold carpet, striped supergraphics, beanbags, bright chairs.
            let floor = if r.chance(0.5) { FloorMat::Terrazzo { base: hex("#e7e1d6"), chips: [hex("#d9644a"), hex("#3f7fbf"), hex("#e8b33a")] } } else { FloorMat::Carpet { a: hex("#5a7fb0"), b: hex("#4f72a2"), tile: 0.5 } };
            let wall = if r.chance(0.5) { WallFinish::Stripes { a: hex("#f0e8da"), b: pick(&mut r, &["#e8b33a", "#d9644a", "#3f7fbf"]) } } else { WallFinish::Acoustic { a: hex("#3f7fbf"), b: hex("#e8b33a") } };
            Kit { tool, family: tool.family(), accent, layout, jitter: 0.2, floor, wall,
                desk: *r.pick(&[DeskKind::Round, DeskKind::FourLeg]), desk_top: hex("#ece6da"), desk_frame: hex("#4a4a52"),
                chair: *r.pick(&[ChairKind::Ball, ChairKind::Task]), chair_c: pick(&mut r, &["#d9644a", "#3f7fbf", "#e8b33a", "#4aa37a"]),
                screens: Screens::Single,
                plants: vec![Plant::Palm, Plant::Monstera], pot: Pot::Ceramic(pick(&mut r, &["#e8b33a", "#3f7fbf"])), greenery: 3,
                divider: Divider::Planters, nook: *r.pick(&[Nook::Beanbags, Nook::PingPong]),
                warmth: 0.7, sign: SignKind::Pennant, rug: hex("#e8b33a"), lamps: false, pendants: false }
        }
        Tool::Cursor => {
            // Lab: dark carpet, acoustic panels, L-shaped desks, a calm cool light.
            let floor = FloorMat::Carpet { a: hex("#3a3f4a"), b: hex("#353a44"), tile: 0.5 };
            let wall = WallFinish::Acoustic { a: hex("#2f3440"), b: pick(&mut r, &["#3a404d", "#344a4f"]) };
            Kit { tool, family: tool.family(), accent, layout, jitter: 0.0, floor, wall,
                desk: DeskKind::LShape, desk_top: hex("#2a2d33"), desk_frame: hex("#16171a"),
                chair: ChairKind::Mesh, chair_c: hex("#1f2024"),
                screens: Screens::Dual,
                plants: vec![Plant::Snake, Plant::Succulent], pot: Pot::Concrete, greenery: 2,
                divider: Divider::Glass, nook: Nook::Huddle,
                warmth: 0.3, sign: SignKind::Blade, rug: hex("#2a2d33"), lamps: true, pendants: false }
        }
    }
}
