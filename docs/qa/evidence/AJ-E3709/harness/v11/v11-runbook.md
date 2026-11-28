# Harness v11 runbook: AJ-E3830 (watches) on Apple's model

Written by Opus, 2026-11-26, for the coordinator's v11 design. **Nothing here has been run against a model, and nothing is downloaded.** v11 imports v10 to v5 and edits none of them; `gate.py` stays byte-identical to 7acec4f50.

## The design: full coverage, failing closed by construction

v10 (`../../v10-results/summary.md`) claimed 3 unsupported requests: German "nicht von der Wetterstation" was read as the target, "unless it ran for under an hour" was dropped, and "8 failed retries in a row" lost its count. Nearest was offered on 72 % of unsupported requests, because W11 and W58 opened the form. v11 changes how a draft is decided:

1. **The code parser stays first** (`watches::draft::cover::read`). It claims a request only when it places every word. A placed word is one of:
   - a target's name said whole, a kind word, or "any" before a name;
   - a condition's cue word;
   - a condition's number with its unit ("10 minutes", "half an hour", "20%");
   - a lifetime or quiet-hours phrase;
   - a word from a closed filler list.

   Any leftover word, or a number no condition said takes, and code doesn't claim the request.
2. **Then the model labels** (`cover::told_back`). A guided schema (`WatchSpans`, through DynamicGenerationSchema) splits the request into spans and gives each one slot: target, exclusion, condition, condition_param (with its number and unit), lifetime, quiet_hours, filler or other. Code checks the spans repeat the request word for word, with no gaps and no overlaps. It retries twice, then falls back to code's own reading. Code uses the spans only for two things: which words to tell back, and where the nearest option's target is. **The model never makes a request supported.** Changing what the labeller says changes no outcome. The selftest checks this with a liar labeller.
3. **The nearest option always comes with an unsupported answer.** The app's tests make this an invariant. The nearest is never widened and never the thing left out: if an unplaced word leaves something out, the nearest names no target and the form asks for one.
4. v7's call 2 (the model picking a condition when no cue word was said) is gone. With no cue word there is no condition, so the form opens. v7's call 1 (the model picking among several targets, each named in full) stays.

**v11 runs the app's own code.** Earlier harnesses re-implemented the app's rules in Python and drifted from it. `watch7` drives `watches::draft::draft` through the example `crates/juggler-app/examples/watch_round.rs` over stdin and stdout. The harness still makes every model call the app asks for (the probe, sampling, retries, attempt records), so `report_v11.py` replays a run offline as before.

| v10 miss | v11 |
|---|---|
| W03 "… die nicht von der Wetterstation kommen" (claimed) | "nicht", "solchen", "kommen" etc. unplaced: told back, nearest "fails" with no target |
| W40 "… finishes unless it ran for under an hour" (claimed) | "unless", "ran", "under" unplaced: told back "unless it ran for under an hour", nearest "finishes" |
| W60 "… has made 8 failed retries in a row" (claimed) | "made", "8", "retries", "row" unplaced (a number with no unit a condition takes): told back, nearest "fails" |
| W11 "… logs grow past 500 megabytes" (form) | told back with the nearest |
| W58 "Notify me about failures, but only the ones that aren't timeouts" (not read as a request) | "notify … about" + a condition is a watch request; told back, nearest "fails" |
| W20 "… asks me anything", W59 "Every time …" (over-refused) | "anything" after "asks" and "every time" are not a scope |
| W05 "Wake me up, even during quiet hours, if checkout-api fails" | the session over a project of the same name, and "Even during quiet hours" set (the app never set it before) |
| W54 "Any time the billing-sync session asks me" (lifetime always) | "any time" is "when": the lifetime stays unsaid |

The app tests behind these: `watches::draft::cover::tests` (v10's misses, the labels never make a request supported, an exclusion's label, the nearest invariant, code's reading) and `watches::draft::tests` (every earlier miss from v6 to v9). On made-up items, v11 now tells back four kinds of request that v9 and v10 drafted:
- an exclusion beside a named target ("… and ignore aspen-board");
- a clause code can't place ("it was split off from …");
- "say," used as filler;
- "out of every session I have open".

## Bars

1. 0 unsupported claimed. This is a hard bar and comes first.
2. Exact rule match on supported requests ≥ 90 %, counting ADR-0036's default for an unsaid lifetime as a match. The strict figure is reported beside it.
3. Nearest offered on unsupported requests ≥ 90 %.

Watches written by the job must be 0. Watches stay `not_yet` until all three bars pass.

## The run

A Sonnet agent runs it; Opus verifies the round blind. One model at a time, and the set is judged once.

The v11 set is `../../heldout/v11/watch-v11.json`. **Whoever writes prompts or code never opens it.**

```sh
# the app's drafting binary (the build env as always: ensure_swift_runtime.sh's RUSTFLAGS, the shared target)
L="$(scripts/ensure_swift_runtime.sh)"; export RUSTFLAGS="${L#RUSTFLAGS=}" CARGO_TARGET_DIR=<the shared target>
scripts/heavy.sh cargo build --offline -p juggler-app --example watch_round

cd docs/qa/evidence/AJ-E3709/harness/v11
python3 selftest_v11.py                       # 41 checks: every fixture of v5..v11, three labellers, the v10 misses
O=<scratch>/v11; mkdir -p "$O"
A="$O/apple_probe"; swiftc -O -parse-as-library ../apple_probe.swift -o "$A"   # the harness's own probe, the system compiler
H=../../heldout/v11

# 0. the set loads (counts and unmapped words only)
python3 runner_v11.py --check --corpus watch11=$H/watch-v11.json

# 1. Apple's model
python3 runner_v11.py --cmd "$A" --corpus watch11=$H/watch-v11.json --out "$O/v11-apple-watch7.json"

# 2. the report (the hard bar first; candidates for Opus)
python3 report_v11.py "$O/v11-apple-watch7.json" --corpus watch11=$H/watch-v11.json --md "$O/report-apple-watch7.md" \
    --candidates "$O/candidates" > "$O/report-apple-watch7.json"
```

`replay_mismatches` must be empty, which needs the same `watch_round` build as the run. Record the build's commit in the summary.

## Recording a verdict

As before: set the `watches` verdict in `crates/juggler-app/src/features/ai/models/builtin.json`, and the `models` section (`not_yet` → `beta`) with its note. Record a D-number. No code change. If v11 misses, report the causes; v12 isn't started without the coordinator.

## Files

- `watch7.py`
- `runner_v11.py`
- `report_v11.py`
- `selftest_v11.py`
- `fixtures/v11-selftest.json` (v10's misses as quoted, in a made-up context, plus 20 made-up items: exclusions, "unless", counts, German)
- `prompts/watch7.md` (generated from the app's prompt files with `selftest_v11.py --write-prompts`)
