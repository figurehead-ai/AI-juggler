# Held-out watch set v12 (AJ-E3830)

60 new watch requests (R12-W01..R12-W60, shuffled, seed 12), 30 English and 30 German, written blind by a Sonnet QA session.
Sources read: v5r3's JSON shape (two items) and ADR-0036 section 2.2 only. All session, project, App and account names are made up.
Gold: condition type names from ADR-0036's table in snake case; "today" is 2026-11-26, so a request with no lifetime gets until:2026-11-26T23:59:00Z; even_in_quiet_hours true only when the request asks for it.

| Category (tag) | EN | DE |
|---|---|---|
| lifetime | 5 | 5 |
| entity | 4 | 4 |
| negation | 7 | 7 |
| count_threshold | 4 | 4 |
| plain | 7 | 7 |
| quiet_hours | 3 | 3 |

Each item's tags hold the category and the language (en/de). Negation and count_threshold items are unsupported with the nearest condition; the rest are supported.
