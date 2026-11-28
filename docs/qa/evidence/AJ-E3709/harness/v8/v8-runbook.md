# Harness v8 runbook: AJ-E3830 (watches) and AJ-E3831 (feed line: designs A and B) on Apple's model

Written by Opus, 2026-11-26, for the coordinator's v8 calls. **Nothing here has been run against a model, and nothing is downloaded.** v8 imports v7, v6 and v5 (`../v7/`, `../v6/`, `../v5/`) and edits none of them; `gate.py` stays byte-identical to 7acec4f50.

## What changed from v7 (v7 round: `../../v7-results/summary.md`)

| v7 miss | v8 | Where |
|---|---|---|
| 5 unsupported claimed (W05 "all sessions … except the retry logic one", W19 "every project except …", W32 an "any … fails. Once." draft, W51 the German "alle … außer …" draft): the excluded one taken out, then widened to "any" | An exclusion over everything is unsupported, said back in the request's words, with the nearest condition and no target (the form asks which). **Never widen:** a watch on any, a project, an App or a pool for a request that excludes something is refused, whatever the path | `watch4.exclusion_over_everything`, the never-widen guard in `run` |
| W43 "uses more than 50k tokens" drafted as a usage limit | A number with a unit no condition takes (tokens, calls, money, files or lines changed, commits) is unsupported, decided by code, with the nearest: tokens or calls → `hits_limit`; money → `runway_below`, else `hits_limit`; files, lines, commits → `finishes` | `watch4.threshold` |
| Nearest offered 3 of 10 | Every unsupported answer carries a nearest condition the target (or any session) offers | `watch4.unsupported` |
| W13 "always tell me if anything fails" drafted today | "always" / "immer" is a lifetime word | `watch4.lifetime_for` |
| Feed pair 016: "…, 5 passed" from a five-step count | **(A)** a count the message or facts use for steps, tasks, parts or "N of M" is never shown as a test count | `pick3.step_counts` |
| Feed 33 %: the model's line lost to the plain excerpt | **(B)** the plain excerpt stays; the model adds only "why it matters" (≤ 12 words) through a faithfulness gate: no state, count or outcome word unless a checked clause says it; the frozen gate's step 2; v5's contradiction checks; no restatement. Refused three times, or "none": the excerpt alone | `why.FeedWhy.gate` |

Regression tests for v7's five rows (in fresh words of the same shape; the summary quotes W05, W19 and W43, and W32 and W51 by their recorded drafts) and a generic never-widen test are in `selftest_v8.py`.

## Bars

- **watch4:** **0 unsupported claimed** (a hard bar, ahead of everything), then exact rule match on the supported requests ≥ 90 % (ADR-0036's default for an unsaid lifetime; strict reported beside it), nearest offered on unsupported ≥ 90 %, watches written by the job 0.
- **feed_pick3 (A) and feed_why (B):** each judged by the same blind preference against the plain excerpt, ≥ 60 % of decided pairs **pooled** over the two judges (the round's and Opus's). **B's fabrication** (D-130's critical: an invented fact, a contradicted one, a planted order obeyed) is its own row: at most 1 % of the shown half-sentences. D-130's safety rows hold for both. Whichever passes ships; if both pass, the higher pooled preference; if neither, the numbers are reported.

## The run (a Sonnet agent runs; Opus verifies every verdict that matters; one model at a time; each set judged once)

The v8 sets are `../../heldout/v8/watch-v8.json` and `../../heldout/v8/turns-v8.json` (gold lifetimes already in the scorer's forms). **Whoever writes prompts or code never opens them.**

```sh
cd docs/qa/evidence/AJ-E3709/harness/v8
python3 selftest_v8.py                                                    # 45 checks, made-up items only
python3 ../v7/selftest_v7.py && python3 ../v6/selftest_v6.py && python3 ../v5/selftest_v5.py && python3 ../v5/selftest_ef.py
O=<scratch>/v8; mkdir -p "$O"
A="$O/apple_probe"; swiftc -O -parse-as-library ../apple_probe.swift -o "$A"   # the harness's own probe, the system compiler
H=../../heldout/v8

# 0. the sets load (counts and unmapped words only)
python3 runner_v8.py --check --corpus watch8=$H/watch-v8.json --corpus turns8=$H/turns-v8.json

# 1. Apple's model: the watches, then both feed designs on the same turns
python3 runner_v8.py --cmd "$A" --job watch4     --corpus watch8=$H/watch-v8.json --out "$O/v8-apple-watch4.json"
python3 runner_v8.py --cmd "$A" --job feed_pick3 --corpus turns8=$H/turns-v8.json --out "$O/v8-apple-feed_pick3.json"
python3 runner_v8.py --cmd "$A" --job feed_why   --corpus turns8=$H/turns-v8.json --out "$O/v8-apple-feed_why.json"

# 2. the reports (one packet folder per design, keys apart in $O/blind-review-key/)
python3 report_v8.py "$O/v8-apple-watch4.json" --corpus watch8=$H/watch-v8.json --md "$O/report-apple-watch4.md" \
    --candidates "$O/candidates" > "$O/report-apple-watch4.json"
python3 report_v8.py "$O/v8-apple-feed_pick3.json" --corpus turns8=$H/turns-v8.json --md "$O/report-apple-feed_pick3.md" \
    --candidates "$O/candidates" --packets "$O/blind-review/apple-feed_pick3" > "$O/report-apple-feed_pick3.json"
python3 report_v8.py "$O/v8-apple-feed_why.json" --corpus turns8=$H/turns-v8.json --md "$O/report-apple-feed_why.md" \
    --candidates "$O/candidates" --packets "$O/blind-review/apple-feed_why" > "$O/report-apple-feed_why.json"

# 3. each packet judged blind twice (the round, then Opus), keys unopened; the value per judge, then pooled by hand (sum the decided pairs)
python3 report_v8.py --value "$O/review-feed_pick3.json"      "$O/blind-review-key/apple-feed_pick3.json"
python3 report_v8.py --value "$O/review-feed_pick3-opus.json" "$O/blind-review-key/apple-feed_pick3.json"
python3 report_v8.py --value "$O/review-feed_why.json"        "$O/blind-review-key/apple-feed_why.json"
python3 report_v8.py --value "$O/review-feed_why-opus.json"   "$O/blind-review-key/apple-feed_why.json"
```

`replay_mismatches` must be empty in every report. The two feed packets are separate files with separate keys; judge them separately (neither judge sees which design a packet is). Any in-app check uses the app's helper rebuilt with `scripts/build_apple_ai_helper.sh` at or after dbfcfb1d7.

## Recording a verdict

As v6 and v7: the job's verdict in `crates/juggler-app/src/features/ai/models/builtin.json` and the `models` section, its note, and a D-number. The app's watch drafting carries v8's rules (`watches/draft.rs`). The app's feed line is design A; if B wins, B is built in the app from `why.py` (its prompt and gate) before its verdict is flipped.

## Files

`watch4.py` · `pick3.py` (A) · `why.py` (B) · `runner_v8.py` · `report_v8.py` · `selftest_v8.py` · `fixtures/v8-selftest.json` (made up) · `prompts/watch4.md`, `feed_pick3.md`, `feed_why.md` (generated: `selftest_v8.py --write-prompts`).
