//! The office as a scene (AJ-E3506): a still part built when the company
//! changes (the building, the areas, the furniture) and a moving part
//! built for each drawn frame (people, screens, signals and the things to
//! play with). Every signal comes from the snapshot: sessions and their
//! states, subagents, an App's load and how long it's been unused,
//! Accounts' usage left, This Mac's load and alerts. Everything you can
//! click has a pick id and a [`Target`] the main window opens.
//!
//! No code here branches on a tool: an area's look is its family's kit,
//! and a desktop App's screens show a chat because it is a desktop App.

mod area;
mod building;
mod frame;

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::colour::{Rgba, hex, mark_colour};
use crate::kit::{Kit, Screens, kit_for};
use crate::layout::{Office, Plan, STOREY, pack, units};
use crate::math::{T, V3};
use crate::models::{Item, Look};
use crate::paint::Img;
use crate::snapshot::{FloorAccount, FloorApp, FloorSession, FloorSnapshot};
use crate::state::FloorState;
use crate::tex::{self, Atlas, Full, Icon, ScreenPic, Ui, Uv};

pub use frame::Live;

/// How strongly an area's logo shows through its floor (30–40%, the
/// maintainer's call).
pub const LOGO_OPACITY: f32 = 0.35;

/// At most this many subagents stand round one desk; more show as "+N".
pub const HELPERS_SHOWN: usize = 6;

/// What a click opens in the main window.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    /// An App.
    App(String),
    /// A session, by the rail's key.
    Session(String),
    /// A session's subagents, by its key.
    Helpers(String),
    /// An Account (its usage).
    Account(String),
    /// This Mac.
    ThisMac,
    /// A floor (the lift).
    Floor(usize),
    /// A pool (its meeting room).
    Pool(String),
    /// The notification centre (the lobby's notice board).
    Notices,
    /// Something to play with (no door).
    Prop(usize),
    /// The office cat (no door).
    Cat,
}

/// One pickable thing: what it opens, and its tooltip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pick {
    /// What a click opens.
    pub target: Target,
    /// Its name.
    pub label: String,
    /// Its state, in words.
    pub detail: String,
}

/// A light: where (`p[3]` its reach), its colour, and the pick it belongs
/// to (a desk lamp goes out with its desk).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    /// Its floor (-1: outside).
    pub floor: i32,
    /// Where, and its reach.
    pub p: [f32; 4],
    /// Its colour.
    pub c: [f32; 4],
    /// The pick it belongs to (0: none).
    pub owner: u32,
}

/// An area's name above its sign, for the overlay.
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    /// Its floor.
    pub floor: usize,
    /// Where.
    pub at: V3,
    /// The App.
    pub app: String,
    /// The area's pick.
    pub id: u32,
}

/// A screen-space marker the overlay draws.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mark {
    /// A balloon over a head.
    Balloon {
        /// Where.
        at: V3,
        /// Which.
        icon: Icon,
    },
    /// "+N" over a desk's subagents.
    Plus {
        /// Where.
        at: V3,
        /// How many more.
        n: usize,
    },
}

/// Something to play with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PropKind {
    /// A potted plant.
    Plant {
        /// Its kind.
        kind: crate::kit::Plant,
        /// Its pot.
        pot: crate::kit::Pot,
        /// Its scale.
        s: f32,
        /// Its seed.
        seed: u32,
        /// Its wilt.
        wilt: f32,
    },
    /// The ping-pong table.
    PingPong,
    /// The coffee machine.
    Coffee,
    /// A beanbag.
    Beanbag(Rgba),
    /// The arcade cabinet.
    Arcade,
}

/// A prop on a floor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PropRec {
    /// What.
    pub kind: PropKind,
    /// Its floor.
    pub floor: i32,
    /// Where.
    pub t: T,
    /// Its pick.
    pub id: u32,
}

/// A seated session, for the moving part.
#[derive(Clone, Debug, PartialEq)]
struct DeskRec {
    session: FloorSession,
    state: FloorState,
    desktop: bool,
    floor: i32,
    frame: T,
    seat: f32,
    top: f32,
    screens: Screens,
    look: Look,
    id: u32,
    helper_id: u32,
    accent: Rgba,
    shared: bool,
    width: f32,
    phase: f32,
    /// A crash's smoke stops 10 minutes after it (SPEC §9.7).
    smoke: bool,
}

