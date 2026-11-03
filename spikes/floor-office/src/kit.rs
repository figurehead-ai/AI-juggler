//! Style kits: how one App's area of the office looks.
//!
//! A kit is chosen deterministically. The App's tool picks a design family (so every Codex App, say,
//! reads as industrial), and a seed from the App's id picks the variant inside the family (floor
//! pattern, wall finish, layout, decor, plants), so two Apps of one tool look related but never the
//! same. Big things vary between kits (palette, floor, wall, furniture family, sign style); small
//! things (props, wear, people) vary per desk in the world builder. The App's colour is its mark
//! colour (MARK_COLOURS in ui/src/lib/mark-hues.ts) and appears on its sign, rug and team badges.

use crate::paint::{hex, oklch, Rgba, Rng};

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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool { ClaudeCode, ClaudeApp, Codex, OpenCode, Gemini, Cursor }
impl Tool {
    /// A two-letter monogram for the sign. The real build shows the tool's own icon, read at run time.
    pub fn monogram(self) -> &'static str {
        match self { Tool::ClaudeCode => "CC", Tool::ClaudeApp => "CA", Tool::Codex => "CX", Tool::OpenCode => "OC", Tool::Gemini => "GM", Tool::Cursor => "CU" }
    }
    pub fn family(self) -> &'static str {
        match self { Tool::ClaudeCode => "Workshop", Tool::ClaudeApp => "Studio", Tool::Codex => "Industrial", Tool::OpenCode => "Maker garage", Tool::Gemini => "Campus", Tool::Cursor => "Lab" }
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
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layout { Pods, Bench, Clusters, Studio, Tables }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeskKind { Wood, White, Steel, Standing }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChairKind { Task, Stool, Mesh }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SignKind { Banner, Neon, Plaque, Blade, Pennant }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screens { Single, Dual, Laptop }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Plant { Fern, Monstera, Snake, Cactus, Olive, Pothos, Herbs, Palm }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Decor { Bookshelf, Posters, Pendants, Whiteboard, Pegboard, Bikes, Sofa, Beanbags, Lamps, Records, Shelving, Fishtank }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Nook { Sofa, Beanbags, Huddle, Library, Workbench }

#[derive(Clone, Debug)]
pub struct Kit {
    pub tool: Tool,
    pub family: &'static str,
    pub variant: String,
    pub accent: Rgba,
    pub floor: FloorMat,
    pub wall: WallFinish,
    pub desk_top: Rgba,
    pub desk_frame: Rgba,
    pub desk: DeskKind,
    pub chair: Rgba,
    pub chair_kind: ChairKind,
    pub layout: Layout,
    pub screens: Screens,
    pub plants: Vec<Plant>,
    pub greenery: f32,
    pub decor: Vec<Decor>,
    pub nook: Nook,
    pub warmth: f32,
    pub sign: SignKind,
    pub windows: usize,
    pub rug: Rgba,
}

