<!-- captures: e0e74572c5d0 -->
# 80-marks: telling sixteen projects apart

Design critique (D-57, AJ-E2814) against the reference screens: the row menus (77-menus), the rail's order menu, Usage and Processes. The baseline note says what changed on purpose; this says whether it is good.

## Checklist

| Check | Finding |
|---|---|
| Grid | Marks sit in the rail's mark column; History's rows put tool, project and account marks in fixed columns. |
| Hierarchy | A project's colour and mark first, its name beside it. |
| Spacing | The rail's rhythm. |
| Icons | Real icons only where they are real images; otherwise a coloured monogram. |
| Shared controls | The mark sheet's choices are buttons in a grid. |
| Disabled reasons | Not applicable. |
| Plain words | Each mark names its project on hover and to screen readers. |
| Type sizes | Monograms 12 px and up. |
| Both themes | Checked dark and light; every colour keeps its contrast. |
| 1280×800 and 1024×700 | Captured at 1280; the folded column is 69-rail-narrow's. |

## Good

- Marks unchanged; the folded column has its Group and View door at the top.

## Weak

- rail-emoji-1280x800-light, rail-folded-1280x800-light: the dashed "Show 2 more apps" card is barely visible on the grey canvas.
- rail-emoji-1280x800, rail-folded-1280x800 (both themes): the rail rows' meta is cut at "Idle · outside AI Juggler ·…".

## Fixed

- Re-render: the folded column's door.

## Captures

| Capture | What is on it | What changed in 0.3 |
|---|---|---|
| rail-folded-1280x800 | The replica's rail: marks, App stacks and a folded project as a stripe in its colour. | The 0.3 header and grouping switch. |
| history-1280x800 | History's rows with each row's tool, project and account. | The 0.3 header. |
| sheet-1280x800, sheet-emoji-1280x800 | The mark sheet, then an emoji chosen on its colour. | × to close (AJ-E2813). |
| rail-emoji-1280x800 | The rail with the emoji mark. | The 0.3 header. |

## 0.3.2

The marks themselves are unchanged and still easy to tell apart: in rail-emoji and rail-folded each project's coloured square (or the rocket emoji on payments-api) sits in the same column with the stack of its Apps' coloured marks to its right (0.4: colour is the Apps', SPEC §9.3), the folded payments-api row keeps its coloured stripe, and History's rows keep project and account marks in fixed columns with the dates right-aligned. The mark sheet (sheet, sheet-emoji) sits a step above the dimmed page in dark and as a white sheet with a shadow in light; Save is the solid violet primary, the chosen colour has a clear ring, and the emoji field shows its focus ring. After the design fixes the dark accent is lighter: it shows on the logo mark, New session, History's focused search ring, and the emoji sheet's selected "An emoji" radio and field ring, all a touch brighter on the dark surfaces. The home behind rail-emoji and rail-folded has cards that lift off the canvas, but in light the dashed "Show 2 more apps" card still almost disappears on the grey canvas, and every Project-rail row cuts its meta at "Idle · outside AI Juggler ·…". The second pass touched only history's rail ages, which after its last fix stay whole beside short projects ("blog · 1 min ago", "infra · 1 min ago"); nothing else on these screens changed.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: sheet-emoji (dark), rail-folded (light), and the 0.3.2 notes. Sixteen projects told apart by colour and letters or an emoji: the sheet "Change payments-api's mark" (preview, twelve colour swatches, It shows: Letters / An emoji as two choice cards, the emoji field focused with its hint "Your Mac's emoji picker opens with Control-Command-Space"), and the rail with each project's mark, its Apps' dots and a "+4" overflow.

- **Since the last critique:** the chosen choice card is a fill ("An emoji"), the other a quiet outline; the sheet's foot is Cancel and the filled Save; the rail's rows are calm, and a folded project shows its stripe under its row (`rail-folded-stripe`).
- **Weak:** the chosen swatch is a white ring (a swatch has no other way to look chosen; kept). The Home behind the rail shows the empty feed's chips (F56, Home's).

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Pick a project's look once; it shows wherever the project does. |
| 2 | Prominence | 4 | Preview first, colour, then letters or emoji. |
| 3 | Default | 4 | Letters from the name by default; the colour from the project's slot. |
| 4 | Mode | 4 | Calm dialog on the overlay tone. |
| 5 | Mock-up | 4 | No mock of its own; the D-126 sheet kit. |
| 6 | Areas | 4 | Preview, colour, choice, field: four areas, more space between than inside. |
| 7 | Colour | 4 | The swatches are the mark hues (AJ-E1550's slots); accent on the focused field and Save only. |
| 8 | Type | 4 | Labels 13 px semibold, the hint 12 px muted. |
| 9 | Actions | 5 | Save, filled, last; Cancel beside it. |
| 10 | Serious | n/a | Nothing destructive; Cancel leaves it. |
| 11 | Doors | 4 | Change mark… from the project's ⋯; the rail updates in place. |
| 12 | Words | 4 | "It shows wherever the project does." |
| 13 | States | 4 | Sixteen real-sized projects in the rail. |
| 14 | Consistency | 4 | Choice cards as the pool setup's; swatches as Settings' colour picks. |
| 15 | Accessibility | 4 | Radio group and a labelled field; Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | The PL1 walk (AJ-E2323) drove Home and the rail in Dev; not re-walked here. |
| 17 | System prompts | 5 | None (the emoji picker is the user's own keys). |
| 18 | Usability | 4 | Many projects stay apart at a glance. |

Lowest: 4.

**Re-captured after F56 and F55 (2026-11-24, Opus):** all ten re-rendered; opened rail-folded-1280x800 dark and the sheet. The Home behind the rail shows the empty feed as its card alone (F56: no chips over "Quiet so far…"); the marks, the sheet and the history are unchanged. Lowest stays 4.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Behind the dialog the rail and Needs you are in their 0.5.1 style; the dialog itself is unchanged. Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
