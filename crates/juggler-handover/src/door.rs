//! The hybrid door (AJ-E206; decision D-03; pass2/02 §8 verbatim).
//!
//! The door is chosen by *risk computed at preflight* — never by
//! habit or a setting: 1-click iff the mechanism is
//! transplant/native-resume AND fidelity is F3/F4 AND there are no
//! conflicts, no listed Drops, and target headroom ≥ Tight. Otherwise
//! the wizard opens, and the
//! wizard renders ONLY the steps whose data is non-trivial — 1-click
//! is the wizard with all conditional steps proven empty, not a
//! lesser flow. Ranking suggests; execution is always the user's
//! click; copy never says "we switched you".
//!
//! Amended (D-03, the maintainer, 2026-10-27): another account is the
//! point of a switch, so "not the usual account" is no risk and no
//! longer opens the wizard (the usual account stays a ranking hint);
//! and a compacted source loses nothing — a transplant copies the whole
//! session file and the target resumes it as the source tool would.

use crate::fidelity::FidelityLabel;
use crate::pipeline::Mechanism;

/// The door inputs (everything the §8 tree asks; computed at
/// preflight from live engine state). The bool set IS the tree's
/// literal conjunction — grouping it would obscure the spec mapping.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct DoorInputs {
    /// The mechanism computed for the ordered pair.
    pub mechanism: Option<Mechanism>,
    /// The fidelity label computed for the ordered pair.
    pub fidelity: Option<FidelityLabel>,
    /// Conflicts exist (three-way reconciliation, AJ-E207).
    pub conflicts: bool,
    /// The carry-list lists any Drop.
    pub drops_listed: bool,
    /// Target headroom ≥ Tight (the §Q6 tier).
    pub target_headroom_ok: bool,
    /// Rules or the worktree diverged (wizard step 4).
    pub rules_diverged: bool,
}

/// One wizard step (rendered only when its data is non-trivial).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WizardStep {
    /// 1 Target (always).
    Target,
    /// 2 What's carried — carry-list + pack preview (always; no
    /// execution from an unseen preview).
    Carried,
    /// 3 Conflicts (only when any).
    Conflicts,
    /// 4 Rules/diff (only when diverged).
    RulesDiff,
    /// 5 Confirm — cost, fidelity label, headroom (always).
    Confirm,
}

impl WizardStep {
    /// The step whose [`Self::title`] is `title`.
    #[must_use]
    pub fn from_title(title: &str) -> Option<Self> {
        [
            Self::Target,
            Self::Carried,
            Self::Conflicts,
            Self::RulesDiff,
            Self::Confirm,
        ]
        .into_iter()
        .find(|step| step.title() == title)
    }

    /// The step's title (the wizard's step indicator).
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::Target => "Target",
            Self::Carried => "What's carried",
            Self::Conflicts => "Conflicts",
            Self::RulesDiff => "Rules & diff",
            Self::Confirm => "Confirm",
        }
    }
}

/// The chosen door.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Door {
    /// All §8 conditions hold — execute via the confirm popover with
    /// the undo window.
    OneClick,
    /// The wizard with exactly the steps whose data is non-trivial.
    Wizard {
        /// Rendered steps, in §8 order.
        steps: Vec<WizardStep>,
    },
}

