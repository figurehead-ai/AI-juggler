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
    cargo clippy --workspace --all-targets --no-default-features -- -D warnings
    cargo clippy -p juggler-app --all-targets --features dev-bridge -- -D warnings
    cargo test --workspace
    cargo test -p juggler-app --features dev-bridge --lib dev_bridge
    ./scripts/leaked_fixtures.sh
    LC_ALL=en_US.UTF-8 ./scripts/qa/runtime_lab_snapshot.sh selftest
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
    just evidence
    just terminology
    just tool-literals
    just platform
    just structure
    just task-counts
    just stats-doc
    just stats-events
    just polling-audit
    just release-selftest
    just dev-bridge-check
    python3 scripts/capabilities.py check

# D-20 / D-24: the release scripts' self-tests (versions, nightly notes,
# channel requirements). A release only runs them for real once a night.
release-selftest:
    python3 scripts/release_version.py --selftest
    python3 scripts/nightly_notes.py --selftest
    python3 scripts/downgrade_note.py --selftest
    python3 scripts/promotion_rules.py --selftest
    python3 scripts/apple_retry.py --selftest
    python3 scripts/updater_sig_check.py --selftest

# AJ-E809: the weekly dogfood report from runs.csv (exit 1 until beta-exit is met).
dogfood-report:
    python3 scripts/dogfood_report.py

# AJ-E805: the load proof on the reference Mac — SPEC budgets plus the
# >10 % regression rule against ui/tests/visual/load-baseline.json;
# writes docs/qa/evidence/AJ-E805/ui-load.json and the load captures.
load-gate:
    cd ui && AJ_LOAD_GATE=1 pnpm exec playwright test tests/visual/load.spec.ts
    # AJ-E1906: 200 task rows in under 100 ms (a wall clock, so timed here only).
    cd ui && AJ_LOAD_GATE=1 pnpm exec vitest run src/pages/tasks/tasks.test.tsx -t "200 tasks"
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

# Terminology gate (AJ-E108): legacy user-facing nouns cannot creep back;
# AJ-E1535: nor can developer words on screen (its allowlist only shrinks).
terminology:
    ./scripts/terminology_lint.sh
    python3 scripts/plain_words_lint.py --selftest
    python3 scripts/plain_words_lint.py
    python3 scripts/docs_links_check.py

# AJ-E1525 (ADR-0019 §5): no tool-id literal outside the adapters; the
# allowlist of today's sites only shrinks.
tool-literals:
    python3 scripts/tool_literal_lint.py --selftest
    python3 scripts/tool_literal_lint.py

# Cross-platform readiness (docs/research/cross-platform-readiness.md §5.1):
# no new macOS- or Unix-only assumption outside the platform seams; the
# allowlist of today's sites only shrinks.
platform:
    python3 scripts/platform_lint.py --selftest
    python3 scripts/platform_lint.py

# AJ-E2011 (ENGINEERING-GUIDE A1/A6, A3/T1/I3/I4, C.10): the crate graph,
# the IPC rules and the size budget. Each allowlist only shrinks.
structure:
    python3 scripts/check_layering.py --selftest
    python3 scripts/check_layering.py
    python3 scripts/ipc_audit.py --selftest
    python3 scripts/ipc_audit.py
    python3 scripts/size_budget.py --selftest
    python3 scripts/size_budget.py

# The docs that quote task counts and ranges agree with the epic files.
task-counts:
    python3 scripts/check_task_counts.py --selftest
    python3 scripts/check_task_counts.py

# UI gate (AJ-E806): typecheck + build. Visual (AJ-E803) and a11y (AJ-E804)
# gates attach to UI-changing PRs; perf (AJ-E805) at milestones.
ui:
    pnpm --dir ui build

fmt:
    cargo fmt --all

lint:
    cargo clippy --workspace --all-targets -- -D warnings
    cargo clippy --workspace --all-targets --no-default-features -- -D warnings

test:
    cargo test --workspace

# AJ-E807: (re)build the beta gate packet from the manifests, then check
# it; `gate-signed` exits 1 until the maintainer signs it.
gate-packet:
    python3 scripts/gate_packet.py build docs/qa/gates/beta.json
    python3 scripts/gate_packet.py check docs/qa/gate-beta.md

gate-signed:
    python3 scripts/gate_packet.py signed docs/qa/gate-beta.md

