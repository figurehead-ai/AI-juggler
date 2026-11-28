<!-- captures: 9ac71cd5fd8c -->
<!-- captures: pending -->
# 99-floor-settings: Settings › The Floor, and Re-create

Design critique (D-57) against the approved wireframes (D-78) and the other Settings panes (99-settings-keyboard, Desktop apps). New family (AJ-E3511).

## Checklist

| Check | Finding |
|---|---|
| Grid | The Settings pane layout: a heading, a line on what the Floor is, then two cards (Opening, The office), each choice a row with its words on the left and its control on the right. |
| Hierarchy | Row names in ink, their explanation muted under them; section headings above each card. |
| Spacing | The cards' row rhythm; "More · Apps hidden from the Floor (1)" sits under the office card as a disclosure. |
| Icons | The Floor icon in the Settings list, beside the other panes'. |
| Shared controls | Switch, the −/+ stepper, Segmented (Follow the system, Full, Calm, Off; Look: Pixel art, Full resolution), secondary buttons, Sheet. |
| Disabled reasons | The stepper's − and + are disabled at 4 and 16 with the range in their labels. |
| Plain words | "Calm: half as often, no one walks. Off: nothing moves; it draws only when something changes." Look (D-80): "Pixel art: the Floor's own look, drawn in big pixels. Full resolution: smooth edges at your screen's own pixels; on a large window it takes about 2.7% of a core while animating, more than the Floor's 2% budget." The sheet says what changes (floor, walls, desks, nook, layout) and what stays (Apps, sessions, history, accounts, the main window), with an example. |
| Type sizes | 14 px rows, 12 px explanations. |
| Both themes | Checked. |
| 1280×800 and 1024×700 | Both; the rail folds at 1024 and the pane keeps its width. |

## Good

- Each choice says what it costs or saves (Full is "about 2% of one core").
- Re-create asks first, names both lists, and links to how the Floor works.
- D-80's Look row sits under Motion, the same Segmented control, Pixel art chosen; its line says the cost with the measured number, so the budget-breaking case is said where it's chosen.

## Weak

- Re-create's example names one App; a reader with many Apps can't preview their own layout before choosing.
- The Look line runs to three lines at 1280 and 1024, the longest in the card; it's the price of saying the number.

## Fixed

- Nothing on first render.
- 2026-11-05 (D-80): the Look row added; light and dark, 1280×800 and 1024×700 checked by eye: one row, nothing clipped, the control aligned with Motion's.

## 0.4.1 re-score (2026-11-07, AJ-E3305)

The whole family was retaken on release/0.4.1 for the 14-line re-score (AJ-E3305, `docs/qa/0.4-walks/design.md`) and opened on contact sheets, both themes, 1280 and 1024. Shared with every screen since the last shoot: the header's New session is tinted (one filled button per view), the Floor button is a split control with its own menu, This Mac's line is its dot below 1120 px, and the rail's App headers put the profile first.

Unchanged in design: Opening, The office (Apps per floor, Motion, Look), Re-create's sheet with Changes and Stays.

## 0.5.1

Re-captured on 0.5.1; only the shared chrome changed in these screens. The rail's head now starts past the traffic lights, its rows hold a fixed dot column (tools on hover), cards and panels separate by tone instead of a border, a chosen item is a fill, and the top bar's bottom line is calmer. The Floor settings are unchanged.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Behind the dialog the rail and Needs you are in their 0.5.1 style; the dialog itself is unchanged. Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.

## 0.5.2 (2026-11-25, AJ-E1558, Opus side-by-side, both themes)

Re-captured on the host from a clean export of HEAD (the shared tree held another agent's unbuilt Bot work), both themes, every changed capture diffed (changed-pixel count and box) and looked at old beside new in contact sheets; the ones named below also opened at full size. Mock-ups: `d126-rich-ui/kit-dark.png` and `notif-dark.png` (rich mode), `calm-ui/mock-r2-*.png` (the rail). Captures: pane, hidden and recreate at 1024×700 and 1280×800, both themes.

**What changed.** Each of the Floor's eight rows leads with its icon (open at start, on top, apps per floor, motion, look, dust, reset, re-create), switches the kit's; the re-create sheet carries its icon tile.

**Judged.** Dark and light side by side: in dark the chips and the accent's soft step read clearly against the cards; in light the soft step is paler but the lit chip's violet glyph, the switch's accent fill and the chosen fill still mark what is on (two cues each, never colour alone). No outline came back (PAT-SURF): separation is the card's tone and spacing. Text keeps the type roles (14 px titles, 13 px help), AA in both themes (the gate's contrast spec). Kit rows.

| # | Line | Score | Note |
|---|---|---|---|
| 4 | Mode | 4 | Rich, the kit's dials: icons on every row and choice, the accent for on and chosen. |
| 6 | Areas | 4 | Tone, not outlines; no box in a box. |
| 7 | Colour | 4 | The accent means on, chosen, recommended or navigation; status keeps its hue (amber words on neutral, never a fill). |
| 14 | Consistency | 4 | The kit's own pieces (RowChip, the switch, ChoiceOption cards, SELECTED_FILL, Modal's icon tile). |
| 19 | Rich-mode colour | 4 | Before (42aab7661): 2, monochrome rows, bare checkboxes, a dot or a ✓ alone for chosen. Now every row and choice its icon, on and chosen in the accent's soft step, chosen a fill. |

The other lines are as the 0.5.1 critique scored them (unchanged by this work; lowest 4).
