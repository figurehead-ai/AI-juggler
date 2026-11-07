# AJ-E3205 — The Account page

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | Limits first, then the week; plan, renewal and fee on Settings; named, never an id: walks A2, A5, A8 (`docs/qa/0.4-walks/`) |
| R2 | PASS | Tabs Overview, Apps, Sessions, Projects, Pools, Usage, Settings with counts equal to the reference (walk A2) |
| R3 | PASS | Opened from the Accounts card (A2), Home's runway (A5), a session's Where it runs (A3); a limit notice's banner on it (A5) |
| R4 | PASS | No tool letter on the cards (captures `walks-0.4/A2/0-accounts-work-card-*`) |
| R5 | PASS | At phone width (390×844, both themes, 2026-11-06, release/0.4.1; `folded-390x844-dark.png`, `-light.png` here) the folded rail shows each session's app icon (`folded-tool-mark` with the app's own icon, no letter); at 1024 in the walks' captures. A Claude Code session and a Codex one told apart by their apps, not a letter: `ui/src/components/rail-folded.test.tsx` "tells a Claude Code session from a Codex one by their apps, not a letter" (the design preview has no Codex session to show it on screen) |
| L3 | PASS | `ui/tests/visual/l3-pages.spec.ts`, one journey per Account row of `docs/qa/pairwise/pages.csv` (rows 22–28 and 51: each tab, sizes 0 to 1 005, filter, search, 3 new arriving, each first state, both themes and widths, as the all-pairs table gives them), reached from the App page's Account door: the remembered tab, its promise (its Apps listed by name), the Sessions tab's count the page's own, the list ordered by state, searchable and filterable past 8 rows with the count line the rows', new rows held behind "3 new". 8 of 8 pass on 2026-11-06 (d4ebc327c); on 2026-11-07 with the other pages' rows (70f7ba35f), 38 rows, 64 clicks, feedback p95 14 ms, max 22 ms (`docs/qa/evidence/l3/pages.json`) |
| Oracle | PASS | Usage equals the transcripts' own on a replica of 9 setups and 340 conversations: tokens per account in 5 h and 7 d 18 of 18 (×1.00), 340 of 340 sessions' cost rows, Codex all time ×1.00 (`usage-oracle-2026-11-06.md`). 42 `gpt-5-codex` rows have no dollar price (not in the bundled table): a finding for the usage area |
| Persona | OPEN | AJ-E3307, on the release candidate |
