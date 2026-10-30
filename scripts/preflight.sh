#!/usr/bin/env bash
# The whole of CI on this Mac, on the commit about to be pushed (#198:
# a local run that skipped a gate after a late change, and races waved
# through as "passes alone", cost four CI rounds).
#
# - The tree must be clean: what passes is exactly the commit (HEAD).
# - Every step runs, failed or not, and the summary names each failure.
# - Steps: CI's verify job (`just verify`, kept in step by
#   scripts/ci_parity.py), CI's ui job (vitest, the build), Linux clippy
#   in a container (CI's Linux jobs), and, when ui/ changed against the
#   base as CI decides it, the visual gate in CI's pinned container and
#   the a11y gate. The UI suites rewrite tracked evidence; it is restored.
# - On success it stamps HEAD; scripts/hooks/pre-push refuses a push to
#   main or release/* of a commit without a stamp.
#
# Usage: scripts/preflight.sh [--base REF]   (default: origin/main)
set -uo pipefail
cd "$(dirname "$0")/.." || exit 2
BASE=origin/main
while [ $# -gt 0 ]; do
  case "$1" in
    --base) BASE="$2"; shift 2 ;;
    *) echo "preflight: unknown argument $1" >&2; exit 2 ;;
  esac
done

if [ -n "$(git status --porcelain)" ]; then
  echo "preflight: commit or set aside your changes first; it checks the commit, not the working tree." >&2
  git status --short >&2
  exit 2
fi
SHA=$(git rev-parse HEAD)
LOGS="target/preflight/${SHA:0:12}"
mkdir -p "$LOGS"
STAMPS="$(git rev-parse --git-common-dir)/aij-preflight"
mkdir -p "$STAMPS"
echo "preflight: $(git branch --show-current) at ${SHA:0:12}, against $BASE; logs in $LOGS"

failed=()
step() {
  local name="$1"; shift
  local log
  log="$LOGS/$(echo "$name" | tr ' /' '__').log"
  local t0=$SECONDS
  printf '  %-34s' "$name"
  if "$@" >"$log" 2>&1; then
    printf 'ok    (%ss)\n' $((SECONDS - t0))
  else
    printf 'FAIL  (%ss)  %s\n' $((SECONDS - t0)) "$log"
    failed+=("$name")
  fi
}
restore_evidence() {
  git checkout -q -- docs/qa/evidence && git clean -fdq docs/qa/evidence
}

git fetch -q origin 2>/dev/null || true
# CI's rule for the visual and a11y jobs. No `| grep -q`: under pipefail
# grep's early exit cut git off mid-write and read as "no UI change".
UI=false
if ! git rev-parse -q --verify "$BASE" >/dev/null \
  || [ -n "$(git diff --name-only "$BASE" HEAD -- ui scripts/a11y_gate.sh scripts/axe_summary.py .github/workflows/ci.yml)" ]; then
  UI=true
fi

# A target shared with other checkouts keeps their paths in this
# workspace's crates (CARGO_MANIFEST_DIR); build ours afresh.
if [ -n "${CARGO_TARGET_DIR:-}" ]; then
  for pkg in $(cargo metadata -q --no-deps --format-version 1 | python3 -c 'import json,sys; print(" ".join(p["name"] for p in json.load(sys.stdin)["packages"]))'); do
    cargo clean -q -p "$pkg" 2>/dev/null
  done
fi
step "verify (CI's verify job)" just verify
step "ui: vitest" bash -c 'cd ui && pnpm test'
step "ui: build" bash -c 'cd ui && pnpm build'
step "Linux clippy (3 variants)" ./scripts/linux_clippy.sh
if [ "$UI" = true ]; then
  step "visual gate (pinned container)" ./scripts/visual_gate.sh --reporter=line
  step "a11y gate" ./scripts/a11y_gate.sh
  restore_evidence
else
  echo "  visual and a11y gates             skipped: no UI change against $BASE (as CI decides)"
fi

if [ "$(git rev-parse HEAD)" != "$SHA" ] || [ -n "$(git status --porcelain)" ]; then
  echo "preflight: the tree changed while it ran; run it again on the commit you mean to push." >&2
  git status --short >&2
  exit 1
fi
if [ ${#failed[@]} -gt 0 ]; then
  echo "preflight: ${#failed[@]} failed: ${failed[*]}"
  exit 1
fi
printf '%s %s ui=%s base=%s\n' "$SHA" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$UI" "$BASE" > "$STAMPS/$SHA"
echo "preflight: all passed; ${SHA:0:12} may be pushed to main or release/*."
