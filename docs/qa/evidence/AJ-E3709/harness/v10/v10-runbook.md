# Harness v10 runbook: AJ-E3830 (watches) on Apple's model

Written by Opus, 2026-11-26, for the coordinator's v10 calls on v9's results. **Nothing here has been run against a model, and nothing is downloaded.** v10 imports v9 to v5 and edits none of them; `gate.py` stays byte-identical to 7acec4f50.

## What changed from v9 (`../../v9-results/summary.md`: 0 unsupported claimed, nearest 92 %, exact 68.75 %; code decided 58 of 60)

| v9 miss | v10 (still fail closed) |
|---|---|
| Lifetimes said in words read as today: W01 "dauerhaft", W03 "permanently", W32 "keep that on permanently" (gold always); W30 "einmal", W40, W56, W58 (gold once) | always: "permanently", "from now on", "every time", "keep watching", "keep that on", "for good", "dauerhaft", "immer", "any time"; once: "just this once", "the next time", "einmal", "nur beim nächsten Mal"; today: "for the rest of today", "for today", "bis heute Abend", "für heute". Unsaid stays unsaid (ADR-0036's default) |
| 8 over-refusals: a named entity that is the target beside a broad word (W02 "Bei all meinen Projekten: … wenn gartenplaner scheitert", W18, W25, W36, W37, W55) | Entity wins only when it is the subject or object of the condition's own clause: that clause names it and holds no broad word, or the broad word is "any time", an indefinite "anything … with <entity>", or stands right before its name ("any OpenCode session"); never with a meaning-changer |
| W21 "but hold it until quiet hours are over" read as a meaning-changer; W24 a bare "say" read as what an agent says; W41 "I need to know the moment …" read as no watch | "but" before quiet-hours words isn't a meaning-changer; a bare "say" isn't a cue; "need/want to know", "the moment" ask to be told |
| W28 "the Pattern export one" gave an invented 1-minute wait | A minutes value only with a time unit ("10 minutes", "an hour"); a percent only with "%", "percent", "Prozent"; of two names said where one holds the other ("Pattern export", "pattern"), the longer |
| Nearest missing: W17 "I want to know about every new session, leave out OpenCode", W47 "more than 1.5 million tokens" | An ask about a condition is a watch request (then "leave out" is told back with the nearest); "1.5 million tokens" is a threshold |

The app carries the same rules (`crates/juggler-app/src/features/watches/draft.rs`, `fail_closed.rs`, `resolve.rs`), with a regression test for each v9 miss (`v9s_misses_draft_as_said`).

## Bars

0 unsupported claimed (hard, first), then exact rule match on the supported requests ≥ 90 % (ADR-0036's default for an unsaid lifetime; strict reported beside it), nearest offered on unsupported ≥ 90 %, watches written by the job 0.

## The run (a Sonnet agent runs; Opus verifies every verdict that matters; one model at a time; the set judged once)

The v10 set is `../../heldout/v10/watch-v10.json` (60 requests). **Whoever writes prompts or code never opens it.**

```sh
cd docs/qa/evidence/AJ-E3709/harness/v10
python3 selftest_v10.py                                                   # 31 checks, made-up items and v9's misses
python3 ../v9/selftest_v9.py && python3 ../v8/selftest_v8.py && python3 ../v7/selftest_v7.py && python3 ../v6/selftest_v6.py
O=<scratch>/v10; mkdir -p "$O"
A="$O/apple_probe"; swiftc -O -parse-as-library ../apple_probe.swift -o "$A"   # the harness's own probe, the system compiler
H=../../heldout/v10

# 0. the set loads (counts and unmapped words only)
python3 runner_v10.py --check --corpus watch10=$H/watch-v10.json

# 1. Apple's model
python3 runner_v10.py --cmd "$A" --corpus watch10=$H/watch-v10.json --out "$O/v10-apple-watch6.json"

# 2. the report (the hard bar first; candidates for Opus)
python3 report_v10.py "$O/v10-apple-watch6.json" --corpus watch10=$H/watch-v10.json --md "$O/report-apple-watch6.md" \
    --candidates "$O/candidates" > "$O/report-apple-watch6.json"
```

`replay_mismatches` must be empty. Any in-app check uses the app's helper rebuilt with `scripts/build_apple_ai_helper.sh` at or after dbfcfb1d7.

## Recording a verdict

As before: the `watches` verdict in `crates/juggler-app/src/features/ai/models/builtin.json` and the `models` section (`not_yet` → `beta`) with its note, and a D-number. No code change.

## Files

`watch6.py` · `runner_v10.py` · `report_v10.py` · `selftest_v10.py` · `fixtures/v10-selftest.json` · `prompts/watch6.md` (generated: `selftest_v10.py --write-prompts`).
