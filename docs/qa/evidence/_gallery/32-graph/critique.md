<!-- captures: d0fd47e80c95 -->
# 32-graph: how a piece of work moved between accounts, the project overview, and the "Continue on another account" ranking

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

In dark the project and session pages read as cards on a canvas: the "How this work travelled" strip and the Every move card in fork-merge, linear, failed and node-menu sit a clear step above the page, and in light the grey canvas under white cards does the same. New session is one solid violet in every top bar, and the "Continue on another account" dialog (targets, targets-why) gives the ranking room, with the suggested Work Max card ringed in the lighter violet and the amber "Not proven yet" chips easy to tell from the violet "Proven" one. fork-merge and linear are now framed on the travel strip and the Every move chain, and they differ: fork-merge shows Payments fix with its "fork" and "merge" chips, linear shows Login refactor's three plain switches. At 800x600 badges and targets keep the top bar on one row with the cleanup pill as a broom and "7". Several things are still off. The light badges, targets and targets-why captures still render the whole app dark. In fork-merge the Every move row opens scrolled sideways, so its first card is cut to "x" and a sliver of the fork chip, and the move labels are cut ("Same conversation, ne…", "Full sto…"). At 800x600 the targets dialog cuts its last card (Lab) at the bottom edge with no sign that it scrolls, node-menu's Every move card is scrolled sideways so its left edge is cut ("actor", "nal · Codex", "ne (⌘R)"), and the pages that keep the full rail (failed, fork-merge, linear, node-menu) wrap the top bar to two rows and the project tabs onto a second line with "History" alone. In targets-why each account prints its headroom line twice. The rail's status lines now show whole ("Stopped with an error").

## Weak

- fork-merge-1280x800-*.png: the Every move row is scrolled sideways; the first card is cut to "x" and part of the fork chip, and the move labels are cut ("Same conversation, ne…", "Full sto…").
- fork-merge-*, linear-*, failed-*, node-menu-*: the strip says "No moves between accounts in this range" right above an Every move list of switches.
- badges-*-light, targets-*-light, targets-why-*-light: the whole app renders in the dark theme.
- targets-800x600-*, targets-why-800x600-*: the dialog cuts the Lab card (and the "What carries is checked…" footnote) at the bottom with no scroll cue.
- node-menu-800x600-*: the "Every move" card is scrolled sideways and clips its left edge ("actor", "nal · Codex", "ne (⌘R)").
- failed-800x600-*, fork-merge-800x600-*, linear-800x600-*, node-menu-800x600-*: with the full rail kept at 800 wide the top bar wraps to two rows (broom pill and New session on the second) and the project tabs wrap, leaving "History" alone; badges and targets fold the rail to icons and stay on one row.
- failed-*: the red reason box opens "Switch failed: failed · target launch failed", the word doubled; at 800x600 the box is cut by the page footer.
- targets-why-*: "80% headroom · used most here" and "35% headroom · quiet" appear twice in their cards.
- badges-*, targets-*, targets-why-*: ages read "748 d" in the rail and "started 748d ago" in the header; the fixture's dates are not pinned to the capture clock.
- targets-* and targets-why-*: "exhausts ≈262 min · Measured" sits right-aligned on its own line, leaving a gap under the name and chips.
