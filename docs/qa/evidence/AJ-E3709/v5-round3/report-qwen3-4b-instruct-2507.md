# Harness v5 report: local model (llama.cpp, Metal) via local_probe (qwen3-4b-instruct-2507, sampling card, datamark on)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## chat / chat

Items 50; decided by code 14; model called 36: first attempt 26, after a retry 4, fallback 6.
Shown model items 18; critical reaching the user 1 ({'gold_forbidden': 1}); critical caught on 10 items ({'wrong_value': 3, 'wrong_question': 14, 'anchoring': 5}); omissions 21 ({'detail_dropped': 2, 'no_answer_on_answerable': 19}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 5.56 | NO |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 42.0 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / attention

Items 50; decided by code 19; model called 31: first attempt 30, after a retry 1, fallback 0.
Shown model items 0; critical reaching the user 0 ({}); critical caught on 0 items ({}); omissions 11 ({'attention_miss': 11}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 22.0 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |
| attention misses, % of items that need you (facts given) | <= 10.0 | 36.67 | NO |
| attention false alarms, % of calm items (facts given) | <= 10.0 | 0.0 | yes |

- facts_given: miss 11/30 (36.67 %), false alarms 0/20 (0.0 %; model 0, facts 0)
- facts_hidden: not in this group (v5 decides attention from the facts; v4's re-score reports facts hidden as its own group)

## turns / feed_line

Items 50; decided by code 23; model called 27: first attempt 8, after a retry 4, fallback 15.
Shown model items 12; critical reaching the user 1 ({'gold_forbidden': 1}); critical caught on 16 items ({'contradiction': 21, 'anchoring': 12, 'outcome_or_caveat': 18, 'invented': 2}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 8.33 | NO |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / review

Items 50; decided by code 0; model called 50: first attempt 23, after a retry 15, fallback 12.
Shown model items 37; critical reaching the user 0 ({}); critical caught on 15 items ({'contradiction': 26, 'invented': 2, 'anchoring': 5}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

