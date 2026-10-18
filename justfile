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
    cargo doc --workspace --no-deps
    just evidence
    just terminology

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

doc:
    cargo doc --workspace --no-deps

# Run the desktop app in dev mode (UI hot-reload + Rust rebuild on change).
dev:
    cargo tauri dev

# Release-mode app bundle under target/release/bundle/ (unsigned dev build).
app:
    cargo tauri build
