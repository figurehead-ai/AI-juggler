# Harness v5 report: Apple on-device Foundation Model via apple_probe.swift (apple, sampling greedy, datamark on)

Criteria: D-130 as clarified 2026-11-18 (0.5 chat and summary jobs only). Critical = fabrication only; omissions tolerated to ~10 %. Script flags are candidates: Opus confirms every critical by eye.

## chat / chat

Items 51; decided by code 26; model called 25: first attempt 19, after a retry 2, fallback 4.
Shown model items 15; critical reaching the user 1 ({'answered_unanswerable': 1}); critical caught on 6 items ({'wrong_question': 14}); omissions 19 ({'detail_dropped': 2, 'no_answer_on_answerable': 17}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 6.67 | NO |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 37.25 | NO |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / attention

Items 50; decided by code 32; model called 18: first attempt 18, after a retry 0, fallback 0.
Shown model items 3; critical reaching the user 0 ({}); critical caught on 0 items ({}); omissions 3 ({'emphasis': 3}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 6.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |
| attention misses, % of items that need you (facts given) | <= 10.0 | 0.0 | yes |
| attention false alarms, % of calm items (facts given) | <= 10.0 | 16.67 | NO |

- facts_given: miss 0/32 (0.0 %), false alarms 3/18 (16.67 %; model 3, facts 0)
- facts_hidden: not in this group (v5 decides attention from the facts; v4's re-score reports facts hidden as its own group)

## turns / feed_line

Items 50; decided by code 35; model called 15: first attempt 7, after a retry 0, fallback 8.
Shown model items 7; critical reaching the user 0 ({}); critical caught on 6 items ({'anchoring': 18, 'contradiction': 3}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 0.0 | yes |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

## turns / review

Items 50; decided by code 0; model called 50: first attempt 8, after a retry 1, fallback 41.
Shown model items 9; critical reaching the user 1 ({'gold_forbidden': 1}); critical caught on 5 items ({'contradiction': 24, 'anchoring': 6, 'invented': 1}); omissions 0 ({}).

| Bar | Bar value | Measured | Holds |
|---|---|---|---|
| critical reaching the user, % of shown model items | <= 1.0 | 11.11 | NO |
| systematic critical patterns | == 0 | 0 | yes |
| omissions, % of shown items (Chat: of all questions) | <= 10.0 | 0.0 | yes |
| questions or failures hidden | == 0 | 0 | yes |
| model text on a turn that needs you (feed) | == 0 | 0 | yes |
| value: AI preferred, % of decided pairs | >= 60.0 | None | n/a |

