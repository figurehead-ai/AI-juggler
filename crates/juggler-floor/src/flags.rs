//! AJ-E3515: labels by zoom. How big a desk is on screen picks the band:
//! close (the scene speaks: balloons only), middle (a flag over each desk
//! that needs you, one name flag per area) or far (one flag per App that
//! needs you; the building view is always far). Flags stand on a pole
//! straight above what they mark and never move aside: one with no room
//! joins its neighbour's count, so no count is lost. Pure, for the tests;
//! the overlay draws what this decides.

/// Which labels show (Settings › The Floor and the View menu,
/// `floor.labels`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Labels {
    /// By zoom: what needs you, and area names in the middle band.
    #[default]
    Smart,
    /// Only what needs you.
    Attention,
    /// Every area's name too, in the middle and far bands.
    Names,
}

/// A zoom band.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Band {
    /// Desks ≥ [`CLOSE_PX`]: no labels besides balloons and the hover
    /// card.
    Close,
    /// Desks between [`FAR_PX`] and [`CLOSE_PX`]: a flag per desk that
    /// needs you, a name flag per area.
    Middle,
    /// Desks under [`FAR_PX`], and the building: a flag per App that
    /// needs you.
    Far,
}

/// A desk this wide on screen or more is close (points, on a
/// 640-point-wide view: [`crate::overlay::desk_px`]).
pub const CLOSE_PX: f32 = 28.0;
/// Under this it's far: a floor zoomed all the way out (a desk 11.8 at
/// 1.6 × the home view) is far, the home view (18.9) in the middle.
pub const FAR_PX: f32 = 14.0;
/// A band holds until the size passes its edge by this much (no flicker
/// at a zoom right on the edge).
pub const HYSTERESIS: f32 = 0.15;

/// The band for a desk `desk_px` wide on screen, given the band shown
/// before (`None` the first time).
#[must_use]
pub fn band(desk_px: f32, building: bool, before: Option<Band>) -> Band {
    if building {
        return Band::Far;
    }
    let edge = |px: f32, held: bool| {
        if before.is_none() {
            px
        } else if held {
            px * (1.0 - HYSTERESIS)
        } else {
            px * (1.0 + HYSTERESIS)
        }
    };
    let close = edge(CLOSE_PX, before == Some(Band::Close));
    let middle = edge(FAR_PX, matches!(before, Some(Band::Close | Band::Middle)));
    if desk_px >= close {
        Band::Close
    } else if desk_px >= middle {
        Band::Middle
    } else {
        Band::Far
    }
}

/// What a flag says needs you, most urgent first (the placement order).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Need {
    /// Crashed.
    Crashed,
    /// Asks you.
    Asks,
    /// Stuck.
    Stuck,
    /// The machine runs hot.
    Hot,
    /// Done.
    Done,
    /// Out of room (its Account's usage spent).
    Out,
    /// Low on room: its Account has [`LOW_LEFT`]% or less left.
    Low,
}

/// At or under this much left an Account is low on room: the app's one
/// rule (`ui/src/lib/room.ts`'s `LOW_LEFT`, its "Low" chip and the
/// suggestion rule), so the Floor's LOW says what the rest of the app does.
pub const LOW_LEFT: f32 = 25.0;

impl Need {
    /// The need a session in `state` shows (the hover card's badge; an
    /// App's flag counts them): none for working, idle and ended.
    #[must_use]
    pub const fn of_state(state: crate::state::FloorState) -> Option<Self> {
        use crate::state::FloorState;
        match state {
            FloorState::Crashed => Some(Self::Crashed),
            FloorState::Asks => Some(Self::Asks),
            FloorState::Stuck => Some(Self::Stuck),
            FloorState::Done => Some(Self::Done),
            FloorState::Limit => Some(Self::Out),
            FloorState::Working | FloorState::Idle | FloorState::Ended => None,
        }
    }

