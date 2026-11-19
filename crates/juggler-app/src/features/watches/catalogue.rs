//! The condition catalogue for a target (ADR-0036 §2.2, D-127): only the
//! conditions offered on that kind of target that its Apps can report.
//! What can't be reported is left out, with no "unsupported" mark. The
//! Watches view's choice cards, Chat's catalogue section (AJ-E3815) and
//! the create command's check all read it.

use juggler_adapter::Observes;
use serde::Serialize;

use super::conditions::{ConditionRule, ParamKind, Registry};
use super::model::{TargetKind, Watch, WatchError};
use super::words::Lang;

/// A condition's parameter, for a form.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CatalogueParam {
    /// Its name (`for_min`).
    pub name: String,
    /// `minutes`, `percent`, `task_state` or `agent`.
    pub kind: String,
    /// Least, for a number.
    pub min: Option<u32>,
    /// Most, for a number.
    pub max: Option<u32>,
    /// It must be given.
    pub required: bool,
}

/// One condition on offer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct CatalogueEntry {
    /// Its code.
    pub code: String,
    /// Its label ("Finishes").
    pub label: String,
    /// Its parameter, when it takes one.
    pub param: Option<CatalogueParam>,
}

fn param_info(rule: &dyn ConditionRule) -> Option<CatalogueParam> {
    let spec = rule.param()?;
    let (kind, min, max) = match spec.kind {
        ParamKind::Minutes { min, max } => ("minutes", Some(min), Some(max)),
        ParamKind::Percent { min, max } => ("percent", Some(u32::from(min)), Some(u32::from(max))),
        ParamKind::TaskState => ("task_state", None, None),
        ParamKind::Agent => ("agent", None, None),
    };
    Some(CatalogueParam {
        name: spec.name.to_owned(),
        kind: kind.to_owned(),
        min,
        max,
        required: spec.required,
    })
}

/// The rule is offered on `kind` with `observed` (the target's Apps'
/// observations and AI Juggler's own).
#[must_use]
pub fn offered(rule: &dyn ConditionRule, kind: TargetKind, observed: Observes) -> bool {
    rule.targets().contains(&kind) && rule.needs().intersects(observed)
}

/// The conditions on offer for a target of `kind` whose Apps observe
/// `observed`, in the registry's order.
#[must_use]
pub fn catalogue(
    registry: &Registry,
    kind: TargetKind,
    observed: Observes,
    lang: Lang,
) -> Vec<CatalogueEntry> {
    registry
        .iter()
        .filter(|rule| offered(*rule, kind, observed))
        .map(|rule| CatalogueEntry {
            code: rule.code().to_owned(),
            label: match lang {
                Lang::En => rule.words().label_en,
                Lang::De => rule.words().label_de,
            }
            .to_owned(),
            param: param_info(rule),
        })
        .collect()
}

/// Every condition of `watch` is on offer for every target, given what
/// each target observes (`observed[i]` for `watch.targets[i]`).
///
/// # Errors
///
/// [`WatchError::ConditionNotForTarget`] for the first one that isn't.
pub fn check_offered(
    registry: &Registry,
    watch: &Watch,
    observed: &[Observes],
) -> Result<(), WatchError> {
    for (target, seen) in watch.targets.iter().zip(observed) {
        for condition in &watch.conditions {
            let fits = registry
                .get(&condition.code)
                .is_some_and(|rule| offered(rule, target.kind(), *seen));
            if !fits {
                return Err(WatchError::ConditionNotForTarget(
                    condition.code.clone(),
                    target.kind(),
                ));
            }
        }
    }
    Ok(())
}
