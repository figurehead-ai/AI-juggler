# Harness v5 report: local model (llama.cpp, Metal) via local_probe (qwen3, sampling low, datamark off)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## chat / chat

Items 40; decided by code 26; model called 14: first attempt 4, after a retry 0, fallback 6.
Shown model items 3; critical reaching the user 0 ({}); critical caught on 7 items ({'wrong_value': 6, 'wrong_question': 15, 'anchoring': 3}); omissions 18 ({'no_answer_on_answerable': 17, 'detail_dropped': 1}).

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
Shown model items 1; critical reaching the user 0 ({}); critical caught on 0 items ({}); omissions 8 ({'attention_miss': 7, 'emphasis': 1}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 20.0 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |
| attention misses, % of items that need you (facts given) | <= 10.0 | 35.0 | NO |
| attention false alarms, % of calm items (facts given) | <= 10.0 | 10.0 | yes |

- facts_given: miss 7/20 (35.0 %), false alarms 2/20 (10.0 %; model 1, facts 1)
- facts_hidden: not in this group (v5 decides attention from the facts; v4's re-score reports facts hidden as its own group)

## turns / feed_line

Items 40; decided by code 14; model called 26: first attempt 10, after a retry 0, fallback 16.
Shown model items 10; critical reaching the user 0 ({}); critical caught on 15 items ({'contradiction': 17, 'anchoring': 28, 'invented': 3, 'outcome_or_caveat': 10}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / review

Items 40; decided by code 0; model called 40: first attempt 11, after a retry 0, fallback 29.
Shown model items 11; critical reaching the user 0 ({}); critical caught on 26 items ({'contradiction': 52, 'anchoring': 100, 'invented': 15}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

Replay mismatches: 12 (listed in the JSON).

