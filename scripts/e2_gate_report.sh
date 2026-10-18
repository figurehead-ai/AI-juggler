#!/bin/sh
# AJ-E210: regenerate docs/qa/gate-E2-validation.md from the
# validation matrix (the same table the UI badges derive from).
set -e
cd "$(dirname "$0")/.."
if [ -n "$RUSTFLAGS_FILE" ]; then :; fi
exec cargo run -q -p juggler-handover --example e2_gate_report
