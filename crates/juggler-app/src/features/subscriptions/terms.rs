//! D-62 (AJ-E3202 req. 4): plan, renewal and fee, set once on the Account.
//!
//! The rule lives in `juggler_core::storage::terms` (the move, the choice,
//! what applies to a folder). Here: the checks on what the user enters,
//! the commands' cores, and the choice as the UI gets it.

use std::sync::Arc;

use juggler_core::storage::{Choice, Term};

use crate::commands::lock_storage_arc;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// A term an Account's Apps disagree on: the user chooses once.
#[derive(Clone, Debug, PartialEq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TermChoiceInfo {
    /// `plan`, `renewal` or `fee`.
    pub term: String,
    /// Each value said, the Account's own first when it has one.
    pub options: Vec<TermOptionInfo>,
}

/// One value to choose.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct TermOptionInfo {
    /// The value: a plan, a `YYYY-MM-DD` date, a fee in dollars (`200`).
    pub value: String,
    /// The folders (their CLI Apps) that say it.
    pub folders: Vec<String>,
    /// Whether the Account has it already.
    pub account: bool,
}

impl From<Choice> for TermChoiceInfo {
    fn from(choice: Choice) -> Self {
        Self {
            term: choice.term.as_str().to_owned(),
            options: choice
                .options
                .into_iter()
                .map(|option| TermOptionInfo {
                    value: option.value,
                    folders: option.folders,
                    account: option.account,
                })
                .collect(),
        }
    }
}

/// The longest plan name kept.
const PLAN_MAX: usize = 40;

/// What the user entered for `term`, checked: trimmed, blank as none; a
/// renewal a real `YYYY-MM-DD` date; a fee a number of dollars a month.
///
/// # Errors
///
/// [`AppError::Validation`] for a value that isn't one.
pub fn checked(term: Term, value: Option<&str>) -> AppResult<Option<String>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    match term {
        Term::Plan if value.chars().count() > PLAN_MAX => Err(AppError::Validation {
            subject: "plan",
            message: format!("keep it to {PLAN_MAX} characters"),
        }),
        Term::Renewal if juggler_core::renewal::CalendarDate::parse(value).is_none() => {
            Err(AppError::Validation {
                subject: "renewal date",
                message: format!("must be YYYY-MM-DD (got {value:?})"),
            })
        }
        Term::Plan | Term::Renewal => Ok(Some(value.to_owned())),
        Term::Fee => match value.parse::<f64>() {
            Ok(fee) if (0.0..=100_000.0).contains(&fee) => Ok(Some(format!("{fee}"))),
            _ => Err(AppError::Validation {
                subject: "subscription fee",
                message: "must be between 0 and 100000 USD/month".into(),
            }),
        },
    }
}

/// Sets term `term` (`plan`, `renewal`, `fee`) on Account `id`; `None`
/// clears it. Where its Apps disagreed, this is the choice, made once.
///
/// # Errors
///
/// [`AppError::Refused`] for an Account that isn't stored or a term that
/// isn't one; [`AppError::Validation`] for a value that isn't one; storage
/// errors.
pub fn set_impl(state: &Arc<AppState>, id: &str, term: &str, value: Option<&str>) -> AppResult<()> {
    let term = Term::parse(term)
        .ok_or_else(|| AppError::Refused(format!("An account has no {term:?} to set")))?;
    let value = checked(term, value)?;
    let storage = lock_storage_arc(state)?;
    if storage.subscriptions().get(id)?.is_none() {
        return Err(AppError::Refused(
            "AI Juggler doesn't know that account".to_owned(),
        ));
    }
    storage.terms().set(id, term, value.as_deref())?;
    Ok(())
}

/// Sets term `term` for folder `folder_id`: on its Account when it has
/// one, else on the folder (the older per-folder commands, kept for
/// folders whose sign-in isn't read yet).
///
/// # Errors
///
/// [`AppError::NotFound`] for a folder that isn't stored;
/// [`AppError::Validation`] for a value that isn't one; storage errors.
pub fn set_for_folder(
    state: &Arc<AppState>,
    folder_id: &str,
    term: Term,
    value: Option<&str>,
) -> AppResult<()> {
    let value = checked(term, value)?;
    let storage = lock_storage_arc(state)?;
    if storage.terms().own(folder_id)?.is_none() {
        return Err(AppError::NotFound {
            entity: "account",
            id: folder_id.to_owned(),
        });
    }
    storage
        .terms()
        .set_for_folder(folder_id, term, value.as_deref())?;
    Ok(())
}