# Real Claude Code and Codex on accounts whose owner agreed, in a new
# throwaway project: it spends a little usage and adds the lab's sessions
# to the accounts. Arguments: a new project folder, two Claude config
# folders, a Codex home, and the report's path (keep it out of the
# repository: it holds real paths). AIJ_LAB_PROBE=<name> also checks the
# set-up step.
# AJ-E1539: the live switching lab (read the test's header first).
switch-lab project claude_a claude_b codex out:
    cargo test -p juggler-app --test switch_lab_live --no-run
    env -u CLAUDE_CONFIG_DIR -u CODEX_HOME \
      AIJ_LAB_DATA="$(mktemp -d)/data" AIJ_LAB_PROJECT="{{project}}" \
      AIJ_LAB_CLAUDE_A="{{claude_a}}" AIJ_LAB_CLAUDE_B="{{claude_b}}" \
      AIJ_LAB_CODEX="{{codex}}" AIJ_LAB_OUT="{{out}}" \
      cargo test -p juggler-app --test switch_lab_live -- --ignored --nocapture switch_lab_live --exact

# AJ-E1539 labs 12+ (0.1.8): switches from a session another app runs and the
# old path, on one Claude profile and the Codex home (see switch_lab_live.rs).
switch-lab-outside root claude codex out probe="aij-lab-probe" journeys="j1,j2,j3":
    cargo test -p juggler-app --test switch_lab_live --no-run
    env -u CLAUDE_CONFIG_DIR -u CODEX_HOME DISABLE_AUTOUPDATER=1 AIJ_LAB_SUITE=outside \
      AIJ_LAB_DATA="$(mktemp -d)/data" AIJ_LAB_PROJECT="{{root}}" \
      AIJ_LAB_CLAUDE_A="{{claude}}" AIJ_LAB_CODEX="{{codex}}" AIJ_LAB_OUT="{{out}}" \
      AIJ_LAB_PROBE="{{probe}}" AIJ_LAB_JOURNEYS="{{journeys}}" \
      cargo test -p juggler-app --test switch_lab_live -- --ignored --nocapture switch_lab_outside_live --exact

# AJ-E808: render the provider-pair fidelity register (a test fails on drift).
fidelity-register:
    mkdir -p docs/qa/evidence/AJ-E808
    cargo run -q -p juggler-handover --example fidelity_register > docs/qa/evidence/AJ-E808/register.md

# AJ-E210: regenerate the E2 validation gate report from the matrix.
e2-gate:
    ./scripts/e2_gate_report.sh > docs/qa/gate-E2-validation.md

# AJ-E710: the Fleet gate's automated portion, filed as evidence.
gate-e7:
    mkdir -p docs/qa/evidence/AJ-E710
    ./scripts/gate_e7.sh > docs/qa/evidence/AJ-E710/automated-$(date +%Y-%m-%d).md

doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

# AI Juggler Dev (D-23) has its own data folder, Keychain items, Fleet port
# and single-instance lock: it runs next to an installed AI Juggler or AI
# Juggler Nightly and never touches their data.
# Run the app as AI Juggler Dev (UI hot-reload + Rust rebuild on change).
dev: dev-icon agent-bridge
    cargo tauri dev --config crates/juggler-app/tauri.dev.conf.json

# AJ-E1704: the agent bridge beside the dev build's executable (where the
# app looks for it; a release bundle carries it in Contents/MacOS).
agent-bridge:
    cargo build -q -p juggler-agentbridge --bin juggler-agent

# AJ-E1531: the replica, a throwaway HOME shaped like a heavy real user's
# Mac, for every walk and journey. It refuses a HOME that isn't a replica.
replica_home := env_var_or_default("AIJ_REPLICA_HOME", "/tmp/aij-replica")
target_dir := env_var_or_default("CARGO_TARGET_DIR", "target")

# Build the replica HOME (AIJ_REPLICA_HOME, default /tmp/aij-replica).
replica:
    python3 -I scripts/replica.py build --home {{replica_home}}

# Live sessions on the replica while a walk runs (Ctrl-C stops them all).
replica-live minutes="60" *args="":
    cargo build -q -p juggler-adapters --bins
    python3 -I scripts/replica.py live --home {{replica_home}} --harness {{target_dir}}/debug/fake-harness --desktop {{target_dir}}/debug/fake-desktop --minutes {{minutes}} {{args}}

