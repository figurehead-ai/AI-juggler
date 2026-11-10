<!-- captures: 6eb523dd7c92 -->
# 88-subagents: a session's Subagents tab and a subagent's page

Design critique (D-57, AJ-E2814) against the reference screens: the row menus (77-menus), the rail's order menu, Usage and Processes. The baseline note says what changed on purpose; this says whether it is good.

## Checklist

| Check | Finding |
|---|---|
| Grid | The tab is a table: Subagent · State · Time · Tokens · Model, groups as header rows on the same columns. |
| Hierarchy | The summary line first ("10 subagents · 3 running · 1 failed · 38 % of this session's tokens"), then the views, then the groups. |
| Spacing | Rows on the table's rhythm; the search stretches so the filter bar has no gap at 1280. |
| Icons | The state pills carry their marks; the subagent page's cards use the app's card titles. |
| Shared controls | Running · Needs attention · All is the shared Segmented; Type and Sort are the shared Picker. |
| Disabled reasons | Type: All types is disabled with its reason when every subagent is one type. |
| Plain words | "You asked: …" heads a group; "Its prompt isn't recorded by Claude Code". |
| Type sizes | Rows 14 px, kinds and times 12–13 px. |
| Both themes | Checked dark and light. |
| 1280×800 and 1024×700 | At 1024 the page grows long; the detail's right column ends early at 1280. |

## Good

- Summary line first; Running · Needs attention · All; grouped under the message that started them; tree lines only for who started whom.
- A subagent's page has its task, result, work, usage and processes as cards, and says honestly what the tool doesn't record.
- The 437-subagent session opens grouped and windowed.

## Weak

- Type: All types shows even when every subagent is one type.
- Detail cards at 1280 put Usage and Processes in a right column that ends long before the left.
- tab-*-1280x800, detail-*-1280x800, row-count-1280x800 (both themes): the rail opens scrolled past its search, switch and Needs you header.
- tab-most-tokens-1280x800 (both themes): one row under "6 subagents" with All selected; the sort looks like a filter.

## Fixed

- Re-render: no "├" on first-level rows (lines only show who started whom); the breadcrumb starts with the session's title, not "This session"; an empty tab shows no dead filter bar and says "This session hasn't started any subagents".
- 0.3 (this branch): the old tab's capture (session-subagents) was retired; the tab is tab-*.

## Captures

| Capture | What is on it | What changed in 0.3 |
|---|---|---|
| tab-running-1280x800 | feat/read-model's Subagents tab on Running: three running under "You asked: fix the totals and add tests for rounding". | New tab (AJ-E2810, D-56). |
| tab-all-grouped-1280x800, -1024x700 | All of them, grouped under the message that started each group, with tree lines. | New. |
| tab-search-1280x800 | A search that keeps a match's parent. | New. |
| tab-most-tokens-1280x800 | Sorted by most tokens. | New. |
| tab-none-match-1280x800 | A search with nothing found. | New. |
| tab-437-1280x800 | A session of 437 subagents, grouped and windowed. | New. |
| detail-running-1280x800, -1024x700 | A running subagent's page with its processes. | New. |
| detail-done-1280x800 | A finished one with its result and files. | New. |
| detail-failed-1280x800 | A failed one with its reason. | New. |
| detail-conversation-1280x800 | A subagent's conversation. | New. |
| usage-line-1024x700 | The session's Usage tab with "Subagents used" and its door to the tab. | New. |
| row-count-1280x800 | The rail with View › Subagents on: "+3 agents" on the session's row. | Header and switch only. |

## 0.3.2

The subagents table is a raised card on the darker canvas (white on grey in light), and the state pills (Running cyan, Done green, Failed red, each with its mark) stay readable in both themes; the failed reason in detail-failed is a red-bordered box that reads in both. The search field stretches, so tab-all-grouped, tab-running and tab-none-match line the controls up edge to edge. After the design fixes the dark accent is lighter: it shows on the logo mark, New session, the selected Subagents or Usage tab, tab-search's focused field ring, usage-line's Input bar and dot and the "see which ›" link, a little brighter on the dark cards; the light captures are unchanged apart from the header. The detail pages still end their right column (Usage, Processes) long before the left one in detail-done and detail-failed. On every tab-* and detail-* capture at 1280 the rail is still scrolled so its search box, the App/Project/Account switch and the Needs you header are out of view and the first row is "Personal · Limit reached", which makes the rail look headless. row-count shows its subject again: the fifth Open now row carries "+3 agents" beside Working, and after the second pass's last fix the row cuts the project and keeps the age: "ai-… · 1 h ago · +3 agents"; the same headless rail applies. The second pass otherwise only touches the rail here: the Probably asks you row reads "ai-j… · 3 h ago" on every tab-* and detail-* capture. tab-most-tokens still shows a single row under a group that says "6 subagents" with All selected, so the sort reads like a filter. Only row-count has an earlier committed render to compare against; the tab-*, detail-* and usage-line captures are committed for the first time in this round.

