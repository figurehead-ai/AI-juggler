//! Each App's kit (SPEC §9.7, AJ-E3504): the look of its area.
//!
//! - The family (its tool's data) sets the palette and the furniture
//!   family; the App's saved [`Choice`] (a layout and a seed) picks the
//!   variant inside it, once, the first time the Floor sees the App.
//! - Big things vary between kits (floor, wall, furniture, layout); small
//!   things (props, wear, people) vary per desk in the scene.
//! - The App's colour (its colour slot) marks its sign, flag, rug, badges
//!   and its logo's ring.

use serde::{Deserialize, Serialize};

use crate::colour::{Rgba, hex, mark_colour};
use crate::family::{Family, Layout};
use crate::rng::{Rng, hash};

/// What the Floor keeps per App so its look survives restarts, reflows
/// and new versions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Choice {
    /// How its desks are arranged.
    pub layout: Layout,
    /// The seed for everything else.
    pub seed: u32,
}

/// The first-time choice for an App: from the office seed (Re-create
/// changes it), its family and its id.
#[must_use]
pub fn choose(family: Family, app_id: &str, office_seed: u32) -> Choice {
    let mut r = Rng::of(&format!(
        "choice|{office_seed}|{}|{app_id}",
        family.as_str()
    ));
    Choice {
        layout: *r.pick(family.layouts()),
        seed: hash(&format!("seed|{office_seed}|{app_id}")),
    }
}

/// A floor's material.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FloorMat {
    /// Long boards.
    Planks {
        /// The wood.
        wood: Rgba,
        /// A board's width, metres.
        width: f32,
    },
    /// Herringbone blocks.
    Parquet {
        /// The wood.
        wood: Rgba,
        /// A block's length, metres.
        block: f32,
    },
    /// Carpet tiles in two tones.
    Carpet {
        /// One tone.
        a: Rgba,
        /// The other.
        b: Rgba,
        /// A tile's side, metres.
        tile: f32,
    },
    /// Polished concrete with seams.
    Concrete {
        /// Its tone.
        tone: Rgba,
        /// Between seams, metres.
        seam: f32,
    },
    /// Terrazzo: a base with coloured chips.
    Terrazzo {
        /// The base.
        base: Rgba,
        /// The chips.
        chips: [Rgba; 3],
    },
    /// Rubber sheet.
    Rubber {
        /// Its colour.
        c: Rgba,
    },
}

impl FloorMat {
    /// The surface it's made of (0.4.2's second pass).
    #[must_use]
    pub const fn mat(&self) -> crate::materials::Mat {
        use crate::materials as m;
        match self {
            Self::Planks { .. } | Self::Parquet { .. } => m::WOOD,
            Self::Carpet { .. } => m::CARPET,
            Self::Concrete { .. } => m::PLASTER,
            Self::Terrazzo { .. } => m::CERAMIC,
            Self::Rubber { .. } => m::RUBBER,
        }
    }
}

/// A wall's finish.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WallFinish {
    /// Paint above a dado rail.
    Paint {
        /// The paint.
        c: Rgba,
        /// The dado.
        dado: Rgba,
    },
    /// Brick and mortar.
    Brick {
        /// The brick.
        c: Rgba,
        /// The mortar.
        mortar: Rgba,
    },
    /// Wood slats on a dark back.
    Slats {
        /// The slats.
        wood: Rgba,
        /// Behind them.
        back: Rgba,
    },
    /// Tiles and grout.
    Tiles {
        /// The tile.
        c: Rgba,
        /// The grout.
        grout: Rgba,
    },
    /// Two-tone stripes (supergraphics).
    Stripes {
        /// One stripe.
        a: Rgba,
        /// The other.
        b: Rgba,
    },
    /// Acoustic panels.
    Acoustic {
        /// One panel tone.
        a: Rgba,
        /// The other.
        b: Rgba,
    },
}

