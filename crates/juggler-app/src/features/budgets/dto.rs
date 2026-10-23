//! The monthly caps' wire types (AJ-E1514).

/// One cap and how far this month has gone towards it.
#[derive(Clone, Debug, PartialEq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BudgetInfo {
    /// Its id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// USD a month.
    pub monthly_usd: f64,
    /// The account it counts, for an account's cap.
    pub account_id: Option<String>,
    /// The projects it counts, for a group of projects (empty otherwise).
    pub project_ids: Vec<String>,
    /// This month's estimate at list prices; `None` when none of it could
    /// be priced.
    pub spent_usd: Option<f64>,
    /// Some of this month's usage couldn't be priced, so the estimate is
    /// lower than the usage.
    pub unpriced: bool,
    /// `under`, `near` (80 % or more) or `over` (100 % or more).
    pub level: String,
    /// The user has seen this level this month (Needs you leaves it out).
    pub seen: bool,
}

/// A cap as the form sends it: a new one without an id.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct BudgetInput {
    /// The cap being changed; `None` for a new one.
    pub id: Option<String>,
    /// Its name.
    pub name: String,
    /// USD a month.
    pub monthly_usd: f64,
    /// The account it counts; `None` for a group of projects.
    pub account_id: Option<String>,
    /// The projects it counts, when it isn't an account's.
    pub project_ids: Vec<String>,
}
