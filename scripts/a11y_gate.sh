#!/usr/bin/env bash
# AJ-E804: the accessibility gate, run per release candidate.
#   - axe-core (WCAG 2.0/2.1/2.2 A+AA) on every gallery state, against a
#     solid canvas so contrast is really evaluated;
#   - keyboard-only traversals of the six core flows;
#   - 200 % zoom and reduced-motion captures of rail, inspector, wizard;
#   - the rail's Apps with nothing open: Inactive apps, an App's last 3
#     under "Recent", then "All N sessions ›" (D-103.1/.2);
#   - what the 0.4.6 RC walks found cut or scrolling sideways, measured
#     (fits-its-room);
#   - the measured contrast table.
# Extra arguments go to Playwright (the release workflow passes
# `--shard=i/n`; each shard sets A11Y_SKIP_SUMMARY=1 and one final job
# summarises every shard's scans).
# Writes docs/qa/evidence/AJ-E804/ (axe/, axe-summary.md, keyboard/,
# zoom-motion/, contrast.md). Fails on any critical or serious axe
# violation, any moderate rule without an owning task, or a broken flow.
set -uo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
[ -n "${A11Y_SKIP_SUMMARY:-}" ] || rm -rf "$root/docs/qa/evidence/AJ-E804/axe"
cd "$root/ui"
# Free disk first (2026-11-21: parallel sessions drove it to 180 MB).
python3 "$root/scripts/disk_guard.py" "the accessibility gate" || exit 1
AJ_AXE_GALLERY=1 AJ_A11Y_EVIDENCE=1 pnpm exec playwright test \
  tests/visual/capture.spec.ts \
  tests/visual/keyboard-flows.spec.ts \
  tests/visual/zoom-motion.spec.ts \
  tests/visual/contrast.spec.ts \
  tests/visual/inactive-apps-gallery.spec.ts \
  tests/visual/a11y.spec.ts \
  tests/visual/idle.spec.ts \
  tests/visual/menus-a11y.spec.ts \
  tests/visual/pool-layout.spec.ts \
  tests/visual/fits-its-room.spec.ts \
  "$@"
tests=$?
summary=0
if [ -z "${A11Y_SKIP_SUMMARY:-}" ]; then
  python3 "$root/scripts/axe_summary.py"
  summary=$?
fi
[ "$tests" -eq 0 ] && [ "$summary" -eq 0 ]
