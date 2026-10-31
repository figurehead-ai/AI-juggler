<!-- captures: 9520f48c8318 -->
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
| rail-folded-1280x800 | The replica's rail: marks, account stacks and a folded project as a stripe in its colour. | The 0.3 header and grouping switch. |
| history-1280x800 | History's rows with each row's tool, project and account. | The 0.3 header. |
| sheet-1280x800, sheet-emoji-1280x800 | The mark sheet, then an emoji chosen on its colour. | × to close (AJ-E2813). |
| rail-emoji-1280x800 | The rail with the emoji mark. | The 0.3 header. |

## 0.3.2

The marks themselves are unchanged and still easy to tell apart: in rail-emoji and rail-folded each project's coloured square (or the rocket emoji on payments-api) sits in the same column with the account-dot stack to its right, the folded payments-api row keeps its coloured stripe, and History's rows keep project and account marks in fixed columns with the dates right-aligned. The mark sheet (sheet, sheet-emoji) sits a step above the dimmed page in dark and as a white sheet with a shadow in light; Save is the solid violet primary, the chosen colour has a clear ring, and the emoji field shows its focus ring. After the design fixes the dark accent is lighter: it shows on the logo mark, New session, History's focused search ring, and the emoji sheet's selected "An emoji" radio and field ring, all a touch brighter on the dark surfaces. The home behind rail-emoji and rail-folded has cards that lift off the canvas, but in light the dashed "Show 2 more apps" card still almost disappears on the grey canvas, and every Project-rail row cuts its meta at "Idle · outside AI Juggler ·…". The second pass touched only history's rail ages, which after its last fix are cut beside short projects ("blog · 1 min …", "infra · 1 min a…"); nothing else on these screens changed.
