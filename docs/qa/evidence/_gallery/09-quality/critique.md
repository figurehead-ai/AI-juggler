<!-- captures: e564bbbd855f -->
# 09-quality: a session's Usage tab ("How it's going"), as small charts and as a table

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The Usage tab is a tidy grid of raised cards, each with a label, a value line and a grey source note ("read from the session", "a best guess"); populated-1440x900 draws Tokens / min as five small violet bars and as-table-1440x900 swaps them for a two-row minutes / tokens table, with the toggle reading "View as table" or "View as chart" as it should. The cards stand off the darker canvas, and the labels are dimmer than the values, which helps scanning. The copy weakens it: "No token counts for it yet" heads the tab while the cards below show 406 tokens a minute and 6,000 tokens of context, "1 errors / 12 turns" is ungrammatical, and Session cost says "for Opus" on a sonnet-5 session. The five-bar chart is so small that it says little more than the sentence beside it. All captures were re-rendered.

## Weak

- populated-1440x900, as-table-1440x900: "No token counts for it yet" contradicts the token figures below; "1 errors"; cost "for Opus" on a sonnet-5 session.
- populated-1440x900: the Tokens / min bar chart is too small to read values from.
