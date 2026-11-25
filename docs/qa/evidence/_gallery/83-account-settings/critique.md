<!-- captures: 98447a585e22 -->
# 83-account-settings: copying one account's settings into another or a new account

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

collapsed, nothing and open show the Add a fresh account form with a styled "Start with the same settings as" chip and a solid violet "Create and sign in"; nothing explains in one line that Personal has nothing to copy. open and account-sheet list each item with its own checkbox and a plain explanation, and walk through the github token step as numbered instructions with a command and Copy button. After the second pass the command block wraps at spaces, so words and flags like "--scope", "user" and "command" stay whole, but it still breaks at hyphens: open-* split the JSON as '["-' / 'y",' and "server-" / "github", and account-sheet-* split "github --" / "scope user" and "server-" / "github". The wrapped command is a line taller, which pushes its Copy button down to the bottom edge of the scroll area: in account-sheet-* and open-* (both themes) only the top of the Copy button shows, cut by the edge, and in open-* the rest of the tall capture below it is empty. account-sheet-done lists what was set up with an Undo beside each and a solid violet Done. signin keeps a two-step header and one Claude Code row with a solid violet Sign in. The design fixes touched only the dark captures: the ticked checkboxes, the Name field's focus ring and signin's step "2" circle use the lighter accent, a little brighter on the near-black dialog; the light captures are unchanged.

## Weak

- account-sheet-dark/light, open-dark/light: the command block still breaks at hyphens ('["-' / 'y",', "server-" / "github", "github --" / "scope"), so the pasted line is hard to check by eye.
- account-sheet-dark/light, open-dark/light: the Copy button under the command is cut by the scroll edge; only its top border shows.
- collapsed, open (both themes): the "Its MCP servers, permissions and status line ▸" line ends in a small stray triangle that reads as a glitch.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B2)

The captures from d86510acc (this batch's capture left them byte-identical), both themes. account-sheet: "Copy settings into Personal · From Work Max · Set up on Personal", "Personal doesn't have 5 things Work Max has. 3 will be set up.", each item with its checkbox, kind and verdict tag (Can copy, Needs a step from you), Show me how for github, and the one primary "Copy 3 settings". The 0.3.2 command-block and Copy-button notes are behind Show me how now.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 5 | |
| 2 | Prominence | 4 | |
| 3 | Default | 5 | What can be copied is ticked; the count in the primary. |
| 4 | Mode | 4 | |
| 5 | Mock-up | 4 | |
| 6 | Areas | 4 | "What does this mean?" under every one of five items reads as a column of repeats. |
| 7 | Colour | 4 | Verdict tags by status; amber where the user has a step. |
| 8 | Type | 4 | |
| 9 | Actions | 5 | "Copy 3 settings". |
| 10 | Serious | 5 | Never its sign-in, history or caches; Undo each in Settings. |
| 11 | Doors | 4 | |
| 12 | Words | 4 | |
| 13 | States | 4 | Nothing to copy, collapsed, open, done. |
| 14 | Consistency | 4 | |
| 15 | Accessibility | 4 | |
| 16 | Hand walk | 4 | Preview. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | |

Lowest: 4.

## 0.5.2 (2026-11-25, AJ-E1558, Opus side-by-side, both themes)

Re-captured on the host from a clean export of HEAD (the shared tree held another agent's unbuilt Bot work), both themes, every changed capture diffed (changed-pixel count and box) and looked at old beside new in contact sheets; the ones named below also opened at full size. Mock-ups: `d126-rich-ui/kit-dark.png` and `notif-dark.png` (rich mode), `calm-ui/mock-r2-*.png` (the rail). Captures: The copy-settings line and sheet, both themes.

**What changed.** The sheet's header carries its icon tile; the From line leads with its chip; each offer leads with its kind's chip (MCP server, permissions, status line), the checkbox at the row's end; the set-up tags are their tint with no edge, a needed step amber words on the neutral tone (no amber fill).

**Judged.** Dark and light side by side: in dark the chips and the accent's soft step read clearly against the cards; in light the soft step is paler but the lit chip's violet glyph, the switch's accent fill and the chosen fill still mark what is on (two cues each, never colour alone). No outline came back (PAT-SURF): separation is the card's tone and spacing. Text keeps the type roles (14 px titles, 13 px help), AA in both themes (the gate's contrast spec). Kit rows; PAT-COLOR-2's warning chip.

| # | Line | Score | Note |
|---|---|---|---|
| 4 | Mode | 4 | Rich, the kit's dials: icons on every row and choice, the accent for on and chosen. |
| 6 | Areas | 4 | Tone, not outlines; no box in a box. |
| 7 | Colour | 4 | The accent means on, chosen, recommended or navigation; status keeps its hue (amber words on neutral, never a fill). |
| 14 | Consistency | 4 | The kit's own pieces (RowChip, the switch, ChoiceOption cards, SELECTED_FILL, Modal's icon tile). |
| 19 | Rich-mode colour | 4 | Before (42aab7661): 2, monochrome rows, bare checkboxes, a dot or a ✓ alone for chosen. Now every row and choice its icon, on and chosen in the accent's soft step, chosen a fill. |

The other lines are as the 0.5.1 critique scored them (unchanged by this work; lowest 4).
