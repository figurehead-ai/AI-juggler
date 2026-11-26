# v6 round: Opus's blind re-judge of the feed-line and digest packets (2026-11-26)

The Sonnet round's value review (review-feed_pick.json, review-digest_mr.json) was re-judged blind by Opus, the keys unopened, each pair read for the busy developer glancing at the feed: which says what happened and what matters, accurately.

| Job | Sonnet (AI / plain / same) | Opus (AI / plain / same) | Pooled decided pairs | Bar (D-130) | Verdict |
|---|---|---|---|---|---|
| Feed line (`feed_pick`, 25 pairs) | 10 / 12 / 3 → 45.5 % | 5 / 17 / 3 → 22.7 % | 15 / 29 → 34 % | 60 % | FAIL: stays plain (`not_yet`) |
| Digest (`digest_mr`, 17 pairs) | 7 / 5 / 5 → 58.3 % | 11 / 5 / 1 → 68.8 % | 18 / 28 → 64.3 % | 60 % | PASS: `beta` (opt-in), D-149 |

Opus's pair calls (digest, pair → preferred side): 001 second, 002 first, 003 first, 004 first, 005 second, 006 second, 007 second, 008 first, 009 same, 010 first, 011 second, 012 first, 013 second, 014 first, 015 second, 016 second, 017 second. Feed: 001 A, 002 B, 003 same, 004 B, 005 A, 006 A, 007 A, 008 A, 009 same, 010 A, 011 B, 012 B, 013 A, 014 same, 015 A, 016 A, 017 B, 018 A, 019 B, 020 B, 021 A, 022 A, 023 B, 024 A, 025 B.

**Why the feed line misses:** the model's pick keeps choosing a file diff ("Changed Cargo.lock (+140 −128), 61 passed") or a side detail ("Deleted the old cron script…") over the turn's point; the plain excerpt already reads well.

**The digest's caveat:** the per-session lines win on compactness and counts, but for a turn whose change was reverted the model's line drops what was tried ("No code changes remain"; pairs 007, 010, 017). No safety row moved (0 criticals, 0 hidden questions or failures). Logged as a finding for the digest's next round.

**Watches** (`watch2`): 63.5 % exact against 90 %: FAIL; reported to the coordinating session before anything ships (D-149).
