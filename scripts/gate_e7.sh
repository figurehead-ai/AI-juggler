#!/bin/sh
# AJ-E710: the automated portion of the Fleet beta gate. Runs the link,
# supervisor and core suites plus every Fleet integration suite of the
# app, and prints a markdown report: what ran, pass counts, and every
# measured timing (`[perf]` lines) — numbers, never estimates.
#
# Usage: scripts/gate_e7.sh > docs/qa/evidence/AJ-E710/automated-<date>.md
# (`just gate-e7` does exactly that). The two-Mac dogfood and the tester
# sign-offs stay human steps in docs/qa/gate-E7.md.
set -eu
cd "$(dirname "$0")/.."
if [ "$(uname)" = "Darwin" ]; then
  RUSTFLAGS="$(./scripts/ensure_swift_runtime.sh | tail -1 | sed 's/^RUSTFLAGS=//')"
  export RUSTFLAGS
fi
log="$(mktemp)"
trap 'rm -f "$log"' EXIT
started=$(date +%s)
status=0
run() {
  echo "### \`$*\`" >> "$log.md"
  if cargo test "$@" -- --nocapture > "$log" 2>&1; then
    result=pass
  else
    result=FAIL
    status=1
  fi
  passed=$(grep -E '^test result:' "$log" | sed -E 's/.* ([0-9]+) passed; ([0-9]+) failed.*/\1 \2/' |
    awk '{p+=$1; f+=$2} END {printf "%d passed, %d failed", p, f}')
  echo "- result: **$result** — $passed" >> "$log.md"
  grep -E '\[perf\]' "$log" | sed 's/^/- /' >> "$log.md" || true
  echo >> "$log.md"
}
: > "$log.md"
run -p juggler-link
run -p juggler-supervisor
run -p juggler-core
run -p juggler-app --test fleet_identity --test fleet_link --test fleet_sync --test fleet_switch
elapsed=$(( $(date +%s) - started ))
cat <<HEADER
# AJ-E710 — Fleet gate, automated portion

- Commit: \`$(git rev-parse --short HEAD)\`
- Host: $(uname -sm), $(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -p)
- Ran: $(date -u +%Y-%m-%dT%H:%MZ) in ${elapsed}s (budget < 15 min)
- Overall: $( [ "$status" = 0 ] && echo "**green**" || echo "**RED — the gate fails closed**" )

Loopback timings are from one machine; the gate-hardware numbers come
from the two-Mac dogfood in \`docs/qa/gate-E7.md\`.

HEADER
cat "$log.md"
rm -f "$log.md"
exit "$status"
