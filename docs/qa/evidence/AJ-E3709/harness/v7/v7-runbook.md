# Harness v7 runbook: AJ-E3830 (watches) and AJ-E3831 (feed line, digest's G7) on Apple's model

Written by Opus (design and code), 2026-11-26, for D-149's amendment (code decides, the model picks). **Nothing here has been run against a model, and nothing is downloaded.** v7 imports v6 (`../v6/`) and v5 (`../v5/`) and edits neither; `gate.py` stays byte-identical to 7acec4f50.

## What changed from v6, and why (v6 round: `../../v6-results/summary.md`, `review-opus.md`)

| v6 miss (Apple) | v7 | Where |
|---|---|---|
| 23 of 52 watches differed only on an unsaid lifetime (the model wrote "today", the request said none) | The lifetime is code's: today, until a time, until it ends, always, once from the request's words, else unsaid (ADR-0036's default). The model has no lifetime field. | `watch3.lifetime_for` |
| Wrong targets: the ignored project picked; a session where the request named its project ("anything in X", "a session in the X project"); a shared name code had settled picked against it | `resolve()`: candidates from the user's own names, spaced names and aliases, owners, "any" words; exclusion cues first ("ignore", "except", "not the … one", "außer"); kind words keep that kind; a shared name settled by code. One candidate is code's; the model picks only among several (an enum of those candidates, or none). The app has the same resolver (`watches/resolve.rs`), to delegate its name lookup to gyrus's `seed` once it lands. | `watch3.resolve` |
| idle and stuck swapped; "finishes or fails" kept only one | The conditions are code's from cue words (each its own condition, in the request's order); idle and stuck stay apart (different states in AI Juggler: `stuck` is AI Juggler's stuck state, `idle_for` no activity at all). The model picks only when no cue word names one, from the target's enum with one-line definitions. | `watch3.cue_conditions`, `DEFINE` |
| `hits_limit` missing when an App's reports didn't list it | The enum mirrors the app's table for the target's kind (ADR-0036 §2.2: `finishes`, `ends`, `asks_you`, `fails`, `stuck`, `idle_for`, `hits_limit`, `runway_below`, `runway_resets`, `leaves_processes`, `restarted`, `pool_message`, `task_changes`, `starts`); `hits_limit` always offered (AI Juggler reads usage limits itself). | `watch3.offered_for`, `kind_conditions` |
| Numbers | Code's: the number the request says for that condition (a percent for `runway_below`, minutes otherwise; "an hour" = 60). | `watch3.param_for` |
| A false "unsupported" (money beside a usage limit; "even at 3am" read as a clock time); nearest offered 3 of 5 | The cue net with both fixed; an unsupported request **always** gets the nearest option the target reports. | `watch3.cues_fixed`, `nearest_always` |
| Feed line 34 % pooled: a file diff or a side detail picked over the point | Code ranks the checked choices: outcome and decision first, the change next, side details last, git's files out unless nothing else; the model picks from the **top three** only. | `pick2.rank` |
| Digest G7: a reverted turn's picked line dropped what was tried | A reverted or abandoned turn keeps its plain rollup line (the excerpt's words, git's "reverted"). | `digest_mr2.reverted_turn` |

### Notes: v6's no-draft cases (the round's report counts 8 on supported requests; the run had 9 form fallbacks), from the recorded attempts

| v6 item | What happened | v7 |
|---|---|---|
| V6-W11 | A shared name the request's other words settled ("resolves to s3"); the model picked the other twin three times | Code settles a shared name before any pick; no call |
| V6-W21 | The model picked a project the request doesn't name (three times) | Candidates are code's; an unnamed one is never offered |
| V6-W33, W39, W40, W41 | The request reached its target through its project or "any"; the model picked sessions the request doesn't name ("Map tiles", "Seed data", "Nightly job", "Retry policy") | The owner or "any" by code |
| V6-W50, W56 (German) | The same, in German ("Offline-Abgleich", "Anmeldung", "Argumente lesen") | The same |
| V6-W43 | `runway_below` at 3 %, outside the form's 5–95: refused three times | The number is code's; out of range, the form opens for the user to set it (no retry loop) |

## Bars (D-149, unchanged)

- **watch3:** exact rule match on the supported requests **≥ 90 %** (an unsaid lifetime read by ADR-0036's default; strict reported beside it); unsupported claimed 0; nearest offered on unsupported ≥ 90 %; watches written by the job 0. `report_v7.py` also gives `decided_by` (code, the model's pick, the picker) per part.
- **feed_pick2:** on only if preferred: **≥ 60 % of decided pairs, pooled over the blind judges** (the Sonnet round and Opus's re-judge, as v6); D-130's safety rows hold.
- **digest_mr2:** the digest is Beta (v6); G7 is a fix. There is no v7 digest set: run it when one is written.

## The run (a Sonnet agent runs; Opus verifies every verdict that matters; one model at a time; each set judged once)

The v7 sets are `../../heldout/v7/watch-v7.json` and `turns-v7.json`. **Whoever writes prompts or code never opens them.**

```sh
cd docs/qa/evidence/AJ-E3709/harness/v7
python3 selftest_v7.py                                                    # 62 checks, made-up items only
python3 ../v6/selftest_v6.py && python3 ../v5/selftest_v5.py && python3 ../v5/selftest_ef.py
O=<scratch>/v7; mkdir -p "$O"
A="$O/apple_probe"; swiftc -O -parse-as-library ../apple_probe.swift -o "$A"   # the harness's own probe, the system compiler
H=../../heldout/v7

# 0. the sets load (counts and unmapped words only)
python3 runner_v7.py --check --corpus watch7=$H/watch-v7.json --corpus turns7=$H/turns-v7.json

# 1. Apple's model
python3 runner_v7.py --cmd "$A" --job watch3     --corpus watch7=$H/watch-v7.json --out "$O/v7-apple-watch3.json"
python3 runner_v7.py --cmd "$A" --job feed_pick2 --corpus turns7=$H/turns-v7.json --out "$O/v7-apple-feed_pick2.json"

# 2. the reports
python3 report_v7.py "$O/v7-apple-watch3.json" --corpus watch7=$H/watch-v7.json --md "$O/report-apple-watch3.md" \
    --candidates "$O/candidates" > "$O/report-apple-watch3.json"
python3 report_v7.py "$O/v7-apple-feed_pick2.json" --corpus turns7=$H/turns-v7.json --md "$O/report-apple-feed_pick2.md" \
    --candidates "$O/candidates" --packets "$O/blind-review/apple-feed_pick2" > "$O/report-apple-feed_pick2.json"

# 3. the blind review of the packet (keys unopened, in $O/blind-review-key/), by the round and by Opus, then the value bar for each:
python3 report_v7.py --value "$O/review-feed_pick2.json" "$O/blind-review-key/apple-feed_pick2.json"
python3 report_v7.py --value "$O/review-feed_pick2-opus.json" "$O/blind-review-key/apple-feed_pick2.json"
```

`replay_mismatches` must be empty. Any in-app check uses the app's helper rebuilt with `scripts/build_apple_ai_helper.sh` at or after dbfcfb1d7.

## Recording a verdict

As v6: the job's verdict in `crates/juggler-app/src/features/ai/models/builtin.json` and the `models` section (`not_yet` → `beta`), its note, and a D-number. No code change.

## Files

`watch3.py` · `pick2.py` · `digest_mr2.py` · `runner_v7.py` (v6's runner, v7's jobs) · `report_v7.py` · `selftest_v7.py` · `fixtures/v7-selftest.json` (made up) · `prompts/watch3.md`, `prompts/feed_pick2.md` (generated: `selftest_v7.py --write-prompts`).
