# AJ-E3206 — Every name opens its own page

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | Rail inactive apps (A1), Accounts cards (A2), a session's App chip in one click and Where it runs' Account (A3, fixed: the chip had opened a menu), runway names (A5), ⌘K (A8); `session-chips.test.tsx`; ⌘K: ↩ on an Account or an App opens its page (`ui/src/components/palette-evidence.test.tsx`, "↩ opens its page" and "Open its page and Bring it forward"); in pickers a name is a choice, not a door (the design review's line 12 is n/a there; K4 walked them) |
| R2 | PASS | One mark and colour per App across the rail, pages and cards in every walk's captures; `one-mark-per-app.test.tsx` |
| R3 | PASS | Each tool's row in Settings › Accounts › Tools on this computer has "Open its page" (the tool menu's own action), which opens that Tool's page (AJ-E3104): `ui/src/screens/settings/accounts-pane-tools.test.tsx`; walked in both themes at 1024 and 1280 (2026-11-06, release/0.4.1), the Tool page there in 99–114 ms (`tool-page/`) |
| K1 | PASS | Walk K1: one click per hop from Home to a pool canvas, counts equal the data (per-row counts fixed to the cache), Back keeps each page's tab (`docs/qa/0.4-walks/K1.md`) |
