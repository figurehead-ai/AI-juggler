# AJ-E808 — provider-pair fidelity register

Rendered from `crates/juggler-handover/data/fidelity-register.json` by `just fidelity-register`;
a test fails when this file is out of date. UNVALIDATED is the default. A cell turns PASS
or FAIL only from a live run on real binaries with a sacrificial project: fixture and
fake-harness runs never decide a cell. A Provider upgrade resets its cells by itself: the
app compares each cell's recorded versions with the installed ones. A pair shows
"Validated" only when its CI row is green, a live cell passed, and every area below is PASS.

**Beta-exit (Req.5):** 65 cells open — same-Provider pairs PASS on every area; cross-Provider pairs PASS or FAIL with a drop note (pack-only pairs excluded: F0 by definition).

## Matrix

| Pair | Switch rows show | Settings › Beta shows | transcript-resume | rules-carry | todos | subagents | file-history | mcp-references | cwd-worktree-edges | switch-back |
|---|---|---|---|---|---|---|---|---|---|---|
| claude-code→claude-code | Same conversation, new account · Beta · Not proven yet | Beta — not proven yet | PASS | UNVALIDATED | PASS | UNVALIDATED | UNVALIDATED | PASS | UNVALIDATED | UNVALIDATED |
| codex→codex | Same conversation, new account · Beta · Not proven yet | Beta — not proven yet | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| zcode→zcode | Full story, fresh start · Beta · Not proven yet | Beta — not proven yet | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| claude-code→codex | Full story, fresh start · Beta · Not proven yet | Beta — not proven yet | PASS | UNVALIDATED | PASS | UNVALIDATED | UNVALIDATED | PASS | UNVALIDATED | PASS |
| codex→claude-code | Full story, fresh start · Beta · Not proven yet | Beta — not proven yet | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| claude-code→zcode | Briefing pack · Beta · Not proven yet | Beta — not proven yet | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| zcode→claude-code | Full story, fresh start · Beta · Not proven yet | Beta — not proven yet | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| codex→zcode | Briefing pack · Beta · Not proven yet | Beta — not proven yet | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| zcode→codex | Full story, fresh start · Beta · Not proven yet | Beta — not proven yet | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| claude-code→gemini-cli | Briefing pack · Beta · Not proven yet | Beta — not proven yet | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| claude-code→opencode | Briefing pack · Beta · Not proven yet | Beta — not proven yet | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |

## Decided cells

| Pair | Area | Verdict | Versions (source → target) | Date | Fixture | Evidence | Drop note |
|---|---|---|---|---|---|---|---|
| claude-code→claude-code | transcript-resume | PASS | 2.1.302 → 2.1.302 | 2026-10-22 | switch-lab journey A→B→Codex→A | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| claude-code→claude-code | todos | PASS | 2.1.302 → 2.1.302 | 2026-10-22 | switch-lab journey A→B→Codex→A | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| claude-code→claude-code | mcp-references | PASS | 2.1.302 → 2.1.302 | 2026-10-22 | switch-lab journey A→B→Codex→A | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| claude-code→codex | transcript-resume | PASS | 2.1.302 → 0.163.0 | 2026-10-22 | switch-lab journey A→B→Codex→A | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| claude-code→codex | todos | PASS | 2.1.302 → 0.163.0 | 2026-10-22 | switch-lab journey A→B→Codex→A | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| claude-code→codex | mcp-references | PASS | 2.1.302 → 0.163.0 | 2026-10-22 | switch-lab journey A→B→Codex→A | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
| claude-code→codex | switch-back | PASS | 2.1.302 → 0.163.0 | 2026-10-22 | switch-lab journey A→B→Codex→A | [docs/qa/evidence/AJ-E1539/manifest.md](../../../../docs/qa/evidence/AJ-E1539/manifest.md) |  |
