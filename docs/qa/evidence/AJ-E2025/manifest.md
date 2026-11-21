# Evidence manifest — AJ-E2025

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2025, 0.4.6's deferred architecture findings, built in 0.4.7's batch 1 on release/0.4.7 (D-138).

| Req | Result | Evidence | Walked |
|---|---|---|---|
| R1 | PASS (test) | 5cf6fdc68, 9b1af3bed: a neutral `TurnStop` kind on the error event, the note (`turn_end`) and `RuntimeInfo.error_end`; the UI says it from the kind (`codec::turn_end::*`, `driver::turn_end::*`; `ask-card.test.tsx` "turn-words.ts names no tool's code") | V2 |
| R2 | PASS (walked) | No unplaced code path was found (fc0bd9684 and 6666e4431 place them); 0.4.7's nightly walk V2 saw every session page say "Running in AI Juggler · This Mac" at once, one sidebar row each | V2 |
