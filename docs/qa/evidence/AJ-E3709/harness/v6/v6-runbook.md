# Harness v6 runbook: AJ-E3830 (watches) and AJ-E3831 (feed line, digest) on Apple's model

Written by Opus (design and code), 2026-11-25, for D-149. **Nothing here has been run against a model, and nothing is downloaded.** Code: this folder. It imports harness v5 (`../v5/`) for every check and never edits it; `gate.py` stays byte-identical to 7acec4f50.

## What v6 asks of the model

| Job | Code (the truth, every check) | The model (a constrained choice) | Plain path |
|---|---|---|---|
| `watch2` (AJ-E3830) | The catalogue; whether the target is named, a shared name settled or the picker; the request's own words for what can't be watched (the cue net); v5's `WatchDraft` checks (observes, parameters, defaults, the nearest option, quiet hours, the echo) | Call 1 `{intent, target}`: one target, an enum of the request's refs. Call 2 `{condition, param, condition2, param2, lifetime, until, unsupported}`: each an enum built for that target (its kind's conditions that some App reports, the numbers and times the request says, a short enum for what can't be watched) | The empty Watches form |
| `feed_pick` (AJ-E3831 R1) | v5's routing (needs you, unknown facts or language, a guard removal: A's excerpt, no call); the CHOICES (the agent's clauses that pass every v5 check, git's kept files); the line (the choice's words and verb, the tests from the facts, the caveat by v4's composer); v5's feed-line checks on it | `{fact, verb}`: a choice's number and a verb from the enum | A's excerpt |
| `digest_mr` (AJ-E3831 R2) | Each agent's line (name, state, stuck/runway/processes, what git undid), the order, the cap of 6, "and N more · Q quiet" (v5's rollup and compose) | One `feed_pick` per calm agent's newest turn | The rollup |

Retries (D-149): a failed check regenerates **that call** with the reason, at most twice, and **samples** (Apple: random, temperature 0.6, top-k 40, a seed per item and attempt; a GGUF model: its arm, temperature at least 0.6, a seed). The first attempt keeps the model's tested decoding (Apple greedy). Each run records every attempt with its step, so the report replays it offline.

The harness clock's today is **2026-11-18** (`core6.HARNESS_TODAY`): the blind writer resolved "today" lifetimes against it.

## Bars (D-149, D-130's safety bars unchanged)

