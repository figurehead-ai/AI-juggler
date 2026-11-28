# Harness v9 runbook: AJ-E3830 (watches, fail closed) on Apple's model

Written by Opus, 2026-11-26, for D-149 amendment 2. **Nothing here has been run against a model, and nothing is downloaded.** v9 imports v8, v7, v6 and v5 and edits none of them; `gate.py` stays byte-identical to 7acec4f50. The feed line stays plain (amendment 2, point 2): no feed round in v9.

## What changed from v8 (`../../v8-results/summary.md`)

Code accepts a draft only when the request parses into a shape it fully understands; otherwise it is told back as unsupported with the nearest option, or the form opens (`watch5.py`):

| v8 miss | v9 |
|---|---|
| W27 "…, skipping the glossary hub" drafted on any session | A meaning-changing word outside a name (but, except, skipping, other than, besides, apart from, aside from, excluding, leave out, not the … one, without; außer, ohne, bis auf, nicht das/die/den/der) is unsupported with the nearest; "but only" / "but just" (narrowing to what follows) is the one reading kept |
| W54 "everything but form-kit" drafted **on** form-kit | The same rule ("but"); a broad quantifier beside a named entity is unsupported unless the entity is clearly the target (a selection, "of all my sessions, … checkout-api"; the quantifier inside it, "anything in pier") |
| W60 "the token count … passes one million" drafted as a usage limit | A number in a unit no condition takes is refused before any model call, the unit before or after the number, the number in digits or words |
| W58, W45, W32, W48: the wrong one of two named, an extra condition from another clause | A request of several clauses is read by the clause that asks to be told (its sentence; from the last comma before the ask when that part names a condition); the lifetime from the whole |
| W23, W32: a project for its namesake session | A session and its own project or App named alike, with no kind word: the session |
| W13, W24: a bare "once" as a lifetime | "once" is a lifetime only as "just once", "only once", "one time", "the first time" |

Never widen (v8) stays. The app carries the same rules (`crates/juggler-app/src/features/watches/fail_closed.rs`, `draft.rs`, `resolve.rs`; a regression test for every unsupported-claimed miss so far), and Bot's line shows the drafted watch in plain words, its target included (`BotWatchDraftWords`). G11 is in `g11.py`; its offline re-run is `g11-rerun.md`.

## Bars (D-149, amendment 2)

**0 unsupported claimed** (a hard bar, ahead of everything), then exact rule match on the supported requests ≥ 90 % (ADR-0036's default for an unsaid lifetime; strict reported beside it), nearest offered on unsupported ≥ 90 %, watches written by the job 0.

## The run (a Sonnet agent runs; Opus verifies every verdict that matters; one model at a time; the set judged once)

The v9 set is `../../heldout/v9/watch-v9.json` (60 requests). **Whoever writes prompts or code never opens it.**

```sh
cd docs/qa/evidence/AJ-E3709/harness/v9
python3 selftest_v9.py                                                    # 27 checks, made-up items and the misses' own requests
python3 ../v8/selftest_v8.py && python3 ../v7/selftest_v7.py && python3 ../v6/selftest_v6.py && python3 ../v5/selftest_v5.py
O=<scratch>/v9; mkdir -p "$O"
A="$O/apple_probe"; swiftc -O -parse-as-library ../apple_probe.swift -o "$A"   # the harness's own probe, the system compiler
H=../../heldout/v9

# 0. the set loads (counts and unmapped words only)
python3 runner_v9.py --check --corpus watch9=$H/watch-v9.json

# 1. Apple's model
python3 runner_v9.py --cmd "$A" --corpus watch9=$H/watch-v9.json --out "$O/v9-apple-watch5.json"

# 2. the report (the hard bar first; candidates for Opus)
python3 report_v9.py "$O/v9-apple-watch5.json" --corpus watch9=$H/watch-v9.json --md "$O/report-apple-watch5.md" \
    --candidates "$O/candidates" > "$O/report-apple-watch5.json"
```

`replay_mismatches` must be empty. Any in-app check uses the app's helper rebuilt with `scripts/build_apple_ai_helper.sh` at or after dbfcfb1d7.

## Recording a verdict

As before: the `watches` verdict in `crates/juggler-app/src/features/ai/models/builtin.json` and the `models` section (`not_yet` → `beta`) with its note, and a D-number. No code change.

## Files

`watch5.py` · `g11.py` · `runner_v9.py` · `report_v9.py` · `selftest_v9.py` · `fixtures/v9-selftest.json` · `prompts/watch5.md` (generated: `selftest_v9.py --write-prompts`) · `g11-rerun.md`.
