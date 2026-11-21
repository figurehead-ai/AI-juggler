# Harness v5 report: local model (llama.cpp, Metal) via local_probe (granite-4.1-3b, sampling greedy, datamark on)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## chat / chat

Items 51; decided by code 26; model called 25: first attempt 16, after a retry 0, fallback 9.
Shown model items 11; critical reaching the user 1 ({'answered_unanswerable': 1}); critical caught on 9 items ({'wrong_question': 22, 'wrong_value': 3, 'anchoring': 6}); omissions 23 ({'detail_dropped': 2, 'no_answer_on_answerable': 21}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 9.09 | NO |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 45.1 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / attention

Items 50; decided by code 32; model called 18: first attempt 16, after a retry 0, fallback 2.
Shown model items 1; critical reaching the user 0 ({}); critical caught on 0 items ({}); omissions 1 ({'emphasis': 1}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 2.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |
| attention misses, % of items that need you (facts given) | <= 10.0 | 0.0 | yes |
| attention false alarms, % of calm items (facts given) | <= 10.0 | 5.56 | yes |

- facts_given: miss 0/32 (0.0 %), false alarms 1/18 (5.56 %; model 1, facts 0)
- facts_hidden: not in this group (v5 decides attention from the facts; v4's re-score reports facts hidden as its own group)

## turns / feed_line

Items 50; decided by code 35; model called 15: first attempt 8, after a retry 3, fallback 4.
Shown model items 11; critical reaching the user 0 ({}); critical caught on 6 items ({'anchoring': 10, 'contradiction': 8}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / review

Items 50; decided by code 0; model called 50: first attempt 24, after a retry 5, fallback 21.
Shown model items 26; critical reaching the user 0 ({}); critical caught on 23 items ({'anchoring': 66, 'contradiction': 35, 'invented': 1}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

