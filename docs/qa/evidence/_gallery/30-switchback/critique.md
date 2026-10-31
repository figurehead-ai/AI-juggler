<!-- captures: deff39d12846 -->
# 30-switchback: switching back to the original account after a reset: the switch card, the review dialog with conflicts, the both-live warning, handover notes, and a return that could not complete

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

The Switch back to Work Max dialog orders its content well (a mode chip, a short explanation, a recap box of what happened while away, Conflicts with radio choices per file, then what to do with the away session) and the dark panel and its inner boxes separate cleanly from the dimmed page; in dark the mode chip and the selected radio now carry the lighter violet. The body text is small and the recap box looks like a raw text dump; in sf7 the turns are labelled "[user]" and "[assistant]" where both-live says "You:" and "Personal:", sf7 says "0 file conflicts need your choice", and both-live and review say "1 file conflicts". At 800x600 both-live and review still run off the bottom with no visible footer, so Resume and Cancel can't be seen, and in both-live-800x600 the dialog has scrolled so its title sits against the top border; the page behind now keeps its top bar on one row with the cleanup pill as a broom and "7". After the second pass "Resume Work Max" is the solid violet primary like the other switch dialogs: full strength in sf7, where nothing waits on a choice, with its "(⏎)" hint in the label's own white, and faded in both-live and review at 1280 while the todos.json conflict still needs a choice, which reads as intentionally disabled rather than as a secondary button. sf8-forked is a dead end: it names "Work and Personal" (the account is Work Max) and offers only Close. In card the session page and terminal read cleanly in both themes, but the "Continue on another account" card is only a heading at the fold in 1280x800 and absent at 800x600; the rail's status lines now show whole ("Stopped with an error"). Behind the dialogs and in card, the terminal's black now runs to its card's edges, with black padding before the text.

## Weak

- both-live-800x600-*.png, review-800x600-*.png: the footer with Resume and Cancel is off-screen.
- both-live-800x600-dark.png, both-live-800x600-light.png: the title row is pressed against the dialog's top border.
- sf7-*.png: "[user]" / "[assistant]" labels in the recap; "0 file conflicts need your choice".
- both-live-*.png, review-*.png: "1 file conflicts need your choice".
- sf8-forked-*.png: "Work" instead of Work Max; only Close, no way to resolve the conflict from here.
- card-800x600-*.png: the switch card the capture is named for is not in view.
- card-1280x800-dark.png, card-1280x800-light.png: a stray grey "…" sits under the clock icon of the selected rail row (feat/read-model, Working · 1 h).
