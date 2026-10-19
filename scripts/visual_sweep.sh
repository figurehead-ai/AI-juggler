#!/usr/bin/env bash
# D-21 / AJ-E811: the full-sweep visual check. Renders the WHOLE gallery
# (everything `pnpm visual:capture` writes to docs/qa/evidence/_gallery/)
# twice — at the base of a change and at its head — in the pinned
# Playwright container the visual gate uses, then diffs every state
# pairwise. No stored baselines: both sides come from one renderer, so
# any differing pixel is the change. The core mode (scripts/visual_gate.sh,
# the required `visual` check) is separate and unchanged.
#
# Each side is rendered twice. A state counts as CHANGED only when base
# and head differ and each side rendered it the same both times; a
# state that moved between two renders of one commit is listed as
# UNSTABLE instead. For settled renders the sweep forces
# prefers-reduced-motion on every page (the app then freezes its
# transitions) and turns Chromium's partial raster off; it retries a
# failed test once and runs the gallery fully parallel.
#
# On the host (needs Docker):
#   scripts/visual_sweep.sh [--base REF] [--head REF] [--tip] [--once] [--out DIR] [-- PLAYWRIGHT ARGS]
#     --base REF  default origin/main (fetched first when it is origin/…)
#     --head REF  default HEAD — committed work only; commit a scratch
#                 change to sweep it
#     --tip       compare with the base ref's tip instead of the merge-base
#                 (by default only the head's own changes show)
#     --once      one render per side: faster, but unstable states are
#                 then reported as changed
#     --out DIR   report directory (default target/visual-sweep/<base>..<head>)
#   Uses the host's native container platform: the renders only need
#   to match each other, and native is several times faster than the
#   emulated amd64 the core baselines need.
#
# Inside the container (what the host mode and the visual-sweep
# workflow run):
#   visual_sweep.sh render TREE OUT [PLAYWRIGHT ARGS]   e.g. --shard=2/4
#   visual_sweep.sh diff SWEEP REPORT [UI_DIR]
#     SWEEP holds base/ and head/ (and base-recheck/, head-recheck/)
set -euo pipefail
IMAGE="mcr.microsoft.com/playwright:v1.64.1-noble"
TOOLS="$(cd "$(dirname "$0")/.." && pwd)"

# --- render: one side (or one shard of it) -------------------------------
if [ "${1:-}" = render ]; then
  tree="$(cd "$2" && pwd)"; out="$3"; shift 3
  mkdir -p "$out/_render"; out="$(cd "$out" && pwd)"
  id="$(printf '%s' "${*:-all}" | tr -c 'A-Za-z0-9' '-')"
  gallery="$tree/docs/qa/evidence/_gallery"
  cd "$tree/ui"
  corepack enable >/dev/null 2>&1 && corepack prepare pnpm@10 --activate >/dev/null 2>&1
  pnpm install --frozen-lockfile >/dev/null
  # Committed captures must not stand in for states this side fails to
  # render. --fully-parallel: the gallery is a few large files, which
  # would otherwise run on one worker each and could not be sharded.
  [ -d "$gallery" ] && find "$gallery" -name '*.png' -delete
  # The side's own config, with reduced motion forced on every page and
  # partial raster off: with it on, Chromium's repaint of a changed
  # region can anti-alias the edges around it differently from a fresh
  # paint (measured: 12 of 22 rail states varied over five identical
  # runs; with it off, none).
  cat > playwright.sweep.config.ts <<'TS'
import base from "./playwright.config";
const use = base.use ?? {};
export default {
  ...base,
  use: {
    ...use,
    contextOptions: { ...use.contextOptions, reducedMotion: "reduce" as const },
    launchOptions: { ...use.launchOptions, args: [...(use.launchOptions?.args ?? []), "--disable-partial-raster"] },
  },
};
TS
  set +e
  CI=1 PLAYWRIGHT_JSON_OUTPUT_NAME="$out/_render/$id.playwright.json" \
    pnpm exec playwright test -c playwright.sweep.config.ts --fully-parallel --retries=1 --reporter=dot,json "$@"
  status=$?
  set -e
  (cd "$gallery" && find . -name '*.png' -print0 | tar --null -cf - -T -) | (cd "$out" && tar -xf -)
  sha="${SWEEP_SHA:-$(git -C "$tree" rev-parse HEAD 2>/dev/null || echo unknown)}"
  printf '{"sha":"%s","ref":"%s","exit":%d,"captures":%d}\n' "$sha" "${SWEEP_REF:-}" "$status" \
    "$(find "$out" -name '*.png' | wc -l | tr -d ' ')" > "$out/_render/$id.meta.json"
  echo "rendered $(find "$out" -name '*.png' | wc -l | tr -d ' ') captures at ${sha:0:12} (playwright exit $status)"
  # Bind mounts on a Linux host: hand the files back to the caller.
  if [ -n "${HOST_UID:-}" ]; then chown -R "$HOST_UID:${HOST_GID:-$HOST_UID}" "$out" "$tree" 2>/dev/null || true; fi
  exit 0
