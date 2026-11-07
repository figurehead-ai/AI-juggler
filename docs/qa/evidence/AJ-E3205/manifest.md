# AJ-E3205 — The Account page

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | Limits first, then the week; plan, renewal and fee on Settings; named, never an id: walks A2, A5, A8 (`docs/qa/0.4-walks/`) |
| R2 | PASS | Tabs Overview, Apps, Sessions, Projects, Pools, Usage, Settings with counts equal to the reference (walk A2) |
| R3 | PASS | Opened from the Accounts card (A2), Home's runway (A5), a session's Where it runs (A3); a limit notice's banner on it (A5) |
| R4 | PASS | No tool letter on the cards (captures `walks-0.4/A2/0-accounts-work-card-*`) |
| R5 | PASS | At phone width (390×844, both themes, 2026-11-06, release/0.4.1; `folded-390x844-dark.png`, `-light.png` here) the folded rail shows each session's app icon (`folded-tool-mark` with the app's own icon, no letter); at 1024 in the walks' captures. A Claude Code session and a Codex one told apart by their apps, not a letter: `ui/src/components/rail-folded.test.tsx` "tells a Claude Code session from a Codex one by their apps, not a letter" (the design preview has no Codex session to show it on screen) |
| L3 | GAP | The L3 pairs and the usage oracle aren't run yet |
