<!-- captures: 42ca8ed82a22 -->
# 85-diagnostics: Settings › Advanced, previewing the diagnostics zip and the line after saving it

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

In preview the zip's contents sit in a nested panel ("What the zip holds") with about.json opened as a code block and the other two files as folds with their sizes, and Save… is the single solid violet button beside a plain Cancel; the code block reads well on both the dark card and the light grey well. The new "Backend response times" fold sits after the Cancel and Save… row, still inside the Diagnostics card, so it reads as an afterthought below the card's actions. In saved the result is one plain grey sentence ("Saved to ~/Desktop/AI Juggler diagnostics 2026-10-23.zip (5 KB). Attach it to your report.") with no mark, so success looks the same weight as the hints around it. All four captures open scrolled so the Size limit row's label is sliced at the top edge, leaving "Automatic: 1.3 GB now…" and its picker without a name. Cards stand off the canvas in both themes and nothing overlaps.

## Weak

- preview-1280x800-dark.png, preview-1280x800-light.png: "Backend response times" sits below the Save… row instead of above the actions.
- saved-1280x800-dark.png, saved-1280x800-light.png: the saved line has no success mark and reads as another hint.
- All four: the Size limit label is cut at the top edge.
