//! When the Floor draws (ADR-0026 §4, AJ-E3501 req. 3), as a state
//! machine the render thread runs and a headless clock test drives:
//!
//! - it draws for input, a camera move, a new snapshot, and an animation
//!   tick only while something moving is in view and the window can be
//!   seen;
//! - with nothing moving, nothing is drawn; covered or minimised, nothing
//!   is drawn (it asks whether it can be seen at 2 Hz, only while it
//!   would draw or is hidden);
//! - a refused frame is retried every 100 ms while visible, never
//!   dropped.
//!
//! Time is a [`Duration`] since the Floor opened, so tests choose it.

use std::collections::HashMap;
use std::time::Duration;

use crate::camera::{Cam, Frame, MIN_DIST, PITCH_MAX, PITCH_MIN, TWEEN_S, View};

/// How much moves (Settings › The Floor › Motion).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Motion {
    /// 8 frames a second, people walking.
    #[default]
    Full,
    /// 4 frames a second, no walkers.
    Calm,
    /// Nothing moves: a frame only on a change.
    Off,
}

impl Motion {
    /// Animation frames a second.
    #[must_use]
    pub const fn fps(self) -> u32 {
        match self {
            Self::Full => 8,
            Self::Calm => 4,
            Self::Off => 0,
        }
    }

    /// People walk the aisle.
    #[must_use]
    pub const fn walkers(self) -> bool {
        matches!(self, Self::Full)
    }
}

/// A key the Floor answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// A character (lower case).
    Char(char),
    /// ←.
    Left,
    /// →.
    Right,
    /// ↑.
    Up,
    /// ↓.
    Down,
    /// Tab: the next area.
    Tab,
    /// Shift-Tab: the area before.
    BackTab,
    /// Enter: open what's selected.
    Enter,
    /// Page Up: the lift up a floor (AJ-E3516).
    PageUp,
    /// Page Down: the lift down a floor (from the building, the top one).
    PageDown,
    /// Escape: closes a hover card.
    Escape,
}

/// What the window sends (positions in points from the top left).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Input {
    /// The window's new size in device pixels, and its scale.
    Resize {
        /// Width.
        w: u32,
        /// Height.
        h: u32,
        /// Device pixels per point.
        scale: f32,
    },
    /// Whether the window can be seen.
    Visible(bool),
    /// A scroll: a trackpad's (`precise`) pans, a wheel or ⌘ zooms.
    Scroll {
        /// Across.
        dx: f32,
        /// Down.
        dy: f32,
        /// A trackpad.
        precise: bool,
        /// Where, across.
        x: f32,
        /// Where, down.
        y: f32,
        /// ⌘ held.
        zoom: bool,
    },
    /// A pinch.
    Magnify {
        /// How much.
        m: f32,
        /// Where, across.
        x: f32,
        /// Where, down.
        y: f32,
    },
    /// A two-finger twist, degrees.
    Twist(f32),
    /// A drag; `rotate` for right-drag or ⌥-drag.
    Drag {
        /// Across.
        dx: f32,
        /// Down.
        dy: f32,
        /// Turn and tilt instead of pan.
        rotate: bool,
    },
    /// The pointer moved.
    Move {
        /// Across.
        x: f32,
        /// Down.
        y: f32,
    },
    /// A click.
    Click {
        /// Across.
        x: f32,
        /// Down.
        y: f32,
    },
    /// A key.
    Key(Key),
    /// The company changed (a new snapshot is in the scene).
    Changed,
    /// The overload alert went on or off.
    Alert(bool),
}

/// What the render thread should do after an input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    /// Open the pickable thing with this id in the main window.
    Open(u32),
    /// Play with the prop with this id.
    Play(u32),
    /// AJ-E3517: bring the app the session with this pick id is open in
    /// forward (its adapter's Go to it).
    GoTo(u32),
    /// AJ-E3519: open the View menu under its button (its bottom left,
    /// device pixels).
    ViewMenu((f32, f32)),
}

/// AJ-E3517: a hover card's button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardButton {
    /// Open in AI Juggler (at the question).
    Open,
    /// Open in the tool's own app (Go to it).
    GoTo,
}

/// How long the pointer rests on a person before their card opens.
pub const CARD_AFTER: Duration = Duration::from_millis(250);
/// How long a card stays after the pointer has left it and its person.
pub const CARD_STAYS: Duration = Duration::from_millis(300);

/// A floor-switcher chip's place (device pixels) and the floor it shows
/// (`None`: the building).
pub type Chip = ([f32; 4], Option<usize>);

