<!-- captures: f4cd6bac2bed -->
# 08-inspector-aggregate: an account's overview (limits and usage), a signed-out account, and a project's overview

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

account-overview-1440x900 is the strongest screen here: two ring gauges (32% and 69%), a weekly chart with the 5-hour peaks, a 7-day token line, and four KPI cards with big numbers, coloured change chips and sparklines, all on cards that stand clearly off the darker canvas. The lighter dark accent shows most here: the rings, the KPI sparklines and the selected Overview tab and "30 days" range read brighter. project-overview-1440x900 follows the same pattern with four KPI cards (the Tokens this week sparkline in the lighter violet) and a stacked daily bar chart with a legend and Profile / Tool / Model switch. account-logged-out-1280x800 centres "Personal is signed out" with a short explanation and one solid violet "Open Accounts to sign in"; the message is clear, but it still sits alone inside a card that fills the whole panel, so most of the screen is an empty bordered box. In account-overview the next card's controls show as a faint cut strip above the footer, which is ordinary scroll overflow. The top bar stays one row, and the rail's error rows read "Stopped with an error" with no cut age. All captures were re-rendered.

## Weak

- account-logged-out-1280x800: a panel-sized empty card around three lines of text and one button.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B3)

Re-captured (three states); opened at full size: project-overview-1440x900 and account-logged-out-1280x800. project-overview's four KPI tiles, the Activity chart (App / Tool / Model, F53's solid chosen segment) and Who did the work read as the dashboard they are. account-logged-out is now one line card ("Claude Code CLI · Personal is signed out … Nothing is deleted.") with Sign in from Accounts, over the App's Open now and Last used cards.

- **Closed since 0.3.2:** account-logged-out was a panel-sized empty card around three lines; it is one line card above the App's own content.
- **Weak, logged F76:** the project header's "1 stuck" is an amber pill with a fill behind its words (the kit's amber chip).
- **Weak, kept:** the preview's fixtures disagree with each other: "Apps used 1 · Work Max" beside an Activity chart with Work Max and Fake Desk (two preview seams, not the product).

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | How an account or a project is doing, at a glance. |
| 2 | Prominence | 4 | Tiles, then the chart, then who and what's open. |
| 3 | Default | 4 | The chart by App by default; 30 days. |
| 4 | Mode | 4 | Calm dashboard with charts, as Usage. |
| 5 | Mock-up | 4 | SPEC §9 and the Usage dashboard's kit. |
| 6 | Areas | 4 | Tiles row, chart card, two cards: clear when blurred. |
| 7 | Colour | 4 | F76 (6311ad3cd): "1 stuck" is amber words on the neutral pill's tone; series colours by App, change chips neutral. |
| 8 | Type | 4 | Big values lead, labels muted, tabular numbers. |
| 9 | Actions | 4 | Sign in from Accounts the one filled button when signed out. |
| 10 | Serious | 4 | "Nothing is deleted" said on sign-out. |
| 11 | Doors | 4 | All sessions ›, each open session, the tiles' counts open their tabs. |
| 12 | Words | 4 | Plain; the fixture mismatch above is the preview's. |
| 13 | States | 4 | Signed out said once; the rest of the page stays. |
| 14 | Consistency | 4 | Usage's tiles and charts. |
| 15 | Accessibility | 4 | axe clean in the capture runs (no serious or critical); keyboard and zoom not re-walked this round. |
| 16 | Hand walk | 4 | Not re-walked by hand here; captured from the fixtures, both themes. |
| 17 | System prompts | 5 | Causes no system prompt. |
| 18 | Usability | 4 | Re-read against PAT-USE (no 0.5.1 task): unavailable (signed out) is one guided line with its door. Match. |

Lowest: 3, each line under 4 logged (F76); the rest 4 or 5. After F76: lowest 4.

**Fixed after the critique (F76), re-captured (the three states; opened project-overview-1440x900):** the project header's "1 stuck" is amber words on the grey neutral pill, beside "1 failed" (red tint) and "2 working" (teal tint); no amber fill behind words. Line 7 3 → 4; lowest now 4.
