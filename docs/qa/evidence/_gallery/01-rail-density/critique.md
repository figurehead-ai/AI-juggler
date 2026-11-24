<!-- captures: b38604a1d573 -->
# 01-rail-density: the session rail in its three densities (comfortable, compact, minimal icons), the minimal rail's tooltip, and a row marked stale

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

Comfortable shows two-line rows (name, coloured status · age); compact folds each row to one line with the age at the right, and minimal leaves a strip of tool icons with attention badges and a "!5" count. On the darker canvas the cards on Home stand off well and "Open its page" is the one solid violet button in the Needs you card; the lighter dark accent shows on it, on the logo mark, the Work Max runway bar and the rail's New session button. The error rows now read "Stopped with an error" with no cut age in comfortable-* and stale-row-1440x900. At 1024 px the top bar stays one row by showing the cleanup pill as a broom and "7" and dropping the This Mac line (comfortable-1024x768, compact-1024x768). In compact-1024x768 and compact-1440x900 the first row's name is still cut to "Fa…" and the third to "Fake D…" while the middle row shows "Fake Desk": the app tag and age squeeze the name. minimal-tooltip-1440x900 still shows no tooltip at all, so it cannot prove the tooltip it is named for. stale-row-1440x900 marks its row with a small amber "stale · 3m" chip after "Working", which reads clearly. All captures were re-rendered.

## Weak

- compact-1024x768, compact-1440x900: session names cut to "Fa…" and "Fake D…" by the app tag and age.
- minimal-tooltip-1440x900: no tooltip is visible.
- minimal-*: the minimal rail's icons are told apart only by tiny coloured corner dots and badges.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: comfortable-1440x900 and compact-1024x768 (new and the 0.3.2 ones), minimal-tooltip-1440x900 and stale-row-1440x900; the family is dark only, so light is judged from the same rows in 17-ledger and 53-parked. Beside the calm reference (`calm-ui/shots/01-rail_all-statuses-1440x900-*`) the rail now matches it: the mark and name past the traffic lights (F22), App · Project · Account lenses, a fixed dot gutter, two-line rows (title, then state · age) with the App's icon and the account's dot at the end (AJ-E1554), Home behind it in D-118's two columns.

- **What changed since 0.3.2:** comfortable rows read "Port the settings parser / Stopped with an error" with a red ✕ in the gutter, where 0.3.2 showed branch names ("feat/read-model") and a coloured status; compact is one line with the age at the end ("Port the setting… 7 h ago"); the Home behind is the calm two columns (Needs you now cards, the feed, Your apps, Runway by account) in place of the 0.3.2 cards; the dark canvas is a step lighter.
- **Weak:** the desktop row with its always-shown ⋯ cuts its state word mid-letter ("Stopped with an erı"), and in compact at 1024 the same row keeps its "app" chip, age and tools and leaves its title one letter ("R.."): logged **F55** (the fix re-renders most rail baselines). minimal-tooltip still shows no tip: the icons' only name is the browser's `title`, which no capture draws and which never shows on keyboard focus: logged **F61**. "Port the settings parser" carries the model-drift ⇄ pressed against its title's end.
- The captures predate F53 (1432a68d9): the chosen lens is now a solid neutral fill; they are re-rendered with F53's own round.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Three densities of one list; the stale row says "stale - 3m" in its line. |
| 2 | Prominence | 4 | Needs-you rows first inside each project, the state word muted unless it needs you (AJ-E3603). |
| 3 | Default | 4 | Comfortable by default; compact and minimal in Settings. |
| 4 | Mode | 4 | Calm: rows by tone, the selected one a fill, no lines. |
| 5 | Mock-up | 4 | Matches calm-ui's rail shot; the "app" chip is ours (D-100, the desktop app's session). |
| 6 | Areas | 4 | F55 fixed: the state word ends on "…" ("Stopped with an …"), and compact's desktop row keeps a readable title ("Rename the ph…"): its chip and age give way first. |
| 7 | Colour | 4 | Red ✕ only for the two that broke, amber for asks and limits, App and account hues once per row. |
| 8 | Type | 4 | Titles 13 px ink-2, state and age 12 px muted; tabular ages. |
| 9 | Actions | 5 | The rail has no primary; Home's Open its page is the one filled button. |
| 10 | Serious | n/a | Nothing destructive here. |
| 11 | Doors | 4 | Every row opens its session; ⋯ and right-click its one menu. |
| 12 | Words | 4 | Plain states (Stopped with an error, Probably stuck, Ended). |
| 13 | States | 4 | Stale said once in its row; the fixture is real-sized (13 sessions). |
| 14 | Consistency | 4 | The rail's row pattern everywhere (Needs you, Open now, project lens). |
| 15 | Accessibility | 4 | F61 fixed: the folded rail's icons name themselves in a drawn tip on hover and keyboard focus. Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | The PL1 walk (AJ-E2323) drove Home and the rail in Dev; not re-walked here. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | What's running and what needs you, one glance; the minimal rail's icons say their names on hover and focus. |

Lowest as first scored: 3 (lines 6 and 15: F55, F61, logged). No fix here: both re-render most rail baselines or need a drawn tip.

**Re-captured after F55 and F61 (2026-11-24, Opus):** all eight captures re-rendered and opened. comfortable-1024x768 and -1440x900: the desktop row beside its always-shown ⋯ reads "Stopped with an …" (it was cut mid-letter, "Stopped with an erı"). compact-1024x768: that row is "Rename the ph…" with its age and "app" chip given way (it was "R.."), and the other desktop rows drop the chip before their titles go under about ten letters ("Sketch the pricing… 10 h"). minimal-tooltip-1440x900 now shows a tip: "Group and View — show the list (⌘B)" drawn beside its icon, as it shows on keyboard focus too (it showed nothing). The chosen lens is F53's solid fill in every capture.

Lowest: 4 (lines 6 and 15 were 3: F55, F61 fixed).
