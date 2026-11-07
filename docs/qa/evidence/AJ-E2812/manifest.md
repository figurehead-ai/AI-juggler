# Evidence manifest — AJ-E2812 Cross-links: the session's Usage and Processes, and the project's subagents

Task: `docs/tasks/E28-finding-your-way.md` AJ-E2812, judged against its
amended text (2026-11-06: the project's newest sessions together, each group
saying its session). Built in 0.3 (AJ-E1520: e6c96f2e3, ad9d5ad64) with 0.4.1
additions (2b00b4401, f054c1028, 8bafa6ac4); re-checked against the code, its
tests and the committed gallery on `release/0.4.1`, 2026-11-06.

Vitest was run for this manifest (`subagents.test.tsx`: 1 file, 6/6 passed).
The cargo test for R4 is as observed at 8bafa6ac4; the others run in CI.
Playwright was not run here.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 the Usage tab: "Subagents: 41% of this session's tokens" and its three most expensive, each a link to its detail | `cd ui && pnpm -s exec vitest run src/pages/subagents/subagents.test.tsx`: "the Usage tab says the subagents' share and opens them sorted by most tokens" (`Subagents: 41% of this session's tokens·see which ›`), "the Usage tab names the three that used most, each opening its own page (AJ-E2812 R1)" (by own tokens, the link opens `{ subagent: "a2" }`) | the line and three links | pass. The gallery is stale: `_gallery/88-subagents/usage-line-1024x700-*.png` still shows the old "Subagents used 38% of these tokens" with no "Used most" line, and `subagents-gallery.spec.ts` "the narrower window, the Usage tab's line, and the session's line" still expects "Subagents used" (see Gate) | PASS | 2b00b4401 | 2026-11-06 | Claude (self) |
| R2 the Processes tab: "probably started by <subagent>" on a matched process, linking to it | vitest `subagents.test.tsx` "a process a subagent probably started says so, and in a session's tab opens it" (`probably started by subagent ‘Audit the cart’ ›`, a link in a session's tab, plain text elsewhere); juggler-app `src/features/subagents/attribute.rs` `a_process_is_matched_to_the_latest_command_shortly_before_it` | the line and its link | vitest passes; cargo in CI | PASS | f054c1028 | 2026-11-06 | Claude (self) |
| R3 the project page's Subagents tab: the project's newest sessions together, each group saying its session, then by message | vitest `subagents.test.tsx` "is the same view over its sessions, each group saying its session" (`in Fix the cart totals`, "of these sessions' tokens"); juggler-app `src/features/subagents/tests.rs` `a_projects_view_is_its_sessions_together_each_group_saying_its_session` | each group names its session | vitest passes; cargo in CI | PASS | ad9d5ad64 | 2026-11-06 | Claude (self) |
| R4 Processes keeps its stop rules; the subagent line is information only | `cargo test -p juggler-app --test processes_graph a_subagent_line_never_changes_how_a_process_may_be_stopped` | stop rule unchanged with and without the line | passed at 8bafa6ac4 (observed then) | PASS | 8bafa6ac4 | 2026-11-06 | Claude (self) |
| Gate: the cross-links' tests | `cd ui && pnpm -s exec vitest run src/pages/session/subagents-tab.test.tsx src/pages/session/subagents-keys.test.tsx src/pages/session/subagents-tree-keys.test.tsx src/pages/subagents/subagents.test.tsx src/lib/process-states.test.ts`; `pnpm -C ui exec playwright test tests/visual/subagents-gallery.spec.ts` (axe on every capture); `cargo test -p juggler-app --test processes_graph` | pass | vitest 34/34; gallery 12/12 with the Usage line's new words (f5a3fc2af) and `usage-line-1024x700-{dark,light}.png` re-shot and looked at; the row-count axe failure (a `RoomChip` aria-label) fixed in 8d457d2fd; processes_graph 14/14 at 8bafa6ac4 | PASS | 8d457d2fd | 2026-11-07 | Claude (self) |

## Binary artifacts

None added. `_gallery/88-subagents/usage-line-1024x700-*.png` is stale (before
2b00b4401) and not cited as evidence. No capture shows the "probably started by"
line on the Processes tab.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none)
