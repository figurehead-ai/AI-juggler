# Evidence manifest — AJ-E2811 A subagent's detail: its prompt, result, work, usage and processes

Task: `docs/tasks/E28-finding-your-way.md` AJ-E2811, judged against its
amended text (2026-11-06: the crumb starts at the session; five files named;
the share shown with a note where tokens are part of the session's; no Stop
button). Built in 0.3 (AJ-E1520: ad9d5ad64, e6c96f2e3) with 0.4.1 additions
(6110ce284, f054c1028); re-checked against the code, its tests and the
committed gallery on `release/0.4.1`, 2026-11-06.

Vitest was run for this manifest (`subagents-tab.test.tsx` and
`subagents-keys.test.tsx`: 2 files, 17/17 passed). Cargo tests named here run
in CI; Playwright was not run here, the captures are the committed ones
(`detail-running-1280x800-dark.png` opened and looked at).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 the crumb <session> › Subagents › <parent> … › <name>, each step a link; Esc one step back | `cd ui && pnpm -s exec vitest run src/pages/session/subagents-tab.test.tsx src/pages/session/subagents-keys.test.tsx`: "a subagent's page: its task, result, work, usage, processes and how to stop it; Esc goes back" (crumb `^Fix the cart totals›Subagents›`), "Esc goes back one step: from a parent opened on a subagent's page, to that page, then the list (AJ-E2811 R1)" | crumb from the session; Esc: parent → child → list | pass | PASS | 6110ce284 | 2026-11-06 | Claude (self) |
| R2 the head: name, state and time ("● Running · 1m"), type · model, "in the background", "started by <parent>" | vitest "a subagent's page: its task, result, work, usage, processes and how to stop it; Esc goes back" (`Task n1`, `Running · 1m`); `SubagentDetail.tsx` (facts line, `in the background`); `_gallery/88-subagents/detail-running-1280x800-{dark,light}.png` | every part on the head | passes; the capture reads "Running · 9m" and "Frontend worker · Opus 4.1 · started by the session · in the background" | PASS | ad9d5ad64 | 2026-11-06 | Claude (self) |
| R3 Task it was given: the first lines and "Show the whole prompt" | vitest (same test: `subagent-prompt`); juggler-app `src/features/subagents/tests.rs` `a_subagents_page_says_what_it_was_given_did_and_used`; `_gallery/88-subagents/detail-running-1280x800-dark.png` | prompt shown, the rest on request | passes; the capture shows the clipped prompt and "Show the whole prompt" | PASS | ad9d5ad64 | 2026-11-06 | Claude (self) |
| R4 Result: its summary, or "It's still running. Its answer shows here when it ends.", or the failure's reason in one line | vitest (same test: "It's still running."); juggler-adapters `src/claude_subagents/tests.rs` `derived_counts_stand_in_where_claude_code_reported_none` (failure = first line, "3 tests failed: rounding"); `subagents-gallery.spec.ts` "a subagent's page: running with its processes, done with its result, failed, its conversation"; `_gallery/88-subagents/detail-done-*.png`, `detail-failed-*.png` | each of the three | vitest passes; cargo in CI | PASS | ad9d5ad64 | 2026-11-06 | Claude (self) |
| R5 Work: tool uses by kind, lines, five files then "+n more", "Open its conversation" | vitest (same test: `38 · 12 reads · 9 searches · 6 commands · 4 edits · 7 other`, `+120 −34`, `tax.ts, store.ts, a.ts, b.ts, c.ts, +1 more`); "from a subagent's page, its conversation, and back"; `_gallery/88-subagents/detail-conversation-*.png` | five named, then +n | pass | PASS | ad9d5ad64 | 2026-11-06 | Claude (self) |
| R6 Usage: tokens, "~$0.04", share; where tokens are part of the session's (Claude Code), a note that they are already in its total | vitest (same test: `~$0.04`, `0.6%`); juggler-app `tests.rs` `subagents_tokens_are_a_share_of_the_sessions_never_added_on_top`, `a_subagents_page_says_what_it_was_given_did_and_used` (cost 0.05, share 2.5); `_gallery/88-subagents/detail-running-1280x800-dark.png` | estimate with ~; share never added | passes; the capture's Usage box reads "~$0.92 … 6.1% … Part of the session's total, not added to it." | PASS | ad9d5ad64 | 2026-11-06 | Claude (self) |
| R7 Processes: each matched from its commands, marked probably, with its memory, and a link to the session's Processes tab; none: "No process matched its commands." | vitest "a subagent's page: its task, result, work, usage, processes and how to stop it; Esc goes back" (the process opens the `processes` tab); `subagents-keys.test.tsx` "a quiet running one says when it last did something; its processes box says how it matches and links the tab (AJ-E2809 R8, AJ-E2811 R7, R9)"; juggler-app `src/features/subagents/attribute.rs` `a_process_is_matched_to_the_latest_command_shortly_before_it`; `_gallery/88-subagents/detail-running-1280x800-dark.png` | probably, memory, link; the none line | pass; the capture shows "npm  probably  202 MB" | PASS | f054c1028 | 2026-11-06 | Claude (self) |
| R8 Stop: no Stop button; the line names the tool's own way (the adapter's stop hint), else that stopping the session stops all of them | vitest `subagents-tab.test.tsx` (the stop box with "/tasks, select it, press x"); `subagents-tree-keys.test.tsx` "says stopping the session stops them all, and offers no Stop button" | both branches; no button | both pass | PASS | 18d6b57dd | 2026-11-07 | Claude (self) |
| R9 no CPU or memory of the subagent itself: "Every subagent runs inside its tool's one process" (D-56) | vitest `subagents-keys.test.tsx` "a quiet running one says when it last did something; its processes box says how it matches and links the tab (AJ-E2809 R8, AJ-E2811 R7, R9)" | the line said; no own CPU/memory | passes | PASS | f054c1028 | 2026-11-06 | Claude (self) |
| Gate: the detail's tests | `cd ui && pnpm -s exec vitest run src/pages/session/subagents-tab.test.tsx src/pages/session/subagents-keys.test.tsx src/pages/session/subagents-tree-keys.test.tsx src/pages/subagents/subagents.test.tsx src/lib/process-states.test.ts`; `pnpm -C ui exec playwright test tests/visual/subagents-gallery.spec.ts` (axe on every capture) | pass, every requirement PASS | vitest 5 files, 34/34; gallery 12/12, axe 0 critical/serious (2026-11-07) | PASS | f5a3fc2af | 2026-11-07 | Claude (self) |

## Binary artifacts

None added; cited: `_gallery/88-subagents/detail-running-{1024x700,1280x800}-*.png`,
`detail-done-*.png`, `detail-failed-*.png`, `detail-conversation-*.png`.
Not in the gallery: stopped and unknown states, a tool without prompts, the prompt
expanded.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none)