/// The pictures screens show.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Pictures {
    code: [Uv; 4],
    chat: Uv,
    error: Uv,
    frozen: Uv,
    off: Uv,
    done: Uv,
}

/// What the scene is built from.
#[derive(Clone, Copy, Debug)]
pub struct Inputs<'a> {
    /// The company now.
    pub snapshot: &'a FloorSnapshot,
    /// The office: order, sizes and looks.
    pub office: &'a Office,
    /// Who sits at which desk, per App ([`crate::seating::Seating`]).
    pub seats: &'a BTreeMap<String, Vec<Option<String>>>,
    /// Show unused Apps' decay (Settings › The Floor).
    pub decay: bool,
    /// 0.4.2's decay: dust sheets over the desks of an App unused for
    /// over a week (`docs/research/floor-04x.md` §4); off keeps 0.4.0's.
    pub covers: bool,
    /// 0.4.2's second pass: smooth people with faces that show the
    /// state, materials on everything (`docs/research/floor-04x.md` §12);
    /// off keeps the first pass's figures.
    pub detail: bool,
    /// This computer's name ("This Mac"), from the platform's noun.
    pub machine: &'a str,
    /// Now, epoch ms.
    pub now_ms: i64,
    /// Each App's floor logo the main window drew (AJ-E3508); without
    /// one, the Floor writes the letters itself.
    pub logos: &'a BTreeMap<String, Img>,
}

/// How unused an App is: 0 open, 1 under a week, 2 over a week, 3 over a
/// month (SPEC §9.7).
#[must_use]
pub fn unused(app: &FloorApp, now_ms: i64, decay: bool) -> u8 {
    const DAY: i64 = 24 * 60 * 60 * 1000;
    if app.open {
        return 0;
    }
    let idle = now_ms - app.last_active_ms.unwrap_or(0);
    let level = if idle < 7 * DAY {
        1
    } else if idle < 30 * DAY {
        2
    } else {
        3
    };
    if decay { level } else { level.min(1) }
}

/// How bright an unused App's area is, by [`unused`]'s level.
#[must_use]
pub const fn dimming(level: u8) -> f32 {
    match level {
        0 => 1.0,
        1 => 0.8,
        2 => 0.62,
        _ => 0.38,
    }
}

/// The floor's height above the ground.
#[must_use]
pub fn floor_y(f: usize) -> f32 {
    f as f32 * STOREY
}

/// An area's mark letters, from its tool's name ("Claude Code · Work" →
/// "CC") until the main window hands over the tool's own logo.
#[must_use]
pub fn mark_letters(name: &str) -> String {
    name.split(" · ")
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(2)
        .flat_map(char::to_uppercase)
        .collect()
}

/// The office, built.
#[derive(Clone, Debug)]
pub struct Scene {
    /// Where everything goes.
    pub plan: Plan,
    /// Each placed App's kit, by id.
    pub kits: BTreeMap<String, Kit>,
    /// The still parts.
    pub stat: Vec<Item>,
    /// The textures (only the rows in use).
    pub atlas: Img,
    /// What a texture place's v and height are scaled by: the atlas's
    /// full height over the rows kept.
    pub atlas_v: f32,
    /// The overlay's glyphs and balloons.
    pub ui: Ui,
    /// Everything pickable; pick id `n` is `picks[n - 1]`.
    pub picks: Vec<Pick>,
    /// The still lights.
    pub lights: Vec<Light>,
    /// Things to play with.
    pub props: Vec<PropRec>,
    /// Area names, for the overlay.
    pub labels: Vec<Label>,
    /// How many floors.
    pub floors: usize,
    /// A floor's width.
    pub width: f32,
    apps: BTreeMap<String, FloorApp>,
    unused: BTreeMap<String, u8>,
    covers: bool,
    detail: bool,
    notices: u32,
    accounts: Vec<FloorAccount>,
    desks: Vec<DeskRec>,
    signs: Vec<(i32, T, Option<String>)>,
    fires: Vec<(i32, T)>,
    boards: Vec<(i32, T, Vec<u32>)>,
    racks: Vec<(i32, T)>,
    beacons: Vec<(i32, V3)>,
    pictures: Pictures,
    cat: u32,
    now_ms: i64,
}

