<!-- captures: 61c59e6309bf -->
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

The subagents table is a raised card on the darker canvas (white on grey in light), and the state pills (Running cyan, Done green, Failed red, each with its mark) stay readable in both themes; the failed reason in detail-failed is a red-bordered box that reads in both. The search field stretches, so tab-all-grouped, tab-running and tab-none-match line the controls up edge to edge. After the design fixes the dark accent is lighter: it shows on the logo mark, New session, the selected Subagents or Usage tab, tab-search's focused field ring, usage-line's Input bar and dot and the "see which ›" link, a little brighter on the dark cards; the light captures are unchanged apart from the header. The detail pages still end their right column (Usage, Processes) long before the left one in detail-done and detail-failed. On every tab-* and detail-* capture at 1280 the rail is still scrolled so its search box, the App/Project/Account switch and the Needs you header are out of view and the first row is "Personal · Limit reached", which makes the rail look headless. row-count shows its subject again: the fifth Open now row carries "+3 agents" beside Working, and after the second pass's last fix the row cuts both: "ai-j… · 1 h a… · +3 agents"; the same headless rail applies. The second pass otherwise only touches the rail here: the Probably asks you row reads "ai-j… · 3 h ago" on every tab-* and detail-* capture. tab-most-tokens still shows a single row under a group that says "6 subagents" with All selected, so the sort reads like a filter. Only row-count has an earlier committed render to compare against; the tab-*, detail-* and usage-line captures are committed for the first time in this round.
