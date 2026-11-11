<!-- captures: d504c93c5590 -->
<!-- captures: pending -->
# 99-settings-keyboard: Settings › Keyboard

Design critique (D-57) against the reference screens: Settings' other panes (65-settings-sections), the ? sheet and Desktop apps' go-to key. New family (AJ-E2905).

## Checklist

| Check | Finding |
|---|---|
| Grid | One card per area; each row's name on the left, its keys in one column, its actions in a fixed-width slot on the right, so every key lines up whether a row has Change… or not. |
| Hierarchy | Pane title, then the changed bar (only when something is changed), then the area titles over their cards. |
| Spacing | The settings cards' row rhythm; a fixed row's reason sits on a second line under its name. |
| Icons | None added: keys are keycaps, as on the ? sheet. |
| Shared controls | The shared Button for Change…, Reset and Reset all; the go-to row is Desktop apps' own control (one capture component for both). |
| Disabled reasons | Fixed shortcuts say why in words ("Esc closes what's open everywhere.", "⇧F10 is the system's key for a menu."). |
| Plain words | "You changed 1 shortcut.", "⌘N is New session. Change that one first.", "Press the new keys, or Esc to cancel." |
| Type sizes | Row names 14 px; keys and reasons 12 px. |
| Both themes | Dark and light checked; the refusal reads in amber in both. |
| 1280×800 and 1024×700 | Both; the actions slot keeps its width at 1024. |

## Good

- A changed key shows "changed" beside its keycap and a Reset beside Change…, and the bar at the top counts the changes, so a rebinding is never hidden (the task's handover).
- A refused key says whose it is and what to do, under the row, without leaving capture.

## Weak

- The go-to row's "None" sits in the actions slot rather than the keys column, so its value is a column to the right of the others' keys.
- Capturing has no visible timer or pulse beyond the accent border; the line under the row carries the instruction.

## Fixed

- First render (seen by eye): the capture button's "Press the keys… (Esc cancels)" wrapped onto two lines inside the actions slot. In Keyboard it now says "Press keys…" and the row's own line says "Press the new keys, or Esc to cancel."; Desktop apps keeps the full words.
- Axe (label-content-name-mismatch): the capture's spoken name didn't contain its visible words. The visible words are now its name, with the shortcut added for screen readers only.

## 0.4.1 re-score (2026-11-07, AJ-E3305)

The whole family was retaken on release/0.4.1 for the 14-line re-score (AJ-E3305, `docs/qa/0.4-walks/design.md`) and opened on contact sheets, both themes, 1280 and 1024. Shared with every screen since the last shoot: the header's New session is tinted (one filled button per view), the Floor button is a split control with its own menu, This Mac's line is its dot below 1120 px, and the rail's App headers put the profile first.

Unchanged in design: the keycap is the button, "Press keys…" while capturing, "changed" with Reset on a changed row.
