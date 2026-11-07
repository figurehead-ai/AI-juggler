# Evidence manifest — AJ-E2807 The Pools screen

Task: `docs/tasks/E28-finding-your-way.md` AJ-E2807, judged against the
requirements as amended 2026-11-06 (the card says "1 needs you"; New pool opens
the new-pool wizard, AJ-E3007, which ends on the new pool's canvas). Built in
0.3 (bebd861f2, the screen and its cards; 13f9ef058, the wizard) and finished on
`release/0.4.1` (04b037424: the card's menu, the order, pages of 24, the empty
text); re-checked against the code, its tests and the gallery, 2026-11-06.

The screen: `ui/src/screens/pools/PoolsScreen.tsx`, `PoolCard.tsx`,
`PoolMini.tsx`; its menu `ui/src/lib/action-specs-pool.ts`.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 a card: a small map of its members (lead marked, one with an ask marked), its name, "3 members · 1 needs you · 2 tasks moving" or "quiet"; a project's own pool says "(project pool)" | `pnpm -C ui exec vitest run src/pages/pool/canvas.test.tsx`: "says a card's line in words"; "the pool's header says what its Pools card says (the 0.3 review: \"1 asks you\" beside \"4 need you\")"; `src/screens/pools/pools-screen.test.tsx`: "a project's own pool can't be renamed or deleted, and its menu says why" (`pool-card-scope` is "(project pool)"); `PoolMini.tsx:40` (lead accent, alert amber); `_gallery/97-pool-canvas/pools-screen-1280x800-dark.png` | the line's words; the map's marks | pass: "3 members · 1 needs you · 2 tasks moving", "1 member · quiet"; the capture shows Checkout team's map with the lead in the accent and the asking members amber, "6 members · 4 need you · 2 tasks moving", and docs-site / storefront "project pool · 1 member · quiet" | PASS | 04b037424 | 2026-11-06 | Claude (self) |
| R2 sorted: pools that ask you first, then by recent activity; paged when there are more than 24 | `pnpm -C ui exec vitest run src/screens/pools/pools-screen.test.tsx`: "puts the pools that ask you first, then the latest active (R2)", "shows 24 cards at a time, the rest a click away, never all at once (R2)" | order as stated; 24 a page | both pass: 32 pools show 24, "Show 8 more of 8" shows the rest, then no button | PASS | 2fccac014 | 2026-11-07 | Claude (self) |
| R3 New pool opens the new-pool wizard (AJ-E3007), which ends on the new pool's canvas | `PoolsScreen.tsx:156` (`pools-new` opens `NewPool`); `pnpm -C ui exec vitest run src/screens/pools/new-pool.test.tsx`: "picks, sets talk, reviews and makes the pool with the user's own calls" (ends in `openPool(<new id>)`), "Escape leaves nothing made"; `pool-canvas-gallery.spec.ts` "the new pool wizard, each step" and "a new pool by hand"; `_gallery/97-pool-canvas/new-pool-1280x800-dark.png` | the wizard opens; its end opens the canvas | vitest pass: `create:Checkout team`, then the new pool's page opens; the by-hand capture shows the empty canvas with its name ready to type; Playwright runs in CI / the gallery gate | PASS | 13f9ef058 | 2026-11-06 | Claude (self) |
| R4 a card opens the pool's canvas; its ⋯ menu offers Open, Rename…, Delete… (saying what happens to its members), icons on every line | `pnpm -C ui exec vitest run src/screens/pools/pools-screen.test.tsx`: "gives a card its menu: open, rename, and delete saying what happens to its members (R4)"; `src/screens/nav-screens.test.tsx`: "Pools shows every pool with someone in it as a card, each opening its page" | three lines with icons; Delete says where members go | pass: `["Open its canvas", "Rename…", "Delete…"]`, each with an icon, from right-click, ⋯ and ⇧F10; "Its 2 sessions go back to their projects' own pools"; a card click calls `pool:` | PASS | 04b037424 | 2026-11-06 | Claude (self) |
| R5 with no project: "Pools appear with your first project." | `pnpm -C ui exec vitest run src/screens/pools/pools-screen.test.tsx`: "with no project yet, says pools come with it (R5)" | the exact words | pass | PASS | 04b037424 | 2026-11-06 | Claude (self) |
| Gate: the touched screens' tests | `cd ui && pnpm -s exec vitest run src/screens/pools/pools-screen.test.tsx src/screens/pools/new-pool.test.tsx src/pages/pool/canvas.test.tsx src/screens/nav-screens.test.tsx`; `pool-canvas-gallery.spec.ts` runs in CI / the gallery gate | 0 failures | 4 files, 65 tests passed (2026-11-06 run); Playwright not run here | PASS | 04b037424 | 2026-11-06 | Claude (self) |

## Binary artifacts

None added; cited: `_gallery/97-pool-canvas/pools-screen-*.png`,
`_gallery/97-pool-canvas/new-pool-1280x800-*.png`. The wizard's step captures
(`new-pool-{template,who,roles,review}-*`) are written by the spec but not
committed.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none)
