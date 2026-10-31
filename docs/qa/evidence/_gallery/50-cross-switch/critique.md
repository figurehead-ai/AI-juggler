<!-- captures: 97285ad898a8 -->
# 50-cross-switch: continuing a session on another account or Mac: the target list, the switch wizard's steps, progress, conflicts and switching back

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

targets opens the switch list as a "Continue on another account · Now on Work Max" dialog over the session page, with accounts first (Codex, Lab) and then machines (Desktop Max and Personal on Studio Mac, Team on Mac mini), each with an amber "what carries" badge and a one-line readiness note; the capture opens scrolled to the other-machine list (a capture setup, not the app), so the top accounts are cut under the dialog header. The wizard dialogs (wizard-t2-ready, wizard-t2-blocked, path-mapping, carried, confirm) are well paced, with a step label, pill stepper, a large title and one decision per step, and the amber "Stays behind" box and the blocked version mismatch stand out properly. The design fix lands here: Continue and Switch now are now solid violet (the lighter accent in dark), so the next step clearly outranks Cancel and Back. The second pass carries it to the dialogs outside the wizard: Apply on Laptop in conflict-keep-both and Switch back to Laptop… in switch-back-preview are now the same solid violet primary, and in conflict-sheet, where no choice is made yet, "Choose one" is that primary faded (in dark a dim violet with grey text), so the dialog says it is waiting on a radio. progress still draws a focus outline around its "Switching to Team on Mac mini…" title, which looks like a stray box; its progress bar now takes the lighter violet in dark. conflict-sheet and resolved are clear and readable in both themes. Elsewhere the second pass changed only the dimmed session page behind the dialogs, whose terminal is now black to its card's edge. Every capture was re-rendered.

## Weak

- progress: a focus outline boxes the dialog title.
- targets: the dialog is titled "Continue on another account" but half of it is "Continue on another machine".
