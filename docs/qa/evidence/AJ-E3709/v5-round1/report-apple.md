# Harness v5 report: Apple on-device Foundation Model via apple_probe.swift (apple, sampling greedy, datamark on)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## chat / chat

Items 40; decided by code 26; model called 14: first attempt 8, after a retry 5, fallback 1.
Shown model items 8; critical reaching the user 4 ({'wrong_attribute': 3, 'gold_forbidden': 1, 'answered_unanswerable': 2}); critical caught on 6 items ({'anchoring': 3, 'wrong_value': 4, 'wrong_question': 4}); omissions 16 ({'no_answer_on_answerable': 14, 'detail_dropped': 2}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 50.0 | NO |
| systematic critical patterns | == 0 | 1 | NO |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 40.0 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / attention

Items 40; decided by code 14; model called 26: first attempt 26, after a retry 0, fallback 0.
Shown model items 1; critical reaching the user 0 ({}); critical caught on 0 items ({}); omissions 0 ({'attention_miss': 6}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / feed_line

Items 40; decided by code 14; model called 26: first attempt 5, after a retry 0, fallback 21.
Shown model items 5; critical reaching the user 0 ({}); critical caught on 3 items ({'anchoring': 6, 'outcome_or_caveat': 6}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / review

Items 40; decided by code 0; model called 40: first attempt 17, after a retry 2, fallback 21.
Shown model items 19; critical reaching the user 5 ({'gold_forbidden': 4, 'success_on_failure': 1}); critical caught on 7 items ({'anchoring': 17, 'contradiction': 8}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 26.32 | NO |
| systematic critical patterns | == 0 | 1 | NO |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