## 0.3.4: the Overview's switcher, and the summary that adds up

The maintainer, testing 0.3.3: "a quick way to switch conversation of a session between subagents, so I can easily and quickly drill in", matching how the tools' own apps show them; the Subagents tab stays the full list.

| Check | Finding |
|---|---|
| Grid | At 1280 the switcher is a 12 rem column on the conversation's left; the conversation keeps the rest (about 415 px), the side column its 340 px. Narrower than 576 px, the list sits above the conversation in a short scrolling box, never clipped. |
| Hierarchy | "This session" first, chosen; then Running · 3, Started from here · 1, and Done · 7 folded: what runs, what it started, then history. |
| Spacing | Rows on the rail's rhythm: a dot, a two-line title, its kind or time under it. |
| Icons | A live dot pulses on running ones (still under reduced motion), a red one on a failed ended one; no new icons. |
| Shared controls | Back is the shared Button, the same as the Subagents tab's conversation; Done is a disclosure like the work row's. |
| Plain words | "This session · Its terminal", "Running · 3", "Started from here · 1", "Done · 7", "All 10 in Subagents ›". |
| Type sizes | Titles 13 px medium, their second lines 12 px muted, section heads 12 px semibold muted. |
| Both themes | Checked dark and light; axe clean on both shots. |
| The summary line | "· 2 with no recorded end" beside "1 failed": the line adds up to Needs attention 3. |

### Good

- One click from the conversation to a running subagent's, Esc back; the terminal is hidden, never rebuilt, and keeps its place.
- What the native apps list beside a session is here in the same order of importance: running now, the sessions it started, then what ended.

### Weak

- At 1280 the conversation pane narrows to about 415 px while the switcher shows; a wider window gives it more. Folding the side column when the switcher is open is a 0.4 question (AJ-E3106).
- A started session shows only while its app is open: the app's own record is read from its running instances, as its sidebar is.


## 0.3.5

The maintainer on 0.3.4's Overview column: "you need to be clearer what is this session, and what are subagents … right now it is not clear that what you are seeing is subagents. Why not use lines and branch graphics like Claude does."

**switcher-own, switcher-subagent.** The column is a tree now. At the top, the root: a ring mark, "This session", and under it "Its terminal" (or "Its main conversation"). A line runs down from it; each kind of conversation hangs off it on an elbow: "Subagents running · 3", "Sessions it started · 1", "Finished subagents · 7" (folded). Each of them has its own elbows to its leaves, and each leaf a mark for its kind: a stack for a subagent (with a live dot while it runs, red when it failed), a chat mark for a session it started. The group names say what they hold; their tooltips add "helpers working for it now" and "separate sessions, each with its own page". The lines are the faint hairline colour, so they guide without competing with the names.

## Weak

- On a narrow page the tree sits in its own small scrolling box above the conversation, where the lines take a little of the width.

## 0.4.1 (E28, 2026-11-07)

What changed in these captures, each looked at:
- **usage-line-1024x700** (both themes): the line reads "Subagents: 38% of this session's tokens · see which ›", with "Used most:" naming the three that used most, each a link (AJ-E2812 R1). The second name truncates at its column; it reads, but the gap before its "288k" is wider than the others'.
- **detail-done-1280x800** (both themes): the Processes card no longer offers "Stop it from the session's Processes tab" for a subagent that ran no commands. With commands it says "See the session's processes ›" when none matched, and "Stop one from the session's Processes tab ›" when one did.
- **detail-running-1280x800 and 1024x700**: the head says how long it has run ("Running · 9m"); the breadcrumb goes back one step at a time (Esc), the spec walking it twice.
- **Every capture**: the header's ‹ › Back and Forward and the rail's App headers ("Per… Claude Code") come from other 0.4.1 work, not this family's.

Weak, not this family's to fix: at 1280 the rail cuts its App names to "Per…" and "Wor…" beside "Claude Code", and the header's room pill shrinks to "P…". row-count (both themes) fails axe on a `RoomChip` with an `aria-label` its role doesn't allow (c71c77e2d).

## 0.4.1 CI #218 (2026-11-09)

The five tab captures at 1280x800 dark (tab-running, tab-all-grouped, tab-search, tab-most-tokens, tab-none-match) re-taken from CI #218's renders (the gate's captures; the light ones and the 1024 ones are not in the gate and stay as they were). Each looked at by eye. The Subagents tab itself is unchanged; the one change is the rail's: its head (search, the App/Project/Account lenses) stays put at the top and the list scrolls under it, so the search field is whole where it used to be scrolled out of view (8cc1b38bb).

Weak: the list's first visible line ("feat/read-model · Probably stuck") is cut at the lenses' bottom edge with no fade.
