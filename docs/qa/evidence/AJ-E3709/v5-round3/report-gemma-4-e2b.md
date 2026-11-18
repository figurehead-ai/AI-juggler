# Harness v5 report: local model (llama.cpp, Metal) via local_probe (gemma-4-e2b, sampling low, datamark on)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## chat / chat

Items 50; decided by code 14; model called 36: first attempt 27, after a retry 2, fallback 7.
Shown model items 18; critical reaching the user 1 ({'gold_forbidden': 1}); critical caught on 9 items ({'wrong_value': 3, 'wrong_question': 24, 'anchoring': 3}); omissions 20 ({'detail_dropped': 1, 'no_answer_on_answerable': 19}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 5.56 | NO |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 40.0 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / attention

Items 50; decided by code 19; model called 31: first attempt 29, after a retry 0, fallback 2.
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

Items 50; decided by code 23; model called 27: first attempt 12, after a retry 3, fallback 12.
Shown model items 15; critical reaching the user 1 ({'gold_forbidden': 1}); critical caught on 11 items ({'outcome_or_caveat': 25, 'contradiction': 3, 'anchoring': 5}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 6.67 | NO |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / review

Items 50; decided by code 0; model called 50: first attempt 16, after a retry 9, fallback 25.
Shown model items 24; critical reaching the user 1 ({'gold_forbidden': 1}); critical caught on 24 items ({'contradiction': 51, 'invented': 9, 'anchoring': 20, 'injection': 3}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 4.17 | NO |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

