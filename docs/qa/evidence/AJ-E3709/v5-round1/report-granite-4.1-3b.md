# Harness v5 report: local model (llama.cpp, Metal) via local_probe (granite-4.1-3b, sampling greedy, datamark on)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## chat / chat

Items 40; decided by code 26; model called 14: first attempt 7, after a retry 0, fallback 3.
Shown model items 3; critical reaching the user 0 ({}); critical caught on 6 items ({'wrong_value': 18, 'invented': 2, 'wrong_question': 9}); omissions 18 ({'no_answer_on_answerable': 17, 'detail_dropped': 1}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 45.0 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / attention

Items 40; decided by code 14; model called 26: first attempt 26, after a retry 0, fallback 0.
Shown model items 5; critical reaching the user 0 ({}); critical caught on 0 items ({}); omissions 12 ({'attention_miss': 7, 'emphasis': 5}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 30.0 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |
| attention misses, % of items that need you (facts given) | <= 10.0 | 35.0 | NO |
| attention false alarms, % of calm items (facts given) | <= 10.0 | 30.0 | NO |

- facts_given: miss 7/20 (35.0 %), false alarms 6/20 (30.0 %; model 5, facts 1)
- facts_hidden: not in this group (v5 decides attention from the facts; v4's re-score reports facts hidden as its own group)

## turns / feed_line

Items 40; decided by code 14; model called 26: first attempt 8, after a retry 0, fallback 18.
Shown model items 8; critical reaching the user 0 ({}); critical caught on 15 items ({'contradiction': 9, 'anchoring': 29, 'outcome_or_caveat': 10, 'invented': 4}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / review

Items 40; decided by code 0; model called 40: first attempt 14, after a retry 2, fallback 24.
Shown model items 16; critical reaching the user 0 ({}); critical caught on 21 items ({'contradiction': 36, 'anchoring': 41, 'invented': 1}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

Replay mismatches: 15 (listed in the JSON).

