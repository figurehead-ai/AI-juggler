//! The Floor's render thread (ADR-0026 §4): it owns the office, the
//! seating, the scene, the renderer and the driver, and draws only when
//! the driver says so. The window's events, new snapshots and its close
//! arrive as [`Msg`]s; a click on a door goes to the main window.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use juggler_floor::driver::{Driver, Effect, Input, PickKind, Picker};
use juggler_floor::family::Family;
use juggler_floor::layout::Office;
use juggler_floor::scene::{self, Live, Scene, Target};
use juggler_floor::seating::Seating;
use juggler_floor::snapshot::FloorSnapshot;
use juggler_floor_render::{Extras, Renderer, wgpu};

use super::store::{self, Settings};

/// 0.4.0's classic look instead of 0.4.2's (D-86): only when the budget
/// harness asks, to measure the two beside each other (and for rollback).
fn classic() -> bool {
    #[cfg(feature = "floor-harness")]
    {
        super::harness::classic()
    }
    #[cfg(not(feature = "floor-harness"))]
    {
        false
    }
}
use crate::features::notifications::OpenRequest;
use crate::state::AppState;

/// What the render thread has done, for the budgets (SPEC §9.7): read by
/// the measurement harness and the frame log, a few relaxed adds a frame.
#[derive(Debug, Default)]
pub struct Counters {
    /// Frames presented.
    pub frames: AtomicU64,
    /// Time spent drawing them, nanoseconds.
    pub draw_ns: AtomicU64,
    /// Time spent building scenes, nanoseconds.
    pub build_ns: AtomicU64,
    /// Scenes built.
    pub builds: AtomicU64,
    /// The last open's first frame, milliseconds after it was asked for.
    pub first_ms: AtomicU64,
    /// The last change's time to its frame on screen, microseconds.
    pub change_us: AtomicU64,
    /// The window's last visibility answer: 0 none yet, 1 seen, 2 not.
    pub visible: AtomicU8,
    /// Snapshots the render thread took in.
    pub snapshots: AtomicU64,
    /// The window's inputs taken in (resizes, visibility, pointer, keys).
    pub inputs: AtomicU64,
    /// Of those, resizes.
    pub resizes: AtomicU64,
    /// Time spent rebuilding the snapshot (the feed), nanoseconds.
    pub feed_ns: AtomicU64,
}

/// The last open's steps: each one's name and when it ended (ms after
/// the Floor was asked for).
pub static OPEN_STEPS: Mutex<Vec<(&'static str, u64)>> = Mutex::new(Vec::new());

/// Notes that the open's step `name` ended now.
pub fn open_step(name: &'static str, opened: Instant) {
    if let Ok(mut steps) = OPEN_STEPS.lock() {
        if name == "asked" {
            steps.clear();
        }
        steps.push((
            name,
            u64::try_from(opened.elapsed().as_millis()).unwrap_or(u64::MAX),
        ));
    }
}

/// The render thread's counters.
pub static COUNTERS: Counters = Counters {
    frames: AtomicU64::new(0),
    draw_ns: AtomicU64::new(0),
    build_ns: AtomicU64::new(0),
    builds: AtomicU64::new(0),
    first_ms: AtomicU64::new(0),
    change_us: AtomicU64::new(0),
    visible: AtomicU8::new(0),
    snapshots: AtomicU64::new(0),
    inputs: AtomicU64::new(0),
    resizes: AtomicU64::new(0),
    feed_ns: AtomicU64::new(0),
};

pub(crate) fn nanos(d: Duration) -> u64 {
    u64::try_from(d.as_nanos()).unwrap_or(u64::MAX)
}

/// What the render thread is told.
pub enum Msg {
    /// The window's input, resize or visibility.
    Input(Input),
    /// The company changed.
    Snapshot(Arc<FloorSnapshot>),
    /// The settings or the saved office changed (Settings › The Floor,
    /// Re-create): read them again.
    Reload,
    /// The window closed: stop and let the GPU go.
    Close,
    /// AJ-E3511: Reset the view (the first floor) — show floor n.
    ShowFloor(usize),
    /// AJ-E3512: Show on the Floor — the floor holding this App's area.
    ShowApp(String),
}

/// Where a click on `target` leads in the main window (AJ-E3510's door
/// table): `None` for a prop, the cat or a floor (those act on the Floor).
#[must_use]
pub fn door(target: &Target) -> Option<OpenRequest> {
    let page = |page: &str, id: &str| OpenRequest {
        session_id: None,
        page: Some(page.to_owned()),
        machine_id: None,
        id: Some(id.to_owned()),
    };
    Some(match target {
        Target::App(id) => page("app", id),
        Target::Session(key) => match (key.strip_prefix("s:"), key.strip_prefix("c:")) {
            (Some(own), _) => OpenRequest {
                session_id: Some(own.to_owned()),
                page: None,
                machine_id: None,
                id: None,
            },
            (None, Some(other)) => page("conversation", other),
            (None, None) => return None,
        },
        Target::Helpers(key) => page("subagents", key),
        Target::Account(id) => page("usage", id),
        Target::ThisMac => OpenRequest {
            session_id: None,
            page: Some("this_mac".to_owned()),
            machine_id: None,
            id: None,
        },
        Target::Pool(id) => page("pool", id),
        Target::Notices => OpenRequest {
            session_id: None,
            page: Some("notifications".to_owned()),
            machine_id: None,
            id: None,
        },
        Target::Floor(_) | Target::Prop(_) | Target::Cat => return None,
    })
}

/// What the window side does for the render thread.
pub trait Window: Send + 'static {
    /// Ask (on the main thread) whether the window can be seen; the
    /// answer comes back as [`Input::Visible`].
    fn ask_visible(&self, tx: Sender<Msg>);
    /// The window's title: the floor in view in words (its accessible
    /// summary, AJ-E3512 req. 3).
    fn title(&self, summary: String);
    /// Open a door in the main window and bring it forward.
    fn open(&self, door: OpenRequest);
    /// The pointer as a hand (over something that does something on a
    /// click: a door, a toy, a floor tab) or the arrow (0.4.1, finding 1).
    fn cursor(&self, hand: bool) {
        let _ = hand;
    }
}

