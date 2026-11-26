# Harness v13 runbook: AJ-E3830 (watches) on Apple's model

Written by Opus on 2026-11-26 for the coordinator's v13 calls. **Nothing here has been run against a model, and nothing is downloaded.** v13 imports v12 down to v5 and edits none of them. `gate.py` stays byte-identical to 7acec4f50. Apple's on-device model is the only model: no local model file is on this Mac.

## What changed from v12

v12 (`../../v12-results/summary.md`): 0 unsupported claimed, nearest 95.5 %, exact 31.6 %. The span labels never verified, because Apple's model repeated the request word for word in only about 30 of 84 answers. v13 runs the app at its v13 commit through `examples/watch_round`.

1. **Labels are word-indexed** (`watches::draft::cover::labels`).
   - Code splits the words and places every word it can.
   - The model sees all the words numbered, but labels only the ones code couldn't place.
   - The answer is an array of exactly that many slots (`DynamicGenerationSchema` with `minimumElements` equal to `maximumElements`), so coverage holds by construction.
   - Code's slot stands for every word it placed.
   - A wrong length is refused and asked again. It is never padded or cut.
2. **Filler from the model passes a second guard.** These are never filler: a number or a word for one, a time or date word, a word for how often or which, a name or part of one, a condition's cue word, a negation, a unit, or a word for code work. A word like that labelled filler means the request is told back, with the nearest option. A target label stands only as a name word of the one entity the target words resolve to. Any other label on a word code can't place means the request is told back. The plain-words confirm card stays the last check.
3. **Scope-neutral negations are exempt only as closed, exact phrases:**
   - "don't care about the rest (of my apps)"
   - "quiet hours don't matter" (told even then)
   - "(aber) in der Ruhezeit bitte nicht" (a quiet-hours setting: held)

   The negation guard is otherwise unchanged.
4. **Code fixes from v12's misses:**
   - the paraphrase map;
   - ways of asking: "buzz me", "shout", "meld dich", "say when";
   - lifetimes;
   - "immer noch" read as "still";
   - context phrases;
   - dash and space name variants;
   - -Agent and -Konto compounds;
   - W04's session over a project of the same name.
5. **The size budget.** `draft.rs` is split along its seams into `readings.rs` (cue words, numbers, lifetimes) and `cover/labels.rs` (the labels and their checks).

The app tests are in `watches::draft::cover::tests`:
- `v12s_misses_draft_as_said`: each v12 miss, with a labeller that calls unknown small words filler;
- `the_labels_never_make_a_request_supported`: liars, and a wrong-length array;
- `verified_labels_draft_and_liars_never_do`: content, a code-work word, a count, a time and a name part labelled filler;
- the v10 and v11 tests.

`watches::draft::tests` keeps the v6 to v9 tests.

## Bars

The bars are reported for Apple, the only model:

1. 0 unsupported claimed. This is a hard bar and comes first.
2. Exact rule match on supported requests ≥ 90 %, counting ADR-0036's default for an unsaid lifetime as a match. The strict figure is reported beside it.
3. Nearest offered on unsupported requests ≥ 90 %.

Also report latency, median and p95. Watches written by the job must be 0. Watches stay `not_yet` until all three bars pass. If v13 misses, the numbers and causes go into the manifest and the task as **pending the maintainer's choice**. Don't write a deferral, and don't plan v14.

## The run

A Sonnet agent runs it and Opus verifies. Judge the set once.

```sh
L="$(scripts/ensure_swift_runtime.sh)"; export RUSTFLAGS="${L#RUSTFLAGS=}" CARGO_TARGET_DIR=<the shared target>
scripts/heavy.sh cargo build --offline -p juggler-app --example watch_round        # at v13's commit

cd docs/qa/evidence/AJ-E3709/harness/v13
python3 selftest_v13.py                       # every fixture of v5..v13, six labellers
O=<scratch>/v13; mkdir -p "$O"
A="$O/apple_probe"; swiftc -O -parse-as-library ../apple_probe.swift -o "$A"
H=../../heldout/v13
python3 runner_v13.py --check --corpus watch13=$H/watch-v13.json
python3 runner_v13.py --cmd "$A" --corpus watch13=$H/watch-v13.json --out "$O/v13-apple-watch9.json"
python3 report_v13.py "$O/v13-apple-watch9.json" --corpus watch13=$H/watch-v13.json --md "$O/report-apple-watch9.md" \
    --candidates "$O/candidates" > "$O/report-apple-watch9.json"
```

`replay_mismatches` must be empty, which needs the same `watch_round` build. The set is `../../heldout/v13/watch-v13.json`, and whoever writes prompts or code never opens it before the run.

## Files

- `watch9.py`
- `runner_v13.py`
- `report_v13.py`
- `selftest_v13.py`
- `fixtures/v13-selftest.json`: v12's misses as quoted, in a made-up context, plus 12 made-up items (new phrasings of negations, content hidden as filler, counts, paraphrases)
- `prompts/watch9.md`, generated
