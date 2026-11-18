# Harness v5 report: local model (llama.cpp, Metal) via local_probe (nemotron, sampling low, datamark off)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## chat / chat

Items 40; decided by code 26; model called 14: first attempt 8, after a retry 3, fallback 3.
Shown model items 5; critical reaching the user 3 ({'wrong_attribute': 3, 'gold_forbidden': 1, 'answered_unanswerable': 1}); critical caught on 6 items ({'wrong_question': 13}); omissions 17 ({'no_answer_on_answerable': 16, 'detail_dropped': 1}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 60.0 | NO |
| systematic critical patterns | == 0 | 1 | NO |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 42.5 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / attention

Items 40; decided by code 14; model called 26: first attempt 26, after a retry 0, fallback 0.
Shown model items 1; critical reaching the user 0 ({}); critical caught on 0 items ({}); omissions 1 ({'attention_miss': 7, 'question_or_failure_hidden': 13, 'emphasis': 1}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 100.0 | NO |
| questions or failures hidden | == 0 | 13 | NO |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / feed_line

Items 40; decided by code 14; model called 26: first attempt 15, after a retry 0, fallback 11.
Shown model items 15; critical reaching the user 0 ({}); critical caught on 10 items ({'outcome_or_caveat': 14, 'anchoring': 10, 'contradiction': 12}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / review

Items 40; decided by code 0; model called 40: first attempt 19, after a retry 6, fallback 15.
Shown model items 24; critical reaching the user 4 ({'polarity_script': 4, 'gold_forbidden': 2}); critical caught on 21 items ({'contradiction': 45, 'anchoring': 49, 'invented': 5}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 16.67 | NO |
| systematic critical patterns | == 0 | 1 | NO |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