impl Scene {
    fn pick(&mut self, target: Target, label: impl Into<String>, detail: impl Into<String>) -> u32 {
        self.picks.push(Pick {
            target,
            label: label.into(),
            detail: detail.into(),
        });
        u32::try_from(self.picks.len()).unwrap_or(u32::MAX)
    }

    /// The pickable thing with pick id `id`.
    #[must_use]
    pub fn pick_of(&self, id: u32) -> Option<&Pick> {
        let index = usize::try_from(id).ok()?.checked_sub(1)?;
        self.picks.get(index)
    }

    /// Where a desk's sitter is, for pick id `id` (to point at it).
    #[must_use]
    pub fn anchor(&self, id: u32) -> Option<V3> {
        self.desks
            .iter()
            .find(|d| d.id == id)
            .map(|d| d.frame.0.point([0.0, 1.2, 0.62]))
    }

    /// Where a desk's sitter is (pick id `id`) and which way they face,
    /// along the floor (to frame them up close).
    #[must_use]
    pub fn sitter(&self, id: u32) -> Option<(V3, V3)> {
        let d = self.desks.iter().find(|d| d.id == id)?;
        let at = d.frame.0.point([0.0, 1.2, 0.62]);
        let ahead = d.frame.0.point([0.0, 1.2, -1.0]);
        let f = [ahead[0] - at[0], 0.0, ahead[2] - at[2]];
        let l = (f[0] * f[0] + f[2] * f[2]).sqrt().max(1e-6);
        Some((at, [f[0] / l, 0.0, f[2] / l]))
    }

    /// The pick of a session, by the rail's key (Show on the Floor).
    #[must_use]
    pub fn pick_of_session(&self, key: &str) -> Option<u32> {
        self.desks
            .iter()
            .find(|d| d.session.key == key)
            .map(|d| d.id)
    }

    /// Builds the office.
    ///
    /// # Errors
    ///
    /// [`Full`] if the textures outgrow the atlas (a test holds a large
    /// office inside it).
    pub fn build(inputs: Inputs<'_>) -> Result<Self, Full> {
        let plan = pack(inputs.office);
        let apps: BTreeMap<String, FloorApp> = inputs
            .snapshot
            .apps
            .iter()
            .map(|a| (a.id.clone(), a.clone()))
            .collect();
        let levels = apps
            .iter()
            .map(|(id, app)| (id.clone(), unused(app, inputs.now_ms, inputs.decay)))
            .collect();
        let mut kits = BTreeMap::new();
        for id in &inputs.office.placed {
            if let (Some(state), Some(app)) = (inputs.office.apps.get(id), apps.get(id)) {
                kits.insert(
                    id.clone(),
                    kit_for(state.family, state.choice, usize::from(app.colour_slot)),
                );
            }
        }
        let mut atlas = Atlas::new(2048, 2048);
        let mut add_screen = |key: &str, pic: ScreenPic, accent: Rgba, seed: u32| {
            atlas.add(key, &tex::screen_img(pic, accent, seed))
        };
        let code_accent = hex("#8fb4ff");
        let pictures = Pictures {
            code: [
                add_screen("code0", ScreenPic::Code, code_accent, 40)?,
                add_screen("code1", ScreenPic::Code, code_accent, 41)?,
                add_screen("code2", ScreenPic::Code, code_accent, 42)?,
                add_screen("code3", ScreenPic::Code, code_accent, 43)?,
            ],
            chat: add_screen("chat", ScreenPic::Chat, hex("#e8a07a"), 3)?,
            error: add_screen("error", ScreenPic::Error, hex("#ffffff"), 5)?,
            frozen: add_screen("frozen", ScreenPic::Frozen, hex("#ffffff"), 6)?,
            off: add_screen("off", ScreenPic::Off, hex("#ffffff"), 7)?,
            done: add_screen("done", ScreenPic::Done, hex("#ffffff"), 8)?,
        };
        let floors = plan.floors.max(1);
        let width = plan.width;
        let mut scene = Self {
            plan,
            kits,
            stat: Vec::new(),
            atlas: Img::new(1, 1),
            atlas_v: 1.0,
            ui: tex::ui_atlas()?,
            picks: Vec::new(),
            lights: Vec::new(),
            props: Vec::new(),
            labels: Vec::new(),
            floors,
            width,
            apps,
            unused: levels,
            covers: inputs.covers,
            detail: inputs.detail,
            notices: inputs.snapshot.notices,
            accounts: inputs.snapshot.accounts.clone(),
            desks: Vec::new(),
            signs: Vec::new(),
            fires: Vec::new(),
            boards: Vec::new(),
            racks: Vec::new(),
            beacons: Vec::new(),
            pictures,
            cat: 0,
            now_ms: inputs.now_ms,
        };
        let mut out = Vec::new();
        scene.skyline(&mut out, &mut atlas)?;
        for f in 0..floors {
            scene.building_floor(&mut out, &mut atlas, f, inputs.machine)?;
        }
        let areas = scene.plan.areas.clone();
        for placed in &areas {
            scene.area(&mut out, &mut atlas, placed, &inputs)?;
        }
        let mut rooms = scene.meeting_rooms(&inputs.snapshot.pools);
        let mut commons = vec![true; floors];
        for (f, what, rect) in scene.plan.free.clone() {
            let kitchen =
                std::mem::replace(&mut commons[f], false) && what == crate::layout::Free::Kitchen;
            match rooms
                .iter()
                .position(|(floor, _)| *floor == f)
                .filter(|_| !kitchen)
            {
                Some(at) => {
                    let (_, pool) = rooms.remove(at);
                    scene.meeting_room(&mut out, &mut atlas, f, rect, &pool)?;
                }
                None => scene.free_cell(&mut out, f, rect, kitchen),
            }
        }
        scene.cat = scene.pick(Target::Cat, "The office cat", "pet it");
        scene.stat = out;
        // Only the rows in use go to the GPU (and stay in memory); the
        // places the atlas gave are in its full height, so the renderer
        // scales their v by `atlas_v`.
        (scene.atlas, scene.atlas_v) = atlas.trimmed();
        Ok(scene)
    }