impl WallFinish {
    /// The surface it's made of (0.4.2's second pass).
    #[must_use]
    pub const fn mat(&self) -> crate::materials::Mat {
        use crate::materials as m;
        match self {
            Self::Paint { .. } | Self::Brick { .. } | Self::Stripes { .. } => m::PLASTER,
            Self::Slats { .. } => m::WOOD,
            Self::Tiles { .. } => m::CERAMIC,
            Self::Acoustic { .. } => m::FABRIC,
        }
    }
}

/// A desk model.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeskKind {
    /// A table on four legs.
    FourLeg,
    /// A top on two trestles.
    Trestle,
    /// A top on a drawer pedestal.
    Pedestal,
    /// A tall standing desk.
    Standing,
    /// An L-shaped corner desk.
    LShape,
    /// A small round table.
    Round,
}

/// A chair type.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChairKind {
    /// A task chair on casters.
    Task,
    /// A mesh-backed chair.
    Mesh,
    /// A stool.
    Stool,
    /// A saddle stool.
    Saddle,
    /// A ball chair.
    Ball,
}

/// What stands on a desk to work at.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screens {
    /// One monitor.
    Single,
    /// Two monitors.
    Dual,
    /// A laptop.
    Laptop,
    /// One wide monitor.
    Wide,
}

/// A plant.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Plant {
    /// Split-leaf monstera.
    Monstera,
    /// A fern.
    Fern,
    /// A snake plant.
    Snake,
    /// A cactus.
    Cactus,
    /// A fiddle-leaf fig.
    Fig,
    /// A palm.
    Palm,
    /// A box of herbs.
    Herbs,
    /// Succulents.
    Succulent,
}

/// A pot.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pot {
    /// Terracotta.
    Terracotta,
    /// Glazed ceramic.
    Ceramic(Rgba),
    /// A painted box.
    Box(Rgba),
    /// A woven basket.
    Basket,
    /// Concrete.
    Concrete,
}

/// What divides an area from the aisle.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Divider {
    /// A row of planters.
    Planters,
    /// Open shelves.
    Shelves,
    /// A glass screen.
    Glass,
    /// A low wall.
    LowWall,
    /// Lockers.
    Lockers,
    /// Pegboard.
    Pegboard,
}

/// An area's corner to step away to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Nook {
    /// A sofa and a low table.
    Sofa,
    /// Beanbags.
    Beanbags,
    /// A huddle table.
    Huddle,
    /// Bookcases and an armchair.
    Library,
    /// A workbench.
    Workbench,
    /// A ping-pong table.
    PingPong,
    /// An arcade cabinet.
    Arcade,
}

/// How the area's sign hangs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SignKind {
    /// A plate.
    Plate,
    /// Neon letters.
    Neon,
    /// A cloth banner.
    Banner,
    /// Pennants.
    Pennant,
    /// A blade sign at right angles.
    Blade,
}

/// One App's kit.
#[derive(Clone, Debug, PartialEq)]
pub struct Kit {
    /// Its family.
    pub family: Family,
    /// The App's colour.
    pub accent: Rgba,
    /// How its desks are arranged.
    pub layout: Layout,
    /// Up to this much random turn per desk group, radians.
    pub jitter: f32,
    /// The floor.
    pub floor: FloorMat,
    /// The walls.
    pub wall: WallFinish,
    /// The desk model.
    pub desk: DeskKind,
    /// The desks' top.
    pub desk_top: Rgba,
    /// The desks' frame.
    pub desk_frame: Rgba,
    /// The chair type.
    pub chair: ChairKind,
    /// The chairs' colour.
    pub chair_colour: Rgba,
    /// What stands on the desks.
    pub screens: Screens,
    /// The plants it uses.
    pub plants: Vec<Plant>,
    /// Their pots.
    pub pot: Pot,
    /// How many plants it has, roughly.
    pub greenery: usize,
    /// Its divider.
    pub divider: Divider,
    /// Its nook.
    pub nook: Nook,
    /// How warm its light is, 0–1.
    pub warmth: f32,
    /// Its sign.
    pub sign: SignKind,
    /// Its rug.
    pub rug: Rgba,
    /// Desk lamps.
    pub lamps: bool,
    /// Pendant lights.
    pub pendants: bool,
}

