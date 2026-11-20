# Evidence manifest — AJ-E815

The relation counts' speed test measures the work, not the clock. Run on release/0.4.6 (2026-11-15); the commit is the one that adds this file.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 cached counts proven cached by work done | `cargo nextest run -p juggler-app --test relations ten_thousand_sessions_count_from_the_cache` | `builds()` unchanged across 540 counts and one `counts_many` ask; a change rebuilds one part | passed; `builds()` stayed at 2, then 3 after the change (a part's build is the only reader of the session rows, `features/relations/mod.rs::current`) | PASS | this commit | 2026-11-15 | Opus |
| R1 fails when the cache is bypassed | scratch: `let on = false && …` in `relations::current`, the same test, then reverted | the test fails | FAILED: "540 counts built nothing again", left 1082, right 2 | PASS | this commit | 2026-11-15 | Opus |
| R2 the cold rebuild keeps a generous bound only | the same test | cold rebuild < 10 s (unloaded about 0.2 s, so about 50x) | passed; the cold build checked by work too (exactly the two parts built) | PASS | this commit | 2026-11-15 | Opus |
| R3 timings in a benchmark that is not a gate | `cargo nextest run -p juggler-app --test relations --run-ignored only relation_counts_timings --no-capture` | `#[ignore]`d, prints `[perf]` lines beside the 1 s / 5 ms budgets, asserts no time | cold 197 ms (budget 1 s), slowest of 540 cached 80 µs (budget 5 ms), one ask 214 µs (budget 5 ms) | PASS | this commit | 2026-11-15 | Opus |
| R4 every other gate test under 50 ms listed and handled | `.config/nextest.toml` "Wall-clock budgets" list, and a grep of its binaries for elapsed-time bars | each changed or kept with its reason | `juggler-agentbridge::bridge calls_answer_in_under_50_ms`: the slowest of 20 calls < 50 ms became the median < 50 ms plus the slowest < 500 ms (the same max-of-many shape that failed). `load_budgets` terminal echo, 50 ms: kept, as it already gates the median of a round of 20, best of 3 rounds. Every other listed bar is 200 ms or more | PASS | this commit | 2026-11-15 | Opus |
| Gate: passes under load | both tests with 8 busy cores (`perl -e "alarm 150; 1 while 1"` ×8, trapped) on a shared 30-core Mac, load average 19 | pass | both passed: bridge median 3.5 ms, slowest 6.2 ms; workers left afterwards: 0 | PASS | this commit | 2026-11-15 | Opus |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content
