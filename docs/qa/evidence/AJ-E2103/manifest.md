# Evidence manifest — AJ-E2103 Open first: the rail's order

Task: `docs/tasks/E21-desktop-first.md` AJ-E2103. Re-audited against the
code, its tests and the captures on `release/0.4.1` (HEAD 92d6e0ce6),
2026-11-07. Status: **IN-PROGRESS**. R1–R4 pass. R5 (D-47, Your order)
is built and mostly tested, but two of its parts have no test: the
pointer drag, and the order surviving a backup.

## What exists now

In the rail's default view (`ui/src/components/RailSections.tsx`,
`PlaceBoard.tsx`):

1. **Needs you**: AI Juggler's sessions that need you, then the ones
   other apps say ask you or are done. Each shows once.
2. **Open now**: grouped by App (D-62). Each App lists its open sessions.
   By App, an inactive session sits under its app as "Last n inactive"
   (D-64); it isn't counted on the Also line.
3. **One line**: "Also: n ended today · n background · History ›". Each
   count turns its view on below the line, and History › opens History.

Other apps' ended sessions are not added under every project. A session
open in two places is one row that says "open in 2 places".

D-47 (Your order, the default): a row keeps its position; a new session
joins the end of its group; an ended one frees its slot; pinned rows lead
their group. You arrange rows and Apps by dragging their handle
(`rail-arrange.ts` `useArrange`) or with ⌥⌘↑ / ⌥⌘↓, and a polite live
region says each move. Open now's header and Settings › General have an
order menu: Your order, Latest activity, Name, Reset my order (with
Undo). The record (`rail_order` in AI Juggler's settings,
`lib/rail-order-store.ts`) keeps the order, pins, the Apps' order, folds
and the mode.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 sections: Needs you, Open now, then "Also: n ended today · n inactive · n background · History ›", each part turning its view on | vitest `ui/src/components/rail-sections.test.tsx` "reads Needs you, then Open now, then the Also line", "each part of the Also line turns its view on, and History opens History"; `ui/src/components/place-board.test.tsx` "reads Needs you, then Open now by place in a stable order, then Also"; gallery `_gallery/94-places/rail-8-{dark,light}.png` | that order; each count toggles its view; History opens History | pass; capture opened: Needs you 3, Open now 8 by App, "Last 2 inactive" under its App. By App the inactive count sits under its app (D-64), so in the place-board test the line reads "1 ended today · 1 background · History ›" | PASS | ac0e41dc0 (captures e1858aa02) | 2026-11-07 | Claude (self) |
| R2 other apps' ended sessions no longer added under every project | vitest `ui/src/components/outside-conversations.test.tsx` "AJ-E2103: shows the open ones only; the Ended filter the newest few, and counts the rest"; `rail-sections.test.tsx` "AI Juggler's own ended today wait behind the Also line, older ones in History; a pinned one stays" | default: open ones only; Ended filter: newest 3 + a count | pass | PASS | ac0e41dc0 | 2026-11-07 | Claude (self) |
| R3 a session open in two places is one row that says so | vitest `rail-sections.test.tsx` "a session open in two places is one row that says so"; `outside-conversations.test.tsx` "a click or ↩ opens its page, never its app; ↗ and ⌘↩ go to it, never the page (the maintainer's review of 0.1.4)" ("open in 2 places", "open in Claude · Work and Terminal") | one row, "open in 2 places", both places named | pass | PASS | ac0e41dc0 | 2026-11-07 | Claude (self) |
| R4 (D-47) one that needs you stays in its place in Open now, its state in amber, and Needs you lists it too; each list and the badge count it once | vitest `ui/src/lib/rail-order.test.ts` "one asking is in Needs you and in its place, amber, counted once in each"; `ui/src/components/rail-order.test.tsx` "one that needs you stays in its place, its state in amber, and is in Needs you too"; Playwright `ui/tests/visual/regression.spec.ts` "rail order — ${cell}" (baselines `ui/tests/visual/__baselines__/linux/rail-order-{dark,light}-comfortable-1280x800.png`; runs in CI / the gallery gate) | in Needs you once and in its place once, amber; counts once | pass; baseline opened: the asking rows are in Needs you (4) and under their App, "Asks you" in amber | PASS | f6fb096cb (baselines 989304b23) | 2026-11-07 | Claude (self) |
| R5 (D-47) Your order by default: a row keeps its position, new ones at the end, pins, ⌥⌘↑/↓ and drag by its grip, Reset with Undo, kept across restarts and backups | vitest `components/rail-order.test.tsx` (Your order, pins, keys, menu, Reset/Undo, restart); `components/rail-drag.test.tsx` "lands the row where it's let go, says where, and keeps it", "Escape and a cancelled pointer move nothing", "counts the rows whose middle is above the pointer…"; cargo `juggler-app/tests/backup_restore.rs` `the_rails_order_survives_a_restore_and_a_machine_only_setting_does_not` | each as stated | vitest 3/3 (drag); cargo 1/1 (2026-11-07) | PASS | bc38602c6 | 2026-11-07 | Claude (self) |
| Gate: the cited vitest files | `cd ui && pnpm -s exec vitest run src/components/rail-sections.test.tsx src/lib/rail-order.test.ts src/components/rail-order.test.tsx src/components/outside-conversations.test.tsx src/lib/key-hints.test.tsx`; Playwright as cited runs in CI / the gallery gate (not run here) | pass | 5 files, 54 tests passed | PASS | 92d6e0ce6 | 2026-11-07 | Claude (self) |

## R5's gaps: what a test should check

- **Drag:** a vitest beside `rail-order.test.tsx`. Render the place
  board in Your order and `pointerDown` on a row's `order-grip`. Fire
  `pointermove` on `window` past the next row's box (stub
  `getBoundingClientRect`), then `pointerup`. Check that the row moved one
  place, the polite region says "Moved to position n of m", and
  `railOrder().rows` holds the new order. Also check that Escape or
  `pointercancel` moves nothing. A pure test of `landing(y, boxes, self)`
  covers the maths.
- **Backup:** a cargo test in `crates/juggler-app/tests/backup_restore.rs`.
  Set `app_settings` `rail_order` to a JSON value, export, restore into a
  fresh state, and check the value survives. Alongside it, check that a
  `MACHINE_ONLY_SETTINGS` key does not survive.

## Earlier evidence still true

- The 2026-10-27 hand check in the browser preview (1280×800 and
  1024×680): states flipped six times with the order unchanged, and a drag
  and ⌥⌘↓/↑ moved one row and one place, saying "Moved to position n of
  m". That was a hand check only; Gap 1 needs a repeatable test.
- `ui/src/lib/key-hints.test.tsx` checks the grip's title names the drag
  and its keys ("Drag to move Fix billing (or ⌥⌘↑ and ⌥⌘↓)").

## Binary artifacts

None added. Cited: `_gallery/94-places/rail-8-*.png` and
`ui/tests/visual/__baselines__/linux/rail-order-*-comfortable-1280x800.png`
(6 baselines: rail, hover, menu × both themes).

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none)
