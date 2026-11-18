# Harness v5 report: local model (llama.cpp, Metal) via local_probe (qwen3-4b-instruct-2507, sampling card, datamark on)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## chat / chat

Items 40; decided by code 26; model called 14: first attempt 8, after a retry 3, fallback 3.
Shown model items 6; critical reaching the user 2 ({'wrong_attribute': 2, 'gold_forbidden': 1}); critical caught on 6 items ({'wrong_value': 13, 'wrong_question': 8}); omissions 15 ({'no_answer_on_answerable': 14, 'detail_dropped': 1}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 33.33 | NO |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 37.5 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / attention

Items 40; decided by code 14; model called 26: first attempt 26, after a retry 0, fallback 0.
Shown model items 0; critical reaching the user 0 ({}); critical caught on 0 items ({}); omissions 0 ({'attention_miss': 7}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / feed_line

Items 40; decided by code 14; model called 26: first attempt 8, after a retry 1, fallback 17.
Shown model items 9; critical reaching the user 1 ({'polarity_script': 1}); critical caught on 15 items ({'contradiction': 12, 'anchoring': 24, 'invented': 9, 'outcome_or_caveat': 16}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 11.11 | NO |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / review

Items 40; decided by code 0; model called 40: first attempt 24, after a retry 11, fallback 5.
Shown model items 31; critical reaching the user 4 ({'polarity_script': 3, 'gold_forbidden': 4}); critical caught on 16 items ({'anchoring': 20, 'contradiction': 10, 'invented': 7}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 12.9 | NO |
| systematic critical patterns | == 0 | 2 | NO |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