- **watch2:** exact rule match on the supported requests **≥ 90 %** (an unsaid lifetime read by ADR-0036's default; the strict figure is reported beside it), unsupported claimed 0, watches written by the job 0. Below 90 %: report to the coordinator before anything ships (AJ-E3830 R3).
- **feed_pick, digest_mr:** switched on **only if preferred** over the plain text in the blind checks. The value row uses D-130's bar (**≥ 60 % of decided pairs**, ties left out); the coordinator confirms that this is what "preferred" means before a verdict is recorded. The safety rows still hold: critical reaching the user ≤ 1 % and none systematic, omissions ≤ ~10 %, questions or failures hidden 0, model text on a turn that needs you 0.

## The run (a Sonnet agent runs; Opus checks every verdict that matters; one model at a time; each set judged once)

The v6 blind sets are `../../heldout/v6/watch-v6.json` (60 requests), `turns-v6.json` (50 turns) and `digest-v6.json` (30 windows). **Whoever writes prompts or code never opens them.** The commands below print counts and scores, not item text, until the blind review.

```sh
cd docs/qa/evidence/AJ-E3709/harness/v6
python3 selftest_v6.py                                                    # 70 checks, made-up items only
python3 ../v5/selftest_v5.py && python3 ../v5/selftest_ef.py              # v5 unchanged underneath
O=<scratch>/v6; mkdir -p "$O"                                             # a scratch folder outside the repo
A="$O/apple_probe"; swiftc -O -parse-as-library ../apple_probe.swift -o "$A"   # the system compiler; no download
H=../../heldout/v6

# 0. the sets load: counts and unmapped words only (no item text); every count must be as the README says and every unmapped count 0
python3 runner_v6.py --check --corpus watch6=$H/watch-v6.json --corpus turns6=$H/turns-v6.json --corpus digest6=$H/digest-v6.json

# 1. Apple's model, one job at a time
python3 runner_v6.py --cmd "$A" --job watch2    --corpus watch6=$H/watch-v6.json   --out "$O/v6-apple-watch2.json"
python3 runner_v6.py --cmd "$A" --job feed_pick --corpus turns6=$H/turns-v6.json   --out "$O/v6-apple-feed_pick.json"
python3 runner_v6.py --cmd "$A" --job digest_mr --corpus digest6=$H/digest-v6.json --out "$O/v6-apple-digest_mr.json"

# 2. the reports (scores, candidates for Opus, blind packets with their keys kept apart)
python3 report_v6.py "$O/v6-apple-watch2.json" --corpus watch6=$H/watch-v6.json --md "$O/report-apple-watch2.md" \
    --candidates "$O/candidates" > "$O/report-apple-watch2.json"
python3 report_v6.py "$O/v6-apple-feed_pick.json" --corpus turns6=$H/turns-v6.json --md "$O/report-apple-feed_pick.md" \
    --candidates "$O/candidates" --packets "$O/blind-review/apple-feed_pick" > "$O/report-apple-feed_pick.json"
python3 report_v6.py "$O/v6-apple-digest_mr.json" --corpus digest6=$H/digest-v6.json --md "$O/report-apple-digest_mr.md" \
    --candidates "$O/candidates" --packets "$O/blind-review/apple-digest_mr" > "$O/report-apple-digest_mr.json"

# 3. a fresh blind review of each packet (keys unopened: they are in $O/blind-review-key/), written as
#    {"entries": [{"pair": "<pair id>", "prefer": "first" | "second" | "same"}]}, then the value bar:
python3 report_v6.py --value "$O/review-feed_pick.json" "$O/blind-review-key/apple-feed_pick.json"
python3 report_v6.py --value "$O/review-digest_mr.json" "$O/blind-review-key/apple-digest_mr.json"
```

The runs above use the harness's own probe (`../apple_probe.swift`, compiled fresh each time), not the app's helper. Any in-app check of these jobs (AJ-E3829's real-model check, a walk) uses the app's helper rebuilt with `scripts/build_apple_ai_helper.sh` at or after dbfcfb1d7 (it reads bare-word field shapes and returns every field typed to its shape); v6's schemas keep every enum as `{"type": "enum", …}` either way.

Every report lists `replay_mismatches` (must be empty). The candidates files are script flags, never verdicts: Opus reads each critical one by eye before a verdict is recorded.

## Recording a verdict (no code change)

Each job's on/off in the app follows its verdict for the model (`crates/juggler-app/src/features/ai/jobs.rs`): the flags are on, and the `models` section's verdict decides. A passing round is recorded as data:

- `crates/juggler-app/src/features/ai/models/builtin.json` (Apple's built-in entry) and `config/remote/nightly.json`'s `models` section: the job's verdict from `not_yet` to `beta` (the user's Beta switch then offers it), with a one-line note of what was measured ("Matched the request in 93 % of fresh tests."; "Preferred over the plain line in 64 % of blind checks.").
- A job that stays plain keeps `not_yet` and its one-line why, which Settings › AI › Models shows.
- The decision log gets the verdict as a D-number (next after every branch's highest), and the AJ-E3830 / AJ-E3831 manifests their R3 lines.

## Files

`core6.py` (the run loop, sampled retries, the harness clock) · `watch2.py` · `pick.py` · `digest_mr.py` · `runner_v6.py` · `report_v6.py` · `selftest_v6.py` · `fixtures/v6-selftest.json` (made up) · `prompts/feed_pick.md`, `prompts/watch2.md` (generated: `selftest_v6.py --write-prompts`; the digest's only model task is the feed pick).
