# Evidence manifest — AJ-E3008

Task: `docs/tasks/E30-pools-redesign.md` AJ-E3008, Set up by hand: the canvas that teaches, member group nodes, the add palette, a line's own rule. Status: **DONE-EVIDENCED** for 0.4.1's release check: every requirement's proof run on today's tree; R2's "past 50 members" built (9ccba36db), R1, R3 and R4 given direct tests.

## What exists now

`ui/src/pages/pool/CanvasTeach.tsx` (the empty canvas), `RuleNodes.tsx` (a broad member's dashed group node), `useCanvas.ts` (`drawnMembers`, `GROUP_PAST`), `Palette.tsx` and `PaletteMore.tsx` (the add palette), `EdgeTalk.tsx`, `CanvasEdges.tsx` and `Inspector.tsx` (a selected line: its rule, its held list, Deliver).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 the empty canvas: three steps, the next the only primary, "Use a template" | `cd ui && npx vitest run src/pages/pool/canvas-teach.test.tsx src/pages/pool/mode.test.ts` ("shows three steps; the first is the only primary until where it works is chosen", "once where it works is chosen, adding is the one primary step", "an empty pool opens on the canvas"); walk `docs/qa/0.4-walks/P10.md` (pass 3 PASS after its fixes, 0.4.0 Dev, 2026-11-05) | three steps; one primary; the template a click away | 2 + 2 pass; P10 passed | PASS | 9ccba36db | 2026-11-09 | Opus (verified) |
| R2 a broad member as one dashed group node with its sessions and "new ones join"; past 50 members broad members group | `cd ui && npx vitest run src/pages/pool/rule-nodes.test.tsx` ("an App's node says what it brings, with a line to each session on the canvas"); `src/pages/pool/drawn-members.test.ts` ("AJ-E3008 R2: past 50 members, a broad member's sessions are its group node; the named and watched are drawn"); built in 9ccba36db (`drawnMembers`, `GROUP_PAST` = 50, by the pool's total) | at 51 members only the named and watched sessions drawn; at 50 every open one | pass | PASS | 9ccba36db | 2026-11-09 | Opus (verified) |
| R3 the add palette lists every candidate by kind with its reason; drag or Return adds | `cd ui && npx vitest run src/pages/pool/palette-more.test.tsx` (by kind with what each brings; a blocked one with why and its step; an add is a button, so Return adds as a click does, joining as a member); recorded L3 `docs/qa/evidence/l3/canvas.json` (`docs/qa/pairwise/canvas.csv`, 42 rows: each row's member kind joins from the palette, `ui/tests/visual/l3-canvas.spec.ts`); drag: `Palette.tsx` `onStartDrag` per item and `canvas.test.tsx` | every kind listed with its reason; adding works | 2 pass; L3 42 rows recorded | PASS | 9ccba36db | 2026-11-09 | Opus (verified) |
| R4 a selected line shows its rule, changes it for that pair only, and lists what's held with Deliver and Refuse | `cd ui && npx vitest run src/pages/pool/canvas-answers.test.tsx` ("a line with its own talk rule says so on its label, and a blocked one is drawn apart"); `src/pages/pool/canvas.test.tsx` ("below 100 % what waits on an edge is a badge that opens its answer": Deliver; "AJ-E3008 R4: a line paused at the hop limit offers Let it continue and Don't deliver"); `EdgeTalk.tsx` (the pair's rule, set for this pair only, back to the pair's clears it) and `Inspector.tsx` ("Waiting on you": each held message and why) | rule, per-pair change, Deliver and Don't deliver | 38 + 9 pass. A message only waiting to be read has Deliver (nothing to refuse); one held by the hop limit has Let it continue and Don't deliver; one held by "ask first" is answered on its sender, in Needs you (Deliver, Don't deliver) | PASS | 9ccba36db | 2026-11-09 | Opus (verified) |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths generalised where they reveal private layout (none)
- [x] No transcript excerpts
