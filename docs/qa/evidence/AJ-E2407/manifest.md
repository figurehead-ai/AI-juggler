# Evidence manifest — AJ-E2407

Task: `docs/tasks/E24-processes.md` AJ-E2407, One source: the session tab, Cleanup and This Mac on the graph. Branch `e24-processes`. Status: **IN-PROGRESS**.

## What exists now

The E410 `OrphanTracker` and its 10 s clock are gone (`ffdb5a4a`); leftovers are the ledger's and the sightings'. A session's Processes tab is `ProcessesView` with that session's id (any app's session). Cleanup is the Processes page. This Mac's "Left behind" is the graph's to-clean count, and its "What's using it" lists every other app's AI session the graph found (`machine_tools::graph_trees`), beside the Claude tools' own scan (`running_setups` unchanged).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 a session's tab reads the graph, its session alone | `pnpm exec vitest run src/components/processes-tab.test.tsx` | that session's tree only; Stop… opens the verified sheet; the zombie explained; empty says so | pass (6 tests) | PASS | e8e10949 | 2026-10-26 | agent |
| Req.2 the tracker's own detection is retired; stops take the verified path | `cargo test -p juggler-app --test processes --test signal_paths --test guardrails` | pass; `stop::execute` and `processes_observed_stop` listed as verified-ladder sites | pass (3, 6, 12) | PASS | ffdb5a4a | 2026-10-26 | agent |
| Req.3 This Mac's Left behind and the top bar's count are the graph's | `cargo test -p juggler-app --test processes_live` (This Mac step); `pnpm exec vitest run src/screens/processes-page.test.tsx` | the same number | pass | PASS | 0663d3fc | 2026-10-26 | agent |
| This Mac's groups from the graph: every app's AI sessions, by app and tool, not counted twice | `cargo test -p juggler-app --lib machine_tools`; `pnpm exec vitest run src/screens/this-mac-page.test.tsx` | ChatGPT and "Codex in Terminal" groups, heaviest first; a session inside the Claude app's tree left to its row; the alert names the group | pass | PASS | 93d4c166 | 2026-10-26 | agent |
| Budget: no new work on the tick | review: the groups read the graph the tick already built and the collector's trees for the view | no new scan | by reading | PASS | 93d4c166 | 2026-10-26 | agent |
| Visual: a session's Processes tab | `AIJ_PW_PORT=4291 pnpm exec playwright test tests/visual/processes-gallery.spec.ts -g "Processes tab"` → `_gallery/90-processes/session-tab-1680x1000-{dark,light}.png` | looked at; axe 0 serious | looked at; axe 0 | PASS | — | 2026-10-26 | agent |
| Live: the tab with another app's session | `docs/qa/processes-lab.md` step 7, 2026-10-26 ([lab-1-2026-10-26.md](docs/qa/evidence/AJ-E2406/lab-1-2026-10-26.md)) | as written | the outside Claude Code session's own rows (`claude`, `zsh`, `Python` :8766) came back for its session id, the same as on the page | PASS (backend; the window: BLOCKED) | — | 2026-10-26 | agent |
| Live: This Mac lists another app's session | the same lab, step 10 (bug 5) | under its app's group, by its title, "not started by AI Juggler" | "Claude app" (count 6) listed only the desktop app's own heaviest processes; the lab's Claude Code session under it was missing | FAIL | — | 2026-10-26 | agent |
| Live: Include other apps | the same lab, step 10 | names and memory, no Stop | "Other apps on this Mac", "Not in an AI session", programs by name and memory, nothing to stop | PASS (backend) | — | 2026-10-26 | agent |
| Live: the counts agree | the same lab, step 2 | one number everywhere | `to_clean` 2 = `leftovers()` 2 = This Mac `leftover_processes` 2 (one server and its shell: finding 3) | PASS | — | 2026-10-26 | agent |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