    /// Its word on a flag: over one desk (`ASKS`), or after a count
    /// (`2 ASKING`).
    #[must_use]
    pub const fn word(self, counted: bool) -> &'static str {
        match (self, counted) {
            (Self::Crashed, _) => "CRASHED",
            (Self::Asks, false) => "ASKS",
            (Self::Asks, true) => "ASKING",
            (Self::Stuck, _) => "STUCK",
            (Self::Hot, _) => "HOT",
            (Self::Done, _) => "DONE",
            (Self::Out, _) => "OUT",
            (Self::Low, _) => "LOW",
        }
    }

    /// Its colour: the pole's flag edge and its badge.
    #[must_use]
    pub const fn colour(self) -> [f32; 4] {
        match self {
            Self::Crashed => [0.94, 0.33, 0.31, 1.0],
            Self::Asks => [1.0, 0.83, 0.3, 1.0],
            Self::Stuck => [0.94, 0.65, 0.25, 1.0],
            Self::Hot => [1.0, 0.48, 0.24, 1.0],
            Self::Done => [0.37, 0.83, 0.55, 1.0],
            // the app's Out chip and Low chip: danger red, warning amber
            // (0.4.1: Out was a blue-grey the rest of the app didn't use)
            Self::Out => [0.97, 0.44, 0.44, 1.0],
            Self::Low => [0.96, 0.62, 0.04, 1.0],
        }
    }
}

/// A flag asking for a place.
#[derive(Clone, Debug, PartialEq)]
pub struct FlagAsk {
    /// What it marks, on screen: the pole's foot.
    pub anchor: (f32, f32),
    /// The flag's size, device pixels.
    pub size: (f32, f32),
    /// What needs you (`None`: a name flag, never counted).
    pub need: Option<Need>,
    /// How many it stands for.
    pub count: u32,
    /// Outlines it never touches (in the building view, the other
    /// floors: a flag stays over its own).
    pub keep_out: Vec<Vec<(f32, f32)>>,
    /// It may hang under what it marks when there's no room above (the
    /// building view, where above is the next floor).
    pub hang: bool,
}

/// Where a flag went.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Slot {
    /// Drawn here, standing for `total` (its own count and those that
    /// joined it).
    Placed {
        /// `(x, y, w, h)`.
        rect: [f32; 4],
        /// What it stands for in all.
        total: u32,
        /// How many flags joined it.
        joined: u32,
    },
    /// Joined the flag at this index (its count added there).
    Joined(usize),
    /// A name flag with no room: left out (names come back closer).
    Dropped,
}

/// The pole's lengths a flag tries, shortest first: `rise`, then a row
/// higher each time, up to three rows.
fn heights(rise: f32, row: f32) -> [f32; 4] {
    [rise, rise + row, rise + 2.0 * row, rise + 3.0 * row]
}

