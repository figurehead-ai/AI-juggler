# Evidence manifest — AJ-E2104

Task: `docs/tasks/E21-desktop-first.md` AJ-E2104, places, rows and the
View. Branch `e21-rail`. Status: **IN-PROGRESS**: every requirement is
built and tested below, and the new gallery (`places-gallery.spec.ts`,
94-places) passes when run on this Mac (not in the pinned container: the
container gate is the maintainer's run, and its baselines change, see
the list at the end). Open: real app icons (a desktop app shows its two
letters, a terminal and an editor their kind's glyph: the host registry
carries no icon yet), Pool (the lens is there, greyed, until pools
exist), and a look on the real setup.

## What exists now

- **Places** (`ui/src/lib/rail-items.ts`, `rail-places.ts`,
  `components/PlaceBoard.tsx`): Open now groups by place, the app and
  profile a session runs in (from AJ-E2101's `PlaceInfo`); AI Juggler's
  own sessions are the place "AI Juggler". A header shows the app's icon
  and name, the profile chip in its colour, a count, a needs-you count,
  and **Open**, which brings the running app forward by its process
  (`conversations_bring_forward`: only a process an open session runs in,
  never a launch, within 1.5 s). The order is stable (AI Juggler's
  first, then by app and profile), so ⌘1–9 always reach the same place.
- **Rows** (`components/PlaceRows.tsx`, `PlaceMarks.tsx`): Needs you rows
  have the app's icon with the profile's badge (a one- or two-letter
  monogram, white on its colour, 4.5:1 or better; six hues kept 20° or
  more from every status colour), the title in bold, "project · app ·
  profile", and why in amber. Open now rows sit under their place with a
  left edge in the profile's colour: a state dot, the title, "project ·
  viewed N min ago", the state word in its colour; runway only when low,
  "working your Mac hard" only when it is. Labels lead with the state.
  The same mark is the session page's header and Home's (AJ-E2108).
- **One vocabulary**: Asks you, Working, Idle, Done (review it), Ended;
  every tool's and app's status maps onto it in one place
  (`sessionItem`, `conversationItem`); worked-out ones say "probably".
- **Folding** (`lib/rail-lines.ts`): a quiet place folds to one header
  ("Work · 4 idle"); a place shows its working rows and the last 3, then
  "+n quiet"; other Macs fold after this Mac; ended, inactive and
  background runs wait behind the Also line; apps turned off in Settings
  › Desktop apps leave "n in Cursor hidden · Settings" there.
- **The View** (`components/ViewPopover.tsx`, `lib/rail-view.ts`): ⇧V or
  its button (a dot when not at the defaults). Group by: Place, Project,
  Pool (greyed until pools exist), Account (the subscription, AJ-E2109).
  Show: inactive, quiet, ended, background runs, sub-agents, other Macs,
  archived. Only: app, profile, account, project, Mac. Back to the
  defaults. Kept per window, read guarded. Search and ⌘K take the same
  words (`is:ended is:inactive is:background is:quiet is:archived
  is:asks app: profile: account: project: mac:`).
- **Keys**: ⌘1–9 go to places (Home moved to ⌘0, Accounts ⌘⇧A, Usage
  ⌘⇧U, Fleet ⌘⇧F; the ? sheet reads the registry); ↑↓ move between rows
  and headers; rows hold still while the pointer or focus is in the
  list; past 30 rows only what's in view is drawn.
- RailPools' auto-switch does not exist on this branch (it's on
  release/0.2): `grep -rn RailPools ui/src` finds nothing.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 place header: icon, profile, count, Open (running app only) | `cd ui && npx vitest run src/components/place-board.test.tsx` ("Open brings the running app forward…"); `cargo test -p juggler-app --lib features::conversations::goto` (`a_places_open_brings_its_running_app_forward_and_never_a_reused_pid`) | Open → the app's pid; a reused pid is never activated | pass | PASS | 13e58677, cdcad91b | 2026-10-24 | agent |
