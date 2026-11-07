# AJ-E3204 — The App page

Build: release/0.4.1 at 82e794c59, 2026-11-06. Captures: both themes, 1024 and 1280, checked by eye.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | The header: tool chip (a CLI App), "runs in a terminal" or the desktop App's state, the Account door, "room shared with n Apps" (the others named on hover), and **This Mac**, a door to This Mac (105 ms). Go to it brings a desktop App forward (walk A1); a CLI App's main button is New session. `header-chips/app1-*`; vitest `app-page.test.tsx` ("its header names the Mac…"). Shared is matched by the Account each App resolves to, so a fresh App counts too (it was by stored id only, empty until each App was read) |
| R2 | PASS | Tabs with counts: Overview, Sessions, Projects, Pools, Launch settings (a CLI App, after Pools), Usage, Processes, Settings. The Launch settings tab lists the App's own sets with Launch and delete; it opens in 39 ms. `launch-settings/launch-tab-*`; vitest "a CLI App has its launch settings as a tab of its own; a desktop App doesn't" |
| R3 | PASS | Settings holds the name, **Show in the rail**, a CLI App's **Launch settings** (opens the tab, 39 ms), Start and keep alive (a desktop App), plan, renewal and fee (its Account's), tidy-up and the window preview (a CLI App's folder). Show in the rail off (28–39 ms): the App leaves Open now, Inactive apps and the folded rail's icons; its sessions that need you still show and a search still finds it; the Also line says "Work Max off the rail (1 open)" and opens its Settings, where it's turned back on. `rail-shown/settings-on-*`, `settings-off-*`; vitest `rail-shown.test.ts` (6) |
| Budget | PASS | Opens ≤ 150 ms from cache: 140 ms click to page on the lab (walk A1, step 1); 115 ms from a session's App chip (walk A3) |
| A1 | PASS | `docs/qa/0.4-walks/A1.md`: only its own sessions; two findings fixed and re-walked |
| A3 | PASS | `docs/qa/0.4-walks/A3.md`: the session's App chip opens the App page in one click (fixed: it opened a menu) |
| A4 | PASS | `docs/qa/0.4-walks/A4.md`: Projects lists what the App worked on, its count the list's length |
| Critique | PASS | Gallery 99-app-page critiqued; this round's additions critiqued by eye: the Also line's link shortened to the profile name (it wrapped in two), the sections renamed "Where it shows" and "Starting it" (their titles repeated the rows') |
| L3 | OPEN | The pages' pairwise spec (with AJ-E3205) |
| Persona | OPEN | AJ-E3307, on the release candidate |

Preview note: the browser preview knows no desktop places, so its desktop App opens the CLI App's page and "room shared" doesn't show there; the real backend has them (walk A1) and the unit test covers it.