impl Kit {
    /// The variant in words (the kit table, a log line).
    #[must_use]
    pub fn variant(&self) -> String {
        let floor = match self.floor {
            FloorMat::Planks { .. } => "planks",
            FloorMat::Parquet { .. } => "parquet",
            FloorMat::Carpet { .. } => "carpet",
            FloorMat::Concrete { .. } => "concrete",
            FloorMat::Terrazzo { .. } => "terrazzo",
            FloorMat::Rubber { .. } => "rubber",
        };
        let wall = match self.wall {
            WallFinish::Paint { .. } => "paint",
            WallFinish::Brick { .. } => "brick",
            WallFinish::Slats { .. } => "slats",
            WallFinish::Tiles { .. } => "tiles",
            WallFinish::Stripes { .. } => "stripes",
            WallFinish::Acoustic { .. } => "acoustic",
        };
        format!(
            "{} · {} layout, {floor}, {wall}, {:?} desks, {:?} chairs, {:?} divider, {:?} nook, {:?} sign",
            self.family.name(),
            self.layout.as_str(),
            self.desk,
            self.chair,
            self.divider,
            self.nook,
            self.sign
        )
    }
}

/// The kit for an App: its family's look, varied by its saved choice,
/// marked with its colour slot.
#[must_use]
pub fn kit_for(family: Family, choice: Choice, slot: usize) -> Kit {
    let mut r = Rng(choice.seed | 1);
    let colour = |r: &mut Rng, xs: &[&str]| hex(r.pick(xs));
    let base = |floor, wall| Kit {
        family,
        accent: mark_colour(slot),
        layout: choice.layout,
        jitter: family.jitter(),
        floor,
        wall,
        desk: DeskKind::FourLeg,
        desk_top: hex("#e8e4dc"),
        desk_frame: hex("#4a4a52"),
        chair: ChairKind::Task,
        chair_colour: hex("#3a3d43"),
        screens: Screens::Single,
        plants: vec![Plant::Fern],
        pot: Pot::Ceramic(hex("#f4efe6")),
        greenery: 3,
        divider: Divider::Planters,
        nook: Nook::Huddle,
        warmth: 0.6,
        sign: SignKind::Plate,
        rug: hex("#3a3d43"),
        lamps: false,
        pendants: false,
    };
    match family {
        Family::Workshop => workshop(&mut r, base, colour),
        Family::Studio => studio(&mut r, base, colour),
        Family::Industrial => industrial(&mut r, base, colour),
        Family::MakerGarage => garage(&mut r, base, colour),
        Family::Campus => campus(&mut r, base, colour),
        Family::Lab => lab(&mut r, base, colour),
        Family::Neutral => neutral(&mut r, base, colour),
    }
}

type Pick = fn(&mut Rng, &[&str]) -> Rgba;

fn workshop(r: &mut Rng, base: impl Fn(FloorMat, WallFinish) -> Kit, colour: Pick) -> Kit {
    let wood = colour(r, &["#b07a4a", "#9a6238", "#c08b57", "#7d5434"]);
    let floor = if r.chance(0.5) {
        FloorMat::Parquet { wood, block: 0.6 }
    } else {
        FloorMat::Planks { wood, width: 0.24 }
    };
    let wall = match r.int(3) {
        0 => WallFinish::Brick {
            c: colour(r, &["#a4543a", "#b5654a", "#8f4a35"]),
            mortar: hex("#d8c7b4"),
        },
        1 => WallFinish::Paint {
            c: colour(r, &["#c86f4f", "#8a9a6e", "#c9925c"]),
            dado: hex("#6b4a32"),
        },
        _ => WallFinish::Slats {
            wood: colour(r, &["#8a5a36", "#a06a40"]),
            back: hex("#3b2a22"),
        },
    };
    Kit {
        desk: *r.pick(&[DeskKind::Trestle, DeskKind::FourLeg, DeskKind::Pedestal]),
        desk_top: colour(r, &["#c99764", "#b5824f", "#d4a873"]),
        desk_frame: hex("#4a3324"),
        chair: ChairKind::Task,
        chair_colour: colour(r, &["#a3502f", "#5c6b3b", "#2f4a6b", "#7a3b4a"]),
        screens: *r.pick(&[Screens::Single, Screens::Dual]),
        plants: vec![Plant::Monstera, Plant::Fern, Plant::Fig],
        pot: *r.pick(&[Pot::Terracotta, Pot::Basket]),
        greenery: 4 + r.int(3),
        divider: *r.pick(&[Divider::Shelves, Divider::Planters]),
        nook: *r.pick(&[Nook::Library, Nook::Sofa]),
        warmth: 0.9,
        sign: *r.pick(&[SignKind::Plate, SignKind::Banner]),
        rug: colour(r, &["#7a3b2e", "#4a5a3a", "#3a4a6a"]),
        lamps: true,
        pendants: r.chance(0.5),
        ..base(floor, wall)
    }
}