/// What the pickable thing under a point is (`picks` reads the last
/// frame's pick buffer); 0 for nothing.
pub trait Picker {
    /// The pick id at (`x`, `y`) device pixels.
    fn pick(&mut self, x: f32, y: f32) -> u32;
    /// Whether pick id `id` is a prop or the cat (played with, not
    /// opened), a floor (`Some(floor)`), or a door.
    fn kind(&self, id: u32) -> PickKind;
}

/// What a click on a pick does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickKind {
    /// Nothing there.
    Nothing,
    /// Opens in the main window.
    Door,
    /// Something to play with.
    Toy,
    /// The lift to a floor.
    Floor(usize),
    /// A person at a desk: opens in the main window, and has a hover card
    /// (AJ-E3517).
    Person,
}

fn inside(r: &[f32; 4], x: f32, y: f32) -> bool {
    x >= r[0] && y >= r[1] && x < r[0] + r[2] && y < r[1] + r[3]
}

/// A place the keyboard moves to: an area (Tab) or a desk waiting on you
/// (N), with its pick id.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stop {
    /// Its floor.
    pub floor: usize,
    /// Where the camera looks.
    pub at: crate::math::V3,
    /// What Enter opens.
    pub id: u32,
}

/// The loop's state.
#[allow(clippy::struct_excessive_bools)] // each flag is its own fact the loop wakes on
#[derive(Clone, Debug)]
pub struct Driver {
    /// What is in view.
    pub view: View,
    /// The office's size.
    pub frame: Frame,
    /// The window's size in device pixels and its scale.
    pub size: (u32, u32, f32),
    /// How much moves.
    pub motion: Motion,
    /// The pick under the pointer.
    pub hover: u32,
    /// The pointer, device pixels.
    pub mouse: Option<(f32, f32)>,
    /// What was played with, and when (seconds).
    pub clicked: HashMap<u32, f32>,
    /// The overload alert is on.
    pub alert: bool,
    /// The floor switcher's chips, from the last frame.
    pub chips: Vec<Chip>,
    /// The areas, in order (Tab).
    pub areas: Vec<Stop>,
    /// The desks waiting on you, in order (N).
    pub waiting: Vec<Stop>,
    /// What the keyboard selected (Enter opens it; it's lit as if hovered).
    pub selected: Option<u32>,
    /// The pointer is over something a click acts on (a door, a toy, a
    /// floor or a floor tab): the window shows the hand (0.4.1).
    pub hand: bool,
    /// AJ-E3517: the hover card open now (its person's pick id), and
    /// where it was asked for (its anchor).
    pub card: Option<(u32, (f32, f32))>,
    /// The open card's place on screen, from the last frame (the card
    /// stays while the pointer is on it).
    pub card_area: Option<[f32; 4]>,
    /// The card's buttons, from the last frame.
    pub card_buttons: Vec<([f32; 4], CardButton)>,
    /// AJ-E3519: the View button, from the last frame.
    pub view_button: Option<[f32; 4]>,
    /// A person the pointer rests on, and since when.
    resting: Option<(u32, Duration)>,
    /// Since when the pointer has been off the card and its person.
    leaving: Option<Duration>,
    tween: Option<(Cam, Cam, Duration)>,
    visible: bool,
    animating: bool,
    retry: bool,
    dirty: bool,
    next_tick: Duration,
    next_check: Duration,
}

const RETRY: Duration = Duration::from_millis(100);
const TWEEN_STEP: Duration = Duration::from_millis(16);
const CHECK: Duration = Duration::from_millis(500);

/// What [`Driver::tick`] says to do now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tick {
    /// Draw a frame.
    pub draw: bool,
    /// Ask the window whether it can be seen.
    pub check_visible: bool,
}

impl Driver {
    /// A driver for an office of `frame`, a window of `size`, showing the
    /// first floor; it draws its first frame at once.
    #[must_use]
    pub fn new(frame: Frame, size: (u32, u32, f32), motion: Motion) -> Self {
        let aspect = size.0 as f32 / size.1.max(1) as f32;
        Self {
            view: View {
                focus: Some(0),
                cam: frame.home(Some(0), aspect),
            },
            frame,
            size,
            motion,
            hover: 0,
            mouse: None,
            clicked: HashMap::new(),
            alert: false,
            chips: Vec::new(),
            areas: Vec::new(),
            waiting: Vec::new(),
            selected: None,
            hand: false,
            card: None,
            card_area: None,
            card_buttons: Vec::new(),
            view_button: None,
            resting: None,
            leaving: None,
            tween: None,
            visible: true,
            animating: false,
            retry: false,
            dirty: true,
            next_tick: Duration::ZERO,
            next_check: CHECK,
        }
    }

