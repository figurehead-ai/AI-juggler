<!-- captures: e08a9219e8b7 -->
# 50-cross-switch: continuing a session on another account or Mac: the target list, the switch wizard's steps, progress, conflicts and switching back

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

targets now opens the switch list as a "Continue on another account · Now on Work Max" dialog over the session page, with accounts first (Codex, Lab) and then machines (Desktop Max and Personal on Studio Mac, Team on Mac mini), each with an amber "what carries" badge and a one-line readiness note; but it opens scrolled, so the Codex card's top is cut under the dialog header and the last Team card is cut at the bottom edge. The wizard dialogs (wizard-t2-ready, wizard-t2-blocked, path-mapping, carried, confirm) are well paced, with a step label, pill stepper, a large title and one decision per step, and the amber "Stays behind" box and the blocked version mismatch stand out properly. Their primary buttons (Continue, Switch now, Apply on Laptop, Switch back to Laptop…) are still a pale outlined violet, not the solid violet primary used everywhere else in 0.3.2, so the next step looks secondary. progress, new in this round, draws a focus outline around its "Switching to Team on Mac mini…" title, which looks like a stray box. conflict-sheet, conflict-keep-both, switch-back-preview and resolved are clear and readable in both themes. Every capture was re-rendered.

## Weak

- targets: the dialog opens scrolled; the Codex card's top is hidden under the header and the Team card is cut at the bottom.
- wizard-t2-ready, wizard-t2-blocked, path-mapping, carried, confirm, conflict-keep-both, switch-back-preview: the primary action is a pale outlined button, not the solid violet primary.
- progress: a focus outline boxes the dialog title.
- targets: the dialog is titled "Continue on another account" but half of it is "Continue on another machine".