fi

# --- diff: the report ----------------------------------------------------
if [ "${1:-}" = diff ]; then
  set +e
  node "$TOOLS/scripts/visual_sweep_diff.mjs" --ui "${4:-$TOOLS/ui}" "$2" "$3"
  status=$?
  if [ -n "${HOST_UID:-}" ]; then chown -R "$HOST_UID:${HOST_GID:-$HOST_UID}" "$3" 2>/dev/null || true; fi
  exit $status
fi

# --- host: base and head in the pinned container --------------------------
base_ref="origin/main"; head_ref="HEAD"; tip=0; once=0; out=""; pw=()
while [ $# -gt 0 ]; do
  case "$1" in
    --base) base_ref="$2"; shift 2 ;;
    --head) head_ref="$2"; shift 2 ;;
    --tip) tip=1; shift ;;
    --once) once=1; shift ;;
    --out) out="$2"; shift 2 ;;
    --) shift; pw=("$@"); break ;;
    *) echo "unknown argument: $1 (see the header of $0)" >&2; exit 2 ;;
  esac
done
cd "$TOOLS"
case "$base_ref" in origin/*) git fetch --quiet origin "${base_ref#origin/}" || echo "warning: could not fetch $base_ref; using the local copy" >&2 ;; esac
head_sha="$(git rev-parse --verify "$head_ref^{commit}")"
if [ "$tip" = 1 ]; then base_sha="$(git rev-parse --verify "$base_ref^{commit}")"
else base_sha="$(git merge-base "$base_ref" "$head_sha")"; fi
[ -n "$out" ] || out="target/visual-sweep/${base_sha:0:9}..${head_sha:0:9}"
mkdir -p "$out"; out="$(cd "$out" && pwd)"
echo "visual sweep: base ${base_sha:0:12} ($base_ref$([ "$tip" = 1 ] && echo ' tip' || echo ' merge-base')) → head ${head_sha:0:12} ($head_ref)"
[ "$base_sha" = "$head_sha" ] && echo "note: base and head are the same commit — this measures the sweep's own noise"

work="$(mktemp -d "${TMPDIR:-/tmp}/aij-visual-sweep.XXXXXX")"
cleanup() {
  for side in base head; do git worktree remove --force --force "$work/$side" >/dev/null 2>&1 || true; done
  rm -rf "$work"
}
trap cleanup EXIT
trap 'exit 130' INT TERM
arch="$(docker version --format '{{.Server.Arch}}')"
outs="out"; times=1; [ "$once" = 1 ] || { outs="out out-recheck"; times=2; }
for side in base head; do
  sha="$base_sha"; ref="$base_ref"
  [ "$side" = head ] && { sha="$head_sha"; ref="$head_ref"; }
  git worktree add --quiet --detach "$work/$side" "$sha"
  renders=""
  for o in $outs; do
    dest="/sweep/$side${o#out}"
    renders="$renders bash /sweep-tools/scripts/visual_sweep.sh render /work $dest \"\$@\" &&"
  done
  echo "rendering the $side gallery (${sha:0:12}), $times time(s)…"
  # One node_modules volume per side, so base and head lockfiles never
  # reinstall over each other; one shared pnpm store.
  docker run --rm --ipc=host --platform "linux/$arch" \
    -v "$work/$side":/work -v "$TOOLS/scripts":/sweep-tools/scripts:ro -v "$work/sweep":/sweep \
    -v "aij-sweep-node-modules-$arch-$side":/work/ui/node_modules \
    -v aij-sweep-pnpm-store:/pnpm-store -e npm_config_store_dir=/pnpm-store \
    -e SWEEP_SHA="$sha" -e SWEEP_REF="$ref" -e HOST_UID="$(id -u)" -e HOST_GID="$(id -g)" \
    "$IMAGE" bash -c "${renders% &&}" sweep ${pw[@]+"${pw[@]}"}
done
echo "diffing…"
docker run --rm --ipc=host --platform "linux/$arch" \
  -v "$work/head":/work -v "$TOOLS/scripts":/sweep-tools/scripts:ro \
  -v "aij-sweep-node-modules-$arch-head":/work/ui/node_modules \
  -v "$work/sweep":/sweep:ro -v "$out":/report \
  -e HOST_UID="$(id -u)" -e HOST_GID="$(id -g)" \
  "$IMAGE" bash /sweep-tools/scripts/visual_sweep.sh diff /sweep /report /work/ui
echo "report: $out/index.html (summary: $out/summary.md)"
