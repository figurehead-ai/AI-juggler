#!/usr/bin/env bash
# The test seams never ship (SECURITY-GUIDELINES C8): a release binary
# must not contain the environment variables that silence or answer AI
# Juggler's dialogs. They're compiled only into debug builds
# (`launch_alert::seams`); this checks the built binary itself.
#
# Usage: scripts/release_seams_check.sh <release binary>
#        scripts/release_seams_check.sh --selftest <debug binary>
#   (the self-test: the debug binary must contain them, so the check
#   can see them when they're there)
set -euo pipefail
SEAMS='AIJ_NO_DIALOGS|AIJ_DIALOG_ANSWER|AIJ_WATCHDOG'
if [ "${1:-}" = "--selftest" ]; then
  bin=${2:?a debug binary}
  if strings -a "$bin" | grep -E "$SEAMS" >/dev/null; then
    echo "release-seams selftest: OK (the debug build has them)"
    exit 0
  fi
  echo "release-seams selftest: the debug binary doesn't show the seams, so the check can't see them" >&2
  exit 1
fi
bin=${1:?a release binary}
if found=$(strings -a "$bin" | grep -E "$SEAMS"); then
  echo "release-seams: the release binary carries a test seam:" >&2
  echo "$found" | sort -u >&2
  exit 1
fi
echo "release-seams: none in $(basename "$bin")"
