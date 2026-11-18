# Evidence manifest — AJ-E3603

Task: AJ-E3603 (the rail and the header, calm, E36, D-117). Status:
**IN-PROGRESS**: the 0.5 review's fix 8 (one quiet mark per need in the
rail) is built; the audit refresh, the persona walk, the densities and
lenses captures and the review page are still to do.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R2 (fix 8) one quiet mark per need: no tinted row in the rail (Needs you's session and notice rows), L3 told from L2 by its ✕ and red word | vitest `ui/src/components/rail-calm-lenses.test.tsx` "the Project lens: …" (the failed row has no `bg-danger-soft`, its word is `text-danger`) | no tinted rail row; the word and glyph carry the level | pass | PASS | the 2026-11-18 rail commit | 2026-11-18 | Claude Opus (self) |
| R2 (fix 8) an Open now row's low room is a fact, muted at every level (its Needs you row is the loud copy) | vitest `ui/src/components/row/row-runway.test.tsx` (asking, stuck, broken and limit rows: "0% left" `text-muted`, `data-level` 0) | never amber or red on the row | pass | PASS | the 2026-11-18 rail commit | 2026-11-18 | Claude Opus (self) |
| R2 the rail's App header room glyph: no amber or red disc; low muted, Out its danger glyph | `ui/src/components/RoomChip.tsx` `iconOnly`; the mock-state capture | no fill | checked by eye, both themes | PASS | the 2026-11-18 rail commit | 2026-11-18 | Claude Opus (self) |
| R3 the attention-budget gate on the rail and the header | `cd ui && ./node_modules/.bin/playwright test tests/visual/attention-budget.spec.ts` | all green | green (in the 122-test run below) | PASS | the 2026-11-18 rail commit | 2026-11-18 | Claude Opus (self) |
| R4 contrast and axe, both themes (Home and the rail, the a11y gate's subset) | `./node_modules/.bin/playwright test tests/visual/attention-budget.spec.ts tests/visual/a11y-glyph-names.spec.ts tests/visual/contrast.spec.ts tests/visual/a11y.spec.ts tests/visual/idle.spec.ts` (122 passed); `home-feed-gallery`, `home-gallery`, `rail-narrow`, `inactive-apps-gallery` (115 passed, axe inside) | 0 critical or serious; idle green | pass | PASS | the 2026-11-18 rail commit | 2026-11-18 | Claude Opus (self) |
| R4 every density (Comfortable, Compact, Minimal) | the rail gallery per density | axe clean | not run | PENDING | — | — | — |
| R1, R5, R7 the audit refresh, the Sonnet persona walk, before and after captures on the review page | per the task | — | not run | PENDING | — | — | — |
| R6 the design-quality critique | below | each deviation named with its reason | written | PASS | the 2026-11-18 rail commit | 2026-11-18 | Claude Opus (self) |

## Design critique against the approved mocks (2026-11-18)

Captures (not committed, no baselines): the home-feed gallery's
"the mock-up's data" state and home-gallery's needs-you state, 1280 × 800,
both themes, against `mock-home-feed-{dark,light}.png` (D-118) and
`mock-r2-{dark,light}.png` (D-117). Checked by eye.

Now as the mock: the rail's need rows are a dot, a title, the level's
word and where, with no tinted block; the Open now copy of a need is its
dot and word in ink; Your apps rows have Open on every running App and a
Not running group with Start; the header has no ‹ › until there is
history, New session is accent words, the room count is a ghost; the
counts line follows the mock's order; runway rows are a disc, name,
figure, a thin meter and one line of facts.

Remaining deviations, each with its reason:

1. **"Plenty of room" shows as the computer glyph and dot at 1280 with
   the rail open.** The bar also holds the Floor button (D-68, newer than
   the mock) and the room count; the words appear from 1120 px of bar.
   Cut words read "P…" (the 0.4.1 gate). The glyph stays: AJ-E3514 ("whose
   room": the Mac's, not an Account's).
2. **Home's tab wears the selected look** (an edge and a raised fill), not
   the mock's bold word: 21633a541's 3:1 selection rule, newer than the
   mock.
3. **The title says "1 session and 1 more thing need you"**, not "2 need
   you": its words agree with the rail and the bell (AJ-E3602, the 0.2
   walk's F4 test). "N finished today" shows only when the backend has
   counted finished sessions today; the mock-state fixture holds no
   notices, so it shows none there.
4. **A "1 message from an agent" line and the legend's "?"** sit by the
   title (HomeExtras, AJ-E403's legend). Not in the mock; kept.
5. **The pinned cards differ** (a message first, its Reply; the ↗):
   feed items 1 and 2 wait for AJ-E3708, E3703 and E3706.
6. **Your apps: a long name puts the needs mark under it**, right-aligned
   (three lines): nothing is cut at 320 px (85488fac0). Short names keep
   the mock's single line.
7. **Runway facts say "5-hour window resets at …"**, not "resets …", and
   "n sessions on it" counts AI Juggler's own live sessions only (other
   Apps' sessions are not linked to an Account's room yet).
8. **The rail's App group header** shows the App icon, the terminal
   glyph, the profile dot and name (the mock: dot, profile, App) and a
   muted gauge glyph for low room: D-100's icon-only room mark, now
   without its amber disc.
9. **A rail notice row** (an agent's message) keeps its kind's glyph and
   its action word ("Open") in amber, where the mock shows the pool's
   task title over the asking agent: AJ-E3702's quoted asks.
10. **Fixture gaps (not this task's):** the "Next steps" banner and the
    sixth App row come from the fixture's Team Plus account.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content
