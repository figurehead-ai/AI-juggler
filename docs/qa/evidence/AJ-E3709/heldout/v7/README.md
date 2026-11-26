# Held-out sets, harness round v7

Written by the blind writer (Claude Opus 5.5 session) on 2026-11-26. Written blind to prompts, code and earlier results; only the JSON shape of `v5r3/watch-v5r3.json` and `v5r3/turns-v5r3.json` was copied.

## Files

- `watch-v7.json`: 60 requests (45 English, 15 German), keys as in v5r3.
- `turns-v7.json`: 50 agent turns (27 success, 9 failure, 7 question, 7 none; 8 German), keys as in v5r3.

## Watch coverage (counted)

- Negation or exclusion: 13 (3 of them "all except", gold unsupported with the nearest option).
- Cannot become a watch: 13 (10 tagged `unsupported`, 3 "all except" ones tagged `negation`), each with nearest option in gold where one exists.
- Two names, one target: 8. Tricky names: 10 (spaces, "session" inside a project name, names that read like instructions).
- Conditions: finishes, fails, asks, stuck (idle minutes), runway_below, starts, and combined "finishes or fails".
- Lifetimes: once, today, until_removed ("always"), and one "through Friday". An unsaid lifetime is gold "today".
- Quiet-hours phrasing: 9 requests, with `even_in_quiet_hours` set.
- Request `context` has the same keys as v5r3 (sessions, apps, projects, accounts); there is no separate alias field, so aliases appear as session titles.

## Clock

"Today" lifetimes resolve against 2026-11-18 (harness clock): `until:2026-11-18T23:59:00Z`; "Friday" is `until:2026-11-20T23:59:00Z`.

## Notes

All names are fictional. Many turns are multi-clause: a main outcome beside side details, reverted or abandoned work, questions, and tests passed, failed, not run or unknown.

**Gold corrections (Opus, 2026-11-26, before any run):** V7-W30 and V7-W59 ("…hit the usage limit on <account>") were marked unsupported; AI Juggler's watches support `hits_limit` (ADR-0036's condition table, `features/watches`), so their gold is a supported `hits_limit` watch on the named account, lifetime today. "All except X" (no exclusion in ADR-0036's set) and "runs too long" with no number (no such condition; nearest `stuck`) stay unsupported.
