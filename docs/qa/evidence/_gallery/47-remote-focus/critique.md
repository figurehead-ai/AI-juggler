<!-- captures: 6c4503482898 -->
# 47-remote-focus: a Mac mini session's Actions tab after Focus on Mac mini, with each outcome, the view-only state and the palette listing the remote session

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The Actions card sits a step off the canvas in both themes, with Stop on Mac mini in a red tint and Focus on Mac mini as a plain outlined button, and a one-line explanation of what Mac mini checks before acting. The outcome line under it is the weak spot: in outcome-not-running, outcome-permission-needed and outcome-window-not-found the failures ("App not running — Relaunch?", "Permission needed on Mac mini", "App opened — window not found") are drawn in success green with a check mark, exactly like the real success in outcome-focused. "Relaunch?" reads as an offer but isn't a button. view-only greys both actions out with the reason beside them, which is clear but low-contrast in light. palette-mixed shows the command palette over a blurred page with the remote Desk row marked "on Mac mini · ● live" and "inspect" instead of "open", which tells the two apart well. Every capture was re-rendered.

## Weak

- outcome-not-running, outcome-permission-needed, outcome-window-not-found: failure outcomes use the green check of success.
- outcome-not-running: "Relaunch?" is plain text, not an action you can click.
- view-only (light): the disabled Stop and Focus buttons are very faint.
- Rail (all non-palette captures): "Stopped with an error · 7 …" cuts the age.
