<!-- captures: e54ad422cbbe -->
# 08-inspector-aggregate: an account's overview (limits and usage), a signed-out account, and a project's overview

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

account-overview-1440x900 is the strongest screen here: two ring gauges (32% and 69%), a weekly chart with the 5-hour peaks, a 7-day token line, and four KPI cards with big numbers, coloured change chips and sparklines, all on cards that stand clearly off the darker canvas. The lighter dark accent shows most here: the rings, the KPI sparklines and the selected Overview tab and "30 days" range read brighter. project-overview-1440x900 follows the same pattern with four KPI cards (the Tokens this week sparkline in the lighter violet) and a stacked daily bar chart with a legend and Profile / Tool / Model switch. account-logged-out-1280x800 centres "Personal is signed out" with a short explanation and one solid violet "Open Accounts to sign in"; the message is clear, but it still sits alone inside a card that fills the whole panel, so most of the screen is an empty bordered box. In account-overview the next card's controls show as a faint cut strip above the footer, which is ordinary scroll overflow. The top bar stays one row, and the rail's error rows read "Stopped with an error" with no cut age. All captures were re-rendered.

## Weak

- account-logged-out-1280x800: a panel-sized empty card around three lines of text and one button.