impl Door {
    /// The door a switch screen shows, from its wire form (`"one-click"`,
    /// or `"wizard"` with its step titles): `None` for anything else.
    #[must_use]
    pub fn from_wire(door: &str, steps: &[String]) -> Option<Self> {
        match door {
            "one-click" if steps.is_empty() => Some(Self::OneClick),
            "wizard" if !steps.is_empty() => steps
                .iter()
                .map(|title| WizardStep::from_title(title))
                .collect::<Option<Vec<_>>>()
                .map(|steps| Self::Wizard { steps }),
            _ => None,
        }
    }
    /// The telemetry discriminator (`switch_oneclick` vs
    /// `switch_wizard{steps_shown}`) — local counters only, never
    /// network.
    #[must_use]
    pub fn telemetry_kind(&self) -> String {
        match self {
            Self::OneClick => "switch_oneclick".to_owned(),
            Self::Wizard { steps } => format!(
                "switch_wizard[{}]",
                steps
                    .iter()
                    .map(|step| step.title())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
}

/// Chooses the door per the §8 tree, verbatim: every condition must
/// hold for 1-click; the wizard's conditional steps render only when
/// their data exists.
#[must_use]
pub fn choose_door(inputs: &DoorInputs) -> Door {
    let proven = matches!(
        inputs.mechanism,
        Some(Mechanism::Transplant | Mechanism::NativeResume)
    ) && matches!(inputs.fidelity, Some(label) if label >= FidelityLabel::F3)
        && !inputs.conflicts
        && !inputs.drops_listed
        && inputs.target_headroom_ok;
    if proven {
        return Door::OneClick;
    }
    let mut steps = vec![WizardStep::Target, WizardStep::Carried];
    if inputs.conflicts {
        steps.push(WizardStep::Conflicts);
    }
    if inputs.rules_diverged {
        steps.push(WizardStep::RulesDiff);
    }
    steps.push(WizardStep::Confirm);
    Door::Wizard { steps }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A door read back from its wire form is the door it was, and
    /// nothing else reads as one.
    #[test]
    fn a_door_reads_back_from_its_wire_form() {
        let wizard = Door::Wizard {
            steps: vec![WizardStep::Target, WizardStep::Carried, WizardStep::Confirm],
        };
        let titles: Vec<String> = [WizardStep::Target, WizardStep::Carried, WizardStep::Confirm]
            .iter()
            .map(|step| step.title().to_owned())
            .collect();
        assert_eq!(Door::from_wire("wizard", &titles), Some(wizard));
        assert_eq!(Door::from_wire("one-click", &[]), Some(Door::OneClick));
        assert_eq!(Door::from_wire("one-click", &titles), None);
        assert_eq!(Door::from_wire("wizard", &[]), None);
        assert_eq!(Door::from_wire("wizard", &["Elsewhere".to_owned()]), None);
        assert_eq!(Door::from_wire("teleport", &[]), None);
    }

    // One breaker per §8 NO leaf.
    fn fail_mechanism(base: &DoorInputs) -> DoorInputs {
        DoorInputs {
            mechanism: Some(Mechanism::Replay),
            ..base.clone()
        }
    }
    fn fail_fidelity(base: &DoorInputs) -> DoorInputs {
        DoorInputs {
            fidelity: Some(FidelityLabel::F1),
            ..base.clone()
        }
    }
    fn fail_drops(base: &DoorInputs) -> DoorInputs {
        DoorInputs {
            drops_listed: true,
            ..base.clone()
        }
    }
    fn fail_headroom(base: &DoorInputs) -> DoorInputs {
        DoorInputs {
            target_headroom_ok: false,
            ..base.clone()
        }
    }

    fn one_click_ready() -> DoorInputs {
        DoorInputs {
            mechanism: Some(Mechanism::Transplant),
            fidelity: Some(FidelityLabel::F3),
            conflicts: false,
            drops_listed: false,
            target_headroom_ok: true,
            rules_diverged: false,
        }
    }

    #[test]
    fn all_conditions_one_click() {
        assert_eq!(choose_door(&one_click_ready()), Door::OneClick);
        assert_eq!(
            choose_door(&one_click_ready()).telemetry_kind(),
            "switch_oneclick"
        );
    }

    #[test]
    fn every_other_leaf_is_the_wizard() {
        // One fixture per failing condition (the §8 tree's NO leaves).
        for mutate in [fail_mechanism, fail_fidelity, fail_drops, fail_headroom] {
            let inputs = mutate(&one_click_ready());
            assert!(
                matches!(choose_door(&inputs), Door::Wizard { .. }),
                "{inputs:?}"
            );
        }
    }

    /// D-03 amendment (2026-10-27): another account, or a compacted
    /// source, is no risk on its own — a proven transplant stays one
    /// click (the fields that said so are gone from the inputs).
    #[test]
    fn a_proven_transplant_to_another_account_is_one_click() {
        assert_eq!(choose_door(&one_click_ready()), Door::OneClick);
    }

    #[test]
    fn wizard_steps_render_only_when_needed() {
        // The minimal wizard: Target, Carried, Confirm (3 steps).
        let minimal = choose_door(&fail_drops(&one_click_ready()));
        let Door::Wizard { steps } = &minimal else {
            unreachable!("a drop ⇒ wizard")
        };
        let steps = steps.clone();
        assert_eq!(
            steps,
            vec![WizardStep::Target, WizardStep::Carried, WizardStep::Confirm]
        );
        assert_eq!(
            minimal.telemetry_kind(),
            "switch_wizard[Target,What's carried,Confirm]"
        );
        // With conflicts AND rules divergence: all five steps.
        let full = choose_door(&DoorInputs {
            conflicts: true,
            rules_diverged: true,
            ..fail_drops(&one_click_ready())
        });
        let Door::Wizard { steps } = &full else {
            unreachable!("conflicts ⇒ wizard")
        };
        assert_eq!(steps.len(), 5);
    }
}
