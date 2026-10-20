# E2 validation gate (D-04) — beta-exit per pair

Generated from `juggler_handover::validation::validation_matrix()` — regenerate with `just e2-gate`. 0 of 11 pairs validated; 11 open beta debt.

A pair graduates out of "Beta" ONLY when its real-adapter CI row is green AND one
live dogfood cell passed on real binaries. Fake-harness mechanism fixtures never
promote a badge. A FAILED live cell lowers that pair's advertised ceiling one level
until fixed and re-run.

| Pair | Mechanism | Researched | Advertised | CI row | Live cell | Status |
|---|---|---|---|---|---|---|
| claude-code→claude-code | transplant | F1 | F1 | green | PENDING | CI fixtures green · live dogfood pending — open beta debt |
| codex→codex | transplant | F1 | F1 | green | PENDING | CI fixtures green · live dogfood pending — open beta debt transplant proven on synthetic 0.163.0 rollouts only (AJ-E1304); resuming a copied paginated rollout is unproven until a live cell |
| zcode→zcode | transplant | F1 | F1 | not landed | PENDING | Real-adapter fixtures not landed · live dogfood pending — open beta debt BLOCKED: chat is schema-coupled (SQLite clone) — transplant stays off until a clone passes against two schema versions (Req.3) |
| claude-code→codex | replay | F1 | F1 | green | PENDING | CI fixtures green · live dogfood pending — open beta debt replay into a primed Codex session, proven against stub binaries (AJ-E1304) |
| codex→claude-code | replay | F1 | F1 | green | PENDING | CI fixtures green · live dogfood pending — open beta debt replay of a Codex rollout, proven against stub binaries (AJ-E1304) |
| claude-code→zcode | replay | F0 | F0 | not landed | PENDING | Real-adapter fixtures not landed · live dogfood pending — open beta debt zcode target prime is F0 until a verified prime path exists (Req.3) |
| zcode→claude-code | replay | F1 | F1 | not landed | PENDING | Real-adapter fixtures not landed · live dogfood pending — open beta debt zcode source adapter not landed |
| codex→zcode | replay | F0 | F0 | not landed | PENDING | Real-adapter fixtures not landed · live dogfood pending — open beta debt zcode adapter not landed; zcode target prime F0 |
| zcode→codex | replay | F1 | F1 | not landed | PENDING | Real-adapter fixtures not landed · live dogfood pending — open beta debt zcode adapter not landed |
| claude-code→gemini-cli | pack-only | F0 | F0 | not landed | PENDING | Real-adapter fixtures not landed · live dogfood pending — open beta debt pack-only: no target adapter planned for beta (SPEC:140 adapter-first) |
| claude-code→opencode | pack-only | F0 | F0 | not landed | PENDING | Real-adapter fixtures not landed · live dogfood pending — open beta debt pack-only: no target adapter planned for beta (SPEC:140 adapter-first) |

## Open beta debt (never listed as passed)

- claude-code→claude-code — CI fixtures green · live dogfood pending — open beta debt
- codex→codex — CI fixtures green · live dogfood pending — open beta debt (transplant proven on synthetic 0.163.0 rollouts only (AJ-E1304); resuming a copied paginated rollout is unproven until a live cell)
- zcode→zcode — Real-adapter fixtures not landed · live dogfood pending — open beta debt (BLOCKED: chat is schema-coupled (SQLite clone) — transplant stays off until a clone passes against two schema versions (Req.3))
- claude-code→codex — CI fixtures green · live dogfood pending — open beta debt (replay into a primed Codex session, proven against stub binaries (AJ-E1304))
- codex→claude-code — CI fixtures green · live dogfood pending — open beta debt (replay of a Codex rollout, proven against stub binaries (AJ-E1304))
- claude-code→zcode — Real-adapter fixtures not landed · live dogfood pending — open beta debt (zcode target prime is F0 until a verified prime path exists (Req.3))
- zcode→claude-code — Real-adapter fixtures not landed · live dogfood pending — open beta debt (zcode source adapter not landed)
- codex→zcode — Real-adapter fixtures not landed · live dogfood pending — open beta debt (zcode adapter not landed; zcode target prime F0)
- zcode→codex — Real-adapter fixtures not landed · live dogfood pending — open beta debt (zcode adapter not landed)
- claude-code→gemini-cli — Real-adapter fixtures not landed · live dogfood pending — open beta debt (pack-only: no target adapter planned for beta (SPEC:140 adapter-first))
- claude-code→opencode — Real-adapter fixtures not landed · live dogfood pending — open beta debt (pack-only: no target adapter planned for beta (SPEC:140 adapter-first))

## Failure-taxonomy evidence (S-F1..S-F9)

S-F1..S-F9 copy + handling are exercised by the E205 pipeline, E206 door, E207 switch-back, and E208 redaction suites; per-pair taxonomy counts accrue as live cells run (recorded per cell, never aggregated).