    /// Shows floor `focus` at once (where the Floor opens).
    pub fn show_floor(&mut self, focus: usize) {
        let focus = focus.min(self.frame.floors.max(1) - 1);
        self.view = View {
            focus: Some(focus),
            cam: self.frame.home(Some(focus), self.aspect()),
        };
        self.dirty = true;
    }

    fn aspect(&self) -> f32 {
        self.size.0 as f32 / self.size.1.max(1) as f32
    }

    fn period(&self) -> Option<Duration> {
        let fps = self.motion.fps();
        (fps > 0).then(|| Duration::from_secs_f64(1.0 / f64::from(fps)))
    }

    /// Something keeps it ticking: an animation in view, or the alert.
    fn ticking(&self) -> bool {
        (self.animating || self.alert) && self.period().is_some()
    }

    /// When to wake next (`None`: only for input).
    #[must_use]
    pub fn wake_at(&self, now: Duration) -> Option<Duration> {
        match (self.wake_for_view(now), self.card_wake()) {
            (Some(a), Some(c)) => Some(a.min(c)),
            (a, c) => a.or(c),
        }
    }

    /// When a resting pointer opens a card, or a card left closes.
    fn card_wake(&self) -> Option<Duration> {
        match (self.resting, self.card, self.leaving) {
            (_, Some(_), Some(left)) => Some(left + CARD_STAYS),
            (Some((id, since)), card, _) if card.map(|c| c.0) != Some(id) => {
                Some(since + CARD_AFTER)
            }
            _ => None,
        }
    }

    fn wake_for_view(&self, now: Duration) -> Option<Duration> {
        if self.tween.is_some() {
            Some(now + TWEEN_STEP)
        } else if self.retry && self.visible {
            Some(now + RETRY)
        } else if self.ticking() && self.visible {
            Some(self.next_tick.min(self.next_check))
        } else if !self.visible {
            Some(self.next_check)
        } else {
            None
        }
    }

    fn move_to(&mut self, to: Cam, now: Duration) {
        if self.motion == Motion::Off {
            // Reduce Motion: a cut, not a ride
            self.view.cam = to;
            self.tween = None;
        } else {
            self.tween = Some((self.view.cam, to, now));
        }
        self.dirty = true;
    }

    fn goal(&self) -> Cam {
        self.tween.map_or(self.view.cam, |(_, b, _)| b)
    }

    fn go_to_floor(&mut self, focus: Option<usize>, now: Duration) {
        let yaw = self.goal().yaw;
        self.view.focus = focus;
        let home = self.frame.home(focus, self.aspect()).with_yaw(yaw);
        self.move_to(home, now);
    }

    /// Takes in one input; what it asks of the render thread.
    pub fn input(
        &mut self,
        input: Input,
        now: Duration,
        picker: &mut dyn Picker,
    ) -> Option<Effect> {
        let (w, h, sf) = self.size;
        let aspect = self.aspect();
        match input {
            Input::Resize { w, h, scale } if w > 0 && h > 0 => {
                self.size = (w, h, scale);
                self.dirty = true;
            }
            Input::Resize { .. } => {}
            Input::Visible(v) => {
                self.dirty |= v && !self.visible;
                self.visible = v;
            }
            Input::Scroll {
                dx,
                dy,
                precise,
                x,
                y,
                zoom,
            } => {
                self.tween = None;
                if precise && !zoom {
                    self.view.cam.pan(dx * sf, dy * sf, h);
                } else {
                    let step = if precise { dy / 30.0 } else { dy.signum() };
                    self.view.zoom_at(
                        &self.frame,
                        step,
                        (x * sf / w as f32, y * sf / h as f32),
                        aspect,
                    );
                }
                self.dirty = true;
            }
            Input::Magnify { m, x, y } => {
                self.tween = None;
                self.view.zoom_at(
                    &self.frame,
                    m * 4.0,
                    (x * sf / w as f32, y * sf / h as f32),
                    aspect,
                );
                self.dirty = true;
            }
            Input::Twist(_) | Input::Drag { .. } if self.view.focus.is_none() => {
                // the tower stands still: a floor is entered, not dragged to
            }
            Input::Twist(deg) => {
                self.tween = None;
                self.view.cam.yaw -= deg.to_radians();
                self.dirty = true;
            }
            Input::Drag { dx, dy, rotate } => {
                self.tween = None;
                if rotate {
                    self.view.cam.turn(dx, dy);
                } else {
                    self.view.cam.pan(dx * sf, dy * sf, h);
                }
                self.dirty = true;
            }
            Input::Move { x, y } => self.moved(x * sf, y * sf, now, picker),
            Input::Click { x, y } => return self.click(x * sf, y * sf, now, picker),
            Input::Key(Key::Enter) => return self.selected.map(Effect::Open),
            Input::Key(key) => self.key(key, now),
            Input::Changed => self.dirty = true,
            Input::Alert(on) => {
                self.dirty |= on != self.alert;
                self.alert = on;
            }
        }
        None
    }

