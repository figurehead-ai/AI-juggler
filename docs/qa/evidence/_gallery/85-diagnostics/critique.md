<!-- captures: 6f9b1df627a7 -->
# 85-diagnostics: Settings › Advanced, previewing the diagnostics zip and the line after saving it

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

In preview the zip's contents sit in a nested panel ("What the zip holds") with about.json opened as a code block and the other two files as folds with their sizes, and Save… is the single solid violet button beside a plain Cancel; in dark it, New session and the logo mark now use the lighter accent, and the code block reads well on both the dark card and the light grey well. The "Backend response times" fold still sits after the Cancel and Save… row, inside the Diagnostics card, so it reads as an afterthought below the card's actions. In saved the result is one plain grey sentence ("Saved to …/AI Juggler diagnostics 2026-10-23.zip (5 KB). Attach it to your report.") with no mark, so success looks the same weight as the hints around it. All four captures still open scrolled so the Size limit row's label is sliced at the top edge, leaving "Automatic: 1.3 GB now…" and its picker without a name. Cards stand off the canvas in both themes and nothing overlaps. The second pass changed only the rail: the Refactor the settings screen row keeps its time whole ("ai-ju… · viewed 2 min ago" where it read "ai-juggler · viewed 2 mi…").

## Weak

- preview-1280x800-dark.png, preview-1280x800-light.png: "Backend response times" sits below the Save… row instead of above the actions.
- saved-1280x800-dark.png, saved-1280x800-light.png: the saved line has no success mark and reads as another hint.
- All four: the Size limit label is cut at the top edge.

## 0.5.1

Re-captured on 0.5.1; only the shared chrome changed in these screens. The rail's head now starts past the traffic lights, its rows hold a fixed dot column (tools on hover), cards and panels separate by tone instead of a border, a chosen item is a fill, and the top bar's bottom line is calmer. Diagnostics content is unchanged; the settings list gained AI and Models.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: The lens switch's chosen segment is a solid fill. Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
