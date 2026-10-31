<!-- captures: cebb4f5e9395 -->
# 01-rail-density: the session rail in its three densities (comfortable, compact, minimal icons), the minimal rail's tooltip, and a row marked stale

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

Comfortable shows two-line rows (name, coloured status · age); compact folds each row to one line with the age at the right, and minimal leaves a strip of tool icons with attention badges and a "!5" count. On the darker canvas the cards on Home stand off well and "Open its page" is the one solid violet button in the Needs you card; the lighter dark accent shows on it, on the logo mark, the Work Max runway bar and the rail's New session button. The error rows now read "Stopped with an error" with no cut age in comfortable-* and stale-row-1440x900. At 1024 px the top bar stays one row by showing the cleanup pill as a broom and "7" and dropping the This Mac line (comfortable-1024x768, compact-1024x768). In compact-1024x768 and compact-1440x900 the first row's name is still cut to "Fa…" and the third to "Fake D…" while the middle row shows "Fake Desk": the app tag and age squeeze the name. minimal-tooltip-1440x900 still shows no tooltip at all, so it cannot prove the tooltip it is named for. stale-row-1440x900 marks its row with a small amber "stale · 3m" chip after "Working", which reads clearly. All captures were re-rendered.

## Weak

- compact-1024x768, compact-1440x900: session names cut to "Fa…" and "Fake D…" by the app tag and age.
- minimal-tooltip-1440x900: no tooltip is visible.
- minimal-*: the minimal rail's icons are told apart only by tiny coloured corner dots and badges.