    /// The pointer moved to `(px, py)` (device pixels): what's under it,
    /// the hand, a person's card resting and staying.
    fn moved(&mut self, px: f32, py: f32, now: Duration, picker: &mut dyn Picker) {
        self.mouse = Some((px, py));
        self.selected = None;
        let on_tab = self.tab_at(px, py).is_some();
        let id = if on_tab { 0 } else { picker.pick(px, py) };
        let on_button = self.card_buttons.iter().any(|(r, _)| inside(r, px, py));
        let on_view = self.view_button.is_some_and(|r| inside(&r, px, py));
        self.hand = on_tab || on_button || on_view || picker.kind(id) != PickKind::Nothing;
        // AJ-E3517: resting on a person opens their card; it
        // stays while the pointer is on it, them or between
        if picker.kind(id) == PickKind::Person {
            if self.resting.map(|r| r.0) != Some(id) {
                self.resting = Some((id, now));
            }
        } else {
            self.resting = None;
        }
        if self.card.is_some() {
            if self.on_card(px, py, id) {
                self.leaving = None;
            } else if self.leaving.is_none() {
                self.leaving = Some(now);
            }
        }
        // A tooltip follows the pointer while over something.
        self.dirty |= id != self.hover || id != 0;
        self.hover = id;
    }

    /// The floor tab under a point, device pixels: `Some(None)` the
    /// building, `Some(Some(f))` floor `f`.
    #[must_use]
    pub fn tab_at(&self, px: f32, py: f32) -> Option<Option<usize>> {
        self.chips
            .iter()
            .find(|(r, _)| px >= r[0] && py >= r[1] && px < r[0] + r[2] && py < r[1] + r[3])
            .map(|(_, f)| *f)
    }

    fn click(
        &mut self,
        px: f32,
        py: f32,
        now: Duration,
        picker: &mut dyn Picker,
    ) -> Option<Effect> {
        if let Some(focus) = self.tab_at(px, py) {
            self.go_to_floor(focus, now);
            return None;
        }
        if let Some(r) = self.view_button.filter(|r| inside(r, px, py)) {
            return Some(Effect::ViewMenu((r[0], r[1] + r[3])));
        }
        if let (Some((card, _)), Some((_, button))) = (
            self.card,
            self.card_buttons.iter().find(|(r, _)| inside(r, px, py)),
        ) {
            return Some(match button {
                CardButton::Open => Effect::Open(card),
                CardButton::GoTo => Effect::GoTo(card),
            });
        }
        let id = picker.pick(px, py);
        self.dirty = true;
        match picker.kind(id) {
            PickKind::Nothing => None,
            PickKind::Floor(f) => {
                if self.view.focus != Some(f) {
                    self.go_to_floor(Some(f), now);
                }
                None
            }
            PickKind::Toy => {
                self.clicked.insert(id, now.as_secs_f32());
                Some(Effect::Play(id))
            }
            PickKind::Door | PickKind::Person => Some(Effect::Open(id)),
        }
    }

    fn key(&mut self, key: Key, now: Duration) {
        let base = self.goal();
        let aspect = self.aspect();
        let to = match key {
            Key::Char(c @ '1'..='9') => {
                let f = c as usize - '1' as usize;
                if f < self.frame.floors {
                    self.go_to_floor(Some(f), now);
                }
                return;
            }
            Key::Char('0' | 'b') => {
                self.go_to_floor(None, now);
                return;
            }
            Key::Escape => {
                if self.card.is_some() {
                    self.close_card();
                }
                return;
            }
            Key::PageUp | Key::PageDown => {
                let top = self.frame.floors.max(1) - 1;
                let to = match (self.view.focus, key == Key::PageUp) {
                    (None, false) => Some(top),
                    (Some(f), true) if f < top => Some(f + 1),
                    (Some(f), false) if f > 0 => Some(f - 1),
                    _ => return,
                };
                self.go_to_floor(to, now);
                return;
            }
            Key::Char('h') => self.frame.home(self.view.focus, aspect),
            Key::Char('q') | Key::Left => Cam {
                yaw: base.yaw + std::f32::consts::FRAC_PI_4,
                ..base
            },
            Key::Char('e') | Key::Right => Cam {
                yaw: base.yaw - std::f32::consts::FRAC_PI_4,
                ..base
            },
            Key::Up => Cam {
                pitch: (base.pitch + 0.2).min(PITCH_MAX),
                ..base
            },
            Key::Down => Cam {
                pitch: (base.pitch - 0.2).max(PITCH_MIN),
                ..base
            },
            Key::Char('+' | '=') => Cam {
                dist: (base.dist * 0.75).max(MIN_DIST),
                ..base
            },
            Key::Char('-') => Cam {
                dist: (base.dist / 0.75).min(if self.view.focus.is_some() {
                    self.frame.floor_max_dist(aspect)
                } else {
                    self.frame.max_dist()
                }),
                ..base
            },
            Key::Tab => return self.step(1, false, now),
            Key::BackTab => return self.step(-1, false, now),
            Key::Char('n') => return self.step(1, true, now),
            Key::Char(_) | Key::Enter => return,
        };
        self.move_to(to, now);
    }

