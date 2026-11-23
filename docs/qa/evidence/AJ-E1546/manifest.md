# Evidence manifest — AJ-E1546

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1546, a guided tour on the real UI,
offered once, and short clips (P1). Status: **IN-REVIEW** (built 2026-11-23).

Captures (1280×800, both themes, generic fixtures, checked by eye):
[Bot's stop with no AI](docs/qa/evidence/AJ-E1546/tour-1-bot-no-ai-dark.png),
[Open now, anchored](docs/qa/evidence/AJ-E1546/tour-2-open-now-light.png),
[Usage](docs/qa/evidence/AJ-E1546/tour-5-usage-dark.png),
[a stop off screen, its clip](docs/qa/evidence/AJ-E1546/tour-6-switch-clip-dark.png).

| Claim | Verdict | Proof |
|---|---|---|
| A registry: each step's feature id, its version, its target by element id, Bot's scripted words (and words without an AI, D-144), its clip, its question | PASS | `tour.test.tsx` |
| Completion stored per id; never offered again; after an update only unseen steps, once, "New in 0.5.x · Show me" | PASS | `tour.test.tsx` |
| Coach marks anchored with a ring; a target off screen shows its clip; Back, Next, Skip, a progress bar, Esc, ← →, Tab kept inside, focus back where it was when it ends | PASS | `tour.test.tsx` |
| Bot is the guide: Bot's mark and voice, first stop Bot's panel (opened and closed by the tour); "Ask Bot about this" only while Bot's AI is ready | PASS | `tour.test.tsx` |
| Offered at the end of setup and from Help › Take the tour (your menu, and the app menu's Help item) | PASS | `setup-wizard.test.tsx`, `notifications.test.tsx` |
| Clips: 10 (5 What it does, 3 How it works, 2 permissions) × 2 themes, VP9 WebM 640×400, 3–6 s, muted, lazy, paused off screen, three plays then rest with Pause/Play, a still poster under Reduce Motion; 10–46 KB each, poster 30–131 KB, all ≤ 400 KB with poster | PASS | `tests/bundle/clips-size.test.ts`; `scripts/record_clips.sh` (Playwright's cached Chromium and ffmpeg-1013, nothing downloaded) |

Not done: Pools and Settings have no clip of their own (their stops anchor
on the real header and your menu, which are always on screen).
