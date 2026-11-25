<!-- captures: 940417b4e19f -->
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

## 0.5.1

Re-captured on 0.5.1; only the shared chrome changed in these screens. The rail's head now starts past the traffic lights, its rows hold a fixed dot column (tools on hover), cards and panels separate by tone instead of a border, a chosen item is a fill, and the top bar's bottom line is calmer. The switch-back dialog keeps its structure; the account is named Claude Max 20x in its first line.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Behind the dialog the rail and Needs you are in their 0.5.1 style; the dialog itself is unchanged. Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
