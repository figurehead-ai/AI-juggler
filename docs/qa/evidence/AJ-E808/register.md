# AJ-E808 — provider-pair fidelity register

Rendered from `crates/juggler-handover/data/fidelity-register.json` by `just fidelity-register`;
a test fails when this file is out of date. UNVALIDATED is the default. A cell turns PASS
or FAIL only from a live run on real binaries with a sacrificial project: fixture and
fake-harness runs never decide a cell. A Provider upgrade resets its cells by itself: the
app compares each cell's recorded versions with the installed ones. A pair shows
"Validated" only when its CI row is green, a live cell passed, and every area below is PASS.

**Beta-exit (Req.5):** 97 cells open — same-Provider pairs PASS on every area; cross-Provider pairs PASS or FAIL with a drop note (pack-only pairs excluded: F0 by definition).

## Matrix

| Pair | Switch rows show | Settings › Beta shows | transcript-resume | rules-carry | todos | subagents | file-history | mcp-references | cwd-worktree-edges | switch-back |
|---|---|---|---|---|---|---|---|---|---|---|
| claude-code→claude-code | Same conversation, new account · Beta · Not proven yet | Beta — not proven yet | PASS | PASS | PASS | PASS | UNVALIDATED | PASS | PASS | PASS |
| codex→codex | Same conversation, new account · Beta · Not proven yet | Beta — not proven yet | PASS | UNVALIDATED | PASS | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| zcode→zcode | Full story, fresh start · Beta · Not proven yet | Beta — not proven yet | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| claude-code→codex | Full story, fresh start · Beta · Not proven yet | Beta — not proven yet | PASS | PASS | PASS | FAIL | FAIL | PASS | PASS | PASS |
| codex→claude-code | Full story, fresh start · Beta · Not proven yet | Beta — not proven yet | PASS | PASS | PASS | UNVALIDATED | UNVALIDATED | PASS | PASS | PASS |
| claude-code→zcode | Briefing pack · Beta · Not proven yet | Beta — not proven yet | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| zcode→claude-code | Full story, fresh start · Beta · Not proven yet | Beta — not proven yet | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| codex→zcode | Briefing pack · Beta · Not proven yet | Beta — not proven yet | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| zcode→codex | Full story, fresh start · Beta · Not proven yet | Beta — not proven yet | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| claude-code→gemini-cli | Briefing pack · Beta · Not proven yet | Beta — not proven yet | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| claude-code→opencode | Briefing pack · Beta · Not proven yet | Beta — not proven yet | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| claude-code→claude-code+service | Same conversation, new account · Beta · Not proven yet | Beta — not proven yet | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| claude-code+service→claude-code | Same conversation, new account · Beta · Not proven yet | Beta — not proven yet | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| claude-code+service→claude-code+service | Same conversation, new account · Beta · Not proven yet | Beta — not proven yet | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| codex→codex+service | Same conversation, new account · Beta · Not proven yet | Beta — not proven yet | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| codex+service→codex | Same conversation, new account · Beta · Not proven yet | Beta — not proven yet | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| codex+service→codex+service | Same conversation, new account · Beta · Not proven yet | Beta — not proven yet | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |

## Decided cells

| Pair | Area | Verdict | Versions (source → target) | Date | Fixture | Evidence | Drop note |
|---|---|---|---|---|---|---|---|
| claude-code→claude-code | transcript-resume | PASS | 2.1.302 → 2.1.302 | 2026-10-22 | switch-lab journey A→B→Codex→A | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| claude-code→claude-code | rules-carry | PASS | 2.1.302 → 2.1.302 | 2026-10-23 | switch-lab labs 9 and 10 | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| claude-code→claude-code | todos | PASS | 2.1.302 → 2.1.302 | 2026-10-22 | switch-lab journey A→B→Codex→A | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| claude-code→claude-code | subagents | PASS | 2.1.302 → 2.1.302 | 2026-10-23 | switch-lab labs 9 and 10 | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| claude-code→claude-code | mcp-references | PASS | 2.1.302 → 2.1.302 | 2026-10-22 | switch-lab journey A→B→Codex→A | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| claude-code→claude-code | cwd-worktree-edges | PASS | 2.1.302 → 2.1.302 | 2026-10-23 | switch-lab labs 9 and 10 | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| claude-code→claude-code | switch-back | PASS | 2.1.302 → 2.1.302 | 2026-10-23 | switch-lab labs 9 and 10 | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| codex→codex | transcript-resume | PASS | 0.163.0 → 0.163.0 | 2026-10-23 | switch-lab lab 11 | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| codex→codex | todos | PASS | 0.163.0 → 0.163.0 | 2026-10-23 | switch-lab lab 11 | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| claude-code→codex | transcript-resume | PASS | 2.1.310 → 0.163.0 | 2026-10-26 | switch-lab lab 14, J1 and J3 | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| claude-code→codex | rules-carry | PASS | 2.1.310 → 0.163.0 | 2026-10-26 | switch-lab lab 14, J1 | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| claude-code→codex | todos | PASS | 2.1.310 → 0.163.0 | 2026-10-26 | switch-lab lab 14, J1 and J3 | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| claude-code→codex | subagents | FAIL | 2.1.310 → 0.163.0 | 2026-10-26 | switch-lab lab 14, J1 | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) | Doesn't carry: subagent state. The switch screen lists it; a subagent's own transcript stays on Claude Code. |
| claude-code→codex | file-history | FAIL | 2.1.310 → 0.163.0 | 2026-10-26 | switch-lab lab 14, J1 | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) | Claude Code's file checkpoints (rewind) stay on Claude Code: the switch screen says tool state doesn't carry. The edited files stay in the folder, and an exported pack's diffs/ holds the uncommitted changes. |
| claude-code→codex | mcp-references | PASS | 2.1.310 → 0.163.0 | 2026-10-26 | switch-lab lab 14, J1 (AIJ_LAB_PROBE) | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| claude-code→codex | cwd-worktree-edges | PASS | 2.1.310 → 0.163.0 | 2026-10-26 | switch-lab lab 14, J1 (folder) and J3 (git worktree) | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| claude-code→codex | switch-back | PASS | 2.1.310 → 0.163.0 | 2026-10-26 | switch-lab lab 14, J1 and J3 | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| codex→claude-code | transcript-resume | PASS | 0.163.0 → 2.1.310 | 2026-10-26 | switch-lab lab 14, J2 | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| codex→claude-code | rules-carry | PASS | 0.163.0 → 2.1.310 | 2026-10-26 | switch-lab lab 14, J2 | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| codex→claude-code | todos | PASS | 0.163.0 → 2.1.310 | 2026-10-26 | switch-lab lab 14, J2 | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| codex→claude-code | mcp-references | PASS | 0.163.0 → 2.1.310 | 2026-10-26 | switch-lab lab 14, J2 (AIJ_LAB_PROBE) | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| codex→claude-code | cwd-worktree-edges | PASS | 0.163.0 → 2.1.310 | 2026-10-26 | switch-lab lab 14, J2 (git worktree) | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| codex→claude-code | switch-back | PASS | 0.163.0 → 2.1.310 | 2026-10-26 | switch-lab lab 14, J2 | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