fn studio(r: &mut Rng, base: impl Fn(FloorMat, WallFinish) -> Kit, colour: Pick) -> Kit {
    let floor = if r.chance(0.6) {
        FloorMat::Planks {
            wood: colour(r, &["#e1c9a2", "#d8bd94", "#e8d3b0"]),
            width: 0.24,
        }
    } else {
        FloorMat::Carpet {
            a: hex("#c9c3d6"),
            b: hex("#bdb6cc"),
            tile: 0.5,
        }
    };
    let wall = if r.chance(0.6) {
        WallFinish::Paint {
            c: colour(r, &["#e8c4c0", "#c3dccd", "#c4d4e8", "#e6dcc0"]),
            dado: hex("#f4efe6"),
        }
    } else {
        WallFinish::Stripes {
            a: hex("#f1e6d6"),
            b: colour(r, &["#e8b9b0", "#b9d8c6", "#b8c8e6"]),
        }
    };
    Kit {
        desk: *r.pick(&[DeskKind::FourLeg, DeskKind::Round]),
        desk_top: hex("#f2f0ea"),
        desk_frame: hex("#c9c5bb"),
        chair: *r.pick(&[ChairKind::Stool, ChairKind::Task]),
        chair_colour: colour(r, &["#e6a99a", "#9cc7b1", "#a8b9de", "#e8cf8e"]),
        screens: Screens::Laptop,
        plants: vec![Plant::Fig, Plant::Succulent, Plant::Monstera],
        pot: Pot::Ceramic(colour(r, &["#f4efe6", "#e9c9b8", "#cfe0d6"])),
        greenery: 3 + r.int(2),
        divider: *r.pick(&[Divider::Glass, Divider::Planters]),
        nook: *r.pick(&[Nook::Sofa, Nook::Huddle]),
        warmth: 0.65,
        sign: SignKind::Banner,
        rug: colour(r, &["#e9d7c4", "#d6e3dc", "#e3d6e6"]),
        pendants: true,
        ..base(floor, wall)
    }
}

fn industrial(r: &mut Rng, base: impl Fn(FloorMat, WallFinish) -> Kit, colour: Pick) -> Kit {
    let floor = FloorMat::Concrete {
        tone: colour(r, &["#8d8f94", "#7a7c82", "#9a948a"]),
        seam: if r.chance(0.5) { 3.0 } else { 2.0 },
    };
    let wall = if r.chance(0.55) {
        WallFinish::Brick {
            c: colour(r, &["#7e3b2f", "#d9d4cc", "#6b6560"]),
            mortar: hex("#4a4440"),
        }
    } else {
        WallFinish::Tiles {
            c: colour(r, &["#8a8f96", "#a59f95"]),
            grout: hex("#5a5e64"),
        }
    };
    Kit {
        desk: *r.pick(&[DeskKind::FourLeg, DeskKind::Pedestal]),
        desk_top: colour(r, &["#2d2f34", "#6b4c33", "#3a3d43"]),
        desk_frame: hex("#1c1d21"),
        chair: ChairKind::Mesh,
        chair_colour: hex("#25262b"),
        screens: *r.pick(&[Screens::Dual, Screens::Wide]),
        plants: vec![Plant::Snake, Plant::Cactus],
        pot: Pot::Concrete,
        greenery: 2 + r.int(2),
        divider: *r.pick(&[Divider::Lockers, Divider::LowWall]),
        nook: *r.pick(&[Nook::Huddle, Nook::Arcade]),
        warmth: 0.55,
        sign: *r.pick(&[SignKind::Neon, SignKind::Blade]),
        rug: hex("#3a3d43"),
        pendants: true,
        ..base(floor, wall)
    }
}

