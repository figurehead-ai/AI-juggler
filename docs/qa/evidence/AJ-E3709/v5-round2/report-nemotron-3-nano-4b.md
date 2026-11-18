# Harness v5 report: local model (llama.cpp, Metal) via local_probe (nemotron, sampling low, datamark off)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## chat / chat

Items 51; decided by code 26; model called 25: first attempt 15, after a retry 3, fallback 7.
Shown model items 11; critical reaching the user 0 ({}); critical caught on 10 items ({'wrong_question': 31}); omissions 22 ({'detail_dropped': 2, 'no_answer_on_answerable': 20}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 43.14 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / attention

Items 50; decided by code 32; model called 18: first attempt 18, after a retry 0, fallback 0.
Shown model items 0; critical reaching the user 0 ({}); critical caught on 0 items ({}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |
| attention misses, % of items that need you (facts given) | <= 10.0 | 0.0 | yes |
| attention false alarms, % of calm items (facts given) | <= 10.0 | 0.0 | yes |

- facts_given: miss 0/32 (0.0 %), false alarms 0/18 (0.0 %; model 0, facts 0)
- facts_hidden: not in this group (v5 decides attention from the facts; v4's re-score reports facts hidden as its own group)

## turns / feed_line

Items 50; decided by code 35; model called 15: first attempt 9, after a retry 4, fallback 2.
Shown model items 13; critical reaching the user 1 ({'gold_forbidden': 1}); critical caught on 4 items ({'anchoring': 8, 'contradiction': 3}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 7.69 | NO |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / review

Items 50; decided by code 0; model called 50: first attempt 20, after a retry 4, fallback 26.
Shown model items 24; critical reaching the user 1 ({'gold_forbidden': 1}); critical caught on 26 items ({'anchoring': 78, 'contradiction': 38}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 4.17 | NO |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

