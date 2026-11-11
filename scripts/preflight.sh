#!/usr/bin/env bash
# The local check of the commit about to be pushed (ADR-0025). GitHub
# runs every gate on every pull request, split across its free runners;
# this catches what would fail there, quickly and with little load on a
# Mac other projects share.
#
# - It checks a snapshot of HEAD (a detached worktree under the system
#   temp folder, removed afterwards): edit the tree while it runs, and the
#   result still belongs to the commit. Uncommitted changes aren't in it.
# - One at a time (a lock), at low priority (`nice`), with a small fixed
#   number of build jobs and test threads (AIJ_PREFLIGHT_JOBS, default 4).
# - Its own build folder (AIJ_PREFLIGHT_TARGET, default target/preflight-
#   build in the main checkout): only builds from the snapshot's one path
#   use it, so no other checkout's paths end up in its test binaries and
#   no other checkout's build is disturbed (no `cargo clean` needed).
# - Fast mode (the default):
#   1. the release notes this branch changed, as the nightly checks them;
#   2. the static checks (`just verify-static`: lints, fmt, clippy ×3,
#      rustdoc, cargo-deny). A failure here stops the run;
#   3. the tests of the packages the change can affect
#      (scripts/affected_crates.py against the base: all of them when the
#      build as a whole changed), the dev-bridge tests when the app is one,
#      and no fixture left running;
#   4. vitest and the UI build.
#   Every step of 3–4 runs, failed or not. No visual, a11y or Linux-clippy
#   container runs here: CI runs them.
# - `--light` (2026-11-11: CI is the heavy gate, and builds the release
#   candidate, rc-dev-app): the static checks (`just verify-static`), the
#   tests of only the packages the change touches itself
#   (`cargo nextest run -p ...`, no dependents, no doctests; none when the
#   build as a whole changed), vitest only for the UI files it changed
#   (`vitest related`), then the UI's lint, tsc and the debt baseline. No
#   UI build. Its base, unless --base names one, is the branch's upstream
#   (what the push adds), else origin/main.
# - `--full`: the old whole-of-CI run on this Mac: `just verify` (every
#   test), vitest, the UI build, Linux clippy in a container, and the
#   visual and a11y gates when ui/ changed against the base.
# - The log folder (target/preflight/<sha>/) gets one log per step and
#   summary.txt. On success HEAD is stamped (any mode); scripts/hooks/pre-push
#   lets a stamped commit through to main and release/*.
#
# Usage: scripts/preflight.sh [--base REF] [--light | --full]   (default base: origin/main)
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT" || exit 2
BASE=""
MODE=fast
while [ $# -gt 0 ]; do
  case "$1" in
    --base) BASE="$2"; shift 2 ;;
    --full) MODE=full; shift ;;
    --light) MODE=light; shift ;;
    *) echo "preflight: unknown argument $1" >&2; exit 2 ;;
  esac
done
if [ -z "$BASE" ]; then
  BASE=origin/main
  # Light: what the push adds, against the branch's upstream.
  if [ "$MODE" = light ] && upstream="$(git rev-parse -q --abbrev-ref --symbolic-full-name '@{upstream}' 2>/dev/null)"; then
    BASE="$upstream"
  fi
fi

