#!/usr/bin/env bash
# AJ-E803: runs the visual regression gate in the pinned Playwright
# container — the same image CI uses — so baselines and comparisons
# share one renderer. `--update` re-renders the baselines (each change
# needs a per-screen note in the PR: scripts/check_baseline_notes.py);
# `--selftest` seeds a 2 px button shift that must fail.
set -euo pipefail
cd "$(dirname "$0")/../ui"
IMAGE="mcr.microsoft.com/playwright:v1.64.0-noble"
ARGS=""
ENV_SELFTEST=0
for arg in "$@"; do
  case "$arg" in
    --update) ARGS="--update-snapshots" ;;
    --selftest) ENV_SELFTEST=1 ;;
  esac
done
docker run --rm --ipc=host --platform linux/amd64 \
  -v "$PWD":/work -w /work \
  -v aij-visual-node-modules-amd64:/work/node_modules \
  -e CI=1 -e AIJ_VISUAL_SELFTEST="$ENV_SELFTEST" \
  "$IMAGE" bash -lc "corepack enable >/dev/null 2>&1 && corepack prepare pnpm@10 --activate >/dev/null 2>&1 && pnpm install --frozen-lockfile >/dev/null && pnpm exec playwright test -c playwright.regression.config.ts $ARGS"
