<!-- captures: 81b9d8b749c7 -->
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

## 0.5.1

Re-captured against 0.5.1: the capturing row's Press keys ring is the only bright line, the You changed 1 shortcut bar is a tonal strip, and the refused message is amber words with no fill. The 1024 captures re-render identically.

Nothing broken (checked both themes).

## 0.5.2 (2026-11-25, AJ-E1558, Opus side-by-side, both themes)

Re-captured on the host from a clean export of HEAD (the shared tree held another agent's unbuilt Bot work), both themes, every changed capture diffed (changed-pixel count and box) and looked at old beside new in contact sheets; the ones named below also opened at full size. Mock-ups: `d126-rich-ui/kit-dark.png` and `notif-dark.png` (rich mode), `calm-ui/mock-r2-*.png` (the rail). Captures: capturing, changed, refused and go-to at 1024×700 and 1280×800, both themes.

**What changed.** Each shortcut row now leads with its action's icon from the menus (53 rows), the changed one's chip lit in the accent's soft step; the section is its tone with no rules between rows; notes sit under the words, aligned to them.

**Judged.** Dark and light side by side: in dark the chips and the accent's soft step read clearly against the cards; in light the soft step is paler but the lit chip's violet glyph, the switch's accent fill and the chosen fill still mark what is on (two cues each, never colour alone). No outline came back (PAT-SURF): separation is the card's tone and spacing. Text keeps the type roles (14 px titles, 13 px help), AA in both themes (the gate's contrast spec). Kit rows; no mock of this pane.

| # | Line | Score | Note |
|---|---|---|---|
| 4 | Mode | 4 | Rich, the kit's dials: icons on every row and choice, the accent for on and chosen. |
| 6 | Areas | 4 | Tone, not outlines; no box in a box. |
| 7 | Colour | 4 | The accent means on, chosen, recommended or navigation; status keeps its hue (amber words on neutral, never a fill). |
| 14 | Consistency | 4 | The kit's own pieces (RowChip, the switch, ChoiceOption cards, SELECTED_FILL, Modal's icon tile). |
| 19 | Rich-mode colour | 4 | Before (42aab7661): 2, monochrome rows, bare checkboxes, a dot or a ✓ alone for chosen. Now every row and choice its icon, on and chosen in the accent's soft step, chosen a fill. |

The other lines are as the 0.5.1 critique scored them (unchanged by this work; lowest 4).