    /// Each pool with members seated, and the floor most of them sit on
    /// (SPEC §9.7: its meeting room is there), in the snapshot's order.
    fn meeting_rooms(
        &self,
        pools: &[crate::snapshot::FloorPool],
    ) -> Vec<(usize, crate::snapshot::FloorPool)> {
        let floor_of: BTreeMap<&str, usize> = self
            .desks
            .iter()
            .filter_map(|d| Some((d.session.key.as_str(), usize::try_from(d.floor).ok()?)))
            .collect();
        pools
            .iter()
            .filter_map(|pool| {
                let mut counts: BTreeMap<usize, usize> = BTreeMap::new();
                for key in &pool.sessions {
                    if let Some(f) = floor_of.get(key.as_str()) {
                        *counts.entry(*f).or_default() += 1;
                    }
                }
                let (floor, n) = counts
                    .into_iter()
                    .max_by_key(|(f, n)| (*n, std::cmp::Reverse(*f)))?;
                (n >= 2).then(|| (floor, pool.clone()))
            })
            .collect()
    }

    /// The floor holding the App most in need of you (the most sessions
    /// waiting: a question, stuck, out of usage), where the Floor opens;
    /// `None` when nothing waits (it opens on floor 1).
    #[must_use]
    pub fn neediest_floor(&self) -> Option<usize> {
        let mut waiting: BTreeMap<(i32, &str), usize> = BTreeMap::new();
        for d in &self.desks {
            if matches!(
                d.state,
                FloorState::Asks | FloorState::Stuck | FloorState::Limit
            ) {
                *waiting
                    .entry((d.floor, d.session.key.as_str()))
                    .or_default() += 1;
            }
        }
        let mut by_app: BTreeMap<(i32, String), usize> = BTreeMap::new();
        for d in self
            .desks
            .iter()
            .filter(|d| waiting.contains_key(&(d.floor, d.session.key.as_str())))
        {
            let app = self
                .apps
                .values()
                .find(|a| a.sessions.iter().any(|s| s.key == d.session.key))
                .map_or_else(String::new, |a| a.id.clone());
            *by_app.entry((d.floor, app)).or_default() += 1;
        }
        by_app
            .into_iter()
            .max_by_key(|((floor, _), n)| (*n, std::cmp::Reverse(*floor)))
            .and_then(|((floor, _), _)| usize::try_from(floor).ok())
    }

