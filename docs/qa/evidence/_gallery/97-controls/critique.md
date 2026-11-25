<!-- captures: 4bb6759d3a12 -->
# 97-controls: the shared controls and the rail's View

Design critique (D-57, AJ-E2814) against the reference screens: the row menus (77-menus), the rail's order menu, Usage and Processes. The baseline note says what changed on purpose; this says whether it is good.

## Checklist

| Check | Finding |
|---|---|
| Grid | Every line is the shared choice line: icon column, label, ✓ column; Only's values sit in the ✓ column with a chevron. |
| Hierarchy | "View" heading, then Show and Only as small section names, never lines you pick. |
| Spacing | This branch: every View line is one height. |
| Icons | Every Show and Only line has its icon, in the app's stroke style. |
| Shared controls | No system select, radio or checkbox: the View is a menu of choice lines; Settings › General uses the shared radio cards; History's filters are the shared Picker. |
| Disabled reasons | Reset shows only when something changed instead of sitting greyed. |
| Plain words | "Quiet ones", "Background runs", "Other Macs"; hints say what each is. |
| Type sizes | Labels 14 px, hints and section names 12 px. |
| Both themes | Checked dark and light; the grouping switch's chosen segment is faint in light. |
| 1280×800 and 1024×700 | At 1024 the list is opened first (the folded column's Group and View door). |

## Good

- The View is a real menu: icon · label · hint · ✓ on one grid, Only as submenus with ✓ on the chosen value.
- Settings › General's equal cards and shared radios.

## Weak

- Hints sat on some lines and not others (Inactive had one, Ended today, Other Macs and Archived didn't), so rows had two heights (fixed in this branch).
- The menu covers the grouping switch beside it while open.
- History's Picker open wasn't captured (fixed in this branch).
- Second re-render, seen by eye on picker-open-history: History's rows stopped with an error said "7 h" and "12 h" beside their neighbours' "5 h ago" (fixed, below).
- settings-general-cards-1024x700: the three order cards squeeze their hints to one or two words a line ("Stays put. / New ones / join at the / end.") and "Reset my order" wraps onto two lines inside its pill.
- view-open, view-changed-only-open (both widths): the page heading "5 sessions need you" peeks out half-covered above the View menu's top edge.

## Fixed

- Re-render: the grouping switch's segments size by their words (Pool cut "Account" to "Accou…").
- Polish (0.3, this branch): the View's rows are one height; a hint sits on the label's line, muted, in shorter words, only where the label needs it.
- Polish (0.3, this branch): History's Picker open is captured (the fixture always had three projects; the state counted the filters before the lazily loaded screen drew, and skipped).

- Second re-render (seen by eye): a session stopped with an error has ended, so its row says "7 h ago" like every other ended row (History, a project's Sessions); it was timed like a live one ("7 h"). Vitest: "says \"ago\" for every ended session, one stopped with an error too".

## Captures

| Capture | What is on it | What changed in 0.3 |
|---|---|---|
| rail-group-switch-1280x800, -1024x700 | The rail's App · Project · Account · Pool switch with Project chosen (a pool of yours exists). | New (AJ-E2802). |
| view-open-1280x800, -1024x700 | The View as it opens: Show's seven lines, Only's Account and Project, no Reset. | New menu (AJ-E2801, AJ-E2802). This branch: one row height. |
| view-changed-only-open-1280x800, -1024x700 | Quiet ones turned on, Reset shown, Only › Account's submenu open with ✓ on All. | New. This branch: one row height. |
| picker-open-history-1280x800, -1024x700 | History with its Project filter open: "Project" over All ✓ (focused), ai-juggler, storefront, sacrificial, under the Project: All and Profile: All chips. | New state (this branch): the spec counted the filters before History had drawn and skipped. |
| settings-general-cards-1280x800, -1024x700 | Settings › General: theme, density and order as equal cards with the shared radio. | New (AJ-E2813). |

## 0.3.2

The View menu (view-open, view-changed-only-open) and History's Project picker (picker-open-history) float on a lifted surface over the darker canvas, and in light the white menus separate from the grey page. The picker changed most this round: it now opens right under its "Project: All" chip, only as wide as its longest name, with "All" ✓ on top, then ai-juggler, storefront and sacrificial (it used to hang wider and to the left of the chip). In dark the lighter accent shows on the header's New session, the logo mark, the ✓ marks, the Runway bar and Measure its limits; light changes only by a pixel in the This Mac line. The top bar stays one row at both widths: at 1280 with "This Mac · Plenty of room" and "7 to clean up", at 1024 with the cleanup pill shortened to a broom and "7" and the This Mac line left out. rail-group-switch shows the rail age fix: the Project rail's "Stopped with an error" and "Limit reached · 0% left" now end whole instead of on a cut-off "· 7…", while rows with room still show their age ("Asks you · 2 h", "Probably stuck · 3 h"). In settings-general-cards the chosen Light or Dark, Comfortable and Your order cards are a clearly tinted, outlined step above their neighbours in both themes, but at 1024×700 the three order cards still squeeze their hints to one or two words a line and "Reset my order" wraps inside its pill. In the View captures the page heading "5 sessions need you" still peeks out half-covered above the menu's top edge. After the second pass the 1280 captures' rail feat/read-model row keeps its age whole ("ai-j… · 3 h ago" where it read "ai-jugg… · 3 …"); nothing else changed.

## 0.4.1 (E28, 2026-11-07): control-states

- **control-states-1280x800** (both themes, new): the row order's three choices disabled as while a change saves. "Your order" stays on with a muted fill, and the other two are off with dashed rings, so "on but can't change now" reads apart from "off". The chosen theme, reached with Tab, wears the app's focus ring (AJ-E2801 R2).
- Weak: a disabled line's label keeps full ink, so the ring alone says it can't change; its reason is the hint, read by screen readers.

## 0.5.1

Re-captured on 0.5.1; only the shared chrome changed in these screens. The rail's head now starts past the traffic lights, its rows hold a fixed dot column (tools on hover), cards and panels separate by tone instead of a border, a chosen item is a fill, and the top bar's bottom line is calmer. Settings General is unchanged; the list gained AI and Models.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Needs you's cards are tone fills with their level's mark (a stopped one in the broken tone), the most urgent first (F53, F66, F80). Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.

## 0.5.2 (2026-11-25, AJ-E1558, Opus side-by-side, both themes)

Re-captured on the host from a clean export of HEAD (the shared tree held another agent's unbuilt Bot work), both themes, every changed capture diffed (changed-pixel count and box) and looked at old beside new in contact sheets; the ones named below also opened at full size. Mock-ups: `d126-rich-ui/kit-dark.png` and `notif-dark.png` (rich mode), `calm-ui/mock-r2-*.png` (the rail). Captures: Every control state at 1024×700 and 1280×800, both themes (old beside new in the contact sheets; picker-open-history dark also at full size before the change).

**What changed.** General's theme and session-list cards each lead with an icon chip and the chosen one is a fill with its chip lit; a picker's or menu's chosen line (the View menu's Account › All, History's Project › All) is now a fill as well as its ✓, never the mark alone; the rail's Needs you rows lose their App dot (AJ-E1557's amendment), the level's mark staying before why.

**Judged.** Dark and light side by side: in dark the chips and the accent's soft step read clearly against the cards; in light the soft step is paler but the lit chip's violet glyph, the switch's accent fill and the chosen fill still mark what is on (two cues each, never colour alone). No outline came back (PAT-SURF): separation is the card's tone and spacing. Text keeps the type roles (14 px titles, 13 px help), AA in both themes (the gate's contrast spec). Matches the kit's RadioCard and menu choice lines.

| # | Line | Score | Note |
|---|---|---|---|
| 4 | Mode | 4 | Rich, the kit's dials: icons on every row and choice, the accent for on and chosen. |
| 6 | Areas | 4 | Tone, not outlines; no box in a box. |
| 7 | Colour | 4 | The accent means on, chosen, recommended or navigation; status keeps its hue (amber words on neutral, never a fill). |
| 14 | Consistency | 4 | The kit's own pieces (RowChip, the switch, ChoiceOption cards, SELECTED_FILL, Modal's icon tile). |
| 19 | Rich-mode colour | 4 | Before (42aab7661): 2, monochrome rows, bare checkboxes, a dot or a ✓ alone for chosen. Now every row and choice its icon, on and chosen in the accent's soft step, chosen a fill. |

The other lines are as the 0.5.1 critique scored them (unchanged by this work; lowest 4).
