# Harness v5 report: Apple on-device Foundation Model via apple_probe.swift (apple, sampling greedy, datamark on)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## chat / chat

Items 50; decided by code 14; model called 36: first attempt 26, after a retry 4, fallback 6.
Shown model items 21; critical reaching the user 1 ({'gold_forbidden': 1}); critical caught on 10 items ({'wrong_value': 3, 'wrong_question': 7, 'anchoring': 12}); omissions 21 ({'detail_dropped': 5, 'no_answer_on_answerable': 16}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 4.76 | NO |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 42.0 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / attention

Items 50; decided by code 19; model called 31: first attempt 28, after a retry 0, fallback 3.
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

Items 50; decided by code 23; model called 27: first attempt 9, after a retry 1, fallback 17.
Shown model items 10; critical reaching the user 1 ({'gold_forbidden': 1}); critical caught on 8 items ({'contradiction': 8, 'outcome_or_caveat': 11, 'anchoring': 7, 'invented': 3}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 10.0 | NO |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / review

Items 50; decided by code 0; model called 50: first attempt 33, after a retry 0, fallback 17.
Shown model items 30; critical reaching the user 1 ({'gold_forbidden': 1}); critical caught on 12 items ({'contradiction': 19, 'anchoring': 31}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 3.33 | NO |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

