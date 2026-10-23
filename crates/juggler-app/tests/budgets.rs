//! AJ-E1514 evidence: monthly caps. The thresholds, a cap for a group of
//! projects across accounts, a warning the user has seen staying quiet
//! until the next level or month, the form's refusals, and the never-stop
//! guardrail: no cap code can stop, pause or launch a session.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::pedantic
)]
#![cfg(unix)]

use std::path::Path;
use std::sync::Arc;

use juggler_adapter::AdapterRegistry;
use juggler_adapters::{CLAUDE_CODE_ID, ClaudeCodeAdapter};
use juggler_app::commands;
use juggler_app::features::budgets::{self, BudgetInput};
use juggler_app::state::AppState;
use juggler_core::cost_ledger::LedgerPriceRow;
use juggler_core::model::{Confidence, TokenCounts};
use juggler_core::quota::{UsageEvent, UsageSource};

const DAY: i64 = 24 * 60 * 60 * 1000;

/// A transcript for `cwd`, named `name`, in an account's folder.
fn transcript(config: &Path, cwd: &Path, name: &str) {
    let dir = config
        .join("projects")
        .join(cwd.to_string_lossy().replace('/', "-"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{name}.jsonl")),
        format!("{{\"type\":\"user\",\"cwd\":\"{}\"}}\n", cwd.display()),
    )
    .unwrap();
}

struct Lab {
    state: Arc<AppState>,
    work: String,
    personal: String,
    acme: String,
    other: String,
    month: i64,
    _dir: tempfile::TempDir,
}

/// Two accounts; three projects. Sessions `w1` (Work, acme), `p1`
/// (Personal, acme) and `w2` (Work, other). Model `lab-model` costs $1 per
/// 1k input tokens.
fn lab() -> Lab {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let acme = root.join("Dev/acme");
    let other = root.join("Dev/other");
    for repo in [&acme, &other] {
        std::fs::create_dir_all(repo.join(".git")).unwrap();
    }
    let mut registry = AdapterRegistry::new();
    registry.register(Arc::new(ClaudeCodeAdapter::with_bin("/bin/true")));
    let state = Arc::new(AppState::init_with_registry(root.join("app-data"), registry).unwrap());
    let mut ids = Vec::new();
    for (label, sessions) in [
        ("Work", vec![("w1", &acme), ("w2", &other)]),
        ("Personal", vec![("p1", &acme)]),
    ] {
        let config = root.join(label.to_lowercase());
        std::fs::create_dir_all(&config).unwrap();
        std::fs::write(config.join("settings.json"), "{}").unwrap();
        for (name, cwd) in sessions {
            transcript(&config, cwd, name);
        }
        commands::account_import_mode_impl(
            &state,
            CLAUDE_CODE_ID,
            config.to_str().unwrap(),
            label,
            commands::ImportRequest {
                in_place: true,
                ..commands::ImportRequest::default()
            },
        )
        .unwrap();
        let account = state
            .storage
            .lock()
            .unwrap()
            .accounts()
            .list()
            .unwrap()
            .into_iter()
            .find(|account| account.label == label)
            .unwrap();
        ids.push(account.id.to_string());
    }
    commands::projects_backfill_impl(&state).unwrap();
    let projects = commands::projects_list_impl(&state).unwrap();
    let project = |name: &str| projects.iter().find(|p| p.name == name).unwrap().id.clone();
    let now = juggler_core::model::Timestamp::now().millis();
    let month = now - 10 * DAY;
    state
        .storage
        .lock()
        .unwrap()
        .usage()
        .pricing_override_upsert(&LedgerPriceRow {
            model: "lab-model".to_owned(),
            effective_from_ms: 0,
            input_per_1k: 1.0,
            output_per_1k: 1.0,
            cache_read_per_1k: 0.0,
            cache_write_per_1k: 0.0,
            source_url: "test".to_owned(),
            captured: "2026-10-23".to_owned(),
        })
        .unwrap();
    Lab {
        state,
        work: ids[0].clone(),
        personal: ids[1].clone(),
        acme: project("acme"),
        other: project("other"),
        month,
        _dir: dir,
    }
}

impl Lab {
    /// `usd` dollars of `lab-model` on `account`'s `session`, `days_ago`.
    fn used(&self, account: &str, session: &str, usd: u64, days_ago: i64) {
        let now = juggler_core::model::Timestamp::now().millis();
        let event = UsageEvent {
            account: account.to_owned(),
            session: Some(session.to_owned()),
            ts: now - days_ago * DAY,
            model: Some("lab-model".to_owned()),
            tokens: TokenCounts {
                input: usd * 1000,
                ..TokenCounts::default()
            },
            credits: None,
            usd_est: None,
            source: UsageSource::Transcript,
            confidence: Confidence::Measured,
            msg_uuid: Some(format!("{account}-{session}-{usd}-{days_ago}-{now}")),
        };
        self.state
            .storage
            .lock()
            .unwrap()
            .usage()
            .insert_events(&[event])
            .unwrap();
    }

    fn list(&self) -> Vec<budgets::BudgetInfo> {
        budgets::list_impl(&self.state, self.month).unwrap()
    }
}

fn account_cap(account: &str, usd: f64) -> BudgetInput {
    BudgetInput {
        id: None,
        name: "Work".to_owned(),
        monthly_usd: usd,
        account_id: Some(account.to_owned()),
        project_ids: Vec::new(),
    }
}

#[test]
fn an_accounts_cap_warns_at_80_and_100_percent_of_this_month() {
    let lab = lab();
    let id = budgets::save_impl(&lab.state, account_cap(&lab.work, 100.0)).unwrap();
    lab.used(&lab.work, "w1", 500, 20); // last month: not counted
    lab.used(&lab.work, "w1", 79, 2);
    let cap = &lab.list()[0];
    assert_eq!(
        (cap.id.as_str(), cap.level.as_str()),
        (id.as_str(), "under")
    );
    assert_eq!(cap.spent_usd, Some(79.0));
    lab.used(&lab.work, "w2", 1, 1);
    assert_eq!(lab.list()[0].level, "near");
    lab.used(&lab.personal, "p1", 50, 1); // another account: not this cap's
    assert_eq!(lab.list()[0].spent_usd, Some(80.0));
    lab.used(&lab.work, "w1", 20, 0);
    assert_eq!(lab.list()[0].level, "over");
}

#[test]
fn a_group_of_projects_counts_them_on_every_account() {
    let lab = lab();
    budgets::save_impl(
        &lab.state,
        BudgetInput {
            id: None,
            name: "Acme".to_owned(),
            monthly_usd: 10.0,
            account_id: None,
            project_ids: vec![lab.acme.clone()],
        },
    )
    .unwrap();
    lab.used(&lab.work, "w1", 3, 1);
    lab.used(&lab.personal, "p1", 4, 1);
    lab.used(&lab.work, "w2", 100, 1); // the other project
    let cap = &lab.list()[0];
    assert_eq!(cap.spent_usd, Some(7.0));
    assert_eq!(cap.project_ids, vec![lab.acme.clone()]);
    assert_eq!(cap.level, "under");
    let _ = &lab.other;
}

#[test]
fn a_seen_warning_stays_quiet_until_the_next_level() {
    let lab = lab();
    let id = budgets::save_impl(&lab.state, account_cap(&lab.work, 10.0)).unwrap();
    lab.used(&lab.work, "w1", 8, 1);
    assert!(!lab.list()[0].seen);
    budgets::seen_impl(&lab.state, &id, lab.month, "near").unwrap();
    assert!(lab.list()[0].seen, "seen at 80 %");
    lab.used(&lab.work, "w1", 2, 0);
    let cap = &lab.list()[0];
    assert_eq!(
        (cap.level.as_str(), cap.seen),
        ("over", false),
        "100 % warns again"
    );
    // A month the screen sends that isn't this one is refused.
    assert!(budgets::seen_impl(&lab.state, &id, lab.month - 40 * DAY, "over").is_err());
    assert!(budgets::seen_impl(&lab.state, &id, lab.month, "under").is_err());
    // Changing the cap warns afresh.
    budgets::seen_impl(&lab.state, &id, lab.month, "over").unwrap();
    let mut changed = account_cap(&lab.work, 9.0);
    changed.id = Some(id.clone());
    budgets::save_impl(&lab.state, changed).unwrap();
    assert!(!lab.list()[0].seen);
    budgets::remove_impl(&lab.state, &id).unwrap();
    assert!(lab.list().is_empty());
}

#[test]
fn usage_with_no_price_is_flagged_never_counted_as_nothing() {
    let lab = lab();
    budgets::save_impl(&lab.state, account_cap(&lab.work, 10.0)).unwrap();
    lab.used(&lab.work, "w1", 3, 1);
    let mut unknown = UsageEvent {
        account: lab.work.clone(),
        session: Some("w1".to_owned()),
        ts: juggler_core::model::Timestamp::now().millis() - DAY,
        model: Some("a-model-nobody-prices".to_owned()),
        tokens: TokenCounts {
            input: 1_000_000,
            ..TokenCounts::default()
        },
        credits: None,
        usd_est: None,
        source: UsageSource::Transcript,
        confidence: Confidence::Measured,
        msg_uuid: Some("unpriced".to_owned()),
    };
    unknown.tokens.output = 0;
    lab.state
        .storage
        .lock()
        .unwrap()
        .usage()
        .insert_events(&[unknown])
        .unwrap();
    let cap = &lab.list()[0];
    assert_eq!(cap.spent_usd, Some(3.0));
    assert!(cap.unpriced);
}

#[test]
fn the_form_is_refused_in_words() {
    let lab = lab();
    let message = |input: BudgetInput| {
        budgets::save_impl(&lab.state, input)
            .unwrap_err()
            .to_string()
    };
    assert!(message(account_cap(&lab.work, 0.0)).contains("above $0"));
    let mut nameless = account_cap(&lab.work, 5.0);
    nameless.name = " ".to_owned();
    assert!(message(nameless).contains("name"));
    assert!(budgets::save_impl(&lab.state, account_cap("no-such-account", 5.0)).is_err());
    let nothing = BudgetInput {
        id: None,
        name: "Acme".to_owned(),
        monthly_usd: 5.0,
        account_id: None,
        project_ids: Vec::new(),
    };
    assert!(message(nothing).contains("project"));
    let mut unknown = account_cap(&lab.work, 5.0);
    unknown.id = Some("cap-not-there".to_owned());
    assert!(budgets::save_impl(&lab.state, unknown).is_err());
    assert!(
        budgets::list_impl(&lab.state, lab.month + 30 * DAY).is_err(),
        "a month ahead"
    );
}

/// The never-stop guardrail (the task's quality gate): the caps' code
/// names nothing that stops, pauses, kills or starts a session.
#[test]
fn a_cap_never_stops_anything() {
    let files = [
        include_str!("../src/features/budgets/mod.rs"),
        include_str!("../src/features/budgets/commands.rs"),
        include_str!("../src/features/budgets/dto.rs"),
        include_str!("../../juggler-core/src/budget.rs"),
    ];
    for source in files {
        let code: String = source
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .filter(|line| !line.contains("let forbidden") && !line.contains("for word in"))
            .collect::<Vec<_>>()
            .join("\n");
        for word in [
            "supervisor",
            "kill",
            "stop_",
            "_stop",
            "pause",
            "suspend",
            "launch",
            "spawn",
            "switch_",
            "freeze",
        ] {
            assert!(!code.contains(word), "{word}");
        }
    }
}
