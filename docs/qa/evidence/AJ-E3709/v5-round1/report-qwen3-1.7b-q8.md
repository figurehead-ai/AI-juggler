# Harness v5 report: local model (llama.cpp, Metal) via local_probe (qwen3, sampling low, datamark off)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## chat / chat

Items 40; decided by code 26; model called 14: first attempt 7, after a retry 0, fallback 7.
Shown model items 6; critical reaching the user 3 ({'wrong_attribute': 2, 'gold_forbidden': 1, 'answered_unanswerable': 1}); critical caught on 7 items ({'wrong_value': 6, 'wrong_question': 15, 'anchoring': 3}); omissions 16 ({'no_answer_on_answerable': 15, 'detail_dropped': 1}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 50.0 | NO |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 40.0 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / attention

Items 40; decided by code 14; model called 26: first attempt 26, after a retry 0, fallback 0.
Shown model items 1; critical reaching the user 0 ({}); critical caught on 0 items ({}); omissions 1 ({'attention_miss': 7, 'emphasis': 1}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 100.0 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / feed_line

Items 40; decided by code 14; model called 26: first attempt 11, after a retry 0, fallback 15.
Shown model items 11; critical reaching the user 0 ({}); critical caught on 14 items ({'contradiction': 12, 'anchoring': 28, 'invented': 6, 'outcome_or_caveat': 10}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / review

Items 40; decided by code 0; model called 40: first attempt 16, after a retry 0, fallback 24.
Shown model items 16; critical reaching the user 2 ({'gold_forbidden': 1, 'polarity_script': 1}); critical caught on 23 items ({'contradiction': 30, 'anchoring': 100, 'invented': 15}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 12.5 | NO |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

