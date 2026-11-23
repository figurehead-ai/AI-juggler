#!/usr/bin/env bash
# AJ-E1546: records the Welcome's, How it works' and the permission cards'
# clips (ui/public/clips) from the gallery's generic fixtures, in both
# themes, on this computer: Playwright's own cached Chromium and ffmpeg
# (~/Library/Caches/ms-playwright/ffmpeg-1013), nothing downloaded. Not a
# gate. Afterwards: open every clip and poster, and check the size budget
# (`pnpm --dir ui exec vitest run tests/bundle/clips-size.test.ts`).
#
# Usage: scripts/record_clips.sh              # every clip
#        AIJ_CLIPS_ONLY=what-bot,how-apps scripts/record_clips.sh
set -euo pipefail
cd "$(dirname "$0")/../ui"
FFMPEG="${AIJ_FFMPEG:-$HOME/Library/Caches/ms-playwright/ffmpeg-1013/ffmpeg-mac}"
[ -x "$FFMPEG" ] || { echo "record-clips: Playwright's cached ffmpeg isn't at $FFMPEG (nothing is downloaded)"; exit 1; }
RAW="$(mktemp -d -t aij-clips-raw)"
trap 'rm -rf "$RAW"' EXIT
AIJ_CLIPS_RAW="$RAW" AIJ_FFMPEG="$FFMPEG" pnpm exec playwright test -c playwright.clips.config.ts "$@"
ls -l public/clips
