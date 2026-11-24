<!-- captures: 4d346c48b588 -->
# 81-this-mac: the This Mac page in both densities and under memory pressure

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

page-1280x800 (comfortable and compact, both themes) splits the page into Now (Memory, Processor, Swap, AI Juggler cards) and What's using it (a grouped list with sizes and CPU right-aligned); the cards stand off the canvas in both themes and the large figures ("22 GB", "18%") lead each card. The Memory bar's violet AI-tools share and hatched compressed segment are easy to read against its legend; in dark the AI-tools bar, its legend square and the logo mark now use the lighter accent. In the compact captures the rail rows still squeeze their titles to make room for the figures, so "Refactor the settings screen" becomes "Refa…". In the comfortable and other-apps captures the second pass keeps the rail row's time whole: "ai-ju… · viewed 2 min ago" where it read "ai-juggler · viewed 2 mi…". page-other-apps shows the memory-pressure state with a red mark and a "What it means" fold, and its header now stays on one row: the status reads "This Mac · Under pressure" with a red dot, beside "7 to clean up" and New session, so the tabs, status and actions line up as on every other page. The Claude app group's line is cut at "6 windows · not started by AI Jug…" in every capture, which is acceptable since its windows are listed below.

## Weak

- page-1280x800-dark-compact, page-1280x800-light-compact: the compact rail cuts a session title to "Refa…".

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B3)

Re-captured (comfortable and compact, both themes, and the other-apps pressure page); opened at full size: page-other-apps-1280x800-dark. Only the rail's lens moved since the last shoot (F53). Now (Memory with its bar, legend and What it means open) beside What's using it (grouped, sizes right-aligned).

- **Fixed** (51ea3254f, with 72-this-mac): the pressure line read "Swap is growing — 3004 MB a minute"; it reads "2.9 GB a minute".
- **Weak, kept:** "not started by AI Jugg…" cut at 1280; the compact rail cuts titles short ("Refactor the setti…"), F55's rule.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | What uses this Mac's memory and processor. |
| 2 | Prominence | 4 | The state, the Memory card, then the list. |
| 3 | Default | 4 | What it means opens under pressure. |
| 4 | Mode | 4 | Calm. |
| 5 | Mock-up | 4 | AJ-E2105's page. |
| 6 | Areas | 4 | Two columns by tone. |
| 7 | Colour | 4 | Red for pressure, violet for AI tools. |
| 8 | Type | 4 | Figures lead. |
| 9 | Actions | 4 | No primary needed. |
| 10 | Serious | 4 | "Closing the apps that use the most memory eases it" says how. |
| 11 | Doors | 4 | Every App row opens its page. |
| 12 | Words | 4 | Plain, after the GB fix. |
| 13 | States | 4 | Pressure and plenty said. |
| 14 | Consistency | 4 | As 72-this-mac. |
| 15 | Accessibility | 4 | axe clean in the capture runs (no serious or critical); keyboard and zoom not re-walked this round. |
| 16 | Hand walk | 4 | Not re-walked by hand here; captured from the fixtures, both themes. |
| 17 | System prompts | 5 | Causes no system prompt. |
| 18 | Usability | 4 | As 72-this-mac. Match. |

Lowest: 4 (every line 4 or 5).
