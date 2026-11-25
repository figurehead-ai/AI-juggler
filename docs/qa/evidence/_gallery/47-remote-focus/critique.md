<!-- captures: 1fd710b93065 -->
# 47-remote-focus: a Mac mini session's Actions tab after Focus on Mac mini, with each outcome, the view-only state and the palette listing the remote session

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The Actions card sits a step off the canvas in both themes, with Stop on Mac mini in a red tint and Focus on Mac mini as a plain outlined button, and a one-line explanation of what Mac mini checks before acting. The outcome line under it is still the weak spot: in outcome-not-running, outcome-permission-needed and outcome-window-not-found the failures ("App not running — Relaunch?", "Permission needed on Mac mini", "App opened — window not found") are drawn in success green with a check mark, exactly like the real success in outcome-focused, and "Relaunch?" reads as an offer but isn't a button. view-only greys both actions out with the reason beside them, which is clear but low-contrast in light. palette-mixed shows the command palette over a blurred page with the remote Desk row marked "on Mac mini · ● live" and "inspect" instead of "open", which tells the two apart well; in dark its search field's focus ring is now the lighter violet. The other design fixes show in the chrome only: the rail's error rows read "Stopped with an error" whole, and the top bar stays on one row with "7 to clean up" and New session. Every capture was re-rendered.

## Weak

- outcome-not-running, outcome-permission-needed, outcome-window-not-found: failure outcomes use the green check of success.
- outcome-not-running: "Relaunch?" is plain text, not an action you can click.
- view-only (light): the disabled Stop and Focus buttons are very faint.

## AJ-E710 retake (2026-11-07)

Retaken on release/0.4.1 for the Fleet gate's screenshot set (AJ-E710), both themes, every size the family has. First pass by a Sonnet agent against a written sheet; the points marked (checked) were confirmed by eye by Opus.

Failed outcomes now show an amber "⚠ Mac mini: App not running — Relaunch?" line, not the success check the earlier notes describe (checked). "Relaunch?" is still words, not a button.

## 0.5.1

Re-captured on 0.5.1; only the shared chrome changed in these screens. The rail's head now starts past the traffic lights, its rows hold a fixed dot column (tools on hover), cards and panels separate by tone instead of a border, a chosen item is a fill, and the top bar's bottom line is calmer. The Actions tab is unchanged.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Behind the dialog the rail and Needs you are in their 0.5.1 style; the dialog itself is unchanged; the lens switch's chosen segment is a solid fill. Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
