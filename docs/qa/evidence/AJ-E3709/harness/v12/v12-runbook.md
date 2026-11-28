# Harness v12 runbook: AJ-E3830 (watches) on Apple's model

Written by Opus on 2026-11-26 for the coordinator's v12 calls. **Nothing here has been run against a model, and nothing is downloaded.** v12 imports v11 down to v5 and edits none of them. `gate.py` stays byte-identical to 7acec4f50.

## What changed from v11

v11 (`../../v11-results/summary.md`) results:
- 0 unsupported claimed (PASS);
- nearest offered 95.5 % (PASS);
- exact 65.8 % (FAIL).

v11 drafted only what code placed word by word, so the model's labels never drafted. In the agreed design they do once code has verified them. v12 runs the app at its v12 commit through `examples/watch_round`, as v11 did.

1. **Verified labels may draft.** Code decides every check:
   - the spans repeat the request exactly;
   - there is no exclusion or other span;
   - a word code can't place sits only in a target span, and is a name word of the one entity the target spans resolve to; a name said only in part also says its kind word, or two of its words;
   - German compounds ("-Sitzung", "-Session", "-Lauf", "-Projekt") are split before reading;
   - the condition spans name exactly one condition, through the cue table and its closed paraphrase map;
   - every number sits in a parameter span whose unit that condition takes.
2. **A negation guard, whatever the labels say.** One of these words anywhere outside a name or a quiet-hours phrase makes the request unsupported, told back with the nearest option: not/n't, nicht, kein*, except, außer, unless, es sei denn, without, ohne, but not, never, nie, other than, apart from, and also ignore, skip, excluding. "only" and "nur" are scope words, not negations.
3. **Code fixes from v11's misses:**

| v11 miss | Fix |
|---|---|
| W19, W28, W56: "-Sitzung" compounds | The compound is split from its kind word |
| W38, W43, W49, W50, W55: not read as a request | Ask phrases added: "keep telling me", "tell me about any new …", "ich möchte/will wissen", "wann", "I only care when". W55 is now told back with the nearest, by the guard |
| W12, W24: context clauses | A closed list of scope-neutral context phrases counts as filler: "whatever else is running", "egal was sonst läuft", "across everything I run", "von allen meinen Sitzungen". This holds only because the guard has already turned away any request with a negation |
| W07, W40: paraphrases | "sits idle", "new sessions starting" |
| W04: composite condition | "crashes and gets started again" reads as restarted |
| W30: composite condition | "ends and leaves processes running" reads as leaves_processes alone |

4. **The harness clock is the round's own day:** `AIJ_HARNESS_TODAY`, default 2026-11-26, which is v12's set. The results' meta says harness v12.

The app tests behind these are in `watches::draft::cover::tests`:
- `v11s_misses_draft_as_said`;
- `verified_labels_draft_and_liars_never_do`, with a liar target over "nicht von der Wetterstation", an "unless" clause labelled filler, words hidden in filler, a part name with no kind word, and a count inside a condition;
- `the_negation_guard_needs_no_label`;
- the v10 tests.

`watches::draft::tests` keeps the v6 to v9 tests.

## Bars

1. 0 unsupported claimed. This is a hard bar and comes first.
2. Exact rule match on supported requests ≥ 90 %, counting ADR-0036's default for an unsaid lifetime as a match. The strict figure is reported beside it.
3. Nearest offered on unsupported requests ≥ 90 %.

Watches written by the job must be 0. Watches stay `not_yet` until all three bars pass.

## The run

A Sonnet agent runs it and Opus verifies. Use one model at a time, and judge the set once.

The v12 set is `../../heldout/v12/watch-v12.json`. **Whoever writes prompts or code never opens it.**

```sh
L="$(scripts/ensure_swift_runtime.sh)"; export RUSTFLAGS="${L#RUSTFLAGS=}" CARGO_TARGET_DIR=<the shared target>
scripts/heavy.sh cargo build --offline -p juggler-app --example watch_round        # at v12's commit

cd docs/qa/evidence/AJ-E3709/harness/v12
python3 selftest_v12.py                       # 75 checks: every fixture of v5..v12, five labellers (three liars)
O=<scratch>/v12; mkdir -p "$O"
A="$O/apple_probe"; swiftc -O -parse-as-library ../apple_probe.swift -o "$A"
H=../../heldout/v12
python3 runner_v12.py --check --corpus watch12=$H/watch-v12.json
python3 runner_v12.py --cmd "$A" --corpus watch12=$H/watch-v12.json --out "$O/v12-apple-watch8.json"
python3 report_v12.py "$O/v12-apple-watch8.json" --corpus watch12=$H/watch-v12.json --md "$O/report-apple-watch8.md" \
    --candidates "$O/candidates" > "$O/report-apple-watch8.json"
```

`replay_mismatches` must be empty, which needs the same `watch_round` build. Record its commit in the summary.

## Recording a verdict

As before: the `watches` verdict in `crates/juggler-app/src/features/ai/models/builtin.json`, with a D-number. No code change.

## Files

- `watch8.py`
- `runner_v12.py`
- `report_v12.py`
- `selftest_v12.py`
- `fixtures/v12-selftest.json`: v11's misses as quoted, in a made-up context, plus 16 made-up items (new ways of phrasing a negation, paraphrases, German)
- `prompts/watch8.md`, generated