    /// Selects the next (or previous) area, or desk waiting on you, and
    /// moves the camera to it.
    fn step(&mut self, by: isize, waiting: bool, now: Duration) {
        let stops = if waiting { &self.waiting } else { &self.areas };
        if stops.is_empty() {
            return;
        }
        let len = stops.len() as isize;
        let at = self
            .selected
            .and_then(|id| stops.iter().position(|s| s.id == id));
        let next = at.map_or(if by > 0 { 0 } else { len - 1 }, |i| {
            (i as isize + by).rem_euclid(len)
        });
        let stop = stops[usize::try_from(next).unwrap_or(0)];
        self.selected = Some(stop.id);
        self.hover = stop.id;
        self.view.focus = Some(stop.floor);
        let base = self.goal();
        let dist = if waiting { 9.0 } else { 18.0 };
        let to = Cam {
            t: [stop.at[0], crate::scene::floor_y(stop.floor), stop.at[2]],
            dist,
            ..base
        };
        self.move_to(to, now);
    }

    /// AJ-E3517: opens the card of a person the pointer rested on, and
    /// closes one it left.
    fn card_tick(&mut self, now: Duration) {
        if let Some((id, since)) = self.resting
            && self.card.map(|c| c.0) != Some(id)
            && now >= since + CARD_AFTER
        {
            self.card = Some((id, self.mouse.unwrap_or_default()));
            self.card_area = None;
            self.card_buttons.clear();
            self.leaving = None;
            self.dirty = true;
        }
        if self.card.is_some() && self.leaving.is_some_and(|left| now >= left + CARD_STAYS) {
            self.close_card();
        }
    }

    fn close_card(&mut self) {
        self.card = None;
        self.card_area = None;
        self.card_buttons.clear();
        self.leaving = None;
        self.dirty = true;
    }

    /// Whether `(x, y)` is on the open card, its person, or the way
    /// between them (the box holding both).
    fn on_card(&self, x: f32, y: f32, id: u32) -> bool {
        let Some((card, (ax, ay))) = self.card else {
            return false;
        };
        if id == card {
            return true;
        }
        self.card_area.is_some_and(|r| {
            let (x0, y0) = (r[0].min(ax) - 6.0, r[1].min(ay) - 6.0);
            let (x1, y1) = ((r[0] + r[2]).max(ax) + 6.0, (r[1] + r[3]).max(ay) + 6.0);
            x >= x0 && x <= x1 && y >= y0 && y <= y1
        })
    }

    /// Moves the camera along and says whether to draw now (and whether to
    /// ask if the window can be seen).
    pub fn tick(&mut self, now: Duration) -> Tick {
        self.card_tick(now);
        if let Some((a, b, t0)) = self.tween {
            let k = ((now.saturating_sub(t0)).as_secs_f32() / TWEEN_S).min(1.0);
            self.view.cam = Cam::lerp(a, b, 1.0 - (1.0 - k).powi(3));
            if k >= 1.0 {
                self.tween = None;
                self.view.cam = b;
            }
            self.dirty = true;
        }
        if let Some(period) = self.period()
            && self.ticking()
            && self.visible
            && now >= self.next_tick
        {
            self.next_tick = (self.next_tick + period).max(now);
            if self.next_tick == now {
                self.next_tick = now + period;
            }
            self.dirty = true;
        }
        let mut check_visible = false;
        if (self.ticking() || !self.visible) && now >= self.next_check {
            self.next_check = now + CHECK;
            check_visible = true;
        }
        let draw = (self.dirty || self.retry) && self.visible;
        Tick {
            draw,
            check_visible,
        }
    }

