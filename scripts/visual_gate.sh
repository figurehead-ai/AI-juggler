#!/usr/bin/env bash
# AJ-E803: runs the visual regression gate in the pinned Playwright
# container — the same image CI uses — so baselines and comparisons
# share one renderer. `--update` re-renders the baselines (each change
# needs a per-screen note in the PR: scripts/check_baseline_notes.py;
# every baseline is re-rendered, so a change under the comparison
# threshold still lands in the baseline);
# `--selftest` seeds a 2 px button shift that must fail.
# Anything else goes to Playwright as is, to run a part of the gate:
# `--update tests/visual/regression.spec.ts -g "rail with pins"`.
# `--full-sweep [--base REF] …` is the other mode (D-21): the whole
# gallery, base against head, no baselines — scripts/visual_sweep.sh.
set -euo pipefail
if [ "${1:-}" = --full-sweep ]; then shift; exec "$(dirname "$0")/visual_sweep.sh" "$@"; fi
cd "$(dirname "$0")/../ui"
# Pinned by digest: the tag was re-published (2026-11-01) with a Node
# that has no corepack, and CI broke while this Mac's cached image worked.
IMAGE="mcr.microsoft.com/playwright:v1.64.1-noble@sha256:47b510dd9230bf0b9bfc9b20102241e577ea7a99334ab4c3ca19f19770e9b8ea"
ARGS=""
ENV_SELFTEST=0
for arg in "$@"; do
  case "$arg" in
    --update) ARGS="$ARGS --update-snapshots=all" ;;
    --selftest) ENV_SELFTEST=1 ;;
    *) ARGS="$ARGS $(printf '%q' "$arg")" ;;
  esac
done
docker run --rm --ipc=host --platform linux/amd64 \
  -v "$PWD":/work -w /work \
  -v aij-visual-node-modules-amd64:/work/node_modules \
  -e CI=1 -e AIJ_VISUAL_SELFTEST="$ENV_SELFTEST" \
  "$IMAGE" bash -lc "corepack enable >/dev/null 2>&1 && corepack prepare pnpm@10 --activate >/dev/null 2>&1 && pnpm install --frozen-lockfile >/dev/null && pnpm exec playwright test -c playwright.regression.config.ts $ARGS"
