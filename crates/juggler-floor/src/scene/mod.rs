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
pub mod tower;

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
    /// An App's tasks (its whiteboard), by the App's id.
    Tasks(String),
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
    /// 0.4.1's decay: dust sheets over the desks of an App unused for
    /// over a week (`docs/research/floor-04x.md` §4); off keeps 0.4.0's.
    pub covers: bool,
    /// 0.4.1's second pass: smooth people with faces that show the
    /// state, materials on everything (`docs/research/floor-04x.md` §12);
    /// off keeps the first pass's figures.
    pub detail: bool,
    /// AJ-E3519: Apps whose areas are left out of view (Show: Active Apps
    /// only), their cells empty; nothing about them changes.
    pub hide: &'a std::collections::BTreeSet<String>,
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
    const HOUR: i64 = 60 * 60 * 1000;
    if app.open {
        return 0;
    }
    let idle = now_ms - app.last_active_ms.unwrap_or(0);
    // AJ-E3518: closed under an hour (lamps and screens off), to a day
    // (chairs pushed in), to a week (dust), to a month (dust sheets),
    // and longer (all covered, nearly dark)
    let level = if idle < HOUR {
        1
    } else if idle < DAY {
        2
    } else if idle < 7 * DAY {
        3
    } else if idle < 30 * DAY {
        4
    } else {
        5
    };
    if decay { level } else { level.min(1) }
}

/// SPEC §9.7's limits: a crashed session's smoke stops after 10 minutes.
pub const SMOKE_FOR_MS: i64 = 10 * 60 * 1000;
/// A done session stretches for 15 minutes, then sits idle.
pub const DONE_FOR_MS: i64 = 15 * 60 * 1000;

/// How the Floor draws `session` at `now_ms`: its state, with the table's
/// time limits (done for 15 minutes, then idle). Its word and card keep
/// the session's own state, as the rail says it.
#[must_use]
pub const fn drawn_state(session: &FloorSession, now_ms: i64) -> FloorState {
    match session.state {
        FloorState::Done if now_ms - session.last_ms >= DONE_FOR_MS => FloorState::Idle,
        state => state,
    }
}

/// How someone in `state` sits or stands at their desk (SPEC §9.7's
/// table; the same in an office of the building view).
#[must_use]
pub const fn pose_of(state: FloorState) -> crate::models::Pose {
    use crate::models::Pose;
    match state {
        FloorState::Working => Pose::Typing,
        FloorState::Asks => Pose::Wave,
        FloorState::Stuck => Pose::Scratch,
        FloorState::Crashed => Pose::HandsOnHead,
        FloorState::Done => Pose::Stretch,
        FloorState::Limit => Pose::Asleep,
        FloorState::Idle | FloorState::Ended => Pose::Idle,
    }
}

/// The balloon over someone in `state`: `!`, `?`, `X`, a tick, a red Z.
#[must_use]
pub const fn balloon_of(state: FloorState) -> Option<Icon> {
    match state {
        FloorState::Asks => Some(Icon::Asks),
        FloorState::Stuck => Some(Icon::Stuck),
        FloorState::Crashed => Some(Icon::Error),
        FloorState::Done => Some(Icon::Done),
        FloorState::Limit => Some(Icon::Sleep),
        FloorState::Working | FloorState::Idle | FloorState::Ended => None,
    }
}

/// Whether a crashed session still smokes at `now_ms`.
#[must_use]
pub const fn smoking(session: &FloorSession, now_ms: i64) -> bool {
    matches!(session.state, FloorState::Crashed) && now_ms - session.last_ms < SMOKE_FOR_MS
}

/// What the passing of time alone changes in how `snap` is drawn, as of
/// its own time: each App's unused level, each session's drawn state and
/// smoke. The feed publishes a snapshot whose key differs, so a limit
/// that passes (smoke out, done to idle, a day unused) is drawn within a
/// tick without anything else changing.
#[must_use]
pub fn time_limits(snap: &FloorSnapshot) -> Vec<u8> {
    let now = snap.at_ms;
    snap.apps
        .iter()
        .flat_map(|app| {
            std::iter::once(unused(app, now, true)).chain(
                app.sessions
                    .iter()
                    .map(move |s| (drawn_state(s, now) as u8) << 1 | u8::from(smoking(s, now))),
            )
        })
        .collect()
}

/// An unused App's desks go under dust sheets from this level (a week).
pub const COVERED: u8 = 4;