/// The office, seated, and its scene, kept current.
struct World {
    office: Office,
    hidden: Vec<String>,
    seating: Seating,
    scene: Scene,
    snapshot: Arc<FloorSnapshot>,
}

fn machine_word() -> String {
    format!("This {}", juggler_platform::system::computer_noun().0)
}

impl World {
    /// Places the snapshot's Apps; saves what the office learnt; builds.
    fn build(
        state: &AppState,
        office: &mut Office,
        seating: &mut Seating,
        hidden: &[String],
        snapshot: &FloorSnapshot,
        settings: &Settings,
    ) -> Option<Scene> {
        let now = juggler_core::model::Timestamp::now().millis();
        let open: Vec<(String, Family, usize)> = snapshot
            .apps
            .iter()
            .filter(|a| {
                !hidden.contains(&a.id)
                    && (a.open
                        || scene::unused(a, now, settings.decay) < 5
                        || office.apps.contains_key(&a.id))
            })
            .map(|a| {
                (
                    a.id.clone(),
                    Family::of(Some(&a.family)),
                    scene::live_sessions(a).len(),
                )
            })
            .collect();
        let seen = office.observe(&open, now);
        if seen.changed
            && let Ok(storage) = state.storage.lock()
        {
            let _ = store::save(&storage, office);
        }
        let ids: Vec<&str> = snapshot.apps.iter().map(|a| a.id.as_str()).collect();
        seating.keep_only(&ids);
        let seats: BTreeMap<String, Vec<Option<String>>> = snapshot
            .apps
            .iter()
            .map(|a| {
                (
                    a.id.clone(),
                    seating.seat(
                        &a.id,
                        scene::capacity(office, &a.id),
                        &scene::live_sessions(a),
                    ),
                )
            })
            .collect();
        let machine = machine_word();
        let logos = logos_of(state, snapshot);
        let built = Instant::now();
        let scene = Scene::build(scene::Inputs {
            snapshot,
            office,
            seats: &seats,
            decay: settings.decay,
            // AJ-E3518: dust sheets go with the decay setting
            covers: settings.decay && !classic(),
            detail: !classic(),
            machine: &machine,
            now_ms: now,
            logos: &logos,
        })
        .ok();
        COUNTERS
            .build_ns
            .fetch_add(nanos(built.elapsed()), Ordering::Relaxed);
        COUNTERS.builds.fetch_add(1, Ordering::Relaxed);
        scene
    }
}

/// The floor logos the window drew, for the Apps that have one.
fn logos_of(
    state: &AppState,
    snapshot: &FloorSnapshot,
) -> BTreeMap<String, juggler_floor::paint::Img> {
    let dir = state.cache.data_dir().join(super::logos::DIR);
    let gate = state.cache.gate();
    snapshot
        .apps
        .iter()
        .filter_map(|a| {
            Some((
                a.id.clone(),
                super::logos::read(&dir, &gate, &a.id, a.logo.as_deref()?)?,
            ))
        })
        .collect()
}

