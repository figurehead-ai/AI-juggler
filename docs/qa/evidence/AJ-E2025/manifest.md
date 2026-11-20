# Evidence manifest — AJ-E2025

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2025, 0.4.6's deferred architecture findings, built in 0.4.7's batch 1 on release/0.4.7 (D-138).

| Req | Result | Evidence | Walked |
|---|---|---|---|
| R1 | PASS (test) | 5cf6fdc68, 9b1af3bed: a neutral `TurnStop` kind on the error event, the note (`turn_end`) and `RuntimeInfo.error_end`; the UI says it from the kind (`codec::turn_end::*`, `driver::turn_end::*`; `ask-card.test.tsx` "turn-words.ts names no tool's code") | nightly |
| R2 | OPEN, to see live | No code path was found that runs an AI Juggler session in an outside terminal unplaced (fc0bd9684 and 6666e4431 place them). Walked on the nightly: Open in terminal, then the page must say where it runs before the conversation is read; a sighting becomes a finding with its fix | nightly |
