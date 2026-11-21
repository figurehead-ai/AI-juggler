# Evidence manifest — AJ-E3603

Task: AJ-E3603 (the rail and the header, calm, E36, D-117). Status:
**IN-PROGRESS**: the 0.5 review's fix 8 (one quiet mark per need in the
rail) and the 2026-11-19 batch (the audit, the calm pass on every page
and density, R5's L3 rows, the budget gate over densities and an
L3-heavy fixture, the fixture's titles, the focus ring) are built; the
Sonnet persona walk and the review page with before and after captures
are still to do.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R2 (fix 8) one quiet mark per need: no tinted row in the rail (Needs you's session and notice rows), L3 told from L2 by its ✕ and red word | vitest `ui/src/components/rail-calm-lenses.test.tsx` "the Project lens: …" (the failed row has no `bg-danger-soft`, its word is `text-danger`) | no tinted rail row; the word and glyph carry the level | pass | PASS | the 2026-11-18 rail commit | 2026-11-18 | Claude Opus (self) |
| R2 (fix 8) an Open now row's low room is a fact, muted at every level (its Needs you row is the loud copy) | vitest `ui/src/components/row/row-runway.test.tsx` (asking, stuck, broken and limit rows: "0% left" `text-muted`, `data-level` 0) | never amber or red on the row | pass | PASS | the 2026-11-18 rail commit | 2026-11-18 | Claude Opus (self) |
| R2 the rail's App header room glyph: no amber or red disc; low muted, Out its danger glyph | `ui/src/components/RoomChip.tsx` `iconOnly`; the mock-state capture | no fill | checked by eye, both themes | PASS | the 2026-11-18 rail commit | 2026-11-18 | Claude Opus (self) |
| R3 the attention-budget gate on the rail and the header | `cd ui && ./node_modules/.bin/playwright test tests/visual/attention-budget.spec.ts` | all green | green (in the 122-test run below) | PASS | the 2026-11-18 rail commit | 2026-11-18 | Claude Opus (self) |
| R4 contrast and axe, both themes (Home and the rail, the a11y gate's subset) | `./node_modules/.bin/playwright test tests/visual/attention-budget.spec.ts tests/visual/a11y-glyph-names.spec.ts tests/visual/contrast.spec.ts tests/visual/a11y.spec.ts tests/visual/idle.spec.ts` (122 passed); `home-feed-gallery`, `home-gallery`, `rail-narrow`, `inactive-apps-gallery` (115 passed, axe inside) | 0 critical or serious; idle green | pass | PASS | the 2026-11-18 rail commit | 2026-11-18 | Claude Opus (self) |
| R1 the audit refresh, the rail and the header on the current tree | below, "The audit (2026-11-19)" | every loud element listed against the budget | 39 elements: 14 over the budget, 5 layout or focus faults, 5 kept loud by design, 15 already calm | PASS | e66d9de86 (audited) | 2026-11-19 | Claude Opus (self) |
| R2, R5 one quiet mark per need on every page, Settings included, every density: the level's colour on its small mark (amber dot, ring, red ✕), the words in calm ink | vitest `rail-calm-lenses`, `rail-folded-levels`, `rail-notice-row`, `glyph-names`, `row-runway` (1523 component tests green); the gate below | no amber or red words in the rail but Needs you's count | pass | PASS | d07bb2fe0 | 2026-11-19 | Claude Opus (self) |
| R3, R5 the attention-budget gate: the rail and the header on all-statuses, typical, flood and the new L3-heavy fixture, × Comfortable, Compact, Minimal, × both themes, on Home and Settings | `tests/visual/attention-budget.spec.ts` (x) "the rail and the header are calm" | only Needs you's count amber or red; every need row carries `.level-mark`; no amber fill; no tinted row | 34 passed; a mutation (the L3 word back to `text-danger`) fails it | PASS | 7989b7b5c | 2026-11-19 | Claude Opus (self) |
| R4 contrast and axe, both themes, every density | the same gate's axe on the rail and the header (WCAG 2.0–2.2 A/AA); `contrast.spec.ts` (tokens) | 0 critical or serious | 0 in all 24 density × fixture × theme states, on Home and Settings | PASS | 7989b7b5c | 2026-11-19 | Claude Opus (self) |
| The a11y gate's spec list (`scripts/a11y_gate.sh`) plus nav, places, session-pages, project-manage, launch-advisor, rail-narrow, home and home-feed galleries, on this batch | `AJ_AXE_GALLERY=1 AJ_A11Y_EVIDENCE=1 playwright test …` against the batch's build; the failing specs again against e66d9de86's build | no failure this batch adds | 1262 passed, 31 failed; every one also fails on e66d9de86 (28 there; determinism differs by 2 bytes on both) except four this batch caused and then fixed (the keyboard flows: a key-driven Skip to page lost its ring, ea5e13512; two flows found sess-2 by its old name); "the header at 1024 with the list open" fails on e66d9de86 and passes now | PASS (pre-existing failures named for their owners) | ea5e13512 | 2026-11-19 | Claude Opus (self) |
| Captures: the capture fixture's sessions have real titles; "Session started …" only where a state tests it | `ui/tests/visual/fixtures.ts` `SESSION_TITLES`, `ui/src/preview/fixtures.ts`; the clock pin of 3e8f835d2 kept | no "Session started" in the rail captures | checked by eye in the side-by-side captures | PASS | d7b1f8c6d | 2026-11-19 | Claude Opus (self) |
| Focus ring after a scripted navigation: none on `<main>` or a page's name unless a key brought focus there (Skip to page, ↩ on a row: WCAG 2.4.7) | `ui/src/lib/input-modality.ts` (the root's `data-input`), `styles.css`; vitest `input-modality.test.ts`; `keyboard-flows.spec.ts` (Skip to page keeps its ring); the Watches capture (before: an accent line down the main pane's left edge; after: none) | no ring after a click or before any key | pass; checked by eye, both themes | PASS | ef9204410, ea5e13512 | 2026-11-19 | Claude Opus (self) |
| R5, R7 the Sonnet persona walk; before and after captures on the review page | per the task | — | not run | PENDING | — | — | — |
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

## The audit (2026-11-19, R1)

On e66d9de86, before the calm pass: every element of the rail and the
header that spends colour, weight or motion, against D-117's budget and
R5 (one quiet mark per need; the words in the calm text levels; the
level's colour on the small mark only). Read from the code and from
captures of Home, a session page, Settings and Watches in every density.

| ID | Element (file) | Before | Verdict | Now |
|---|---|---|---|---|
| R-a1 | Needs you L3 row's word (`PlaceRows` NeedRow) | "✕ Stopped with an error", a whole red semibold line | over | red ✕ mark, words ink-2 |
| R-a2 | Needs you L2 row's word | "Asks you", "Limit reached" amber semibold | over | amber 6 px dot, words ink-2 |
| R-a3 | Needs you guess ("Probably stuck") | amber ring and amber words: two loud marks | over | the ring alone, words ink-2 |
| R-a4 | Notice row, titled (`RailNoticeRow`) | its action word, or its whole title when that is the mark, amber or red | over | the mark before the action or title, words ink |
| R-a5 | Notice row, subject first | "builder-1 asks you" amber semibold | over | mark, words ink-2 |
| R-a6 | Folded icon's corner (`RailFolded`) | a 14 px filled amber "!" or red "✕" disc | over (amber fill) | the level's 6 px mark on a canvas backing |
| R-a7 | Folded icon's state dot | an 8 px dot in the level's colour beside the badge | over (a second mark) | none on a need's icon; calm dots 6 px |
| R-a8 | Folded tooltip's word | level colour, semibold | over | mark, words ink-2 |
| R-a9 | Open now row's heavy load (`SessionRow`, both densities) | amber words | over | muted (its loud copy is the Needs you row) |
| R-a10 | Open now row at its limit | "0% left" beside "Limit reached", the title cut to "Tidy the …" | fault | the figure dropped on a limit row |
| R-a11 | Project lens row's low room and heavy load (`WorkflowRail` RailRow) | amber words, both densities | over | muted |
| R-a12 | Project lens row's model drift ⇄ | an amber pill on warning-soft | over (amber fill) | muted in a hairline |
| R-a13 | Stale value chip (`indicator` StaleChip, also session cards) | an amber pill on warning-soft | over (amber fill) | muted in a hairline (L1) |
| R-a14 | "Search all history for …" (`PlaceBoardParts`) | accent words | over (R23) | ink-2, underline on hover |
| R-a15 | Legend's levels (`LegendLevels`) | the L3 and L2 words red and amber, L3 drawn as a dot | over | the marks as the rail draws them, words ink |
| R-a16 | App header's needs mark (`PlaceHeader`) | calm, but hidden at the rail's 264 px by a long App word ("Personal Claude Code CLI") | fault | always has its room; the names give way |
| R-a17 | Compact rows' time | "waiting 3 h" cut titles to "Refa…" | fault | the time shows only beside a readable title (container query); in the tooltip and name |
| H-a1 | Focus after a scripted navigation | an accent line down the main pane's left edge | fault | no ring on `<main>` or a page's name |
| H-a2 | The bar at 1024 with the list open | wrapped its whole cluster under the tabs (Bot and the Floor added) | fault | one row (spacing closes up under 820 px of bar) |
| K1 | Needs you's head count ("✕ 5", "⚠ 2") | level colour | kept (R6, the area's one count) | — |
| K2 | The folded rail's Needs you count | level colour | kept (R25) | — |
| K3 | The bell's count | neutral to L2, red at L3 | kept (H4) | — |
| K4 | Brand mark | accent | kept (R1) | — |
| K5 | A pinned row's pin tool, hover only | accent-hi while pinned | kept: a pressed tool under the pointer; the row's pin mark at rest is muted | — |
| C1–C15 | Section labels (`--color-section-label`), Open now without icon or cyan; no App-colour row edge; the 6 px App dot; Working a static calm dot; ↗ on hover, focus or the selected Needs you row; titles medium ink-2, selected and Needs you rows semibold ink; rail-foot New session neutral; lens tabs in the selected look; This Mac's line; clean-up a ghost; header New session accent words, no fill; Bot, Floor, avatar neutral; the current tab semibold, no box | already calm | compliant | — |

Count: 39 elements (C1–C15 count as fifteen): 14 over the budget and 5
layout or focus faults (R-a10, R-a16, R-a17, H-a1, H-a2), all fixed; 5
kept loud by design; 15 already calm.

## Side by side against the round-2 mock (2026-11-19)

Captures: Home, a session page, Settings and Watches, both themes, every
density, 1280 and 1024, plus the Project and Account lenses at 1280
(not committed, no baselines). Against `mock-r2-{dark,light}.png`. One
list; each fixed, or left with its reason:

1. **Fixed: an App header said nothing of its needs at 264 px** (R-a16);
   the app word now gives way first ("Work Max Claude Co… ✕ 2"), the
   profile whole up to 55 % of the line.
2. **Fixed: the bar wrapped at 1024 with the list open** (H-a2).
3. **Fixed: compact rows cut titles to four letters** for the time (R-a17).
4. **Fixed: "0% left" beside "Limit reached"** (R-a10).
5. **Fixed: the accent line after a navigation** (H-a1).
6. **Left: a need's words are ink-2 with the level's mark, not the mock's
   amber words.** R5 (2026-11-19) is newer than the mock: one quiet mark
   per need, never a loud line.
7. **Left: Open now's grip, pin and ↗ show on hover and focus, not on the
   selected row at rest.** The mock draws none on its selected row, and
   the maintainer's report of 2026-11-03 asked that the selected row not
   keep its tools; Needs you's ↗ does show on the selected row (R10).
8. **Left: the App header's kind mark (>_) beside its icon** where no two
   Apps share an icon (R14 shows it only then): D-100 (0.4.2 review) put
   the kind beside every App icon; deviation 8 above stands.
9. **Left: under the pointer, a sliver of the state word shows through the
   tools' 12 px fade** (0.4's re-render, "hidden, never cut through"); at
   rest the row is whole.
10. **Left, for the persona walk: Open now's count is the Apps in the App
    lens ("3") and the sessions in the Account lens ("6")**; the Project
    lens shows none. Not a budget matter; named here so the walk checks
    whether it reads.

Pre-existing failures found by the gate run (they fail on e66d9de86 too,
not this task's): `cursor` (the legend's controls), `fits-its-room` A8
(1024 and 1280), `fleet-pairing` (a timeout), `flows-cde` (e) ("58%"),
`keyboard-flows` page tab walk (no Pools tab), `places-gallery` (Home's
title words), `nav-gallery` (a project's Pool panel; Tasks at 1024 axe),
`project-manage-gallery` (two), `caret-mark`, `home-feed-gallery` (flood
axe at every width; paging 4 pages, not 3), `capture` determinism.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content