# Compare the backend's read models with the oracle on the replica (AJ-E1529).
replica-oracle:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo test -q -p juggler-app --test qa_real_readonly --no-run 2> /dev/null
    bin=$(ls -t {{target_dir}}/debug/deps/qa_real_readonly-* | grep -v '\.d$' | sed -n 1p)
    work=$(mktemp -d)
    env -i HOME={{replica_home}} USER="$USER" TMPDIR="$TMPDIR" PATH=/usr/bin:/bin:/usr/sbin:/sbin \
        AIJ_QA_REAL_DATA="$work/data" AIJ_QA_REAL_OUT="$work/harness.json" AIJ_QA_ICONS=1 "$bin" --ignored --nocapture > "$work/harness.log" 2>&1
    dirs=()
    for d in {{replica_home}}/.claude-*/; do dirs+=(--claude-dir "$d"); done
    for d in {{replica_home}}/.codex-*/; do dirs+=(--codex-dir "$d"); done
    HOME={{replica_home}} python3 -I scripts/qa/real_truth.py "${dirs[@]}" --compare "$work/harness.json"
    rm -rf "$work"

# The replica as the UI's Playwright fixture, made by the backend's readers.
replica-fixture:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo test -q -p juggler-app --test qa_real_readonly --no-run 2> /dev/null
    bin=$(ls -t {{target_dir}}/debug/deps/qa_real_readonly-* | grep -v '\.d$' | sed -n 1p)
    work=$(mktemp -d)
    env -i HOME={{replica_home}} USER="$USER" TMPDIR="$TMPDIR" PATH=/usr/bin:/bin:/usr/sbin:/sbin \
        AIJ_QA_REAL_DATA="$work/data" AIJ_QA_REAL_OUT="$work/harness.json" AIJ_QA_ICONS=1 "$bin" --ignored --nocapture > "$work/harness.log" 2>&1
    python3 -I scripts/qa/replica_fixture.py "$work/harness.json" ui/tests/visual/fixtures/replica.json
    rm -rf "$work"

# The README's screenshots, both themes, from the replica's fixture only
# (`just replica-fixture` makes it, refusing any HOME but the replica's):
# docs/assets/readme/. Never real data.
readme-shots:
    cd ui && AIJ_README_VERSION="${AIJ_README_VERSION:-$(git describe --tags --abbrev=0 --match 'v[0-9]*' | sed 's/^v//')}" AIJ_README_SHOTS=1 AIJ_PW_PORT=${AIJ_PW_PORT:-4595} pnpm exec playwright test readme-shots

# Cargo, rustup and corepack keep their own homes while HOME is the replica.
# Run AI Juggler Dev on the replica HOME.
replica-walk: dev-icon
    CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}" RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}" COREPACK_HOME="${COREPACK_HOME:-$HOME/.cache/node/corepack}" HOME={{replica_home}} cargo tauri dev --config crates/juggler-app/tauri.dev.conf.json

# AI Juggler Dev with the read-only test bridge: the real UI in a browser
# against this build's backend and data, for QA without Accessibility
# (docs/qa/DEV-BRIDGE.md). Then `just dev-bridge-url` for the page.
dev-bridge: dev-icon agent-bridge
    AIJ_DEV_BRIDGE=1 cargo tauri dev --config crates/juggler-app/tauri.dev.conf.json --features dev-bridge

# The page for the running bridge (reads its 0600 token file).
dev-bridge-url:
    python3 scripts/dev_bridge_url.py

# The bridge never ships: off by default, in no release workflow, and a
# release build with it doesn't compile.
dev-bridge-check:
    ./scripts/dev_bridge_check.sh

# Release-mode build of AI Juggler Dev (unsigned, never updates).
app: dev-icon
    cargo tauri build --config crates/juggler-app/tauri.dev.conf.json

# The Dev edition's marked icon (generated; target/ is not committed).
dev-icon:
    swift scripts/edition_icon.swift crates/juggler-app/icons/icon.png target/edition-icons/dev/icon.png Dev C2410C

# AJ-E803 visual regression gate (in the pinned Playwright container):
# compare the deterministic matrix with the approved baselines.
visual-gate:
    ./scripts/visual_gate.sh

# AJ-E804: the accessibility gate (per release candidate): axe on every
# gallery state, the six keyboard flows, 200 % zoom / reduced motion,
# the contrast table — evidence in docs/qa/evidence/AJ-E804/.
a11y-gate:
    ./scripts/a11y_gate.sh

# Re-render the baselines after an intentional UI change; the PR then
# needs a "## Visual baselines" note per changed screen.
visual-baselines:
    ./scripts/visual_gate.sh --update

# The gate's self-test: a seeded 2 px button shift must fail.
visual-selftest:
    ./scripts/visual_gate.sh --selftest

# D-21 full sweep (AJ-E811): render the WHOLE gallery at the merge-base
# with `base` and at `head` in the pinned container and report every
# state that differs (target/visual-sweep/). For new or rebuilt screens
# and on demand; the core gate above runs on every UI PR.
visual-sweep base="origin/main" head="HEAD":
    ./scripts/visual_sweep.sh --base {{base}} --head {{head}}