fn garage(r: &mut Rng, base: impl Fn(FloorMat, WallFinish) -> Kit, colour: Pick) -> Kit {
    let floor = FloorMat::Rubber {
        c: colour(r, &["#3f5a4a", "#3d4a5c", "#55524c"]),
    };
    let wall = if r.chance(0.6) {
        WallFinish::Slats {
            wood: hex("#c9a46c"),
            back: hex("#6b5638"),
        }
    } else {
        WallFinish::Paint {
            c: colour(r, &["#2f4a3f", "#3a3f55"]),
            dado: hex("#c9a46c"),
        }
    };
    Kit {
        desk: DeskKind::Standing,
        desk_top: hex("#d2b07a"),
        desk_frame: hex("#3a3a3e"),
        chair: *r.pick(&[ChairKind::Stool, ChairKind::Saddle]),
        chair_colour: colour(r, &["#e0a03a", "#3a8fb0"]),
        screens: *r.pick(&[Screens::Laptop, Screens::Dual]),
        plants: vec![Plant::Herbs, Plant::Cactus, Plant::Snake],
        pot: Pot::Box(hex("#c9a46c")),
        greenery: 2 + r.int(2),
        divider: Divider::Pegboard,
        nook: *r.pick(&[Nook::Workbench, Nook::PingPong]),
        sign: SignKind::Neon,
        rug: hex("#2f3a44"),
        lamps: true,
        ..base(floor, wall)
    }
}

fn campus(r: &mut Rng, base: impl Fn(FloorMat, WallFinish) -> Kit, colour: Pick) -> Kit {
    let floor = if r.chance(0.5) {
        FloorMat::Terrazzo {
            base: hex("#e7e1d6"),
            chips: [hex("#d9644a"), hex("#3f7fbf"), hex("#e8b33a")],
        }
    } else {
        FloorMat::Carpet {
            a: hex("#5a7fb0"),
            b: hex("#4f72a2"),
            tile: 0.5,
        }
    };
    let wall = if r.chance(0.5) {
        WallFinish::Stripes {
            a: hex("#f0e8da"),
            b: colour(r, &["#e8b33a", "#d9644a", "#3f7fbf"]),
        }
    } else {
        WallFinish::Acoustic {
            a: hex("#3f7fbf"),
            b: hex("#e8b33a"),
        }
    };
    Kit {
        desk: *r.pick(&[DeskKind::Round, DeskKind::FourLeg]),
        desk_top: hex("#ece6da"),
        chair: *r.pick(&[ChairKind::Ball, ChairKind::Task]),
        chair_colour: colour(r, &["#d9644a", "#3f7fbf", "#e8b33a", "#4aa37a"]),
        plants: vec![Plant::Palm, Plant::Monstera],
        pot: Pot::Ceramic(colour(r, &["#e8b33a", "#3f7fbf"])),
        nook: *r.pick(&[Nook::Beanbags, Nook::PingPong]),
        warmth: 0.7,
        sign: SignKind::Pennant,
        rug: hex("#e8b33a"),
        ..base(floor, wall)
    }
}

fn lab(r: &mut Rng, base: impl Fn(FloorMat, WallFinish) -> Kit, colour: Pick) -> Kit {
    let floor = FloorMat::Carpet {
        a: colour(r, &["#3a3f4a", "#2f3a45", "#3d3a48"]),
        b: hex("#353a44"),
        tile: *r.pick(&[0.5, 0.6]),
    };
    let wall = WallFinish::Acoustic {
        a: hex("#2f3440"),
        b: colour(r, &["#3a404d", "#344a4f", "#3f3a4f"]),
    };
    Kit {
        desk: DeskKind::LShape,
        desk_top: hex("#2a2d33"),
        desk_frame: hex("#16171a"),
        chair: ChairKind::Mesh,
        chair_colour: hex("#1f2024"),
        screens: Screens::Dual,
        plants: vec![Plant::Snake, Plant::Succulent],
        pot: Pot::Concrete,
        greenery: 2,
        divider: Divider::Glass,
        warmth: 0.3,
        sign: *r.pick(&[SignKind::Blade, SignKind::Plate]),
        rug: hex("#2a2d33"),
        lamps: true,
        ..base(floor, wall)
    }
}

