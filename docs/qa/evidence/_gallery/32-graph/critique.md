<!-- captures: 69f26b7cd398 -->
# 32-graph: how a piece of work moved between accounts, the project overview, and the "Continue on another account" ranking

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

In dark the project and session pages now read as cards on a canvas: the four stat tiles and the Activity chart in fork-merge and linear, and the "How this work travelled" strip in failed and node-menu, all sit a clear step above the page, and in light the grey canvas under white cards does the same. New session is one solid violet in every top bar, and the "Continue on another account" dialog (targets, targets-why) gives the ranking room, with the suggested Work Max card ringed and the amber "Not proven yet" chips easy to tell from the violet "Proven" one. Several things are off. fork-merge and linear are now the same picture (stats and Activity chart at the top of the overview) and neither shows the fork or merge chain they are named for. The light badges, targets and targets-why captures still render the whole app dark. At 800×600 the targets dialog cuts its last card (Lab) at the bottom edge with no sign that it scrolls, node-menu's "Every move" card is scrolled sideways so its left edge is cut ("actor", "nal · Codex", "ne (⌘R)"), and the project tabs wrap "History" onto a second line. In targets-why each account prints its headroom line twice, and every capture with the rail shows "Stopped with an error · 7…" with the age cut off. All captures were re-rendered.

## Weak

- fork-merge-* and linear-*: byte-identical captures of the overview's top; the travel chain with its fork and merge marks is below the fold, so neither shows what it is named for.
- badges-*-light, targets-*-light, targets-why-*-light: the whole app renders in the dark theme.
- targets-800x600-*, targets-why-800x600-*: the dialog cuts the Lab card (and the "What carries is checked…" footnote) at the bottom with no scroll cue.
- node-menu-800x600-*: the "Every move" card is scrolled sideways and clips its left edge ("actor", "nal · Codex", "ne (⌘R)").
- failed-800x600-*, fork-merge-800x600-*, linear-800x600-*, node-menu-800x600-*: the project tabs wrap, leaving "History" alone on a second row; these keep the full rail at 800 wide while badges and targets fold it to icons.
- targets-why-*: "80% headroom · used most here" and "35% headroom · quiet" appear twice in their cards.
- All captures with the rail: the feat/read-model row reads "Stopped with an error · 7…", the age cut off.
- targets-* and targets-why-*: "exhausts ≈262 min · Measured" sits right-aligned on its own line, leaving a gap under the name and chips.
