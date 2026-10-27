# Evidence manifest — AJ-E2103

Task: `docs/tasks/E21-desktop-first.md` AJ-E2103, open first: the rail's
order. Branch `e21-step1`. Status: **IN-PROGRESS**: the order, the folds
and the one-row rule are built and tested below; the task's visual check
(the rail in both themes, gallery captures) is not made yet: the
Playwright galleries rewrite the evidence store and are the maintainer's
gate run. Seen by hand in the browser preview (dark): Open now, "Also: 2
ended today · History ›", the Ended today view, and an open row's click
saying "Brought Claude · Work forward; pick the session in its sidebar".

## What exists now

In the rail's default view (grouped by project, Needs-me sort, no filter
or search), `ui/src/components/RailSections.tsx`:

1. **Needs you**: AI Juggler's sessions that need you (a snoozed one
   waits in its project, as before), then the ones other apps say ask you
   or are done (`state` `asks_you` / `done_review`). Never grouped; each
   shows once.
2. **Open now**: the projects with something open, their open sessions
   (AI Juggler's live ones, pinned ones, and other apps' open ones). A
   project's header still sums up all its sessions; a project with nothing
   open moves to the Projects list.
3. **One line**: "Also: n ended today · n inactive · n background ·
   History ›". Each count turns its view on below the line (AI Juggler's
   and other apps' sessions that ended today; ones in a running app's
   sidebar with no process; headless runs); History › opens History,
   where older ended sessions are.

Other apps' ended sessions are no longer added under every project
(`railSlice`: the default shows open ones only; the Ended filter keeps the
newest 3 per project). A session open in two places is one row that says
"open in 2 places" (its places named for screen readers and on hover).
With a filter, a search, another sort or grouping, the list is as before.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 sections and the Also line | `cd ui && npx vitest run src/components/rail-sections.test.tsx` | Needs you, then Open now, then the line; each part toggles its view; History opens History | 6 passed | PASS | d10d322d | 2026-10-24 | agent |
| Req.2 no ended ones under every project | `src/components/outside-conversations.test.tsx` "shows the open ones only" | default: open only; Ended filter: newest 3 + count | pass | PASS | 525bf197 | 2026-10-24 | agent |
| Req.3 one row for two places | `rail-sections.test.tsx` "a session open in two places is one row that says so" | one row, "open in 2 places" | pass | PASS | 525bf197 | 2026-10-24 | agent |
| Folds and the old order's tests | `npx vitest run src/components/sorting.test.tsx src/components/rail-folded-needs.test.tsx src/components/project-manage.test.tsx` | the needs order holds; folded marks hold | pass | PASS | d10d322d | 2026-10-24 | agent |
| Whole UI suite | `cd ui && npx vitest run`; `npm run typecheck` | all pass | 115 files, 861 tests; typecheck clean | PASS | d10d322d | 2026-10-24 | agent |
| Visual: the rail in both themes | Playwright galleries | captures checked by eye | not run | UNPROVEN | — | 2026-10-24 | agent |
| Req.4 (D-47) one that needs you stays in its place; each list counts it once | `cd ui && pnpm vitest run src/lib/rail-order.test.ts src/components/rail-order.test.tsx src/lib/rail-model.test.ts` | in Needs you once and in its place once, amber; badge and section counts once | 49 passed | PASS | 683ef529 | 2026-10-27 | agent |
| Req.5 (D-47) Your order: state changes never move a row; new ones at the end; gone ones close up; pins lead; drag and ⌥⌘↑/↓; Reset; kept | same files, plus a hand check in the preview (1280×800 and 1024×680: states flipped six times, order unchanged; a drag and ⌥⌘↓/↑ moved one row and one place, said "Moved to position n of m") | order unchanged across flips; moves and the record round trip | pass | PASS | 683ef529 | 2026-10-27 | agent |
| Visual (D-47): Your order with a pinned row and an asking row in place, the hover handle, the order menu, both themes | `./scripts/visual_gate.sh --update --workers=1 tests/visual/regression.spec.ts -g 'rail order.+'` | captures checked by eye | 6 baselines `rail-order-*` | PASS | 2696b583 | 2026-10-27 | agent |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none)