/// Places `asks`: what needs you first (by [`Need`]'s order, then nearer
/// the bottom of the screen), name flags last. Each stands straight over
/// its anchor (only kept on screen at the edges) on a pole of `rise` or
/// up to three rows more, clear of `taken`, the others and the screen's
/// edge by `gap`. One with no room joins the nearest placed flag that
/// needs you, so no count is lost; a name flag with no room is dropped.
#[must_use]
pub fn place_flags(
    asks: &[FlagAsk],
    taken: &[[f32; 4]],
    screen: (f32, f32),
    gap: f32,
    rise: f32,
) -> Vec<Slot> {
    let mut order: Vec<usize> = (0..asks.len()).collect();
    order.sort_by(|&a, &b| {
        let key = |n: usize| (asks[n].need.is_none(), asks[n].need);
        key(a)
            .cmp(&key(b))
            .then(asks[b].anchor.1.total_cmp(&asks[a].anchor.1))
    });
    let clear = |r: &[f32; 4], o: &[f32; 4]| {
        r[0] + r[2] + gap <= o[0]
            || o[0] + o[2] + gap <= r[0]
            || r[1] + r[3] + gap <= o[1]
            || o[1] + o[3] + gap <= r[1]
    };
    let mut slots = vec![Slot::Dropped; asks.len()];
    let mut placed: Vec<(usize, [f32; 4])> = Vec::new();
    for n in order {
        let a = &asks[n];
        let (w, h) = a.size;
        let x = (a.anchor.0 - w / 2.0)
            .clamp(gap, (screen.0 - w - gap).max(gap))
            .round();
        let above = heights(rise, h + gap).map(|pole| (a.anchor.1 - pole - h).round());
        let below = heights(rise, h + gap).map(|pole| (a.anchor.1 + pole).round());
        let tries = above
            .into_iter()
            .chain(below.into_iter().filter(|_| a.hang));
        let spot = tries.into_iter().find_map(|y| {
            let r = [x, y, w, h];
            let grown = [x - gap, y - gap, w + 2.0 * gap, h + 2.0 * gap];
            (y >= gap
                && y + h <= screen.1 - gap
                && !a.keep_out.iter().any(|p| crate::overlay::meets(&grown, p))
                && taken.iter().all(|t| clear(&r, t))
                && placed.iter().all(|(_, p)| clear(&r, p)))
            .then_some(r)
        });
        if let Some(rect) = spot {
            placed.push((n, rect));
            slots[n] = Slot::Placed {
                rect,
                total: a.count,
                joined: 0,
            };
            continue;
        }
        if a.need.is_none() {
            continue; // a name: it comes back closer
        }
        // no room: join the nearest flag that needs you
        let host = placed
            .iter()
            .filter(|(m, _)| asks[*m].need.is_some())
            .min_by(|(_, p), (_, q)| {
                let d =
                    |r: &[f32; 4]| (r[0] + r[2] / 2.0 - a.anchor.0).hypot(r[1] + r[3] - a.anchor.1);
                d(p).total_cmp(&d(q))
            })
            .map(|(m, _)| *m);
        if let Some(m) = host {
            if let Slot::Placed { total, joined, .. } = &mut slots[m] {
                *total += a.count;
                *joined += 1;
            }
            slots[n] = Slot::Joined(m);
        } else {
            // nothing to join (everything around is taken): it stands at
            // its first place anyway, rather than be lost
            let y = if a.hang {
                below[0]
            } else {
                (a.anchor.1 - rise - h).round()
            };
            let rect = [x, y, w, h];
            placed.push((n, rect));
            slots[n] = Slot::Placed {
                rect,
                total: a.count,
                joined: 0,
            };
        }
    }
    slots
}

#[cfg(test)]
mod tests {
    use super::{Band, FlagAsk, Need, Slot, band, place_flags};

    #[test]
    fn the_band_follows_a_desk_s_size_and_holds_near_its_edges() {
        assert_eq!(band(40.0, false, None), Band::Close);
        assert_eq!(band(20.0, false, None), Band::Middle);
        assert_eq!(band(6.0, false, None), Band::Far);
        // the home view is in the middle; zoomed all the way out, far
        assert_eq!(band(18.9, false, Some(Band::Middle)), Band::Middle);
        assert_eq!(band(11.8, false, Some(Band::Middle)), Band::Far);
        assert_eq!(band(400.0, true, None), Band::Far, "the building is far");
        // just under the close edge, a close view stays close…
        assert_eq!(band(26.0, false, Some(Band::Close)), Band::Close);
        // …and a middle one needs 15% more to become close
        assert_eq!(band(30.0, false, Some(Band::Middle)), Band::Middle);
        assert_eq!(band(33.0, false, Some(Band::Middle)), Band::Close);
        assert_eq!(band(12.5, false, Some(Band::Middle)), Band::Middle);
        assert_eq!(band(11.5, false, Some(Band::Far)), Band::Far);
        assert_eq!(band(15.0, false, Some(Band::Far)), Band::Far);
        assert_eq!(band(16.5, false, Some(Band::Far)), Band::Middle);
    }

