//! The budgets as tests (ADR-0038 §3): each operation's p95 at 10k, 100k
//! and 1M links must stay within its budget, so a regression fails the
//! build. Budgets are for an optimised build: in a debug build the 10k
//! run only checks the workload runs. CI runs
//! `cargo test -p gyrus --release --test budgets -- --include-ignored
//! --test-threads=1` (one size at a time, so the runs don't disturb each other).
//!
//! `AIJ_BUDGET_FACTOR` (ADR-0038 §3, amended 2026-11-27) multiplies every
//! budget: the budgets are the reference Mac's, and GitHub's shared macOS
//! runner measured up to 4× them (seed 16.2 ms against 5 at 100k links), so
//! CI runs at factor 4, which still fails a gross regression. Unset, the
//! factor is 1: the exact gate, `just gyrus-budgets`, a step of the full
//! local preflight (`scripts/preflight.sh --full`) on the reference Mac.

#![allow(
    missing_docs,
    clippy::print_stdout,
    reason = "a gate prints what it measured"
)]

#[path = "../benches/workload.rs"]
mod workload;

/// The factor every budget is multiplied by: `AIJ_BUDGET_FACTOR`, a number
/// of at least 1, else 1 (the exact budgets).
fn factor() -> f64 {
    std::env::var("AIJ_BUDGET_FACTOR")
        .ok()
        .and_then(|value| value.trim().parse::<f64>().ok())
        .filter(|factor| factor.is_finite() && *factor >= 1.0)
        .unwrap_or(1.0)
}

fn gate(links: usize) {
    let factor = factor();
    let w = workload::world(links);
    let measured = workload::measure(&w, 200);
    let mut over = Vec::new();
    println!(
        "{links} links, {} nodes, built in {:.0} ms",
        w.nodes,
        w.build.as_secs_f64() * 1000.0
    );
    for (op, p50, p95) in &measured {
        let budget = workload::BUDGETS
            .iter()
            .find(|(name, _)| name == op)
            .map_or(f64::INFINITY, |(_, b)| *b)
            * factor;
        println!("  {op}: p50 {p50:.3} ms, p95 {p95:.3} ms (budget {budget} ms, factor {factor})");
        if *p95 > budget {
            over.push(format!(
                "{op} p95 {p95:.3} ms > {budget} ms (the budget × factor {factor}, AIJ_BUDGET_FACTOR)"
            ));
        }
    }
    if cfg!(debug_assertions) {
        return;
    }
    assert!(over.is_empty(), "over budget at {links} links: {over:?}");
}

#[test]
fn budgets_at_10k_links() {
    gate(10_000);
}

#[test]
#[ignore = "release builds only: cargo test -p gyrus --release --test budgets -- --include-ignored"]
fn budgets_at_100k_links() {
    gate(100_000);
}

#[test]
#[ignore = "release builds only: cargo test -p gyrus --release --test budgets -- --include-ignored"]
fn budgets_at_1m_links() {
    gate(1_000_000);
}
