<!-- captures: ecb774a8de59 -->
# 75-feedback: loading states, the quit check, and undo toasts

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

loading-accounts and loading-home now read as loading in both themes. In light, after the design fixes, the skeleton bars use a darker grey: the page-title bar on the grey canvas, the four rail skeleton rows and the bars and button blocks inside the white cards all show plainly, where the first pass looked nearly blank. In dark the skeleton cards stand out from the canvas as before. quit-sheet is clear: one short sentence and three buttons, with Quit anyway the solid violet primary, its focus ring now in the lighter dark accent. reviewed and stop-held/stop-undone put the toast bottom right with its Undo (5s) countdown, and the session page behind them has cards that lift off the canvas; the toast still covers the lower part of the right column ("Continue on another account" in stop-held). In the rail of reviewed, stop-held and stop-undone, the feat/read-model row now reads "Stopped with an error" whole, without the cut-off age.

## Weak

- stop-held-1280x800: the toast sits over the "Continue on another account" card.
