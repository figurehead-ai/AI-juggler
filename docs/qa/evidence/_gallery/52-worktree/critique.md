<!-- captures: 63c103649cb6 -->
# 52-worktree: a session in its own git worktree, the offer to make one, and removing a worktree with uncommitted files

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

In chips the Where it runs card stands off both canvases, and it carries the worktree story clearly: "branch juggler/a1b2c3 · 3 uncommitted files · 2 ahead · 0 behind · own worktree", now wrapped over two lines inside the card, with a small Remove worktree… button. New session in the bar is solid violet. After the second pass the terminal's black fills its card to the edges (no lighter frame in dark, no white frame in light), with the text inset from the black's edge. The rail fix shows on the selected row, which now reads "Stopped with an error" whole. Under "More below", the Runway card's heading is still sliced in half by the column's bottom edge. The offer dialog explains the choice well: a highlighted "Its own worktree (recommended)" tile with the path, against "Share the checkout", and "Create the worktree and launch" is now solid violet (the lighter accent in dark), so it clearly outranks Cancel. In dirty-removal the type-to-confirm field has a strong violet focus ring and the warning explains what is lost, but Discard and remove is still a neutral grey outline. In offer and dirty-removal only the blurred terminal behind the dialog changed with the second pass. All eight captures were re-rendered.

## Weak

- chips-1280x800-dark/light: the Runway card's heading is cut in half by the right column's bottom edge just above "More below".
- chips-1280x800-dark/light: the selected rail row shows a small "···" under its status icon with no visible meaning.
- dirty-removal (all four): "Discard and remove" is a neutral grey outline; nothing marks it as destructive once it is enabled.
