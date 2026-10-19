# AI Juggler — developer tasks. Standards: docs/plan/QUALITY.md.
# `just verify` is the local gate; CI runs the same checks.

default: verify

# CLT-only macOS machines need the Swift-bridge link shim (no-op
# elsewhere; see scripts/ensure_swift_runtime.sh).
swift-flags := if os() == "macos" { `./scripts/ensure_swift_runtime.sh | tail -1 | sed 's/^RUSTFLAGS=//'` } else { "" }
export RUSTFLAGS := swift-flags

# Full local gate — everything CI enforces.
verify:
    ./scripts/ensure_swift_runtime.sh > /dev/null
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
    just evidence
    just terminology
    just stats-doc
    just stats-events
    just polling-audit

# AJ-E805: the load proof on the reference Mac — SPEC budgets plus the
# >10 % regression rule against ui/tests/visual/load-baseline.json;
# writes docs/qa/evidence/AJ-E805/ui-load.json and the load captures.
load-gate:
    cd ui && AJ_LOAD_GATE=1 pnpm exec playwright test tests/visual/load.spec.ts
    cargo test -p juggler-app --test load_budgets -- --nocapture
    cargo test -p juggler-app --test load_budgets -- --ignored --test-threads=1 --nocapture
    cargo test -p juggler-handover --test pack_load -- --nocapture

# AJ-E805: every sub-second timer or sleep in shipped code says why.
polling-audit:
    python3 scripts/polling_audit.py > /dev/null

# Usage-statistics doc gate (AJ-E1106): docs/usage-stats.md is generated
# from stats/schema.v1.json and must not drift.
stats-doc:
    python3 scripts/gen_usage_stats_doc.py --check

# Usage-statistics types gate (AJ-E1102): the typed events are generated
# from the schema, and the `stats` feature really leaves the crate out.
stats-events:
    python3 scripts/gen_stats_events.py --check
    ./scripts/stats_feature_check.sh

# Evidence-store gate (AJ-E801): link resolution + secret scan + index freshness.
evidence:
    ./scripts/evidence_check.sh

# Terminology gate (AJ-E108): legacy user-facing nouns cannot creep back.
terminology:
    ./scripts/terminology_lint.sh

# UI gate (AJ-E806): typecheck + build. Visual (AJ-E803) and a11y (AJ-E804)
# gates attach to UI-changing PRs; perf (AJ-E805) at milestones.
ui:
    pnpm --dir ui build

fmt:
    cargo fmt --all

lint:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace

# AJ-E210: regenerate the E2 validation gate report from the matrix.
e2-gate:
    ./scripts/e2_gate_report.sh > docs/qa/gate-E2-validation.md

# AJ-E710: the Fleet gate's automated portion, filed as evidence.
gate-e7:
    mkdir -p docs/qa/evidence/AJ-E710
    ./scripts/gate_e7.sh > docs/qa/evidence/AJ-E710/automated-$(date +%Y-%m-%d).md

doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

# Run the desktop app in dev mode (UI hot-reload + Rust rebuild on change).
dev:
    cargo tauri dev

# Release-mode app bundle under target/release/bundle/ (unsigned dev build).
app:
    cargo tauri build

# AJ-E803 visual regression gate (in the pinned Playwright container):
# compare the deterministic matrix with the approved baselines.
visual-gate:
    ./scripts/visual_gate.sh

# Re-render the baselines after an intentional UI change; the PR then
# needs a "## Visual baselines" note per changed screen.
visual-baselines:
    ./scripts/visual_gate.sh --update

# The gate's self-test: a seeded 2 px button shift must fail.
visual-selftest:
    ./scripts/visual_gate.sh --selftest