    /// The frame was drawn (`shown`), or refused; `moving`: something
    /// moving is in view.
    pub fn drew(&mut self, shown: bool, moving: bool, now: Duration) {
        self.retry = !shown;
        if shown {
            self.dirty = false;
            let was = self.ticking();
            self.animating = moving && self.motion.fps() > 0;
            if self.ticking() && !was {
                self.next_tick = now + self.period().unwrap_or(RETRY);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        CARD_AFTER, CARD_STAYS, CardButton, Driver, Effect, Input, Key, Motion, PickKind, Picker,
    };
    use crate::camera::Frame;

    /// A person (id 5) right of x = 300.
    struct People;

    impl Picker for People {
        fn pick(&mut self, x: f32, _y: f32) -> u32 {
            if x >= 300.0 { 5 } else { 0 }
        }
        fn kind(&self, id: u32) -> PickKind {
            if id == 5 {
                PickKind::Person
            } else {
                PickKind::Nothing
            }
        }
    }

    #[test]
    fn a_person_s_card_opens_after_a_rest_stays_while_near_and_closes() {
        let mut d = driver(Motion::Off);
        let ms = Duration::from_millis;
        run(&mut d, 0, 100, false);
        d.input(Input::Move { x: 320.0, y: 100.0 }, ms(100), &mut People);
        // not yet: a passing pointer opens nothing
        d.tick(ms(200));
        assert_eq!(d.card, None);
        assert_eq!(
            d.wake_at(ms(200)),
            Some(ms(100) + CARD_AFTER),
            "it wakes to open it"
        );
        d.tick(ms(100) + CARD_AFTER);
        assert!(d.card.is_some_and(|(id, _)| id == 5), "open after 250 ms");
        // its place and buttons come back from the frame, in device
        // pixels (this window is at scale 2)
        d.card_area = Some([660.0, 40.0, 400.0, 160.0]);
        d.card_buttons = vec![
            ([680.0, 160.0, 180.0, 26.0], CardButton::Open),
            ([880.0, 160.0, 160.0, 26.0], CardButton::GoTo),
        ];
        // on the card (off the person): it stays
        d.input(Input::Move { x: 400.0, y: 50.0 }, ms(400), &mut People);
        d.tick(ms(1000));
        assert!(d.card.is_some(), "stays while on the card");
        // its buttons
        assert_eq!(
            d.input(Input::Click { x: 350.0, y: 85.0 }, ms(1100), &mut People),
            Some(Effect::Open(5))
        );
        assert_eq!(
            d.input(Input::Click { x: 450.0, y: 85.0 }, ms(1150), &mut People),
            Some(Effect::GoTo(5))
        );
        // away: closes 300 ms later
        d.input(Input::Move { x: 50.0, y: 300.0 }, ms(1200), &mut People);
        d.tick(ms(1490));
        assert_eq!(CARD_STAYS, ms(300));
        assert!(d.card.is_some(), "not before 300 ms");
        d.tick(ms(1200) + CARD_STAYS);
        assert_eq!(d.card, None, "gone 300 ms after leaving");
        // Esc closes one at once
        d.input(Input::Move { x: 320.0, y: 100.0 }, ms(2000), &mut People);
        d.tick(ms(2000) + CARD_AFTER);
        assert!(d.card.is_some());
        d.input(Input::Key(Key::Escape), ms(2400), &mut People);
        assert_eq!(d.card, None);
    }

    struct Office;

    impl Picker for Office {
        fn pick(&mut self, x: f32, _y: f32) -> u32 {
            if x < 100.0 {
                0
            } else if x < 200.0 {
                7
            } else {
                9
            }
        }
        fn kind(&self, id: u32) -> PickKind {
            match id {
                7 => PickKind::Door,
                9 => PickKind::Toy,
                _ => PickKind::Nothing,
            }
        }
    }

    /// Runs the loop from `from` to `to` ms, drawing whenever it says,
    /// with `moving` as the scene's answer; how many frames it drew.
    fn run(d: &mut Driver, from: u64, to: u64, moving: bool) -> usize {
        let mut frames = 0;
        let mut now = Duration::from_millis(from);
        let end = Duration::from_millis(to);
        while now < end {
            let tick = d.tick(now);
            if tick.draw {
                frames += 1;
                d.drew(true, moving, now);
            }
            now = match d.wake_at(now) {
                Some(at) if at > now => at,
                Some(_) => now + Duration::from_millis(1),
                None => end,
            };
        }
        frames
    }

    fn driver(motion: Motion) -> Driver {
        Driver::new(
            Frame {
                width: 44.4,
                floors: 2,
            },
            (1600, 1000, 2.0),
            motion,
        )
    }

    #[test]
    fn at_rest_it_draws_once_and_then_nothing() {
        let mut d = driver(Motion::Full);
        assert_eq!(run(&mut d, 0, 60_000, false), 1);
        assert_eq!(
            d.wake_at(Duration::from_secs(60)),
            None,
            "asleep until input"
        );
    }

    #[test]
    fn animating_it_ticks_at_its_rate_and_hidden_it_draws_nothing() {
        let mut d = driver(Motion::Full);
        let frames = run(&mut d, 0, 10_000, true);
        assert!((78..=82).contains(&frames), "8 a second for 10 s: {frames}");
        d.input(Input::Visible(false), Duration::from_secs(10), &mut Office);
        assert_eq!(run(&mut d, 10_000, 20_000, true), 0, "nothing while hidden");
        let mut calm = driver(Motion::Calm);
        assert!((38..=42).contains(&run(&mut calm, 0, 10_000, true)));
        let mut off = driver(Motion::Off);
        assert_eq!(
            run(&mut off, 0, 10_000, true),
            1,
            "Motion off: nothing moves"
        );
    }

    #[test]
    fn each_change_draws_once_and_a_refused_frame_is_retried() {
        let mut d = driver(Motion::Off);
        assert_eq!(run(&mut d, 0, 1000, false), 1);
        d.input(Input::Changed, Duration::from_secs(1), &mut Office);
        assert_eq!(run(&mut d, 1000, 2000, false), 1);
        d.input(
            Input::Drag {
                dx: 5.0,
                dy: 0.0,
                rotate: false,
            },
            Duration::from_secs(2),
            &mut Office,
        );
        let t = Duration::from_secs(2);
        assert!(d.tick(t).draw);
        d.drew(false, false, t);
        assert_eq!(
            d.wake_at(t),
            Some(t + Duration::from_millis(100)),
            "retried in 100 ms"
        );
        assert_eq!(run(&mut d, 2000, 3000, false), 1, "then drawn once");
    }

    #[test]
    fn a_camera_move_draws_until_it_lands() {
        let mut d = driver(Motion::Calm);
        run(&mut d, 0, 100, false);
        d.input(
            Input::Key(Key::Char('2')),
            Duration::from_millis(100),
            &mut Office,
        );
        let frames = run(&mut d, 100, 2000, false);
        assert!(
            (25..=35).contains(&frames),
            "a 0.45 s move at ~60 a second: {frames}"
        );
        assert_eq!(d.view.focus, Some(1));
        // AJ-E3516: with Reduce Motion the lift cuts, one frame
        let mut d = driver(Motion::Off);
        run(&mut d, 0, 100, false);
        d.input(
            Input::Key(Key::PageUp),
            Duration::from_millis(100),
            &mut Office,
        );
        assert_eq!(run(&mut d, 100, 2000, false), 1, "a cut");
        assert_eq!(d.view.focus, Some(1));
    }

    /// AJ-E3507 R4: H brings the camera home after a turn, a tilt and a
    /// zoom, on the same floor.
    #[test]
    fn h_brings_the_camera_home() {
        let mut d = driver(Motion::Off);
        run(&mut d, 0, 100, false);
        let home = d.view.cam;
        let mut at = 100;
        for key in [Key::Char('q'), Key::Up, Key::Char('+')] {
            d.input(Input::Key(key), Duration::from_millis(at), &mut Office);
            run(&mut d, at, at + 100, false);
            at += 100;
        }
        assert_ne!(d.view.cam, home, "turned, tilted and zoomed");
        let floor = d.view.focus;
        d.input(
            Input::Key(Key::Char('h')),
            Duration::from_millis(at),
            &mut Office,
        );
        run(&mut d, at, at + 100, false);
        assert_eq!(d.view.cam, home);
        assert_eq!(d.view.focus, floor, "home on the same floor");
    }

    /// AJ-E3516, AJ-E3519: a lift button rides to its floor; Page Up
    /// and Page Down stop at the top and the bottom, and Page Down from
    /// the building enters the top floor; the View button asks for its
    /// menu under itself.
    #[test]
    fn lift_buttons_page_keys_and_the_view_button() {
        let mut d = driver(Motion::Off);
        let t = Duration::from_secs(1);
        // chips and buttons in device pixels; clicks in points (scale 2)
        d.chips = vec![([1500.0, 400.0, 40.0, 40.0], Some(1))];
        assert_eq!(
            d.input(Input::Click { x: 755.0, y: 205.0 }, t, &mut Office),
            None
        );
        assert_eq!(d.view.focus, Some(1), "the up button rides to floor 2");
        d.input(Input::Key(Key::PageUp), t, &mut Office);
        assert_eq!(d.view.focus, Some(1), "no floor above the top");
        d.input(Input::Key(Key::PageDown), t, &mut Office);
        assert_eq!(d.view.focus, Some(0));
        d.input(Input::Key(Key::PageDown), t, &mut Office);
        assert_eq!(d.view.focus, Some(0), "no floor below the bottom");
        d.input(Input::Key(Key::Char('0')), t, &mut Office);
        assert_eq!(d.view.focus, None);
        d.input(Input::Key(Key::PageDown), t, &mut Office);
        assert_eq!(
            d.view.focus,
            Some(1),
            "down from the building: the top floor"
        );
        d.chips.clear();
        d.view_button = Some([1500.0, 10.0, 80.0, 28.0]);
        assert_eq!(
            d.input(Input::Click { x: 760.0, y: 10.0 }, t, &mut Office),
            Some(Effect::ViewMenu((1500.0, 38.0)))
        );
    }

    #[test]
    fn clicks_open_doors_and_play_with_toys() {
        let mut d = driver(Motion::Full);
        let t = Duration::from_secs(1);
        assert_eq!(
            d.input(Input::Click { x: 75.0, y: 10.0 }, t, &mut Office),
            Some(Effect::Open(7))
        );
        assert_eq!(
            d.input(Input::Click { x: 150.0, y: 10.0 }, t, &mut Office),
            Some(Effect::Play(9))
        );
        assert!(d.clicked.contains_key(&9));
        assert_eq!(
            d.input(Input::Click { x: 10.0, y: 10.0 }, t, &mut Office),
            None
        );
        d.chips = vec![([0.0, 0.0, 50.0, 50.0], None)];
        assert_eq!(
            d.input(Input::Click { x: 10.0, y: 10.0 }, t, &mut Office),
            None
        );
        assert_eq!(d.view.focus, None, "the Building chip");
    }

    /// 0.4.1, finding 1: anything a click acts on shows the hand; the
    /// floor tabs too, and nothing else does.
    #[test]
    fn the_hand_shows_over_doors_toys_and_floor_tabs_only() {
        let mut d = driver(Motion::Full);
        let t = Duration::from_secs(1);
        for (x, hand) in [(75.0, true), (150.0, true), (10.0, false)] {
            d.input(Input::Move { x, y: 10.0 }, t, &mut Office);
            assert_eq!(d.hand, hand, "at x = {x}");
        }
        d.chips = vec![([0.0, 0.0, 50.0, 50.0], Some(1))];
        d.input(Input::Move { x: 10.0, y: 10.0 }, t, &mut Office);
        assert!(d.hand, "over a floor tab");
        assert_eq!(d.tab_at(10.0, 10.0), Some(Some(1)));
        assert_eq!(d.tab_at(60.0, 10.0), None);
    }

    #[test]
    fn the_alert_keeps_it_ticking_even_at_rest() {
        let mut d = driver(Motion::Calm);
        run(&mut d, 0, 100, false);
        d.input(Input::Alert(true), Duration::from_millis(100), &mut Office);
        assert!(run(&mut d, 100, 5100, false) >= 18);
    }

    #[test]
    fn tab_steps_through_areas_n_through_the_waiting_and_enter_opens() {
        use super::Stop;
        let mut d = driver(Motion::Off);
        let t = Duration::from_secs(1);
        d.areas = vec![
            Stop {
                floor: 0,
                at: [10.0, 0.0, 5.0],
                id: 11,
            },
            Stop {
                floor: 1,
                at: [20.0, 15.0, 5.0],
                id: 12,
            },
        ];
        d.waiting = vec![Stop {
            floor: 1,
            at: [21.0, 15.0, 6.0],
            id: 40,
        }];
        assert_eq!(
            d.input(Input::Key(Key::Enter), t, &mut Office),
            None,
            "nothing selected yet"
        );
        d.input(Input::Key(Key::Tab), t, &mut Office);
        assert_eq!(d.selected, Some(11));
        d.input(Input::Key(Key::Tab), t, &mut Office);
        assert_eq!(
            (d.selected, d.view.focus),
            (Some(12), Some(1)),
            "the camera follows to its floor"
        );
        d.input(Input::Key(Key::BackTab), t, &mut Office);
        assert_eq!(d.selected, Some(11));
        d.input(Input::Key(Key::Char('n')), t, &mut Office);
        assert_eq!(d.selected, Some(40));
        assert_eq!(
            d.input(Input::Key(Key::Enter), t, &mut Office),
            Some(Effect::Open(40))
        );
        d.input(Input::Move { x: 1.0, y: 1.0 }, t, &mut Office);
        assert_eq!(d.selected, None, "the pointer takes over");
    }
}
