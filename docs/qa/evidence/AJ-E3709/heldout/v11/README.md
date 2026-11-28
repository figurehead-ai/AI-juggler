# watch-v11 held-out set (AJ-E3830)

60 watch requests (ids R11-W01 .. R11-W60, shuffled), 40 English and 20 German, with gold answers.
Written blind: only the JSON shape of the v5r3 set and ADR-0036 section 2.2's condition table were read. All names are made up; three small worlds of six sessions each (sessions, Apps, projects, accounts) supply the context.

Gold: supported items carry target, conditions (type names from section 2.2 in snake_case, a param only when the request states one), lifetime (default until:2026-11-18T23:59:00Z; once; until_removed) and even_in_quiet_hours only when asked. Unsupported items carry only the nearest condition type.

Counts per category (English / German):
- lifetime in words: 10 (7 / 3)
- named entity despite a broad word: 8 (6 / 2)
- meaning-changers (exclusions, unless): 14 (9 / 5), all unsupported
- counts or thresholds: 8 (5 / 3), all unsupported
- plain supported: 14 (10 / 4)
- quiet-hours words: 6 (3 / 3); 5 set even_in_quiet_hours, 1 asks to hold until quiet hours end (not set)
