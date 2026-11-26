# Held-out watch set v10 (AJ-E3830)

Written blind on 2026-11-18, without opening the harness, any results, report, review or verdict file, `heldout/v6` to `heldout/v9`, or the feature code. Only v5r3's shape and ADR-0036 section 2.2's condition table were read. Every request is new.

"Today" is 2026-11-18, so the default lifetime when none is said is `until:2026-11-18T23:59:00Z`.

Counts: 60 requests, 45 English and 15 German.

- 12 lifetimes said in words: 6 `until_removed`, 4 `once`, 2 today.
- 10 named entity is the target despite a broad word nearby (supported on that entity).
- 12 meaning-changers (exclusions, English and German): unsupported, with the nearest option.
- 6 numeric thresholds in units no condition takes: unsupported, with the nearest option.
- 20 plain supported requests across `finishes`, `fails`, `asks`, `stuck`, `idle_for`, `hits_limit`, `runway_below`, `starts`, `ends`.
- 6 carry quiet-hours words (`even_in_quiet_hours` true or false in the gold).

A minutes parameter is given only when the request states one. Shape matches v5r3 (`id`, `request`, `context`, `gold`, `tags`; `even_in_quiet_hours` only when said).
