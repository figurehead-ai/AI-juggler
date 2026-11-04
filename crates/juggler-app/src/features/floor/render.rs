//! The Floor's render thread (ADR-0026 §4): it owns the office, the
//! seating, the scene, the renderer and the driver, and draws only when
//! the driver says so. The window's events, new snapshots and its close
//! arrive as [`Msg`]s; a click on a door goes to the main window.

use std::collections::BTreeMap;
use std::sync::Arc;
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
use crate::features::notifications::OpenRequest;
use crate::state::AppState;

/// What the render thread is told.
pub enum Msg {
    /// The window's input, resize or visibility.
    Input(Input),
    /// The company changed.
    Snapshot(Arc<FloorSnapshot>),
    /// The window closed: stop and let the GPU go.
    Close,
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
    /// Open a door in the main window and bring it forward.
    fn open(&self, door: OpenRequest);
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
                        || scene::unused(a, now, settings.decay) < 3
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
        Scene::build(scene::Inputs {
            snapshot,
            office,
            seats: &seats,
            decay: settings.decay,
            machine: &machine,
            now_ms: now,
        })
        .ok()
    }
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
            .map_or(Duration::from_secs(3600), |at| at.saturating_sub(now));
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
        let tick = floor.driver.tick(now);
        if tick.check_visible {
            window.ask_visible(tx.clone());
        }
        if tick.draw {
            floor.frame(now);
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
        let snapshot = super::current(&state).ok()?;
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
        let renderer = Renderer::new(&instance, surface, (size.0, size.1), &world.scene).ok()?;
        let mut driver = Driver::new(world.scene.frame_size(), size, settings.motion);
        driver.alert = world.snapshot.machine.alert_since_ms.is_some();
        if let Some(floor) = world.scene.neediest_floor() {
            driver.show_floor(floor);
        }
        Some(Self {
            state,
            settings,
            world,
            renderer,
            driver,
            started: Instant::now(),
            opened,
            first: true,
        })
    }

    /// Takes in one message; `false` to stop.
    fn message(&mut self, msg: Msg, now: Duration, window: &impl Window) -> bool {
        match msg {
            Msg::Close => return false,
            Msg::Snapshot(snapshot) => {
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
            Msg::Input(input) => {
                if let Input::Resize { w, h, .. } = input {
                    self.renderer.resize(w, h);
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
            }
        }
        true
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
            walkers: self.settings.motion.walkers(),
        };
        let extras = Extras {
            hint: juggler_window_events::hint(),
            alert: w.snapshot.machine.alert.as_deref().filter(|_| d.alert),
            mouse: d.mouse,
        };
        match self
            .renderer
            .present(&w.scene, &d.view, &live, d.size, &extras)
        {
            Ok(shown) => {
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
    eprintln!("floor: first frame in {} ms", took.as_millis());
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
