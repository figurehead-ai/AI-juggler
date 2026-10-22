# Evidence manifest — AJ-E1528

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1528, A This Mac page (P1).
Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR).

Gallery: `docs/qa/evidence/_gallery/81-this-mac/` (six Claude app windows
and a heavy session, both themes, both densities). Visual baselines:
`this-mac-*` in `ui/tests/visual/__baselines__/` (both themes, both
densities, two sizes).

## What exists now

1. **A This Mac screen in the shell** (`screens/ThisMacScreen.tsx`, the
   `thismac` id in `lib/nav.ts`). It opens from four places:
   - Home's "See what's using it" (Needs you);
   - the This Mac panel's "See all";
   - More › This Mac;
   - the rail's "This Mac · …" line (and its dot when the rail is narrow).
2. **From the existing read model, with nothing new measured**
   (`ThisMacContext`, the 5 s `this_mac` the panel already reads):
   - **Now:** the Mac's state in words with every reason, the memory meter,
     CPU of all cores and swap.
   - **What's using it:** each tool (`by_tool`), with its sessions or
     windows behind a disclosure (open for AI Juggler's, folded for the
     tools it didn't start, marked "not started by AI Juggler").
   - **Working your Mac hard:** each heavy tree, AI Juggler's linking to
     its session.
   - **Alerts and advice:** recent alerts, each with its subject in
     `alertSubject`'s words (the one function), and the recommendations.
   - **Left behind:** leftover processes and ports, with Clean up.
   Controls only for what AI Juggler owns (D-08): a window of a tool it
   didn't start has none.
3. **The same rows, meters and type as Home's panel**: they moved into
   `components/this-mac-parts.tsx` (`MachineState`, `MemoryMeter`,
   `TreeRow`), which both draw. Detail sits behind each tool's disclosure
   (D-25). Home's panel stays a summary and links to the page.
4. **The alert subject** comes from `lib/machine.ts`'s `alertSubject`,
   used by Home and the page alike.
5. **The backend decides each tree's group** (`ToolTreeInfo.group`,
   from the same `group_of` that builds `by_tool`): the page lists a
   group's trees by it, and names no tool itself (the tool-literal lint).
6. **The window itself never scrolls.** Found while checking these
   captures: screen-reader words in Home's scroll area had no positioned
   box around them, so they hung below the window (Home measured 1187 px
   in a 768 px window) and the whole app could slide up when a low
   button took focus. The shell and every scroll area are positioned
   now; Home measures 768 px.

## Proof

| Requirement | Evidence | Result |
|---|---|---|
| Req.1 a screen in the shell, opened from See what's using it, the panel, More | Playwright `this-mac-page` (the panel's See all, More, the rail's line); vitest `this-mac` (See what's using it opens the page) | PASS |
| Req.2 state, by tool then session or window, the unowned marked, heavy with links, alerts and advice; controls only for what AI Juggler owns | vitest `this-mac-page` (5): placeholder before the first sample; state and meters; groups and their rows, the unowned with no controls; heavy links and Clean up; alerts in the one subject's words | PASS |
| Req.3 the same rows and meters, detail behind disclosure, Home stays a summary | the shared parts; Playwright: six windows behind the folded group, none with a control; vitest `this-mac` unchanged (21) | PASS |
| Req.4 one alert-subject function | the page calls `alertSubject` (vitest) | PASS |
| Quality gates: a11y, baselines both themes and densities | axe 0 critical/serious in `this-mac-page` (4 variants) and the a11y gate; `this-mac` baselines | PASS |
| The window never scrolls (found here) | Playwright `this-mac-page`: on Home and on the page at 1024×768, the document is no taller than the window; the flows captures (`37-flows`, `38-flows-cde`) are window-sized again | PASS |
| An oracle comparison of the process groups on the replica | `just replica-oracle` with `replica live` running and 2 GB of extra memory pressure ([oracle-replica-this-mac.txt](oracle-replica-this-mac.txt)): the Claude app group has 6 windows, as `ps` has 6 Claude app trees; every group's rows match its count (6 and 6); memory ×1.00; the top three trees match | PASS |

The oracle's "tools 12 vs 6" and "every tree" lines predate this task.
The oracle counts only the Claude app's instances, while This Mac also
counts `claude` running in a terminal (6 here). They're recorded as they
are.

## Walked

On the preview at 1280×800, both themes and densities, with the
preview's six Claude app windows and a heavy session (`?aij_windows=6`,
`?aij_load=heavy`). From Home's panel, from More and from the rail's
line, each opened the page; More is lit while it's open.

1. **Does it solve the real problem?** "See what's using it" opens a page
   that answers it, every window counted, without scrolling Home.
2. **Is it clear without explanation?** Each tool says how many sessions
   or windows it has, and whether AI Juggler started them. The page says
   nothing on it is sent anywhere.
3. **Is it consistent with its siblings?** The same rows and meters as
   Home's panel, the rail's fold chevron, and the Usage page's sections.

The hand walk on the real setup under memory pressure (read-only,
through the Dev bridge) goes to QA's next walk: running the bridge here
would bind port 5173, which other sessions' verify uses.