fn neutral(r: &mut Rng, base: impl Fn(FloorMat, WallFinish) -> Kit, colour: Pick) -> Kit {
    let floor = FloorMat::Carpet {
        a: colour(r, &["#8a8e96", "#7f8790", "#958f86"]),
        b: hex("#80848c"),
        tile: 0.5,
    };
    let wall = WallFinish::Paint {
        c: colour(r, &["#d8d6d0", "#cfd6dc", "#dcd4c8"]),
        dado: hex("#9a9aa0"),
    };
    Kit {
        desk: *r.pick(&[DeskKind::FourLeg, DeskKind::Pedestal]),
        chair: ChairKind::Task,
        chair_colour: colour(r, &["#3a3d43", "#4a5568", "#5a4a3a"]),
        screens: *r.pick(&[Screens::Single, Screens::Dual]),
        plants: vec![Plant::Fern, Plant::Snake],
        divider: *r.pick(&[Divider::Planters, Divider::LowWall]),
        nook: *r.pick(&[Nook::Sofa, Nook::Huddle]),
        sign: SignKind::Plate,
        ..base(floor, wall)
    }
}

#[cfg(test)]
mod tests {
    use super::{Choice, choose, kit_for};
    use crate::family::{Family, Layout};

    #[test]
    fn a_choice_is_the_same_for_the_same_app_and_seed_and_new_after_recreate() {
        let a = choose(Family::Workshop, "cli:work", 7);
        assert_eq!(a, choose(Family::Workshop, "cli:work", 7));
        assert!(Family::Workshop.layouts().contains(&a.layout));
        let others: Vec<_> = (0..20)
            .map(|seed| choose(Family::Workshop, "cli:work", seed).seed)
            .collect();
        assert!(
            others.windows(2).any(|pair| pair[0] != pair[1]),
            "Re-create picks again"
        );
    }

    #[test]
    fn two_apps_of_one_family_differ_in_big_things_and_families_differ() {
        // Over many pairs, two Apps of one family nearly always differ in
        // at least two of floor, wall, layout and sign (the oatmeal rule).
        for family in Family::ALL {
            let mut alike = 0;
            for n in 0..50 {
                let one = kit_for(family, choose(family, &format!("app-{n}"), 1), 0);
                let two = kit_for(family, choose(family, &format!("app-{n}-b"), 1), 1);
                let differ = [
                    one.floor != two.floor,
                    one.wall != two.wall,
                    one.layout != two.layout,
                    one.sign != two.sign,
                ]
                .into_iter()
                .filter(|d| *d)
                .count();
                if differ < 2 {
                    alike += 1;
                }
            }
            assert!(alike <= 30, "{family:?}: {alike} of 50 pairs too alike");
        }
        let workshop = kit_for(
            Family::Workshop,
            Choice {
                layout: Layout::Pods,
                seed: 9,
            },
            0,
        );
        let industrial = kit_for(
            Family::Industrial,
            Choice {
                layout: Layout::Pods,
                seed: 9,
            },
            0,
        );
        assert_ne!(workshop.family, industrial.family);
        assert_ne!(workshop.floor, industrial.floor);
    }

    #[test]
    fn building_every_kit_is_quick() {
        let started = std::time::Instant::now();
        for n in 0..20 {
            for family in Family::ALL {
                let _ = kit_for(family, choose(family, &format!("app-{n}"), 3), n);
            }
        }
        assert!(
            started.elapsed().as_millis() < 50,
            "{:?}",
            started.elapsed()
        );
    }
}
