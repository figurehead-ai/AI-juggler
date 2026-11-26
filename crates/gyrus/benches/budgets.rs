//! Prints each operation's p50 and p95 at 10k, 100k and 1M links against
//! its budget (ADR-0038 §3): `cargo bench -p gyrus`. `tests/budgets.rs`
//! is the gate that fails past a budget.

#![allow(
    missing_docs,
    clippy::print_stdout,
    reason = "a benchmark prints what it measured"
)]

#[path = "workload.rs"]
mod workload;

fn main() {
    let sizes: Vec<usize> = std::env::args()
        .skip(1)
        .filter_map(|a| a.parse().ok())
        .collect();
    let sizes = if sizes.is_empty() {
        vec![10_000, 100_000, 1_000_000]
    } else {
        sizes
    };
    println!("| links | nodes | build ms | operation | p50 ms | p95 ms | budget ms |");
    println!("|---|---|---|---|---|---|---|");
    for links in sizes {
        let w = workload::world(links);
        let build = w.build.as_secs_f64() * 1000.0;
        for (op, p50, p95) in workload::measure(&w, 200) {
            let budget = workload::BUDGETS
                .iter()
                .find(|(name, _)| *name == op)
                .map_or(f64::NAN, |(_, b)| *b);
            println!(
                "| {} | {} | {build:.0} | {op} | {p50:.3} | {p95:.3} | {budget} |",
                w.links, w.nodes
            );
        }
    }
}
