# Evidence manifest — AJ-E1546

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1546, a guided tour on the real UI,
offered once, and short clips (P1). Status: **IN-REVIEW** (built 2026-11-23).

Captures (1280×800, both themes, generic fixtures, checked by eye):
[Bot's stop with no AI](docs/qa/evidence/AJ-E1546/tour-1-bot-no-ai-dark.png),
[Open now, anchored](docs/qa/evidence/AJ-E1546/tour-2-open-now-light.png),
[Usage](docs/qa/evidence/AJ-E1546/tour-5-usage-dark.png),
[a stop off screen, its clip](docs/qa/evidence/AJ-E1546/tour-6-switch-clip-dark.png).

| Claim (R1, R2, R5; each R in the close-out below) | Verdict | Proof |
|---|---|---|
| A registry: each step's feature id, its version, its target by element id, Bot's scripted words (and words without an AI, D-144), its clip, its question | PASS | `tour.test.tsx` |
| Completion stored per id; never offered again; after an update only unseen steps, once, "New in 0.5.x · Show me" | PASS | `tour.test.tsx` |
| Coach marks anchored with a ring; a target off screen shows its clip; Back, Next, Skip, a progress bar, Esc, ← →, Tab kept inside, focus back where it was when it ends | PASS | `tour.test.tsx` |
| Bot is the guide: Bot's mark and voice, first stop Bot's panel (opened and closed by the tour); "Ask Bot about this" only while Bot's AI is ready | PASS | `tour.test.tsx` |
| Offered at the end of setup and from Help › Take the tour (your menu, and the app menu's Help item) | PASS | `setup-wizard.test.tsx`, `notifications.test.tsx` |
| Clips: 10 (5 What it does, 3 How it works, 2 permissions) × 2 themes, VP9 WebM 640×400, 3–6 s, muted, lazy, paused off screen, three plays then rest with Pause/Play, a still poster under Reduce Motion; 10–46 KB each, poster 30–131 KB, all ≤ 400 KB with poster | PASS | `tests/bundle/clips-size.test.ts`; `scripts/record_clips.sh` (Playwright's cached Chromium and ffmpeg-1013, nothing downloaded) |

Not done: Pools and Settings have no clip of their own (their stops anchor
on the real header and your menu, which are always on screen).

## Close-out (0.5.1, 2026-11-23, Opus)

Each test named below was found by name and run, targeted, on `release/0.5.1` (vitest by file; cargo through `scripts/heavy.sh`, `--lib` with module filters). An OPEN line says where it closes.

R1: PASS — vitest `components/tour/tour.test.tsx` "covers the specced stops", "anchors on its element when it's on screen, else shows the clip; Next, Back, the progress bar", "Esc ends it, Skip ends it, Done ends it…"; offered at the end of setup: `setup-wizard.test.tsx` "…then the tour"; Help › Take the tour: `components/notifications.test.tsx`, the app menu's item (275dcc086).
R2: PASS — `tour.test.tsx` "every step has a unique id, a feature, its version…", "stores each step seen, once", "offers nothing to someone who never took it, or once everything is seen", "after an update, offers only the steps not seen, once, as New in the newest version".
R3: PASS — vitest `components/tour/tour.test.tsx` "keeps Tab inside the card, Esc ends it, and focus goes back to what opened it (AJ-E1546 R3)": opened from a button, focus goes into the card; Tab past the last control wraps to the first, Shift+Tab before the first wraps to the last, and Shift+Tab from the focused title stays inside; Esc ends the tour and focus is back on the button that opened it. The test found two faults, fixed in the shared `components/form.tsx` (0.5.1 F17): `keepTabInside` let Shift+Tab from a focused title (tabindex -1, before the first control) leave the dialog, and `useFocusReturn` read the focus after the title had taken it, so ending the tour left focus on the page body. Every test importing `form.tsx` passes (`vitest related`, 2263). Reduce Motion: R4. The hand check stays in PL1 step C13.
R4: OPEN — PL1 step C15. The clips' sizes and both themes are tested (`tests/bundle/clips-size.test.ts`). Muted, paused off screen and the still poster under Reduce Motion have no unit test on `components/Clip.tsx`.
R5: PASS — `tour.test.tsx` "starts at Bot's panel, in Bot's voice, with words for no AI too (D-144)", "Ask Bot about this shows only while Bot's AI is ready; without one, Bot says how to set it up".

### Usability answers, matched against the built tour (`Tour.tsx`, `tour.test.tsx`)

1. A short skippable walk, and "New in 0.5.x" only for what changed: matches (the offer and update tests above).
2. Each stop names the thing; clip where words aren't enough; Back, Next, Skip, progress, Esc always there: matches (the anchor and Esc tests).
3. Teaches on the real UI with the same clips as the Welcome: matches (`Tour.tsx` draws `Clip` from `lib/clips.ts`, the Welcome's own set).
4. A target not on screen shows its clip; Bot's stop says "Set up Bot" with no AI; never offered again once finished: matches (tests above).
5. No system API: matches (the tour calls no gate or OS prompt).
6. Watch-fors: a coach mark pointing at nothing (matches: the clip stands in); offered again after finishing (matches); a step that needs an AI (matches); focus lost when it ends, a clip with sound or moving under Reduce Motion: pending critique and PL1 C13/C15.