/// Reads picks from the last frame for the driver.
struct Picks<'a> {
    renderer: &'a Renderer,
    scene: &'a Scene,
    scale: f32,
}

impl Picker for Picks<'_> {
    fn pick(&mut self, x: f32, y: f32) -> u32 {
        self.renderer.pick(x, y, self.scale)
    }

    fn kind(&self, id: u32) -> PickKind {
        match self.scene.pick_of(id).map(|p| &p.target) {
            None => PickKind::Nothing,
            Some(Target::Prop(_) | Target::Cat) => PickKind::Toy,
            Some(Target::Floor(f)) => PickKind::Floor(*f),
            Some(_) => PickKind::Door,
        }
    }
}

/// Everything the thread starts with.
pub struct Start {
    /// The app.
    pub state: Arc<AppState>,
    /// The GPU instance the surface came from.
    pub instance: wgpu::Instance,
    /// The window's surface (made on the main thread).
    pub surface: wgpu::Surface<'static>,
    /// The window's size in device pixels, and its scale.
    pub size: (u32, u32, f32),
    /// When the Floor was asked for (the first frame's budget).
    pub opened: Instant,
}

/// Runs the Floor until it's told to close.
pub fn run(start: Start, rx: &Receiver<Msg>, tx: &Sender<Msg>, window: &impl Window) {
    let Some(mut floor) = Running::start(start) else {
        return;
    };
    loop {
        let now = floor.started.elapsed();
        let timeout = floor
            .driver
            .wake_at(now)
            .map_or(Duration::from_secs(3600), |at| at.saturating_sub(now))
            .min(floor.until_light());
        let mut msgs = Vec::new();
        match rx.recv_timeout(timeout) {
            Ok(m) => msgs.push(m),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        msgs.extend(rx.try_iter());
        let now = floor.started.elapsed();
        for msg in msgs {
            if !floor.message(msg, now, window) {
                return;
            }
        }
        floor.relight(now);
        let tick = floor.driver.tick(now);
        if tick.check_visible {
            window.ask_visible(tx.clone());
        }
        if tick.draw {
            floor.frame(now);
        }
        let summary = floor.world.scene.summary(floor.driver.view.focus);
        if summary != floor.title {
            window.title(summary.clone());
            floor.title = summary;
        }
    }
}

/// The Floor while it runs.
struct Running {
    state: Arc<AppState>,
    settings: Settings,
    world: World,
    renderer: Renderer,
    driver: Driver,
    started: Instant,
    opened: Instant,
    first: bool,
    title: String,
    /// When the change waiting for its frame arrived.
    changed: Option<Instant>,
    /// The pointer is a hand now (what the window was last told).
    hand: bool,
    /// AJ-E3520: where the time zone is (for the sun), and the light's
    /// step drawn last (the light changes only between steps).
    place: Option<(f32, f32)>,
    lit: Option<(i64, super::store::Light)>,
}

impl Running {
    /// Loads the office, builds the scene and starts the renderer; `None`
    /// when any of it can't be had (the window then stays empty).
    fn start(start: Start) -> Option<Self> {
        let Start {
            state,
            instance,
            surface,
            size,
            opened,
        } = start;
        let settings = state
            .storage
            .lock()
            .ok()
            .and_then(|s| Settings::read(&s).ok())
            .unwrap_or_default();
        let fresh_seed =
            juggler_floor::rng::hash(&format!("office{}", opened.elapsed().as_nanos())) ^ 0x5eed;
        let (mut office, hidden) = state
            .storage
            .lock()
            .ok()
            .and_then(|s| store::load(&s, settings.per_floor, fresh_seed).ok())
            .unwrap_or_else(|| (Office::new(fresh_seed, settings.per_floor), Vec::new()));
        open_step("settings and office", opened);
        let snapshot = super::current(&state).ok()?;
        open_step("snapshot", opened);
        let mut seating = Seating::default();
        let scene = World::build(
            &state,
            &mut office,
            &mut seating,
            &hidden,
            &snapshot,
            &settings,
        )?;
        let world = World {
            office,
            hidden,
            seating,
            scene,
            snapshot,
        };
        open_step("scene", opened);
        let mut renderer =
            Renderer::new(&instance, surface, (size.0, size.1), &world.scene).ok()?;
        renderer.set_full(settings.detail.full());
        renderer.set_labels(settings.labels);
        if classic() {
            renderer.set_look(juggler_floor_render::Look::classic());
        }
        let place = super::light::place();
        open_step("renderer", opened);
        let mut driver = Driver::new(world.scene.frame_size(), size, settings.motion_now());
        driver.alert = world.snapshot.machine.alert_since_ms.is_some();
        if let Some(floor) = world.scene.neediest_floor() {
            driver.show_floor(floor);
        }
        driver.areas = world.scene.area_stops();
        driver.waiting = world.scene.waiting_stops();
        Some(Self {
            state,
            settings,
            world,
            renderer,
            driver,
            started: Instant::now(),
            opened,
            first: true,
            title: String::new(),
            changed: None,
            hand: false,
            place,
            lit: None,
        })
        .map(|mut floor| {
            floor.relight(Duration::ZERO);
            floor
        })
    }

    /// AJ-E3520: how long until the light's next step (an hour when it
    /// doesn't follow the clock).
    fn until_light(&self) -> Duration {
        if self.settings.light == super::store::Light::Clock && !classic() {
            Duration::from_millis(super::light::until_next_step(
                juggler_core::model::Timestamp::now().millis(),
            ))
        } else {
            Duration::from_secs(3600)
        }
    }

    /// AJ-E3520: the light for now, when its step or the setting changed:
    /// one frame then, none in between.
    fn relight(&mut self, now: Duration) {
        if classic() {
            return;
        }
        let now_ms = juggler_core::model::Timestamp::now().millis();
        let at = (super::light::step_of(now_ms), self.settings.light);
        if self.lit == Some(at) {
            return;
        }
        self.lit = Some(at);
        self.renderer.set_look(juggler_floor_render::Look {
            sky: super::light::sky(self.settings.light, self.place, now_ms),
            ..juggler_floor_render::Look::default()
        });
        let mut picks = Picks {
            renderer: &self.renderer,
            scene: &self.world.scene,
            scale: self.driver.size.2,
        };
        self.driver.input(Input::Changed, now, &mut picks);
    }

    /// Takes in one message; `false` to stop.
    fn message(&mut self, msg: Msg, now: Duration, window: &impl Window) -> bool {
        match msg {
            Msg::Close => return false,
            Msg::Reload => self.reload(now),
            Msg::ShowFloor(floor) => self.driver.show_floor(floor),
            Msg::ShowApp(app_id) => {
                let plan = juggler_floor::layout::pack(&self.world.office);
                if let Some(area) = plan
                    .areas
                    .iter()
                    .find(|area| area.id == app_id && area.annex == 0)
                {
                    self.driver.show_floor(area.floor);
                }
            }
            Msg::Snapshot(snapshot) => {
                COUNTERS.snapshots.fetch_add(1, Ordering::Relaxed);
                self.message_snapshot(snapshot, now);
            }
            Msg::Input(input) => {
                COUNTERS.inputs.fetch_add(1, Ordering::Relaxed);
                if let Input::Resize { w, h, .. } = input {
                    COUNTERS.resizes.fetch_add(1, Ordering::Relaxed);
                    self.renderer.resize(w, h);
                }
                if let Input::Visible(seen) = input {
                    COUNTERS
                        .visible
                        .store(if seen { 1 } else { 2 }, Ordering::Relaxed);
                }
                if input == Input::Visible(true) {
                    // Reduce motion may have changed while it was away.
                    self.driver.motion = self.settings.motion_now();
                }
                let mut picks = Picks {
                    renderer: &self.renderer,
                    scene: &self.world.scene,
                    scale: self.driver.size.2,
                };
                if let Some(Effect::Open(id)) = self.driver.input(input, now, &mut picks)
                    && let Some(open) = self.world.scene.pick_of(id).and_then(|p| door(&p.target))
                {
                    window.open(open);
                }
                if self.driver.hand != self.hand {
                    self.hand = self.driver.hand;
                    window.cursor(self.hand);
                }
            }
        }
        true
    }

    /// Rebuilds the scene for `snapshot` and redraws.
    fn message_snapshot(&mut self, snapshot: Arc<FloorSnapshot>, now: Duration) {
        self.changed.get_or_insert_with(Instant::now);
        let w = &mut self.world;
        if let Some(scene) = World::build(
            &self.state,
            &mut w.office,
            &mut w.seating,
            &w.hidden,
            &snapshot,
            &self.settings,
        ) {
            self.renderer.load(&scene);
            w.scene = scene;
            self.driver.frame = w.scene.frame_size();
            self.driver.areas = w.scene.area_stops();
            self.driver.waiting = w.scene.waiting_stops();
        }
        let mut picks = Picks {
            renderer: &self.renderer,
            scene: &w.scene,
            scale: self.driver.size.2,
        };
        self.driver.input(
            Input::Alert(snapshot.machine.alert_since_ms.is_some()),
            now,
            &mut picks,
        );
        self.driver.input(Input::Changed, now, &mut picks);
        w.snapshot = snapshot;
    }

    /// Reads the settings and the saved office again, and redraws.
    fn reload(&mut self, now: Duration) {
        let Ok(storage) = self.state.storage.lock() else {
            return;
        };
        let Ok(settings) = Settings::read(&storage) else {
            return;
        };
        if let Ok((office, hidden)) =
            store::load(&storage, settings.per_floor, self.world.office.seed)
        {
            self.world.office = office;
            self.world.hidden = hidden;
        }
        drop(storage);
        self.settings = settings;
        self.driver.motion = settings.motion_now();
        // D-80: Pixel art or Full resolution, from the next frame; the
        // targets follow the size it asks for, the old ones dropped.
        self.renderer.set_full(settings.detail.full());
        self.renderer.set_labels(settings.labels);
        let snapshot = Arc::clone(&self.world.snapshot);
        self.message_snapshot(snapshot, now);
    }

    /// Draws a frame (a refused one is retried by the driver).
    fn frame(&mut self, now: Duration) {
        let (d, w) = (&mut self.driver, &self.world);
        let live = Live {
            t: now.as_secs_f32(),
            clicked: d.clicked.clone(),
            lamps_off: std::collections::HashSet::new(),
            hover: d.hover,
            alert: d.alert,
            load: w.snapshot.machine.load,
            eye: [0.0; 3],
            walkers: d.motion.walkers(),
        };
        let extras = Extras {
            hint: juggler_window_events::hint(),
            alert: w.snapshot.machine.alert.as_deref().filter(|_| d.alert),
            mouse: d.mouse,
        };
        let drawing = Instant::now();
        match self
            .renderer
            .present(&w.scene, &d.view, &live, d.size, &extras)
        {
            Ok(shown) => {
                COUNTERS.frames.fetch_add(1, Ordering::Relaxed);
                COUNTERS
                    .draw_ns
                    .fetch_add(nanos(drawing.elapsed()), Ordering::Relaxed);
                if let Some(at) = self.changed.take() {
                    COUNTERS.change_us.store(
                        u64::try_from(at.elapsed().as_micros()).unwrap_or(u64::MAX),
                        Ordering::Relaxed,
                    );
                }
                d.chips = shown.chips;
                d.drew(true, shown.moving, now);
                if std::mem::take(&mut self.first) {
                    log_first_frame(self.opened.elapsed());
                }
            }
            Err(_) => d.drew(false, false, now),
        }
    }
}

/// Logs how long the first frame took (the 400 ms budget).
fn log_first_frame(took: Duration) {
    if let Ok(mut steps) = OPEN_STEPS.lock() {
        steps.push((
            "first frame",
            u64::try_from(took.as_millis()).unwrap_or(u64::MAX),
        ));
    }
    COUNTERS.first_ms.store(
        u64::try_from(took.as_millis()).unwrap_or(u64::MAX),
        Ordering::Relaxed,
    );
    tracing::info!("floor: first frame in {} ms", took.as_millis());
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]

    use juggler_floor::scene::Target;

    use super::door;

    #[test]
    fn each_door_opens_its_page_and_toys_open_nothing() {
        let table = [
            (
                Target::App("cli:a".into()),
                (None, Some("app"), Some("cli:a")),
            ),
            (Target::Session("s:9".into()), (Some("9"), None, None)),
            (
                Target::Session("c:acct/77".into()),
                (None, Some("conversation"), Some("acct/77")),
            ),
            (
                Target::Helpers("s:9".into()),
                (None, Some("subagents"), Some("s:9")),
            ),
            (
                Target::Account("acct:1".into()),
                (None, Some("usage"), Some("acct:1")),
            ),
            (Target::ThisMac, (None, Some("this_mac"), None)),
            (
                Target::Pool("pool-1".into()),
                (None, Some("pool"), Some("pool-1")),
            ),
            (Target::Notices, (None, Some("notifications"), None)),
        ];
        for (target, (session, page, id)) in table {
            let open = door(&target).unwrap_or_else(|| panic!("{target:?} opens something"));
            assert_eq!(
                (
                    open.session_id.as_deref(),
                    open.page.as_deref(),
                    open.id.as_deref()
                ),
                (session, page, id),
                "{target:?}"
            );
        }
        for toy in [Target::Prop(3), Target::Cat, Target::Floor(1)] {
            assert_eq!(door(&toy), None, "{toy:?} never navigates");
        }
    }
}
