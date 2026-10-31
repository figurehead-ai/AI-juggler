<!-- captures: 78a421449004 -->
# 75-feedback: loading states, the quit check, and undo toasts

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

In dark, loading-accounts and loading-home show skeleton cards that stand out from the canvas, but in light the skeleton bars nearly vanish: the page-title bar on the grey canvas and the rail's skeleton rows are almost invisible, so the light loading screen reads as blank. quit-sheet is clear: one short sentence and three buttons, with Quit anyway the solid violet primary. reviewed and stop-held/stop-undone put the toast bottom right with its Undo (5s) countdown, and the session page behind them now has cards that lift off the canvas; the toast covers the lower part of the right column ("Continue on another account" in stop-held). In the rail of reviewed, stop-held and stop-undone, the "feat/read-model" row reads "Stopped with an error · 7…", with its age cut off.

## Weak

- loading-accounts-1280x800-light, loading-home-1280x800-light: the skeleton bars barely show on the grey canvas; the title bar is close to invisible.
- reviewed-1280x800, stop-held-1280x800, stop-undone-1280x800: the rail row "Stopped with an error · 7…" loses its age.
- stop-held-1280x800: the toast sits over the "Continue on another account" card.