    fn ask(x: f32, y: f32, need: Option<Need>, count: u32) -> FlagAsk {
        FlagAsk {
            anchor: (x, y),
            size: (80.0, 14.0),
            need,
            count,
            keep_out: Vec::new(),
            hang: false,
        }
    }

    fn rects(slots: &[Slot]) -> Vec<[f32; 4]> {
        slots
            .iter()
            .filter_map(|s| match s {
                Slot::Placed { rect, .. } => Some(*rect),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn flags_never_overlap_never_lose_a_count_and_stand_over_their_place() {
        // a crowd: twelve needs and four names around one spot, a band
        // taken at the top
        let mut asks = Vec::new();
        let needs = [
            Need::Done,
            Need::Asks,
            Need::Crashed,
            Need::Stuck,
            Need::Out,
            Need::Hot,
        ];
        for n in 0..12u32 {
            asks.push(ask(
                300.0 + (n % 4) as f32 * 30.0,
                300.0 + (n / 4) as f32 * 12.0,
                Some(needs[n as usize % needs.len()]),
                1 + n % 3,
            ));
        }
        for n in 0..4 {
            asks.push(ask(320.0 + n as f32 * 40.0, 320.0, None, 0));
        }
        let taken = [[0.0, 0.0, 640.0, 30.0]];
        let slots = place_flags(&asks, &taken, (640.0, 400.0), 2.0, 12.0);
        let r = rects(&slots);
        for (i, a) in r.iter().enumerate() {
            assert!(a[1] >= 32.0, "clear of what's taken: {a:?}");
            for b in &r[i + 1..] {
                let apart = a[0] + a[2] + 2.0 <= b[0]
                    || b[0] + b[2] + 2.0 <= a[0]
                    || a[1] + a[3] + 2.0 <= b[1]
                    || b[1] + b[3] + 2.0 <= a[1];
                assert!(apart, "{a:?} and {b:?} overlap");
            }
        }
        // every count is somewhere
        let asked: u32 = asks
            .iter()
            .filter(|a| a.need.is_some())
            .map(|a| a.count)
            .sum();
        let shown: u32 = slots
            .iter()
            .zip(&asks)
            .filter_map(|(s, a)| match s {
                Slot::Placed { total, .. } if a.need.is_some() => Some(*total),
                _ => None,
            })
            .sum();
        assert_eq!(asked, shown, "no count lost");
        // straight over its anchor: never aside
        for (s, a) in slots.iter().zip(&asks) {
            if let Slot::Placed { rect, .. } = s {
                assert!((rect[0] + rect[2] / 2.0 - a.anchor.0).abs() <= 1.0);
                assert!(rect[1] + rect[3] <= a.anchor.1, "above its place");
            }
        }
    }

    /// AJ-E3515's gate: over many random crowds, placed flags never
    /// overlap or cover what's taken, every count is shown somewhere, a
    /// flag stands straight over its place, and a flag joins only one as
    /// urgent or more.
    #[test]
    fn placement_holds_over_random_crowds() {
        let needs = [
            Need::Crashed,
            Need::Asks,
            Need::Stuck,
            Need::Hot,
            Need::Done,
            Need::Out,
        ];
        for seed in 0..400 {
            let mut r = crate::rng::Rng(seed);
            let n = 1 + r.int(24);
            let asks: Vec<FlagAsk> = (0..n)
                .map(|_| {
                    let need = (!r.chance(0.2)).then(|| needs[r.int(needs.len())]);
                    let mut a = ask(
                        r.range(20.0, 620.0),
                        r.range(60.0, 390.0),
                        need,
                        if need.is_some() {
                            1 + r.int(4) as u32
                        } else {
                            0
                        },
                    );
                    a.size = (r.range(40.0, 140.0), 14.0);
                    a
                })
                .collect();
            let taken = [[0.0, 0.0, 640.0, 30.0]];
            let slots = place_flags(&asks, &taken, (640.0, 400.0), 2.0, 12.0);
            let placed: Vec<(usize, [f32; 4])> = slots
                .iter()
                .enumerate()
                .filter_map(|(i, s)| match s {
                    Slot::Placed { rect, .. } => Some((i, *rect)),
                    _ => None,
                })
                .collect();
            for (k, (i, a)) in placed.iter().enumerate() {
                assert!(a[1] >= 30.0, "seed {seed}: {a:?} under what's taken");
                let anchor = asks[*i].anchor;
                assert!(
                    (a[0] + a[2] / 2.0 - anchor.0).abs() <= 1.0
                        || a[0] <= 2.5
                        || a[0] + a[2] >= 637.5,
                    "seed {seed}: aside"
                );
                for (_, b) in &placed[k + 1..] {
                    let apart = a[0] + a[2] <= b[0]
                        || b[0] + b[2] <= a[0]
                        || a[1] + a[3] <= b[1]
                        || b[1] + b[3] <= a[1];
                    assert!(apart, "seed {seed}: {a:?} and {b:?} overlap");
                }
            }
            let asked: u32 = asks
                .iter()
                .filter(|a| a.need.is_some())
                .map(|a| a.count)
                .sum();
            let shown: u32 = slots
                .iter()
                .zip(&asks)
                .filter_map(|(s, a)| match s {
                    Slot::Placed { total, .. } if a.need.is_some() => Some(*total),
                    _ => None,
                })
                .sum();
            assert_eq!(asked, shown, "seed {seed}: a count lost");
            for (i, s) in slots.iter().enumerate() {
                if let Slot::Joined(to) = s {
                    assert!(
                        asks[*to].need.is_some() && asks[*to].need <= asks[i].need,
                        "seed {seed}: {:?} joined a less urgent {:?}",
                        asks[i].need,
                        asks[*to].need
                    );
                }
            }
        }
    }

    #[test]
    fn the_most_urgent_flag_keeps_its_place() {
        // four flags on one spot with room for one: the crash stands, the
        // others join it
        let asks = vec![
            ask(300.0, 100.0, Some(Need::Done), 1),
            ask(300.0, 100.0, Some(Need::Asks), 2),
            ask(300.0, 100.0, Some(Need::Crashed), 1),
            ask(300.0, 100.0, Some(Need::Out), 1),
        ];
        // only the lowest height is free
        let taken = [[0.0, 0.0, 640.0, 70.0]];
        let slots = place_flags(&asks, &taken, (640.0, 400.0), 2.0, 12.0);
        assert!(
            matches!(
                slots[2],
                Slot::Placed {
                    total: 5,
                    joined: 3,
                    ..
                }
            ),
            "{slots:?}"
        );
        for n in [0, 1, 3] {
            assert_eq!(slots[n], Slot::Joined(2));
        }
    }

    #[test]
    fn in_the_building_a_flag_hangs_below_rather_than_climb_a_floor() {
        // above is another floor's outline: it hangs under its area
        let mut a = ask(300.0, 200.0, Some(Need::Asks), 1);
        a.keep_out = vec![vec![(0.0, 0.0), (640.0, 0.0), (640.0, 195.0), (0.0, 195.0)]];
        a.hang = true;
        let slots = place_flags(&[a], &[], (640.0, 400.0), 2.0, 12.0);
        assert!(
            matches!(slots[0], Slot::Placed { rect, .. } if rect[1] > 200.0),
            "below its area: {slots:?}"
        );
    }

    #[test]
    fn a_name_without_room_waits_for_a_closer_zoom() {
        let asks = vec![
            ask(300.0, 100.0, Some(Need::Asks), 1),
            ask(300.0, 100.0, None, 0),
        ];
        let taken = [[0.0, 0.0, 640.0, 70.0]];
        let slots = place_flags(&asks, &taken, (640.0, 400.0), 2.0, 12.0);
        assert!(matches!(slots[0], Slot::Placed { .. }));
        assert_eq!(slots[1], Slot::Dropped);
    }
}
