<!-- captures: 106aee0eb1d0 -->
# 52-worktree: a session in its own git worktree, the offer to make one, and removing a worktree with uncommitted files

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

In chips the Where it runs card now stands off both canvases, and it carries the worktree story clearly: "branch juggler/a1b2c3 · 3 uncommitted files · 2 ahead · 0 behind · own worktree" with a small Remove worktree… button. New session in the bar is solid violet. Under "More below", the Runway card's heading is sliced in half by the column's bottom edge, and the selected rail row cuts its age to "Stopped with an error · 7…". The offer dialog explains the choice well: a highlighted "Its own worktree (recommended)" tile with the path, against "Share the checkout". But "Create the worktree and launch" is still a pale lavender outline, not the solid violet 0.3.2 gives primaries elsewhere. In dirty-removal, the type-to-confirm field has a strong violet focus ring, and the warning explains what is lost. All eight captures were re-rendered.

## Weak

- offer-1280x800-dark/light: the primary "Create the worktree and launch" is a pale lavender outline that barely outranks Cancel; it did not get the solid violet fill.
- chips-1280x800-dark/light: the Runway card's heading is cut in half by the right column's bottom edge just above "More below".
- chips-1280x800-dark/light: the selected rail row reads "Stopped with an error · 7…", so the age is cut off.
- dirty-removal (all four): "Discard and remove" is a neutral grey outline; nothing marks it as destructive once it is enabled.
