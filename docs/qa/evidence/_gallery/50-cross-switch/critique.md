<!-- captures: bae61129df7b -->
# 50-cross-switch: continuing a session on another account or Mac: the target list, the switch wizard's steps, progress, conflicts and switching back

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

targets opens the switch list as a "Continue on another account · Now on Work Max" dialog over the session page, with accounts first (Codex, Lab) and then machines (Desktop Max and Personal on Studio Mac, Team on Mac mini), each with an amber "what carries" badge and a one-line readiness note; the capture opens scrolled to the other-machine list (a capture setup, not the app), so the top accounts are cut under the dialog header. The wizard dialogs (wizard-t2-ready, wizard-t2-blocked, path-mapping, carried, confirm) are well paced, with a step label, pill stepper, a large title and one decision per step, and the amber "Stays behind" box and the blocked version mismatch stand out properly. The design fix lands here: Continue and Switch now are now solid violet (the lighter accent in dark), so the next step clearly outranks Cancel and Back. The second pass carries it to the dialogs outside the wizard: Apply on Laptop in conflict-keep-both and Switch back to Laptop… in switch-back-preview are now the same solid violet primary, and in conflict-sheet, where no choice is made yet, "Choose one" is that primary faded (in dark a dim violet with grey text), so the dialog says it is waiting on a radio. progress still draws a focus outline around its "Switching to Team on Mac mini…" title, which looks like a stray box; its progress bar now takes the lighter violet in dark. conflict-sheet and resolved are clear and readable in both themes. Elsewhere the second pass changed only the dimmed session page behind the dialogs, whose terminal is now black to its card's edge. Every capture was re-rendered.

## Weak

- progress: a focus outline boxes the dialog title.
- targets: the dialog is titled "Continue on another account" but half of it is "Continue on another machine".

## 0.3.5

Re-shot with the switch dialog's shared frame. The step pills are tinted now: done steps green with a check, the current one accent (confirm: Target, Project, What's carried, then Confirm). The steps themselves are unchanged. A switch to another Mac continues there in a terminal, as before.

Its last button names where it continues, as on this Mac: "Switch to CLI on Mac mini" (it was "Switch now"); a session on another Mac starts there in a terminal.

## AJ-E710 retake (2026-11-07)

Retaken on release/0.4.1 for the Fleet gate's screenshot set (AJ-E710), both themes, every size the family has. First pass by a Sonnet agent against a written sheet; the points marked (checked) were confirmed by eye by Opus.

The dialog is titled "Continue in another app · Now in Claude Code · Work Max" (the earlier notes quote "Continue on another account"); badges "Same conversation, another app · Not proven yet"; the confirm step's stepper tinted with checks and one filled "Switch to CLI on Mac mini". The keyboard sweep reached every step and the final button by Tab (Esc closed it). 1280 only.
