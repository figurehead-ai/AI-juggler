//! Style families (SPEC §9.7, D-67): a tool's areas all read as one
//! family, so every Codex App looks industrial and every Claude Code App
//! a workshop. The family is the tool's own data (its adapter's
//! `ToolSpec::floor_family`, a word); nothing here, or anywhere, branches
//! on a tool. A tool that names none, or one this build doesn't know,
//! gets the neutral office.

use serde::{Deserialize, Serialize};

/// How desks are arranged in an area.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Layout {
    /// Clusters of four facing desks.
    Pods,
    /// Long rows of desks, two facing rows per bench.
    Bench,
    /// Three desks at 120° round a shared centre.
    Pinwheel,
    /// Freestanding desks at loose angles, with clearance.
    Scatter,
    /// Long shared tables, seats both sides.
    Tables,
    /// Rows turned 45° to the room.
    Diagonal,
    /// Desks round a central planter, facing out.
    Radial,
}

impl Layout {
    /// Every layout, in a fixed order.
    pub const ALL: [Self; 7] = [
        Self::Pods,
        Self::Bench,
        Self::Pinwheel,
        Self::Scatter,
        Self::Tables,
        Self::Diagonal,
        Self::Radial,
    ];

    /// The stored word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pods => "pods",
            Self::Bench => "bench",
            Self::Pinwheel => "pinwheel",
            Self::Scatter => "scatter",
            Self::Tables => "tables",
            Self::Diagonal => "diagonal",
            Self::Radial => "radial",
        }
    }

    /// The layout a stored word names.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|layout| layout.as_str() == word)
    }
}

/// A style family.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Family {
    /// Warm timber, brick, craft lamps, a reading nook, lots of green.
    Workshop,
    /// Pale oak, soft pastel walls, white tables, ceramics.
    Studio,
    /// Polished concrete, brick or panels, black steel, Edison pendants.
    Industrial,
    /// Rubber floor, plywood and pegboard, standing desks, neon.
    MakerGarage,
    /// Terrazzo or bold carpet, supergraphics, beanbags, bright chairs.
    Campus,
    /// Dark carpet, acoustic panels, L-shaped desks, cool light.
    Lab,
    /// A calm, plain office: the tool named no family.
    Neutral,
}

impl Family {
    /// Every family, the neutral one last.
    pub const ALL: [Self; 7] = [
        Self::Workshop,
        Self::Studio,
        Self::Industrial,
        Self::MakerGarage,
        Self::Campus,
        Self::Lab,
        Self::Neutral,
    ];

    /// The word a tool's spec names it by.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Workshop => "workshop",
            Self::Studio => "studio",
            Self::Industrial => "industrial",
            Self::MakerGarage => "maker-garage",
            Self::Campus => "campus",
            Self::Lab => "lab",
            Self::Neutral => "neutral",
        }
    }

    /// Its name on screen.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Workshop => "Workshop",
            Self::Studio => "Studio",
            Self::Industrial => "Industrial",
            Self::MakerGarage => "Maker garage",
            Self::Campus => "Campus",
            Self::Lab => "Lab",
            Self::Neutral => "Office",
        }
    }

    /// The family a tool's word names; the neutral office for none, or a
    /// word this build doesn't know.
    #[must_use]
    pub fn of(word: Option<&str>) -> Self {
        word.and_then(|word| Self::ALL.into_iter().find(|family| family.as_str() == word))
            .unwrap_or(Self::Neutral)
    }

    /// The layouts its areas choose from; an App's saved choice is one.
    #[must_use]
    pub const fn layouts(self) -> &'static [Layout] {
        match self {
            Self::Workshop => &[Layout::Pods, Layout::Pinwheel, Layout::Scatter],
            Self::Studio => &[Layout::Tables, Layout::Scatter, Layout::Radial],
            Self::Industrial => &[Layout::Bench, Layout::Diagonal, Layout::Pods],
            Self::MakerGarage => &[Layout::Scatter, Layout::Pinwheel],
            Self::Campus => &[Layout::Pinwheel, Layout::Radial, Layout::Tables],
            Self::Lab => &[Layout::Pods, Layout::Diagonal],
            Self::Neutral => &[Layout::Pods, Layout::Bench],
        }
    }

    /// How far, at most, a desk group turns at random (radians): none for
    /// strict families, loose in the garage.
    #[must_use]
    pub const fn jitter(self) -> f32 {
        match self {
            Self::Workshop => 0.12,
            Self::Studio => 0.08,
            Self::Industrial | Self::Lab | Self::Neutral => 0.0,
            Self::MakerGarage => 0.3,
            Self::Campus => 0.2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Family, Layout};

    #[test]
    fn a_family_is_found_by_its_word_and_an_unknown_one_is_the_neutral_office() {
        for family in Family::ALL {
            assert_eq!(Family::of(Some(family.as_str())), family);
            assert_ne!(family.layouts().len(), 0);
        }
        assert_eq!(Family::of(None), Family::Neutral);
        assert_eq!(Family::of(Some("spaceship")), Family::Neutral);
        for layout in Layout::ALL {
            assert_eq!(Layout::parse(layout.as_str()), Some(layout));
        }
    }
}