/// The kit for an App: family from its tool, variant from its id.
pub fn kit_for(tool: Tool, app_id: &str, slot: usize) -> Kit {
    let mut r = Rng::of(&format!("kit|{:?}|{app_id}", tool));
    let accent = mark_colour(slot);
    let pick = |r: &mut Rng, xs: &[&str]| hex(r.pick(xs));
    match tool {
        Tool::ClaudeCode => {
            // Workshop: warm timber, terracotta and brick, craft lamps, a reading nook, lots of green.
            let wood = pick(&mut r, &["#b07a4a", "#9a6238", "#c08b57", "#7d5434"]);
            let floor = if r.chance(0.5) { FloorMat::Parquet { wood, block: 0.6 } } else { FloorMat::Planks { wood, width: 0.2 } };
            let wall = match r.int(3) {
                0 => WallFinish::Brick { c: pick(&mut r, &["#a4543a", "#b5654a", "#8f4a35"]), mortar: hex("#d8c7b4") },
                1 => WallFinish::Paint { c: pick(&mut r, &["#c86f4f", "#8a9a6e", "#c9925c"]), dado: hex("#6b4a32") },
                _ => WallFinish::Slats { wood: pick(&mut r, &["#8a5a36", "#a06a40"]), back: hex("#3b2a22") },
            };
            Kit { tool, family: tool.family(), variant: variant_name(&floor, &wall), accent, floor, wall,
                desk_top: pick(&mut r, &["#c99764", "#b5824f", "#d4a873"]), desk_frame: hex("#5a3d2a"), desk: DeskKind::Wood,
                chair: pick(&mut r, &["#a3502f", "#5c6b3b", "#2f4a6b", "#7a3b4a"]), chair_kind: ChairKind::Task,
                layout: *r.pick(&[Layout::Pods, Layout::Clusters]), screens: *r.pick(&[Screens::Single, Screens::Dual]),
                plants: vec![Plant::Fern, Plant::Monstera, Plant::Pothos], greenery: 0.85,
                decor: vec![Decor::Bookshelf, Decor::Lamps, Decor::Posters, Decor::Records], nook: *r.pick(&[Nook::Library, Nook::Sofa]),
                warmth: 0.9, sign: *r.pick(&[SignKind::Banner, SignKind::Plaque]), windows: 1 + r.int(2), rug: pick(&mut r, &["#7a3b2e", "#4a5a3a", "#3a4a6a"]) }
        }
        Tool::ClaudeApp => {
            // Studio: pale oak, soft pastel walls, white tables, ceramics, posters.
            let floor = if r.chance(0.6) { FloorMat::Planks { wood: pick(&mut r, &["#e1c9a2", "#d8bd94", "#e8d3b0"]), width: 0.24 } } else { FloorMat::Carpet { a: hex("#c9c3d6"), b: hex("#bdb6cc"), tile: 0.5 } };
            let wall = if r.chance(0.6) { WallFinish::Paint { c: pick(&mut r, &["#e8c4c0", "#c3dccd", "#c4d4e8", "#e6dcc0"]), dado: hex("#f4efe6") } } else { WallFinish::Stripes { a: hex("#f1e6d6"), b: pick(&mut r, &["#e8b9b0", "#b9d8c6", "#b8c8e6"]) } };
            Kit { tool, family: tool.family(), variant: variant_name(&floor, &wall), accent, floor, wall,
                desk_top: hex("#f2f0ea"), desk_frame: hex("#c9c5bb"), desk: DeskKind::White,
                chair: pick(&mut r, &["#e6a99a", "#9cc7b1", "#a8b9de", "#e8cf8e"]), chair_kind: *r.pick(&[ChairKind::Stool, ChairKind::Task]),
                layout: *r.pick(&[Layout::Tables, Layout::Bench]), screens: Screens::Laptop,
                plants: vec![Plant::Olive, Plant::Pothos, Plant::Monstera], greenery: 0.65,
                decor: vec![Decor::Posters, Decor::Pendants, Decor::Sofa], nook: *r.pick(&[Nook::Sofa, Nook::Huddle]),
                warmth: 0.65, sign: SignKind::Banner, windows: 2 + r.int(2), rug: pick(&mut r, &["#e9d7c4", "#d6e3dc", "#e3d6e6"]) }
        }
        Tool::Codex => {
            // Industrial: polished concrete, brick or concrete panels, black steel, Edison pendants.
            let floor = FloorMat::Concrete { tone: pick(&mut r, &["#8d8f94", "#7a7c82", "#9a948a"]), seam: if r.chance(0.5) { 3.0 } else { 2.0 } };
            let wall = if r.chance(0.55) { WallFinish::Brick { c: pick(&mut r, &["#7e3b2f", "#d9d4cc", "#6b6560"]), mortar: hex("#4a4440") } } else { WallFinish::Tiles { c: pick(&mut r, &["#8a8f96", "#a59f95"]), grout: hex("#5a5e64") } };
            Kit { tool, family: tool.family(), variant: variant_name(&floor, &wall), accent, floor, wall,
                desk_top: pick(&mut r, &["#2d2f34", "#6b4c33", "#3a3d43"]), desk_frame: hex("#1c1d21"), desk: DeskKind::Steel,
                chair: hex("#25262b"), chair_kind: ChairKind::Mesh,
                layout: Layout::Bench, screens: Screens::Dual,
                plants: vec![Plant::Snake, Plant::Cactus], greenery: 0.4,
                decor: vec![Decor::Pendants, Decor::Shelving, Decor::Whiteboard], nook: *r.pick(&[Nook::Huddle, Nook::Workbench]),
                warmth: 0.55, sign: *r.pick(&[SignKind::Neon, SignKind::Blade]), windows: 2 + r.int(2), rug: hex("#3a3d43") }
        }
        Tool::OpenCode => {
            // Maker garage: rubber floor, plywood and pegboard, standing desks, bikes, neon.
            let floor = FloorMat::Rubber { c: pick(&mut r, &["#3f5a4a", "#3d4a5c", "#55524c"]) };
            let wall = if r.chance(0.6) { WallFinish::Slats { wood: hex("#c9a46c"), back: hex("#6b5638") } } else { WallFinish::Paint { c: pick(&mut r, &["#2f4a3f", "#3a3f55"]), dado: hex("#c9a46c") } };
            Kit { tool, family: tool.family(), variant: variant_name(&floor, &wall), accent, floor, wall,
                desk_top: hex("#d2b07a"), desk_frame: hex("#3a3a3e"), desk: DeskKind::Standing,
                chair: pick(&mut r, &["#e0a03a", "#3a8fb0"]), chair_kind: ChairKind::Stool,
                layout: Layout::Studio, screens: *r.pick(&[Screens::Laptop, Screens::Dual]),
                plants: vec![Plant::Herbs, Plant::Cactus], greenery: 0.35,
                decor: vec![Decor::Pegboard, Decor::Bikes, Decor::Whiteboard], nook: Nook::Workbench,
                warmth: 0.6, sign: SignKind::Neon, windows: 1 + r.int(2), rug: hex("#2f3a44") }
        }
        Tool::Gemini => {
            // Campus: terrazzo or bold carpet, striped supergraphics, beanbags, bright chairs.
            let floor = if r.chance(0.5) { FloorMat::Terrazzo { base: hex("#e7e1d6"), chips: [hex("#d9644a"), hex("#3f7fbf"), hex("#e8b33a")] } } else { FloorMat::Carpet { a: hex("#5a7fb0"), b: hex("#4f72a2"), tile: 0.5 } };
            let wall = if r.chance(0.5) { WallFinish::Stripes { a: hex("#f0e8da"), b: pick(&mut r, &["#e8b33a", "#d9644a", "#3f7fbf"]) } } else { WallFinish::Acoustic { a: hex("#3f7fbf"), b: hex("#e8b33a") } };
            Kit { tool, family: tool.family(), variant: variant_name(&floor, &wall), accent, floor, wall,
                desk_top: hex("#ece6da"), desk_frame: hex("#4a4a52"), desk: DeskKind::White,
                chair: pick(&mut r, &["#d9644a", "#3f7fbf", "#e8b33a", "#4aa37a"]), chair_kind: ChairKind::Task,
                layout: Layout::Clusters, screens: Screens::Single,
                plants: vec![Plant::Palm, Plant::Monstera], greenery: 0.55,
                decor: vec![Decor::Beanbags, Decor::Posters], nook: Nook::Beanbags,
                warmth: 0.7, sign: SignKind::Pennant, windows: 2, rug: hex("#e8b33a") }
        }
        Tool::Cursor => {
            let floor = FloorMat::Carpet { a: hex("#3a3f4a"), b: hex("#353a44"), tile: 0.5 };
            let wall = WallFinish::Acoustic { a: hex("#2f3440"), b: hex("#3a404d") };
            Kit { tool, family: tool.family(), variant: variant_name(&floor, &wall), accent, floor, wall,
                desk_top: hex("#2a2d33"), desk_frame: hex("#16171a"), desk: DeskKind::Steel,
                chair: hex("#1f2024"), chair_kind: ChairKind::Mesh, layout: Layout::Pods, screens: Screens::Dual,
                plants: vec![Plant::Snake], greenery: 0.25, decor: vec![Decor::Whiteboard, Decor::Lamps], nook: Nook::Huddle,
                warmth: 0.3, sign: SignKind::Blade, windows: 1, rug: hex("#2a2d33") }
        }
    }
}

fn variant_name(f: &FloorMat, w: &WallFinish) -> String {
    let f = match f { FloorMat::Planks { .. } => "plank floor", FloorMat::Parquet { .. } => "parquet", FloorMat::Carpet { .. } => "carpet tiles", FloorMat::Concrete { .. } => "polished concrete", FloorMat::Terrazzo { .. } => "terrazzo", FloorMat::Rubber { .. } => "rubber floor" };
    let w = match w { WallFinish::Paint { .. } => "painted wall", WallFinish::Brick { .. } => "brick", WallFinish::Slats { .. } => "timber slats", WallFinish::Tiles { .. } => "tile panels", WallFinish::Stripes { .. } => "striped supergraphic", WallFinish::Acoustic { .. } => "acoustic panels" };
    format!("{f}, {w}")
}
