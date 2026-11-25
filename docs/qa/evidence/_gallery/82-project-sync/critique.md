<!-- captures: dbfc3dc52423 -->
# 82-project-sync: comparing and syncing a project's rules files and MCP servers

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

header-line shows the project path with an amber "1 rules file differs" link, and menu shows the project menu with "Compare rules and servers…" among icon rows and Forget… last in red. sheet-diff is a clear sheet: radio cards pick the master (CLAUDE.md, tinted violet), the diff shows line numbers with a red removed line, and "Apply to AGENTS.md" is the solid violet primary. In light the added lines are a grey-green barely different from the unchanged ones, while in dark they are a clear green. sheet-done confirms each change with its own small Undo button and points to Settings › Switching for later. The only change from the design fixes is in dark: the master's selected radio in sheet-diff and sheet-done uses the lighter accent, a touch brighter on the violet-tinted card. The status text still reads "1 rules file differs", which is ungrammatical.

## Weak

- sheet-diff-light: added lines in the diff are a faint grey-green, close to the unchanged lines.
- header-line, sheet-diff (both themes): "1 rules file differs" should be singular.

## 0.5.1

Re-captured on 0.5.1; only the shared chrome changed in these screens. The rail's head now starts past the traffic lights, its rows hold a fixed dot column (tools on hover), cards and panels separate by tone instead of a border, a chosen item is a fill, and the top bar's bottom line is calmer. The rules dialog is unchanged; account words read app.

## 0.5.2 (2026-11-25, AJ-E1558, Opus side-by-side, both themes)

Re-captured on the host from a clean export of HEAD (the shared tree held another agent's unbuilt Bot work), both themes, every changed capture diffed (changed-pixel count and box) and looked at old beside new in contact sheets; the ones named below also opened at full size. Mock-ups: `d126-rich-ui/kit-dark.png` and `notif-dark.png` (rich mode), `calm-ui/mock-r2-*.png` (the rail). Captures: sheet-diff and sheet-done, both themes (old beside new in the contact sheet).

**What changed.** The header carries its icon tile; each rules file leads with its chip (the master's lit) on tone instead of an outline; the server offer has its chip; Show/Hide changes is the kit's button.

**Judged.** Dark and light side by side: in dark the chips and the accent's soft step read clearly against the cards; in light the soft step is paler but the lit chip's violet glyph, the switch's accent fill and the chosen fill still mark what is on (two cues each, never colour alone). No outline came back (PAT-SURF): separation is the card's tone and spacing. Text keeps the type roles (14 px titles, 13 px help), AA in both themes (the gate's contrast spec). Kit rows; the master stays its accent-soft fill.

| # | Line | Score | Note |
|---|---|---|---|
| 4 | Mode | 4 | Rich, the kit's dials: icons on every row and choice, the accent for on and chosen. |
| 6 | Areas | 4 | Tone, not outlines; no box in a box. |
| 7 | Colour | 4 | The accent means on, chosen, recommended or navigation; status keeps its hue (amber words on neutral, never a fill). |
| 14 | Consistency | 4 | The kit's own pieces (RowChip, the switch, ChoiceOption cards, SELECTED_FILL, Modal's icon tile). |
| 19 | Rich-mode colour | 4 | Before (42aab7661): 2, monochrome rows, bare checkboxes, a dot or a ✓ alone for chosen. Now every row and choice its icon, on and chosen in the accent's soft step, chosen a fill. |

The other lines are as the 0.5.1 critique scored them (unchanged by this work; lowest 4).
