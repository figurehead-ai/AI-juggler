# AJ-E808 — provider-pair fidelity register

Rendered from `crates/juggler-handover/data/fidelity-register.json` by `just fidelity-register`;
a test fails when this file is out of date. UNVALIDATED is the default. A cell turns PASS
or FAIL only from a live run on real binaries with a sacrificial project: fixture and
fake-harness runs never decide a cell. A Provider upgrade resets its cells by itself: the
app compares each cell's recorded versions with the installed ones. A pair shows
"Validated" only when its CI row is green, a live cell passed, and every area below is PASS.

**Beta-exit (Req.5):** 72 cells open — same-Provider pairs PASS on every area; cross-Provider pairs PASS or FAIL with a drop note (pack-only pairs excluded: F0 by definition).

## Matrix

| Pair | Switch rows show | Settings › Beta shows | transcript-resume | rules-carry | todos | subagents | file-history | mcp-references | cwd-worktree-edges | switch-back |
|---|---|---|---|---|---|---|---|---|---|---|
| claude-code→claude-code | F1 · Beta · Not yet validated | Beta — not yet validated | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| codex→codex | F1 · Beta · Not yet validated | Beta — not yet validated | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| zcode→zcode | F1 · Beta · Not yet validated | Beta — not yet validated | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| claude-code→codex | F1 · Beta · Not yet validated | Beta — not yet validated | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| codex→claude-code | F1 · Beta · Not yet validated | Beta — not yet validated | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| claude-code→zcode | F0 · Beta · Not yet validated | Beta — not yet validated | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| zcode→claude-code | F1 · Beta · Not yet validated | Beta — not yet validated | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| codex→zcode | F0 · Beta · Not yet validated | Beta — not yet validated | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| zcode→codex | F1 · Beta · Not yet validated | Beta — not yet validated | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| claude-code→gemini-cli | F0 · Beta · Not yet validated | Beta — not yet validated | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |
| claude-code→opencode | F0 · Beta · Not yet validated | Beta — not yet validated | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED | UNVALIDATED |

## Decided cells

None yet — no cell has run live. Every pair reads Beta in the product.