| Req.2 row anatomy, badge 4.5:1, colours apart from status | `npx vitest run src/lib/rail-model.test.ts` ("profile badges"); `place-board.test.tsx` (label leads with state) | white ≥ 4.5:1; every hue ≥ 20° from the status hues | pass | PASS | cdcad91b | 2026-10-24 | agent |
| Req.3 one state vocabulary | `rail-model.test.ts` ("every app's status maps onto…") | five words, AI Juggler's and others' | pass | PASS | cdcad91b | 2026-10-24 | agent |
| Req.4 quiet places fold; working + last 3, then +n quiet | `rail-model.test.ts` ("a quiet place folds…"); `place-board.test.tsx` ("a quiet place folds to its header…") | "Work · 2 idle"; 4 shown, "+2 quiet" | pass | PASS | cdcad91b | 2026-10-24 | agent |
| Req.5 the View, its dot, Back to the defaults, kept, guarded; search and ⌘K words | `place-board.test.tsx` (the Also line and the View; search words); `rail-model.test.ts` (guarded read; ⌘K `is:ended`); `src/app-keys.test.tsx` (⇧V, ⌘1) | each part toggles; dot on change; reset; stored; junk = defaults | pass | PASS | cdcad91b | 2026-10-24 | agent |
| Req.5 Group by Account is the subscription | `rail-model.test.ts` ("by account: profiles on one subscription share a group") | two profiles of one account, one group | pass | PASS | 7180ed32 | 2026-10-24 | agent |
| Req.6 AI Juggler a place; other Macs fold after this Mac | `rail-model.test.ts` ("AI Juggler's own sessions are the place…"); `fleet-pairing.spec.ts` (unfold Mac mini) | own place first; a Mac folded to its header | vitest pass; Playwright not run in the container | PASS | cdcad91b | 2026-10-24 | agent |
| Keys: ⌘1–9 to places in a stable order | `app-keys.test.tsx` ("⌘1 goes to the first place…"); `place-board.test.tsx` (go-place index 3 → Claude · Work) | event per digit; same place each time | pass | PASS | cdcad91b | 2026-10-24 | agent |
| Perf: 40 sessions virtualised | `place-board.test.tsx` ("40 sessions: only what's in view is drawn") | fewer than 40 rows in the DOM | pass | PASS | cdcad91b | 2026-10-24 | agent |
| Settings › Desktop apps respected, hidden leaves a count | `place-board.test.tsx` ("an app turned off in Settings leaves its count") | "3 in Terminal hidden · Settings"; its place gone | pass | PASS | cdcad91b | 2026-10-24 | agent |
| Whole UI suite, types, lints | `cd ui && npx vitest run`; `npm run typecheck`; `python3 scripts/size_budget.py`; `just structure`; `python3 scripts/plain_words_lint.py`; `./scripts/terminology_lint.sh`; `just tool-literals`; `./scripts/leaked_fixtures.sh` | all pass | 122 files, 936 tests; clean; OK | PASS | 2f53c25b | 2026-10-24 | agent |
| By hand: browser preview, both themes | `vite --port 5291`, `sessionStorage["aij.preview.places"]="1"`; ⇧V; Open on Terminal; a Done row clicked | the canvas's rail; "Brought Terminal forward"; Needs you 3 → 2 | as expected, dark and light | PASS | cdcad91b | 2026-10-24 | agent |
| Visual: rail at 1, 8, 40, the View; both themes | `AIJ_PW_PORT=4391 npx playwright test tests/visual/places-gallery.spec.ts` (this Mac, not the container) | 14 pass, axe 0 critical/serious, captures checked by eye | 14 passed; captures looked at (kept out of the store: the container makes the baselines) | PASS | c75cc9d5 | 2026-10-24 | agent |
| Visual: the galleries that read the rail, on this Mac | `AIJ_PW_PORT=4391 npx playwright test` a11y, capture, menus, project-manage, places, replica-journey, fleet-pairing, this-mac, overload (this Mac, not the container; its captures discarded) | all pass | 882 passed, 2 failed (identity: a terminal took focus from the dialog), then fixed (2f53c25b) and passing | PASS | 2f53c25b | 2026-10-24 | agent |
| Visual: the container gate | `just visual-gate` | the new and changed baselines approved | not run (the maintainer's gate) | UNPROVEN | — | 2026-10-24 | agent |

## Baselines that change

The rail is in every full-window capture, so these change:
`regression.spec.ts` `home-*` (every theme, density and size),
`chrome-rail-filters-*` (removed; `chrome-rail-toolbar-*` new), and the
gallery families 01-rail, 01-rail-density, 01-rail-sort (quota-risk
removed, `view-1440x900` new), 46-fleet-board, 49-fleet-sync, 51-identity,
69-rail-narrow, 72-this-mac (now This Mac's page), 74-overload,
76-project-manage, 77-menus, 78-outside, 80-marks, the pages and README
shots, and the new 94-places.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
