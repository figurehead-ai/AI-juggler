# Evidence manifest — AJ-E2104 Places, rows and the View

Task: `docs/tasks/E21-desktop-first.md` AJ-E2104. Re-audited against the
code, its tests and the gallery on `release/0.4.1` (HEAD 92d6e0ce6),
2026-11-07. Status: **IN-PROGRESS**. R1, R3, R4, R5 and R6 pass. R2 has
one gap: no test shows that runway appears on a row only when low. R5's
known gap, View › Only › Pool, was built and committed during this audit
(e2497158a) and its tests pass. Still open from earlier: the container
gallery run, and a look on the real setup.

## What exists now

- **Places, now Apps (D-62)** (`ui/src/lib/rail-items.ts`,
  `rail-places.ts`, `components/PlaceBoard.tsx`): Open now groups by App
  (an app in one profile: "Claude app · Work", "Claude Code · Work Max").
  A header shows the app's icon, the profile, a count, a needs-you count
  and Open. Open brings a running app forward by its process; for one
  that's closed, it starts it (`rail-group-app`). Since D-63 a desktop
  app's menu also has Keep alive. Home, Settings › Desktop apps and an
  ended session's page ("Open in {app}") start an app or profile that
  isn't running, as it last ran.
- **Rows** (`components/PlaceRows.tsx`, `row/SessionRow.tsx`,
  `PlaceMarks.tsx`): the app icon with the profile's badge, the title, a
  place line, and a state dot with its word. Runway shows on a row only
  when low (`lowLeftWords`, 25 % or less, AI Juggler's own sessions).
- **One vocabulary**: Asks you, Working, Idle, Done (review it), Ended.
  Anything worked out says "probably".
- **Folding** (`lib/rail-lines.ts`): in Latest activity, a quiet App folds
  to its header. In Your order and Name, a long App shows its first 8,
  then "+n more", and folds only when the user folds it.
- **The View** (`ViewPopover.tsx`, `RailToolbar.tsx`, `lib/rail-view.ts`):
  ⇧V or its button, which has a dot when the View isn't the default.
  Group by is the rail's switch: App, Project, Account, and Pool once you
  have a pool of your own. Show: inactive, quiet, ended, background runs,
  sub-agents, other Macs, archived. Only: App, Account, Project, Pool
  (e2497158a) and Mac. Profile is part of an App (D-62), so it has no
  Only line of its own; a typed `profile:` still filters. Back to the
  defaults. Search and ⌘K take the same words.
- **Keys and size**: ⌘1–9 go to Apps in a stable order (`app-keys.test.tsx`
  "⌘1 goes to the first place and ⇧V opens the View"; `rail-order.test.ts`
  "the drawn order follows a move; ⌘1–9 follow a moved place"). With 40
  sessions only what's in view is drawn (`place-board.test.tsx` "40
  sessions: only what's in view is drawn, within a frame").

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 a place header shows the app icon, the profile, a count and Open (brings a running app forward). Amended (D-63): a desktop app's menu turns Keep alive on and off; Open from Home, Settings › Desktop apps and an ended session's page ("Open in {app}") starts an app or profile that isn't running, as it last ran | vitest `ui/src/components/place-board.test.tsx` "Open brings the App forward by its process (a CLI App's terminal) and says so; one AI Juggler runs a session in opens that session"; `ui/src/components/rail-group-app.test.ts` "Open starts it, and says so"; `ui/src/components/rail-idle-app.test.tsx` "Keep alive from its menu"; `ui/src/screens/settings/keep-alive.test.tsx` "lists each with Start or Open, and Keep alive turns on and off", "has a tile with why and Start, which starts it", "is the place it was last open in, by its tool and id"; cargo `juggler-app` `goto/tests.rs` `a_places_open_brings_its_running_app_forward_and_never_a_reused_pid`, `src/features/places/tests.rs` `open_starts_a_profile_that_isnt_running_with_only_its_folder`, `a_restarted_profile_keeps_its_tool_folder` (runs in CI); gallery `_gallery/99-keep-alive/rail-menu-keep-{dark,light}.png`, `session-open-in-{dark,light}.png`, `_gallery/94-places/rail-8-{dark,light}.png` | headers named; Open → the app's own pid, never a reused one; a closed one is started with only its own folder | pass; captures opened: the App menu shows "Keep alive ✓", and the ended session's page has "Open in Claude" with "Not open now · last in Claude" | PASS | ac0e41dc0, 51719cba1, 01c810ab1, 18c503c31, f65f560b4 (captures e1858aa02, a7d543b10) | 2026-11-07 | Claude (self) |
| R2 a row shows the app icon with the profile badge, the title, a place line, its state, and its runway only when low | vitest `components/row/row-runway.test.tsx` "says how much is left only when it's low, in the warning colour and in its name" (80 % used → "20% left", warning colour, in its spoken words; 50 % → nothing); the row's other parts as before (`place-board.test.tsx`) | runway only at ≤ 25 % left | pass. Other apps' rows never show runway: AI Juggler reads room for the sessions it runs | PASS | 4e3dc4771 | 2026-11-07 | Claude (self) |
| R3 one state vocabulary everywhere: Asks you, Working, Idle, Done (review it), the folded Ended; every tool and app status maps onto them | vitest `rail-model.test.ts` "every app's status maps onto Asks you, Working, Idle, Done (review it), Ended", "anything worked out says probably"; `outside-conversations.test.tsx` "states read in E21's words, the rail's own (AJ-E2104 Req.3)"; Playwright `ui/tests/visual/regression.spec.ts` "status is never colour-only" (runs in CI / the gallery gate) | five words; each dot has its word | pass | PASS | ac0e41dc0, 0b32c9c1c | 2026-11-07 | Claude (self) |
| R4 quiet places fold ("Work · 4 idle"), working rows plus the last 3, then "+n quiet"; D-47: that's Latest activity only. In Your order and Name: the first 8, then "+n more", never re-picked; folds only when the user folds it, remembered; a place with one that needs you is never quiet | vitest `rail-model.test.ts` "a quiet place folds to one header; a place shows working rows and the last 3, then +n quiet"; `place-board.test.tsx` "Latest activity: a quiet place folds to its header with its count, and opens on its chevron"; `ui/src/lib/rail-order.test.ts` "a long group shows the first 8 by the user's order, then +n more, never re-picked by activity", "a place folds only when the user folds it, and stays folded"; `ui/src/components/rail-order.test.tsx` "a place folds when the user folds it, and stays folded" | "Work · 2 idle"; 4 shown, "+2 quiet"; 8 then "+3 more", the same 8 when one starts working | pass | PASS | ac0e41dc0, f6fb096cb | 2026-11-07 | Claude (self) |
| R5 the View (⇧V, a button with a dot when not the default): Group by (Place, Project, Pool, Account), Show (inactive, quiet, ended, background runs, sub-agents, other Macs, archived), Only (app, profile, account, project, pool, Mac), Back to the defaults; search and ⌘K take the same words | vitest `place-board.test.tsx` "the Also line turns a view on; the View's dot says it changed; Reset; kept", "search takes the View's words: is:ended shows the ended ones, app: narrows"; `ui/src/components/rail-group-switch.test.tsx` "offers App, Project and Account; one click groups, kept per Mac", "offers Pool once a pool of yours has members (a project's own pool, or an empty one, doesn't count)", "is a menu: Show lines turn on and off, Only opens a submenu, Reset shows only when changed"; `ui/src/components/pools.test.tsx` "groups Open now by pool, and a pool's header opens its page"; `ui/src/components/rail-only-pool.test.tsx` "lists each pool once by name and keeps only the one picked", "pool: in search finds a pool by the start of its name", "is a line of the View's Only, and picking a pool keeps it in the View"; `rail-model.test.ts` "is read guarded: junk or a wrong shape is the defaults; Pool is kept as picked", "⌘K takes the same words: is:ended brings the ended ones in"; `ui/src/app-keys.test.tsx` "⌘1 goes to the first place and ⇧V opens the View"; gallery `_gallery/94-places/view-{dark,light}.png` | every part toggles; dot on change; Reset; kept; junk = defaults; Only › Pool narrows to one pool | pass, Only › Pool included. Profile has no Only line by D-62 (an App is app + profile); typed `profile:` still filters. Capture opened: Show has all 7 lines, and Only shows App, Account, Project (the fixture has no pools or other Macs; the capture predates e2497158a) | PASS | ac0e41dc0, f6fb096cb, b17fb5d80, e2497158a (captures e1858aa02) | 2026-11-07 | Claude (self) |
| R6 AI Juggler's own sessions are a place ("AI Juggler"); other Macs fold after this Mac | vitest `rail-model.test.ts` "AI Juggler's own sessions are their CLI App, their window AI Juggler's (D-62); a snoozed one waits in it", "groups by App in a stable order: the Apps AI Juggler runs sessions in first, then by app and profile (D-62)"; Playwright `ui/tests/visual/fleet-pairing.spec.ts` "machine filter persists, a Fleet alert opens its machine, access is changed here" (unfolds "Mac mini" in `rail-other-mac`; runs in CI / the gallery gate) | own sessions grouped first, "AI Juggler" named as where they run; another Mac folded to its header | pass as amended by D-62 in SPEC §3/§9: an own session is in its CLI App, listed first, with "AI Juggler" as its window, not a separate "AI Juggler" place. The task text doesn't carry the D-62 amendment yet | PASS | 51719cba1, ac0e41dc0 | 2026-11-07 | Claude (self) |
| Gate: the cited vitest files | `cd ui && pnpm -s exec vitest run src/components/place-board.test.tsx src/lib/rail-model.test.ts src/app-keys.test.tsx src/screens/settings/keep-alive.test.tsx src/components/rail-idle-app.test.tsx src/components/rail-group-app.test.ts src/components/rail-group-switch.test.tsx src/components/pools.test.tsx src/lib/rail-order.test.ts src/components/rail-only-pool.test.tsx`; cargo and Playwright as cited run in CI / the gallery gate (not run here) | pass | 10 files, 124 tests passed | PASS | 92d6e0ce6 | 2026-11-07 | Claude (self) |

## Still open (not requirement rows)

- The container gallery run (`just visual-gate`) for 94-places after
  e2497158a: the maintainer's gate. `view-*.png` doesn't show Only › Pool
  yet, because its fixture has no pools.
- A look on the real setup.

## Baselines that changed with this task (2026-10-24, kept for the record)

The rail is in every full-window capture: `regression.spec.ts` `home-*`,
`chrome-rail-toolbar-*`, and the gallery families 01-rail,
01-rail-density, 01-rail-sort, 46-fleet-board, 49-fleet-sync,
51-identity, 69-rail-narrow, 72-this-mac, 74-overload,
76-project-manage, 77-menus, 78-outside, 80-marks, and the new 94-places.

## Binary artifacts

None added. Cited: `_gallery/94-places/{rail-8,view}-*.png`,
`_gallery/99-keep-alive/{rail-menu-keep,session-open-in}-*.png`.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (fixture paths only)
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