SHA=$(git rev-parse HEAD)
COMMON="$(cd "$(git rev-parse --git-common-dir)" && pwd)"
MAIN_CHECKOUT="$(dirname "$COMMON")"
STAMPS="$COMMON/aij-preflight"
LOGS="$ROOT/target/preflight/${SHA:0:12}"
mkdir -p "$STAMPS" "$LOGS"
rm -f "$LOGS"/*.log "$LOGS/summary.txt"
SUMMARY="$LOGS/summary.txt"
JOBS="${AIJ_PREFLIGHT_JOBS:-4}"
NICE=(nice -n "${AIJ_PREFLIGHT_NICE:-10}")

# One preflight at a time on this repository: two at once fail each
# other's timing tests (6a0b1d5, 2026-11-01).
LOCK="$STAMPS/lock"
if ! mkdir "$LOCK" 2>/dev/null; then
  other="$(cat "$LOCK/pid" 2>/dev/null || true)"
  if [ -n "$other" ] && kill -0 "$other" 2>/dev/null; then
    echo "preflight: another preflight (pid $other) is running; one at a time." >&2
    exit 2
  fi
  rm -rf "$LOCK"
  mkdir "$LOCK" || { echo "preflight: can't take $LOCK" >&2; exit 2; }
fi
echo $$ > "$LOCK/pid"

TMP="${TMPDIR:-/tmp}"
SNAP="${TMP%/}/aij-preflight-$(printf '%s' "$COMMON" | shasum | cut -c1-10)"
remove_snapshot() {
  git -C "$ROOT" worktree remove --force "$SNAP" >/dev/null 2>&1 || rm -rf "$SNAP"
  git -C "$ROOT" worktree prune
}
# shellcheck disable=SC2329 # the EXIT trap calls it
cleanup() { remove_snapshot; rm -rf "$LOCK"; }
trap cleanup EXIT
trap 'exit 130' INT TERM
[ -e "$SNAP" ] && remove_snapshot
git fetch -q origin 2>/dev/null || true
if ! git worktree add -q --detach "$SNAP" "$SHA"; then
  echo "preflight: couldn't make the snapshot at $SNAP" >&2
  exit 2
fi
export CARGO_TARGET_DIR="${AIJ_PREFLIGHT_TARGET:-$MAIN_CHECKOUT/target/preflight-build}"
export CARGO_BUILD_JOBS="$JOBS" NEXTEST_TEST_THREADS="$JOBS"
cd "$SNAP" || exit 2

echo "preflight ($MODE): $(git -C "$ROOT" branch --show-current) at ${SHA:0:12}, against $BASE; logs in $LOGS"
if [ -n "$(git -C "$ROOT" status --porcelain)" ]; then
  echo "  (uncommitted changes in your checkout are not part of it: it checks ${SHA:0:12})"
fi
printf 'preflight %s of %s against %s, %s\n' "$MODE" "$SHA" "$BASE" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$SUMMARY"

started=$SECONDS
failed=()
step() {
  local name="$1"; shift
  local log
  log="$LOGS/$(printf '%s' "$name" | tr -c 'A-Za-z0-9.-' '_').log"
  local t0=$SECONDS
  printf '  %-44s' "$name"
  if "${NICE[@]}" "$@" >"$log" 2>&1; then
    printf 'ok    (%ss)\n' $((SECONDS - t0))
    printf '%-44s ok    %5ss\n' "$name" $((SECONDS - t0)) >> "$SUMMARY"
    return 0
  fi
  printf 'FAIL  (%ss)  %s\n' $((SECONDS - t0)) "$log"
  printf '%-44s FAIL  %5ss  %s\n' "$name" $((SECONDS - t0)) "$log" >> "$SUMMARY"
  failed+=("$name")
  return 1
}
skip() {
  printf '  %-44s%s\n' "$1" "skipped: $2"
  printf '%-44s skip         %s\n' "$1" "$2" >> "$SUMMARY"
}
finish() {
  local total=$((SECONDS - started))
  if [ ${#failed[@]} -gt 0 ]; then
    printf 'result: %s failed (%s) in %ss\n' "${#failed[@]}" "${failed[*]}" "$total" >> "$SUMMARY"
    echo "preflight: ${#failed[@]} failed in ${total}s: ${failed[*]} (summary: $SUMMARY)"
    exit 1
  fi
  printf 'result: passed in %ss\n' "$total" >> "$SUMMARY"
  printf '%s %s mode=%s tests=%s ui=%s base=%s\n' "$SHA" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$MODE" "$TESTS" "$UI" "$BASE" > "$STAMPS/$SHA"
  echo "preflight: all passed in ${total}s; ${SHA:0:12} may be pushed to main or release/* (CI runs every gate on the pull request)."
  exit 0
}

# CI's rule for the visual and a11y jobs (--full only). No `| grep -q`:
# under pipefail grep's early exit cut git off mid-write.
UI=false
if ! git rev-parse -q --verify "$BASE" >/dev/null \
  || [ -n "$(git diff --name-only "$BASE" HEAD -- ui scripts/a11y_gate.sh scripts/axe_summary.py .github/workflows/ci.yml)" ]; then
  UI=true
fi

step "release notes (as the nightly checks)" ./scripts/release_notes_changed.sh "$BASE"
# On a release branch, say what of its roadmap scope is still open (report
# only; the Stable promotion alone fails on it, D-99).
BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [[ "$BRANCH" == release/* ]]; then
  python3 scripts/release_scope.py report "${BRANCH#release/}" | head -1 | sed 's/^/  /'
fi
if ! step "static (just verify-static)" just verify-static; then
  TESTS=none
  echo "  the static checks failed: stopping before the tests (fix them first; the log names each)."
  finish
fi

# Anything a test starts and leaves running from here on is a leak
# (scripts/leaked_fixtures.sh: an AI Juggler a test launched included).
AIJ_TESTS_STARTED=$(date +%s)
export AIJ_TESTS_STARTED
if [ "$MODE" = full ]; then
  TESTS=all
  step "tests (just verify-tests)" just verify-tests
elif [ "$MODE" = light ]; then
  touched="$(python3 scripts/affected_crates.py --direct "$BASE" HEAD)" || touched=ALL
  if [ "$touched" = ALL ]; then
    TESTS=none
    skip "Rust tests" "the build as a whole changed against $BASE: CI runs every test"
  elif [ -z "$touched" ]; then
    TESTS=none
    skip "Rust tests" "no package changed against $BASE"
  else
    # shellcheck disable=SC2086 # one package per word
    set -- $touched
    TESTS="$(echo "$touched" | tr '\n' ',' | sed 's/,$//')"
    packages=()
    for name in "$@"; do packages+=(-p "$name"); done
    step "tests (the $# changed packages)" cargo nextest run --no-fail-fast "${packages[@]}"
    step "no fixture or app left running" ./scripts/leaked_fixtures.sh
  fi
else
  affected="$(python3 scripts/affected_crates.py "$BASE" HEAD)" || affected=ALL
  if [ "$affected" = ALL ]; then
    TESTS=all
    step "tests (all: the build changed)" just verify-tests
  elif [ -z "$affected" ]; then
    TESTS=none
    skip "Rust tests" "no package's tests can change against $BASE"
  else
    # shellcheck disable=SC2086 # one package per word
    set -- $affected
    TESTS="$(echo "$affected" | tr '\n' ',' | sed 's/,$//')"
    step "tests ($# of the packages)" just test-packages "$@"
    if printf '%s\n' "$affected" | grep -qx juggler-app; then
      step "dev-bridge tests" just dev-bridge-tests
      step "dev-bridge never ships (quick)" ./scripts/dev_bridge_check.sh --quick
      step "floor-harness never ships" ./scripts/floor_harness_check.sh
    fi
    step "no fixture or app left running" ./scripts/leaked_fixtures.sh
  fi
fi

if [ "$MODE" = light ]; then
  # The UI's sources and tests this change touched (still in the tree).
  changed_ui=()
  while IFS= read -r path; do
    [ -f "$path" ] && changed_ui+=("${path#ui/}")
  done < <(git diff --name-only "$(git merge-base "$BASE" HEAD 2>/dev/null || echo "$BASE")" HEAD -- 'ui/src/*.ts' 'ui/src/*.tsx')
  if step "ui: install" bash -c 'cd ui && pnpm install --frozen-lockfile --prefer-offline'; then
    if [ ${#changed_ui[@]} -gt 0 ]; then
      # shellcheck disable=SC2016 # expanded by the inner shell
      step "ui: vitest (${#changed_ui[@]} changed files)" bash -c 'cd ui && pnpm exec vitest related --run --passWithNoTests --maxWorkers="$0" "$@"' "$JOBS" "${changed_ui[@]}"
    else
      skip "ui: vitest" "no UI source changed against $BASE"
    fi
    step "ui: lint" bash -c "cd ui && pnpm lint"
    step "ui: tsc" bash -c "cd ui && pnpm exec tsc -b --noEmit"
    step "ui: dead and copied code" python3 scripts/debt_baseline.py
  fi
elif step "ui: install" bash -c 'cd ui && pnpm install --frozen-lockfile --prefer-offline'; then
  step "ui: vitest" bash -c "cd ui && pnpm exec vitest run --maxWorkers=$JOBS"
  step "ui: lint" bash -c "cd ui && pnpm lint"
  step "ui: dead and copied code" python3 scripts/debt_baseline.py
  step "ui: build" bash -c 'cd ui && pnpm build'
fi

if [ "$MODE" = full ]; then
  step "Linux clippy (3 variants)" ./scripts/linux_clippy.sh
  if [ "$UI" = true ]; then
    step "visual gate (pinned container)" ./scripts/visual_gate.sh --reporter=line
    step "a11y gate" ./scripts/a11y_gate.sh
  else
    skip "visual and a11y gates" "no UI change against $BASE (as CI decides)"
  fi
else
  skip "visual, a11y, Linux clippy" "CI runs them on GitHub (--full runs them here)"
  [ "$MODE" = light ] && skip "UI build, dependents' tests, doctests, release app" "CI runs them (rc-dev-app builds the release candidate)"
fi
finish