/// How bright an unused App's area is, by [`unused`]'s level (AJ-E3518:
/// 80% closed under an hour, 55% to a day, 40% to a week, then 32% and
/// 25% after a month; its logo and sign stay readable).
#[must_use]
pub const fn dimming(level: u8) -> f32 {
    match level {
        0 => 1.0,
        1 => 0.8,
        2 => 0.55,
        3 => 0.4,
        4 => 0.32,
        _ => 0.25,
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
    /// AJ-E3516: the tower's floors and its rooms that need you, for the
    /// overlay.
    pub tower_floors: Vec<tower::TowerFloor>,
    /// The tower's rooms that need you.
    pub tower_rooms: Vec<tower::TowerRoom>,
    apps: BTreeMap<String, FloorApp>,
    unused: BTreeMap<String, u8>,
    covers: bool,
    detail: bool,
    hidden: std::collections::BTreeSet<String>,
    notices: u32,
    needs: u32,
    /// This Mac in words, for its rack ("Plenty of room · CPU 23%").
    mac_words: String,
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

/// This Mac in the app's words: its room, then its overload alert or its
/// CPU ("Plenty of room · CPU 23%", "Tight on memory · CPU 96% for 6
/// minutes").
pub(crate) fn mac_words(machine: &crate::snapshot::FloorMachine) -> String {
    #[allow(clippy::cast_possible_truncation)] // a percentage
    let cpu = machine
        .alert
        .clone()
        .unwrap_or_else(|| format!("CPU {}%", (machine.load * 100.0).round() as i32));
    match &machine.room {
        Some(room) => format!("{room} · {cpu}"),
        None => cpu,
    }
}

/// "1 session", "3 sessions".
pub(crate) fn sessions_word(n: usize) -> String {
    if n == 1 {
        "1 session".to_owned()
    } else {
        format!("{n} sessions")
    }
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

    /// The moment the scene was built for (epoch ms).
    #[must_use]
    pub const fn now_ms(&self) -> i64 {
        self.now_ms
    }

    /// AJ-E3517: the person with pick id `id`: their App and session.
    #[must_use]
    pub fn person(&self, id: u32) -> Option<(&FloorApp, &FloorSession)> {
        let d = self.desks.iter().find(|d| d.id == id)?;
        let app = self
            .apps
            .values()
            .find(|a| a.sessions.iter().any(|s| s.key == d.session.key))?;
        Some((app, &d.session))
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

    /// AJ-E3515, AJ-E3516: what needs you in an App, most urgent first:
    /// each need and how many (sessions; one for its Account out of
    /// usage or the machine hot).
    #[must_use]
    pub fn needs_of(&self, app: &FloorApp) -> Vec<(crate::flags::Need, u32)> {
        use crate::flags::Need;
        let count = |state: FloorState| {
            u32::try_from(
                app.sessions
                    .iter()
                    .filter(|s| drawn_state(s, self.now_ms) == state)
                    .count(),
            )
            .unwrap_or(0)
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
        // its Account's room, as the app's Out and Low chips say it
        if let Some(account) = self.account(app.account_id.as_deref()) {
            if account.out {
                needs.push((Need::Out, 1));
            } else if needs.is_empty()
                && account
                    .left_pct
                    .is_some_and(|left| left <= crate::flags::LOW_LEFT)
            {
                // low on room is a state, not something waiting on you: it
                // shows only when nothing else does, and counts in no total
                needs.push((Need::Low, 1));
            }
        }
        needs.sort();
        needs
    }

    /// An area's words under its App's name: what needs you first, then
    /// what's working ("1 asking · 2 working"); "2 sessions open, nothing
    /// waiting" or "No sessions open" otherwise.
    #[must_use]
    pub fn area_words(&self, app: &FloorApp) -> String {
        use crate::flags::Need;
        let mut words = Vec::new();
        // a state of the App or its Account, not a count of sessions: said
        // after the sessions
        let mut states = Vec::new();
        let account = self.account(app.account_id.as_deref());
        let resets = account
            .and_then(|a| a.resets_words.as_ref())
            .map(|when| format!(" · resets {when}"))
            .unwrap_or_default();
        for (need, n) in self.needs_of(app) {
            match need {
                Need::Hot => states.push("working this computer hard".to_owned()),
                Need::Out => states.push(format!("out of usage{resets}")),
                Need::Low => states.push(match account.and_then(|a| a.left_pct) {
                    Some(left) => format!("{}% left{resets}", left.round().max(0.0)),
                    None => "low on usage".to_owned(),
                }),
                _ => words.push(format!("{n} {}", need.word(true).to_lowercase())),
            }
        }
        let state_count = |state: FloorState| {
            app.sessions
                .iter()
                .filter(|s| drawn_state(s, self.now_ms) == state)
                .count()
        };
        let working = state_count(FloorState::Working);
        if working > 0 {
            words.push(format!("{working} working"));
        }
        if words.is_empty() {
            let open = app
                .sessions
                .iter()
                .filter(|s| s.state != FloorState::Ended)
                .count();
            words.push(match open {
                0 => "No sessions open".to_owned(),
                1 => "1 session open, nothing waiting".to_owned(),
                n => format!("{n} sessions open, nothing waiting"),
            });
        }
        words.extend(states);
        words.join(" · ")
    }

    /// What a flag says of `app`'s room, in the app's words: "OUT ·
    /// RESETS 9:31 PM" (or "OUT" when that isn't known), or "LOW · 12%
    /// LEFT"; `None` with room or unmeasured.
    #[must_use]
    pub fn room_words(&self, app: &FloorApp) -> Option<(crate::flags::Need, String)> {
        use crate::flags::{LOW_LEFT, Need};
        let account = self.account(app.account_id.as_deref())?;
        if account.out {
            // and when it resets, as the app's Out chip says it
            let words = account.resets_words.as_ref().map_or_else(
                || "OUT".to_owned(),
                |when| format!("OUT · RESETS {}", when.to_uppercase()),
            );
            return Some((Need::Out, words));
        }
        let left = account.left_pct.filter(|left| *left <= LOW_LEFT)?;
        // about to run dry: how much is left, and when it comes back
        let words = format!("LOW · {}% LEFT", left.round().max(0.0));
        Some((
            Need::Low,
            account.resets_words.as_ref().map_or(words.clone(), |when| {
                format!("{words} · RESETS {}", when.to_uppercase())
            }),
        ))
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
    #[allow(clippy::too_many_lines)] // the whole office, part by part, in order
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
            tower_floors: Vec::new(),
            tower_rooms: Vec::new(),
            apps,
            unused: levels,
            covers: inputs.covers,
            detail: inputs.detail,
            hidden: inputs.hide.clone(),
            notices: inputs.snapshot.notices,
            needs: inputs.snapshot.needs,
            mac_words: mac_words(&inputs.snapshot.machine),
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
        for placed in areas.iter().filter(|p| !inputs.hide.contains(&p.id)) {
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
        // AJ-E3516: the tower, last (its floors' summaries count the desks)
        scene.tower(&mut out, &mut atlas)?;
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

    /// The floor in view in words, for the window's title: its Apps,
    /// Needs you's one count (as Home and the bell say it), then the
    /// sessions waiting on this floor ("Floor 1: 6 Apps · 4 need you ·
    /// 1 asking here").
    #[must_use]
    pub fn summary(&self, focus: Option<usize>) -> String {
        let place = focus.map_or_else(|| "The building".to_owned(), |f| format!("Floor {}", f + 1));
        let (apps, waiting) = self.counts(focus);
        let mut words = vec![apps];
        match self.needs {
            0 => {}
            1 => words.push("1 needs you".to_owned()),
            n => words.push(format!("{n} need you")),
        }
        if let Some(waiting) = waiting {
            words.push(if focus.is_some() {
                format!("{waiting} here")
            } else {
                waiting
            });
        }
        format!("{place}: {}", words.join(" · "))
    }

    /// A floor's Apps and the sessions waiting on it ("6 Apps · 1 session
    /// waiting"), for its door in the building view.
    #[must_use]
    pub fn floor_words(&self, focus: Option<usize>) -> String {
        match self.counts(focus) {
            (apps, Some(waiting)) => format!("{apps} · {waiting}"),
            (apps, None) => apps,
        }
    }

    /// "6 Apps", and "1 asking · 1 out of usage" when any wait (one floor, or all).
    fn counts(&self, focus: Option<usize>) -> (String, Option<String>) {
        let apps = self
            .labels
            .iter()
            .filter(|l| focus.is_none_or(|f| l.floor == f))
            .count();
        let apps = if apps == 1 {
            "1 App".to_owned()
        } else {
            format!("{apps} Apps")
        };
        // By kind (AJ-E3514's F1: "1 session waiting" was a desk out of
        // usage, asleep, and the tester looked for one asking).
        let count = |state: FloorState| {
            self.desks
                .iter()
                .filter(|d| {
                    d.state == state && focus.is_none_or(|f| i32::try_from(f).ok() == Some(d.floor))
                })
                .count()
        };
        let words: Vec<String> = [
            (FloorState::Asks, "asking"),
            (FloorState::Stuck, "stuck"),
            (FloorState::Limit, "out of usage"),
        ]
        .into_iter()
        .filter_map(|(state, word)| match count(state) {
            0 => None,
            n => Some(format!("{n} {word}")),
        })
        .collect();
        let waiting = (!words.is_empty()).then(|| words.join(" · "));
        (apps, waiting)
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
