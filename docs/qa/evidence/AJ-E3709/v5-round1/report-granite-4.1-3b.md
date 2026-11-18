# Harness v5 report: local model (llama.cpp, Metal) via local_probe (granite-4.1-3b, sampling greedy, datamark on)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## chat / chat

Items 40; decided by code 26; model called 14: first attempt 8, after a retry 1, fallback 5.
Shown model items 4; critical reaching the user 1 ({'wrong_attribute': 1, 'gold_forbidden': 1}); critical caught on 6 items ({'wrong_value': 18, 'invented': 2, 'wrong_question': 9}); omissions 17 ({'no_answer_on_answerable': 16, 'detail_dropped': 1}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 25.0 | NO |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 42.5 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / attention

Items 40; decided by code 14; model called 26: first attempt 26, after a retry 0, fallback 0.
Shown model items 5; critical reaching the user 0 ({}); critical caught on 0 items ({}); omissions 5 ({'attention_miss': 7, 'emphasis': 5}).

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
Shown model items 11; critical reaching the user 0 ({}); critical caught on 12 items ({'contradiction': 3, 'anchoring': 29, 'outcome_or_caveat': 10, 'invented': 4}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / review

Items 40; decided by code 0; model called 40: first attempt 21, after a retry 3, fallback 16.
Shown model items 24; critical reaching the user 4 ({'polarity_script': 3, 'gold_forbidden': 2}); critical caught on 19 items ({'contradiction': 30, 'anchoring': 41, 'invented': 1}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 16.67 | NO |
| systematic critical patterns | == 0 | 1 | NO |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

