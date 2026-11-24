<!-- captures: 14de154f0ab3 -->
# 09-quality: a session's Usage tab ("How it's going"), as small charts and as a table

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The Usage tab is a tidy grid of raised cards, each with a label, a value line and a grey source note ("read from the session", "a best guess"); populated-1440x900 draws Tokens / min as five small violet bars, now in the lighter dark accent, and as-table-1440x900 swaps them for a two-row minutes / tokens table, with the toggle reading "View as table" or "View as chart" as it should. The cards stand off the darker canvas, and the labels are dimmer than the values, which helps scanning. The copy still weakens it: "No token counts for it yet" heads the tab while the cards below show 406 tokens a minute and 6,000 tokens of context, "1 errors / 12 turns" is ungrammatical, and Session cost says "for Opus" on a sonnet-5 session. The five-bar chart is so small that it says little more than the sentence beside it. The top bar stays one row, and the rail's selected error row reads "Stopped with an error" with no cut age. All captures were re-rendered.

## Weak

- populated-1440x900, as-table-1440x900: "No token counts for it yet" contradicts the token figures below; "1 errors"; cost "for Opus" on a sonnet-5 session.
- populated-1440x900: the Tokens / min bar chart is too small to read values from.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B3)

Re-captured (populated and as-table at 1440×900); the same Usage tab as 07-inspector-content's instance-usage, opened there at full size. The tab's How it's going grid reads well; the toggle reads View as table / View as chart as it should.

- **Fixed** (51ea3254f): the tab's top line "No token counts for it yet" contradicted the Tokens / min card under it; it reads "No token totals from its app yet" and says the live cards are read from the session itself. Gate `populated-1440x900` re-rendered with a note.
- **Closed since 0.3.2:** "1 error / 12 turns"; no cost card priced for the wrong model.
- **Weak, kept:** the Tokens / min bars are five small bars beside the sentence that says the same; the table view carries the numbers.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | How a session is going, as small charts or a table. |
| 2 | Prominence | 4 | Totals line, model and window, then the six live cards. |
| 3 | Default | 4 | Chart by default; View as table one click. |
| 4 | Mode | 4 | Calm tone cards. |
| 5 | Mock-up | 4 | SPEC §9's Usage tab. |
| 6 | Areas | 4 | Three bands, clear when blurred. |
| 7 | Colour | 4 | Violet only on the bars and Resume. |
| 8 | Type | 4 | Values ink, labels muted, notes faint. |
| 9 | Actions | 4 | Resume the one filled button. |
| 10 | Serious | 4 | n/a: nothing destructive here. |
| 11 | Doors | 4 | How it's measured › opens the guide. |
| 12 | Words | 4 | After the fix: one meaning for "tokens" on the tab. |
| 13 | States | 4 | Each unknown said once ("Collecting first samples…"). |
| 14 | Consistency | 4 | As 07-inspector-content. |
| 15 | Accessibility | 4 | axe clean in the capture runs (no serious or critical); keyboard and zoom not re-walked this round. |
| 16 | Hand walk | 4 | Not re-walked by hand here; captured from the fixtures, both themes. |
| 17 | System prompts | 5 | Causes no system prompt. |
| 18 | Usability | 4 | Re-read against PAT-USE's six questions (no 0.5.1 task covers it): every figure named with its source. Match. |

Lowest: 4 (every line 4 or 5).
