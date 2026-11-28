# Held-out watch set v9 (harness round v9)

Written blind on 2026-11-18, with "today" = 2026-11-18. The writer did not open any harness, results, report, review or verdict file, v6 to v8 held-out sets, or the app's feature source. Shape came from `v5r3/watch-v5r3.json` and the condition table in ADR-0036 section 2.2. Every item is new wording.

File: `watch-v9.json`, 60 requests (45 English, 15 German), same keys as v5r3.

Counts:
- 14 meaning-changers (exclusions, unsupported, each with its nearest option): 10 English, 4 German. Tag `exclusion`.
- 8 numeric thresholds in units no condition takes (tokens, cost, files, lines, commits per hour): tag `numeric`.
- 6 other unsupported requests (deploy, clock reminder, act-then-do, mentions, pull request, another Mac): tag `other`.
- 8 named-entity-is-the-target despite a broad word nearby: tag `broad_word`, supported on that entity.
- 24 other supported, covering finishes, fails, asks, stuck, idle_for, hits_limit, runway_below, starts, ends, one combined, five with quiet-hours words.

Lifetimes: `once` (38), `until:2026-11-18T23:59:00Z` (15, also the default when unsaid for non-session-finish watches), `until_removed` (7). Unsupported items carry `once`, as in v5r3.

Three contexts (two English, one German) list sessions, apps, projects and accounts.
