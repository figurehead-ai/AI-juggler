<!-- captures: 2e3a771945b8 -->
# 99-related-list: the one related list and the one session row (0.4, E31, AJ-E3103)

Design critique (D-57, AJ-E2814). First critique of this family, written from its captures (`ui/tests/visual/related-list-gallery.spec.ts`).

## 0.4

The fixture is one CLI App, Claude Code · Work Max, with eleven sessions: one that asks, two working, one idle, four that ended this week and three older ones. A second App, Side project, has none. Each screen is opened from the rail's App header, as a user would open it.

**app-sessions-long (1280 and 1024, both themes).** The Sessions tab reads "Sessions 11", and the tab's count is at full strength now (see Fixed). A search field fills the row, with two chips at its end: "State: All" and "Project: All". There is no App chip (this is the App's own page) and no Account chip (the App's folder is on one Account). The line "11 sessions · 4 open" heads one card with five section headings in order: Needs you, Working, Open, Last 7 days, Earlier.

Each row has the same anatomy:
- the Claude Code icon, with the state's dot at its corner;
- the title on line 1;
- on line 2, the project's mark and name, then when;
- the state word in its colour at the end of line 2: "Asks you" in amber, "Working" in the live colour, "Idle" and "Ended" muted.

This page is the App's, so the rows leave out the App's name. At 1024 the rail folds to icons and the rows keep both lines; nothing is cut except long titles.

**filter-menu (both themes).** The Project chip's menu is the shared Picker. It lists All ✓, then "ai-juggler · 5", "No project · 3" and "payments-api · 3": every value the list holds, each with how many rows it shows, the most rows first.

**filtered (both themes).** "State: Open now" and "Project: ai-juggler · 2" are lit, with "Clear filters" beside them. The heading reads "2 sessions · 2 open" over the two working rows. There are no section headings, because only one section remains.

**no-match (both themes).** A search for "kubernetes" leaves one card: "Nothing here matches “kubernetes”." and a "Clear the search" link. The Project chip goes away, since there's nothing left to narrow.

**app-empty (both themes).** Claude Code · Side project has no session yet. One card says "No sessions in Claude Code · Side project yet", then "Sessions you start on this folder, here or in a terminal, show up here, what needs you first.", then one primary button, New session.

**home-recent (both themes).** Home's "Pick up where you left off" draws its rows with the same row as the list:
- the icon with a grey dot;
- the title;
- "payments-api · 5 h ago" on line 2, with "Ended" at its end;
- "Mark reviewed" at the end of the title line.

In this narrow column the App's words fold away (container query); the icon still says which app it was.

## Fixed while critiquing

- Axe found that the chosen tab's count failed contrast: "11" at 70 % on the accent tint. The chosen tab's count is at full strength now; other tabs keep theirs dimmer than their name (`pages/PageShell.tsx`). This changes, by a few pixels, every page capture whose chosen tab has a count.
- In the rail, a row with no project read "1 …" for "1 h ago": the time was capped against a project that wasn't there. It reads "1 h ago" now (`row/RowParts.tsx`).
- Home's rows had "Mark reviewed" in a third column, which cut the project to "pa…". It now ends the title line, so the place line has the row's width.
- "Mark reviewed" was 20 px tall (axe target-size). It is 24 px now.
- The project filter's "No project" shared All's value: React warned about duplicate keys, and choosing it showed everything. It has a value of its own now.
- Outside the gallery, the project page's Sessions tab counted only AI Juggler's open sessions in the design preview ("Sessions 3" over a list of 27). It counts what its list shows now (27).

## Weak

- At 1280 a row's state word sits about 850 px from its title, at the end of line 2. Read down, the states form a column; read across, the eye travels far. Columns for project and App at wide widths would fill the gap, but would give the row a second anatomy. Left for the maintainer's review.
- A search that leaves fewer than two values takes that filter's chip away (no-match), so chips appear and disappear as you type.
- On Home every row says "Ended". It is true, and it is the row's anatomy, but in a list that is all ended it repeats.
- The fixture's projects have no marks, so every row shows the neutral folder tile. Real projects show their own marks.
- "↑ 3 new" and "Show in order" are not captured: the design preview has no backend to say sessions changed (not-captured.md).
