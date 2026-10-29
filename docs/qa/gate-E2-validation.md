# E2 validation gate (D-04) — beta-exit per pair

Generated from `juggler_handover::validation::validation_matrix()` — regenerate with `just e2-gate`. 0 of 17 pairs validated; 17 open beta debt.

A pair graduates out of "Beta" ONLY when its real-adapter CI row is green AND one
live dogfood cell passed on real binaries. Fake-harness mechanism fixtures never
promote a badge. A FAILED live cell lowers that pair's advertised ceiling one level
until fixed and re-run.

| Pair | Mechanism | Researched | Advertised | CI row | Live cell | Status |
|---|---|---|---|---|---|---|
| claude-code→claude-code | transplant | F3 | F3 | green | PENDING | Automatic tests pass with the real tools; not tried by hand yet |
| codex→codex | transplant | F3 | F3 | green | PENDING | Automatic tests pass with the real tools; not tried by hand yet Tested on sample Codex 0.163.0 sessions only; resuming a copied long session is unproven until it's tried by hand |
| zcode→zcode | transplant | F1 | F1 | not landed | PENDING | Automatic tests with the real tools aren't in place; not tried by hand yet Not available: Z Code keeps the chat in a database whose layout changes between versions, so moving it stays off until it works on two versions |
| claude-code→codex | replay | F1 | F1 | green | PENDING | Automatic tests pass with the real tools; not tried by hand yet Replays the story into a new Codex session; tested with stand-in tools |
| codex→claude-code | replay | F1 | F1 | green | PENDING | Automatic tests pass with the real tools; not tried by hand yet Replays a Codex session's story; tested with stand-in tools |
| claude-code→zcode | replay | F0 | F0 | not landed | PENDING | Automatic tests with the real tools aren't in place; not tried by hand yet Z Code takes a briefing pack only, until starting it from a story is proven |
| zcode→claude-code | replay | F1 | F1 | not landed | PENDING | Automatic tests with the real tools aren't in place; not tried by hand yet Reading Z Code sessions isn't built yet |
| codex→zcode | replay | F0 | F0 | not landed | PENDING | Automatic tests with the real tools aren't in place; not tried by hand yet Z Code support isn't built yet; it takes a briefing pack only |
| zcode→codex | replay | F1 | F1 | not landed | PENDING | Automatic tests with the real tools aren't in place; not tried by hand yet Reading Z Code sessions isn't built yet |
| claude-code→gemini-cli | pack-only | F0 | F0 | not landed | PENDING | Automatic tests with the real tools aren't in place; not tried by hand yet Briefing pack only: starting this tool from a story isn't planned for the beta |
| claude-code→opencode | pack-only | F0 | F0 | not landed | PENDING | Automatic tests with the real tools aren't in place; not tried by hand yet Briefing pack only: starting this tool from a story isn't planned for the beta |
| claude-code→claude-code+service | transplant | F3 | F3 | not landed | PENDING | Automatic tests with the real tools aren't in place; not tried by hand yet Another service answers on one side: the model changes, and whether it takes the conversation as it is hasn't been tried, so this switch stays in beta |
| claude-code+service→claude-code | transplant | F3 | F3 | not landed | PENDING | Automatic tests with the real tools aren't in place; not tried by hand yet Another service answers on one side: the model changes, and whether it takes the conversation as it is hasn't been tried, so this switch stays in beta |
| claude-code+service→claude-code+service | transplant | F3 | F3 | not landed | PENDING | Automatic tests with the real tools aren't in place; not tried by hand yet Another service answers on one side: the model changes, and whether it takes the conversation as it is hasn't been tried, so this switch stays in beta |
| codex→codex+service | transplant | F3 | F3 | not landed | PENDING | Automatic tests with the real tools aren't in place; not tried by hand yet Another service answers on one side: the model changes, and whether it takes the conversation as it is hasn't been tried, so this switch stays in beta |
| codex+service→codex | transplant | F3 | F3 | not landed | PENDING | Automatic tests with the real tools aren't in place; not tried by hand yet Another service answers on one side: the model changes, and whether it takes the conversation as it is hasn't been tried, so this switch stays in beta |
| codex+service→codex+service | transplant | F3 | F3 | not landed | PENDING | Automatic tests with the real tools aren't in place; not tried by hand yet Another service answers on one side: the model changes, and whether it takes the conversation as it is hasn't been tried, so this switch stays in beta |

## Open beta debt (never listed as passed)

- claude-code→claude-code — Automatic tests pass with the real tools; not tried by hand yet
- codex→codex — Automatic tests pass with the real tools; not tried by hand yet (Tested on sample Codex 0.163.0 sessions only; resuming a copied long session is unproven until it's tried by hand)
- zcode→zcode — Automatic tests with the real tools aren't in place; not tried by hand yet (Not available: Z Code keeps the chat in a database whose layout changes between versions, so moving it stays off until it works on two versions)
- claude-code→codex — Automatic tests pass with the real tools; not tried by hand yet (Replays the story into a new Codex session; tested with stand-in tools)
- codex→claude-code — Automatic tests pass with the real tools; not tried by hand yet (Replays a Codex session's story; tested with stand-in tools)
- claude-code→zcode — Automatic tests with the real tools aren't in place; not tried by hand yet (Z Code takes a briefing pack only, until starting it from a story is proven)
- zcode→claude-code — Automatic tests with the real tools aren't in place; not tried by hand yet (Reading Z Code sessions isn't built yet)
- codex→zcode — Automatic tests with the real tools aren't in place; not tried by hand yet (Z Code support isn't built yet; it takes a briefing pack only)
- zcode→codex — Automatic tests with the real tools aren't in place; not tried by hand yet (Reading Z Code sessions isn't built yet)
- claude-code→gemini-cli — Automatic tests with the real tools aren't in place; not tried by hand yet (Briefing pack only: starting this tool from a story isn't planned for the beta)
- claude-code→opencode — Automatic tests with the real tools aren't in place; not tried by hand yet (Briefing pack only: starting this tool from a story isn't planned for the beta)
- claude-code→claude-code+service — Automatic tests with the real tools aren't in place; not tried by hand yet (Another service answers on one side: the model changes, and whether it takes the conversation as it is hasn't been tried, so this switch stays in beta)
- claude-code+service→claude-code — Automatic tests with the real tools aren't in place; not tried by hand yet (Another service answers on one side: the model changes, and whether it takes the conversation as it is hasn't been tried, so this switch stays in beta)
- claude-code+service→claude-code+service — Automatic tests with the real tools aren't in place; not tried by hand yet (Another service answers on one side: the model changes, and whether it takes the conversation as it is hasn't been tried, so this switch stays in beta)
- codex→codex+service — Automatic tests with the real tools aren't in place; not tried by hand yet (Another service answers on one side: the model changes, and whether it takes the conversation as it is hasn't been tried, so this switch stays in beta)
- codex+service→codex — Automatic tests with the real tools aren't in place; not tried by hand yet (Another service answers on one side: the model changes, and whether it takes the conversation as it is hasn't been tried, so this switch stays in beta)
- codex+service→codex+service — Automatic tests with the real tools aren't in place; not tried by hand yet (Another service answers on one side: the model changes, and whether it takes the conversation as it is hasn't been tried, so this switch stays in beta)

## Failure-taxonomy evidence (S-F1..S-F9)

S-F1..S-F9 copy + handling are exercised by the E205 pipeline, E206 door, E207 switch-back, and E208 redaction suites; per-pair taxonomy counts accrue as live cells run (recorded per cell, never aggregated).
