<!-- captures: df3086b7f193 -->
# 32-graph: how a piece of work moved between accounts, the project overview, and the "Continue on another account" ranking

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

In dark the project and session pages read as cards on a canvas: the "How this work travelled" strip and the Every move card in fork-merge, linear, failed and node-menu sit a clear step above the page, and in light the grey canvas under white cards does the same. New session is one solid violet in every top bar, and the "Continue on another account" dialog (targets, targets-why) gives the ranking room, with the suggested Work Max card ringed in the lighter violet and the amber "Not proven yet" chips easy to tell from the violet "Proven" one. fork-merge and linear are now framed on the travel strip and the Every move chain, and they differ: fork-merge shows Payments fix with its "fork" and "merge" chips, linear shows Login refactor's three plain switches. At 800x600 badges and targets keep the top bar on one row with the cleanup pill as a broom and "7". After the second pass the light badges, targets and targets-why captures render in the light theme (white dialog and cards on the grey page, the amber chips as orange outlines), and the dark ones show real ages ("Probably stuck · 3 h", "started 8h ago", "ran 8h") and "Week resets at 4:13 AM tomorrow" instead of a fixed date; the terminal's black now fills its card to the edges. Several things are still off. In fork-merge the Every move row opens scrolled sideways, so its first card is cut to "x" and a sliver of the fork chip, and the move labels are cut ("Same conversation, ne…", "Full sto…"). At 800x600 the targets dialog cuts its last card (Lab) at the bottom edge with no sign that it scrolls, node-menu's Every move card is scrolled sideways so its left edge is cut ("actor", "nal · Codex", "ne (⌘R)"), and the pages that keep the full rail (failed, fork-merge, linear, node-menu) wrap the top bar to two rows and the project tabs onto a second line with "History" alone. In targets-why each account prints its headroom line twice. The rail's status lines now show whole ("Stopped with an error").

## Weak

- fork-merge-1280x800-*.png: the Every move row is scrolled sideways; the first card is cut to "x" and part of the fork chip, and the move labels are cut ("Same conversation, ne…", "Full sto…").
- fork-merge-*, linear-*, failed-*, node-menu-*: the strip says "No moves between accounts in this range" right above an Every move list of switches.
- targets-800x600-*, targets-why-800x600-*: the dialog cuts the Lab card (and the "What carries is checked…" footnote) at the bottom with no scroll cue.
- node-menu-800x600-*: the "Every move" card is scrolled sideways and clips its left edge ("actor", "nal · Codex", "ne (⌘R)").
- failed-800x600-*, fork-merge-800x600-*, linear-800x600-*, node-menu-800x600-*: with the full rail kept at 800 wide the top bar wraps to two rows (broom pill and New session on the second) and the project tabs wrap, leaving "History" alone; badges and targets fold the rail to icons and stay on one row.
- failed-*: the red reason box opens "Switch failed: failed · target launch failed", the word doubled; at 800x600 the box is cut by the page footer.
- targets-why-*: "80% headroom · used most here" and "35% headroom · quiet" appear twice in their cards.
- targets-* and targets-why-*: "exhausts ≈262 min · Measured" sits right-aligned on its own line, leaving a gap under the name and chips.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B3)

Re-captured at the window's sizes: 1280×800 and, for 800×600 (below the window's minimum, F51's rule), 1024×680 (capture.spec `GRAPH_VP`; the gate state, targets at 1280 dark, unchanged). Opened at full size: failed-1280x800-dark, fork-merge-1024x680-light and fork-merge-1280x800-light after the fix. The project page's How this work travelled strip (a lane per App, a dot per day) reads well, and Continue on another account's targets keep their ranking room.

- **Fixed** (51ea3254f): the strip said "No moves between apps in this range" right above Every move's list of moves. With older moves it now says "No moves in these 30 days; every move, the older ones too, is under Every move below." Re-captured; checked by eye.
- **800×600 → 1024×680:** the fourteen 800×600 captures are gone, fourteen 1024×680 ones added; the 0.3.2 weak items at 800 (two-row top bar, cut dialog) went with that size.
- **Weak, logged F78:** Every move predates D-142's switch frame: outlined boxes inside an outlined fold inside the card, an amber-outlined "fork" chip, labels cut ("Same conversation, an…"), a row that scrolls sideways and cuts its first or last card, and "Switch failed: failed · target launch failed" in a red outlined box.
- **Weak, logged F76:** the project header's "1 stuck" amber pill.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | See how work moved between Apps, and resume or export from any step. |
| 2 | Prominence | 4 | The strip, then Every move folded open. |
| 3 | Default | 4 | Ranked targets with the first suggested. |
| 4 | Mode | 4 | Calm strip; the move list is a rich card that isn't yet on its kit (F78). |
| 5 | Mock-up | 3 | Every move predates D-142's switch frame (F78). |
| 6 | Areas | 3 | Box in a box in a box in Every move (F78). |
| 7 | Colour | 3 | The amber "fork" outline and "1 stuck" pill (F76, F78). |
| 8 | Type | 4 | Lane names ink, ticks muted. |
| 9 | Actions | 4 | Open in Finder and New session here quiet; the dialog's one primary. |
| 10 | Serious | 4 | A failed move says nothing was half-moved and where recovery starts. |
| 11 | Doors | 4 | Each step's session opens; Resume and Export on its menu. |
| 12 | Words | 3 | "Switch failed: failed · …", cut edge labels (F78); the strip's contradiction fixed. |
| 13 | States | 4 | Empty range, no moves and older moves each said. |
| 14 | Consistency | 3 | Every move is not the switch dialog's target card (F78). |
| 15 | Accessibility | 4 | axe clean in the capture runs (no serious or critical); keyboard and zoom not re-walked this round. |
| 16 | Hand walk | 4 | Not re-walked by hand here; captured from the fixtures, both themes. |
| 17 | System prompts | 5 | Causes no system prompt. |
| 18 | Usability | 4 | Re-read against PAT-USE: switching history is in the project where it happened, said once. Match but F78. |

Lowest: 3, each line under 4 logged (F76, F78); the rest 4 or 5.
