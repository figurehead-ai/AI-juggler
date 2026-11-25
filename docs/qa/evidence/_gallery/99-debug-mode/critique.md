<!-- captures: 55bcb0946a8a -->
<!-- captures: pending -->
# 99-debug-mode: Debug mode

Design critique (D-57) against Settings › Advanced (65-settings-sections). New family (AJ-E2912).

## Checklist

| Check | Finding |
|---|---|
| Grid | A settings card: the switch row, then the log row with Show and Clear. |
| Hierarchy | Section title, the switch's one-line promise, the log's state. |
| Spacing | The settings cards' rhythm. |
| Icons | None. |
| Shared controls | The shared switch and Buttons. |
| Disabled reasons | Show and Clear are greyed while the log is empty, and the row says "Empty". |
| Plain words | "Writes down each step of every switch and launch: sizes, timings and the tools' own errors. Never what's in a conversation." |
| Type sizes | 14 px names, 12 px explanations. |
| Both themes | Checked dark and light. |
| 1280×800 and 1024×700 | Both. |

## Good

- What it records and what it never records are both said beside the switch.

## Weak

- The log's size limit (16 MB) and its seven-day off aren't said on the card.

## Fixed

- Nothing on first render.

## 0.4.1 re-score (2026-11-07, AJ-E3305)

The whole family was retaken on release/0.4.1 for the 14-line re-score (AJ-E3305, `docs/qa/0.4-walks/design.md`) and opened on contact sheets, both themes, 1280 and 1024. Shared with every screen since the last shoot: the header's New session is tinted (one filled button per view), the Floor button is a split control with its own menu, This Mac's line is its dot below 1120 px, and the rail's App headers put the profile first.

Unchanged in design: "Keep a detailed log" under Debug mode, the log empty with Show and Clear the log disabled.

## 0.5.1

Re-captured on 0.5.1; only the shared chrome changed in these screens. The rail's head now starts past the traffic lights, its rows hold a fixed dot column (tools on hover), cards and panels separate by tone instead of a border, a chosen item is a fill, and the top bar's bottom line is calmer. Content is unchanged; the settings list gained AI and Models.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: The lens switch's chosen segment is a solid fill. Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.

## 0.5.2 (2026-11-25, AJ-E1558, Opus side-by-side, both themes)

Re-captured on the host from a clean export of HEAD (the shared tree held another agent's unbuilt Bot work), both themes, every changed capture diffed (changed-pixel count and box) and looked at old beside new in contact sheets; the ones named below also opened at full size. Mock-ups: `d126-rich-ui/kit-dark.png` and `notif-dark.png` (rich mode), `calm-ui/mock-r2-*.png` (the rail). Captures: section, both themes.

**What changed.** The detailed log's switch row and the log's row lead with their chips (a bug, a folder).

**Judged.** Dark and light side by side: in dark the chips and the accent's soft step read clearly against the cards; in light the soft step is paler but the lit chip's violet glyph, the switch's accent fill and the chosen fill still mark what is on (two cues each, never colour alone). No outline came back (PAT-SURF): separation is the card's tone and spacing. Text keeps the type roles (14 px titles, 13 px help), AA in both themes (the gate's contrast spec). Kit rows.

| # | Line | Score | Note |
|---|---|---|---|
| 4 | Mode | 4 | Rich, the kit's dials: icons on every row and choice, the accent for on and chosen. |
| 6 | Areas | 4 | Tone, not outlines; no box in a box. |
| 7 | Colour | 4 | The accent means on, chosen, recommended or navigation; status keeps its hue (amber words on neutral, never a fill). |
| 14 | Consistency | 4 | The kit's own pieces (RowChip, the switch, ChoiceOption cards, SELECTED_FILL, Modal's icon tile). |
| 19 | Rich-mode colour | 4 | Before (42aab7661): 2, monochrome rows, bare checkboxes, a dot or a ✓ alone for chosen. Now every row and choice its icon, on and chosen in the accent's soft step, chosen a fill. |

The other lines are as the 0.5.1 critique scored them (unchanged by this work; lowest 4).