    /// The areas for Tab, in the office's order.
    #[must_use]
    pub fn area_stops(&self) -> Vec<crate::driver::Stop> {
        self.labels
            .iter()
            .map(|l| crate::driver::Stop {
                floor: l.floor,
                at: l.at,
                id: l.id,
            })
            .collect()
    }

    /// The desks waiting on you for N: a question first, then stuck, then
    /// out of usage, in the office's order.
    #[must_use]
    pub fn waiting_stops(&self) -> Vec<crate::driver::Stop> {
        let rank = |s: FloorState| match s {
            FloorState::Asks => Some(0),
            FloorState::Stuck => Some(1),
            FloorState::Limit => Some(2),
            _ => None,
        };
        let mut desks: Vec<(u8, &DeskRec)> = self
            .desks
            .iter()
            .filter_map(|d| Some((rank(d.state)?, d)))
            .collect();
        desks.sort_by_key(|(r, d)| (*r, d.floor, d.id));
        desks
            .into_iter()
            .filter_map(|(_, d)| {
                Some(crate::driver::Stop {
                    floor: usize::try_from(d.floor).ok()?,
                    at: d.frame.0.point([0.0, 0.0, 0.0]),
                    id: d.id,
                })
            })
            .collect()
    }

    /// The floor in view in words, for the window's accessible title
    /// ("Floor 1: 6 Apps, 2 waiting on you").
    #[must_use]
    pub fn summary(&self, focus: Option<usize>) -> String {
        let apps = self
            .labels
            .iter()
            .filter(|l| focus.is_none_or(|f| l.floor == f))
            .count();
        let waiting = self
            .waiting_stops()
            .iter()
            .filter(|s| focus.is_none_or(|f| s.floor == f))
            .count();
        let place = focus.map_or_else(|| "The building".to_owned(), |f| format!("Floor {}", f + 1));
        let apps = if apps == 1 {
            "1 App".to_owned()
        } else {
            format!("{apps} Apps")
        };
        match waiting {
            0 => format!("{place}: {apps}"),
            n => format!("{place}: {apps}, {n} waiting on you"),
        }
    }

    /// An Account, by id.
    #[must_use]
    pub fn account(&self, id: Option<&str>) -> Option<&FloorAccount> {
        self.accounts.iter().find(|a| Some(a.id.as_str()) == id)
    }

    /// An App, by id.
    #[must_use]
    pub fn app(&self, id: &str) -> Option<&FloorApp> {
        self.apps.get(id)
    }

    /// How unused an App is ([`unused`]'s level).
    #[must_use]
    pub fn unused_level(&self, id: &str) -> u8 {
        self.unused.get(id).copied().unwrap_or(0)
    }

    /// The office's size, for the camera.
    #[must_use]
    pub fn frame_size(&self) -> crate::camera::Frame {
        crate::camera::Frame {
            width: self.width,
            floors: self.floors,
        }
    }
}

/// The sessions to seat at an App's desks: its live ones, in the
/// snapshot's order (start order).
#[must_use]
pub fn live_sessions(app: &FloorApp) -> Vec<&str> {
    app.sessions
        .iter()
        .filter(|s| s.state != FloorState::Ended)
        .map(|s| s.key.as_str())
        .collect()
}

/// The desks an App's areas hold in all (its main area and annexes).
#[must_use]
pub fn capacity(office: &Office, app: &str) -> usize {
    office
        .apps
        .get(app)
        .map_or(0, |a| units(a.desks).iter().map(|u| u.capacity()).sum())
}

/// An App's accent colour.
#[must_use]
pub fn accent(app: &FloorApp) -> Rgba {
    mark_colour(usize::from(app.colour_slot))
}

/// Pick ids that were clicked, and when (seconds of the clock).
pub type Clicked = HashMap<u32, f32>;
/// Desk lamps switched off, by pick.
pub type LampsOff = HashSet<u32>;

#[cfg(test)]
mod tests;
