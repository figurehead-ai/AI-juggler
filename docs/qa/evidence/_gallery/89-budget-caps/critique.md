<!-- captures: a295c79c24dd -->
# 89-budget-caps: monthly caps, setting one, its alerts on Home, and the caps on Usage

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

cap-sheet is a compact dialog (Name, Amount a month, Counts as two radios with a project checklist) with Save as one small solid violet button beside a plain Cancel, and the fields sit a step lighter than the dialog in dark; there the chosen "A group" radio and the ticked ai-juggler box now use the lighter accent. On home-needs the Needs you card puts the sign-in ask first (Sign in is the page's one violet button) and lists the two cap alerts below it as their own bordered rows, an amber triangle for Acme at 81 % and a red square for Side project over, each with a plain "Got it" and an outlined "See usage", and cards stand off the canvas in both themes. After the second pass the sign-in ask gives its detail on two lines, ending whole on "…AI Juggler never sees your password.", so the card grows and the two cap rows sit a line lower; the rail's Refactor the settings screen row keeps its time whole ("ai-ju… · viewed 2 min ago") here and on usage-caps. In cap-sheet only the blurred page behind the dialog moved. The rail still cuts both alerts' titles at the word that matters ("Acme is at 81% of its monthl…", "Side project is over its month…"), and the Acme row leaves an empty line below "See the cap" that the other Needs you rows don't. On usage-caps the Monthly caps card fills only the left half of the page with nothing to its right, the three cap cards leave Work alone on a second row, and the top of the capture slices the row above; the bars read well (amber in dark but brown in light for Acme, red past 100 %, violet for Work).

## Weak

- home-needs-1280x800-dark.png, home-needs-1280x800-light.png: rail titles of the two cap alerts are cut before "cap".
- home-needs-1280x800, usage-caps-1280x800 (both themes): the rail's Acme alert row has an empty line under "See the cap", so the Needs you rows are unevenly spaced.
- usage-caps-1280x800-dark.png, usage-caps-1280x800-light.png: the Monthly caps card takes half the width and leaves the right half of the page empty.
- usage-caps-1280x800-dark.png, usage-caps-1280x800-light.png: the row above Monthly caps is sliced at the top edge.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: usage-caps-1280x800 light, home-needs-1280x800 dark and cap-sheet-1280x800 dark (two renders), beside the 0.3.2 notes. Home is D-118's two columns in every capture: Needs you now as one card per need (the first card's primary the one filled button), the feed with its chips and Showing:, then Your apps and Runway by account on the right.

- **home-needs:** two cap needs as cards with their mark: "Acme is at 81% of its monthly cap — ≈$243 of $300 this month · ai-juggler, payments-api. Nothing is stopped." (amber) and "Side project is over its monthly cap" (red), each Got it · See usage; the rail says each once, whole ("… of its monthly cap", the 0.3.2 cut is gone).
- **usage-caps:** Monthly caps in two columns (Acme 81 %, Side project 122 %, Work 27 %), each bar in its meaning's colour, Edit · Remove, and the estimate's honest line; the 0.3.2 half-empty card is gone.
- **cap-sheet:** "Set a monthly cap — You'll hear at 80 % and 100 %. Nothing is stopped." Name, Amount a month (USD), Counts: One app / A group of projects, in any app (its projects as checkboxes), Save filled. Two renders differ only in the page scrolled behind the sheet; the tracked one kept.
- **Weak:** the projects' checkboxes sit in a bordered box under the radio (a box under a choice).

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Set a monthly money cap, hear at 80 % and 100 %, see it on Usage. |
| 2 | Prominence | 4 | Caps under Usage's table; their alerts in Needs you. |
| 3 | Default | 4 | Nothing stopped, said in every place. |
| 4 | Mode | 4 | Calm. |
| 5 | Mock-up | 4 | No mock; Usage's cards and Needs you's. |
| 6 | Areas | 4 | Two-column caps; the sheet's project box (weak). |
| 7 | Colour | 4 | Amber at 81 %, red over, accent for under: one meaning each. |
| 8 | Type | 4 | Amounts tabular; labels 12 px. |
| 9 | Actions | 5 | Save filled; Got it and See usage tinted. |
| 10 | Serious | 4 | Remove is reversible by setting again; nothing stops work. |
| 11 | Doors | 4 | See usage and See the cap open Usage at the caps. |
| 12 | Words | 4 | ≈ and "Some usage has no price yet, so it isn't counted.": honest numbers. |
| 13 | States | 4 | Said once in the rail, Home and the bell. |
| 14 | Consistency | 4 | Usage's card and the sheet kit. |
| 15 | Accessibility | 4 | Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | The PL1 walk (AJ-E2323) drove Home and the rail in Dev; not re-walked here. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | A newcomer sees it's an estimate and that nothing is stopped. |

Lowest: 4.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Behind the dialog the rail and Needs you are in their 0.5.1 style; the dialog itself is unchanged. Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.

## 0.5.2 (2026-11-25, AJ-E1558, Opus side-by-side, both themes)

Re-captured on the host from a clean export of HEAD (the shared tree held another agent's unbuilt Bot work), both themes, every changed capture diffed (changed-pixel count and box) and looked at old beside new in contact sheets; the ones named below also opened at full size. Mock-ups: `d126-rich-ui/kit-dark.png` and `notif-dark.png` (rich mode), `calm-ui/mock-r2-*.png` (the rail). Captures: cap-sheet at 1280×800, both themes (old beside new in the contact sheet).

**What changed.** The sheet's header carries its icon tile (a coin); One app and A group of projects are choice lines with icons, the chosen one a fill; the projects are the kit's check rows with chips on an inset tone, no outlined box.

**Judged.** Dark and light side by side: in dark the chips and the accent's soft step read clearly against the cards; in light the soft step is paler but the lit chip's violet glyph, the switch's accent fill and the chosen fill still mark what is on (two cues each, never colour alone). No outline came back (PAT-SURF): separation is the card's tone and spacing. Text keeps the type roles (14 px titles, 13 px help), AA in both themes (the gate's contrast spec). Kit dialog anatomy (§4.2) and rows.

| # | Line | Score | Note |
|---|---|---|---|
| 4 | Mode | 4 | Rich, the kit's dials: icons on every row and choice, the accent for on and chosen. |
| 6 | Areas | 4 | Tone, not outlines; no box in a box. |
| 7 | Colour | 4 | The accent means on, chosen, recommended or navigation; status keeps its hue (amber words on neutral, never a fill). |
| 14 | Consistency | 4 | The kit's own pieces (RowChip, the switch, ChoiceOption cards, SELECTED_FILL, Modal's icon tile). |
| 19 | Rich-mode colour | 4 | Before (42aab7661): 2, monochrome rows, bare checkboxes, a dot or a ✓ alone for chosen. Now every row and choice its icon, on and chosen in the accent's soft step, chosen a fill. |

The other lines are as the 0.5.1 critique scored them (unchanged by this work; lowest 4).
