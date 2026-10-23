//! Monthly caps (AJ-E1514): a user-named USD amount a month, for one
//! account or for a group of projects, and how far this month's usage has
//! gone towards it. A cap only warns: at 80 % and at 100 %, in Needs you.
//! Nothing here can stop, pause or hold back a session, and nothing calls
//! anything that could.
//!
//! The spend is the cost ledger's own estimate at list prices
//! ([`crate::cost_ledger::rollup`]), over the same events Usage shows, so
//! a cap and the Usage table always agree. Usage a price can't be found
//! for is flagged, never counted as $0.

use crate::cost_ledger::{PricingCatalog, RollupDimension, rollup};
use crate::quota::UsageEvent;

/// The longest name a cap may have.
pub const NAME_MAX: usize = 40;
/// The largest monthly amount, in USD.
pub const AMOUNT_MAX: f64 = 1_000_000.0;
/// The share of a cap that first warns.
pub const NEAR: f64 = 0.8;

/// What a cap counts.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BudgetScope {
    /// Everything one account used.
    Account {
        /// The account's id.
        account_id: String,
    },
    /// Everything used in these projects, on any account.
    Projects {
        /// The projects' ids.
        project_ids: Vec<String>,
    },
}

/// One monthly cap, as the user set it.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BudgetCap {
    /// Its id.
    pub id: String,
    /// Its name ("Acme", "Personal").
    pub name: String,
    /// USD a month.
    pub monthly_usd: f64,
    /// What it counts.
    pub scope: BudgetScope,
}

/// How far this month has gone towards a cap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetLevel {
    /// Under 80 %.
    Under,
    /// 80 % or more.
    Near,
    /// 100 % or more.
    Over,
}

/// The level `spent` is at against `cap` (a cap of 0 or less is never
/// saved; read as under).
#[must_use]
pub fn level(spent: f64, cap: f64) -> BudgetLevel {
    if cap <= 0.0 || !spent.is_finite() {
        BudgetLevel::Under
    } else if spent >= cap {
        BudgetLevel::Over
    } else if spent >= cap * NEAR {
        BudgetLevel::Near
    } else {
        BudgetLevel::Under
    }
}

/// Why a cap can't be saved, in words for the form; `None` when it can.
#[must_use]
pub fn invalid(cap: &BudgetCap) -> Option<&'static str> {
    let name = cap.name.trim();
    if name.is_empty() {
        return Some("Give it a name");
    }
    if name.chars().count() > NAME_MAX {
        return Some("Keep the name to 40 characters or fewer");
    }
    if !cap.monthly_usd.is_finite() || cap.monthly_usd <= 0.0 {
        return Some("Enter an amount above $0");
    }
    if cap.monthly_usd > AMOUNT_MAX {
        return Some("Enter an amount up to $1,000,000");
    }
    match &cap.scope {
        BudgetScope::Account { account_id } if account_id.is_empty() => Some("Choose an account"),
        BudgetScope::Projects { project_ids } if project_ids.is_empty() => {
            Some("Choose at least one project")
        }
        _ => None,
    }
}

/// This month's estimate over `events` (already narrowed to the cap's
/// scope and month): the priced total, and whether any usage couldn't be
/// priced. `None` when nothing could be priced at all.
#[must_use]
pub fn spent(events: &[UsageEvent], catalog: &PricingCatalog) -> (Option<f64>, bool) {
    let rows = rollup(events, catalog, RollupDimension::All);
    let unpriced = rows.iter().any(|row| row.usd.usd.is_none());
    let priced: Vec<f64> = rows.iter().filter_map(|row| row.usd.usd).collect();
    let total = (!priced.is_empty()).then(|| priced.iter().sum());
    (total.or_else(|| rows.is_empty().then_some(0.0)), unpriced)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap(scope: BudgetScope) -> BudgetCap {
        BudgetCap {
            id: "cap-1".to_owned(),
            name: "Acme".to_owned(),
            monthly_usd: 300.0,
            scope,
        }
    }

    #[test]
    fn the_levels_turn_at_80_and_100_percent() {
        assert_eq!(level(239.99, 300.0), BudgetLevel::Under);
        assert_eq!(level(240.0, 300.0), BudgetLevel::Near);
        assert_eq!(level(299.99, 300.0), BudgetLevel::Near);
        assert_eq!(level(300.0, 300.0), BudgetLevel::Over);
        assert_eq!(level(900.0, 300.0), BudgetLevel::Over);
        assert_eq!(level(10.0, 0.0), BudgetLevel::Under);
        assert_eq!(level(f64::NAN, 300.0), BudgetLevel::Under);
    }

    #[test]
    fn a_cap_needs_a_name_an_amount_and_something_to_count() {
        let account = || BudgetScope::Account {
            account_id: "claude-code-1".to_owned(),
        };
        assert_eq!(invalid(&cap(account())), None);
        let mut bad = cap(account());
        bad.name = "  ".to_owned();
        assert_eq!(invalid(&bad), Some("Give it a name"));
        bad = cap(account());
        bad.name = "x".repeat(41);
        assert!(invalid(&bad).is_some());
        for amount in [0.0, -5.0, f64::NAN, f64::INFINITY, 1_000_001.0] {
            bad = cap(account());
            bad.monthly_usd = amount;
            assert!(invalid(&bad).is_some(), "{amount}");
        }
        assert_eq!(
            invalid(&cap(BudgetScope::Projects {
                project_ids: Vec::new()
            })),
            Some("Choose at least one project")
        );
        assert_eq!(
            invalid(&cap(BudgetScope::Account {
                account_id: String::new()
            })),
            Some("Choose an account")
        );
    }

    #[test]
    fn a_scope_reads_back_as_it_was_saved() {
        let saved = cap(BudgetScope::Projects {
            project_ids: vec!["p-1".to_owned(), "p-2".to_owned()],
        });
        let text = serde_json::to_string(&saved).unwrap();
        assert!(text.contains(r#""kind":"projects""#), "{text}");
        assert_eq!(serde_json::from_str::<BudgetCap>(&text).unwrap(), saved);
    }

    /// The never-stop guardrail: nothing in this module names anything
    /// that stops, pauses, kills or launches a session.
    #[test]
    fn a_cap_only_warns() {
        let source = include_str!("budget.rs");
        let code: String = source
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .filter(|line| !line.contains("\"stop") && !line.contains("forbidden"))
            .collect::<Vec<_>>()
            .join("\n");
        let forbidden = [
            "supervisor",
            "kill",
            "signal",
            "stop_",
            "pause",
            "suspend",
            "launch",
        ];
        for word in forbidden {
            assert!(!code.contains(word), "{word}");
        }
    }
}
