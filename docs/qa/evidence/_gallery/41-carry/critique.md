<!-- captures: dbf91912657f -->
<!-- captures: 41-carry -->
# 41-carry: the switch's advanced carry choice (AJ-E211, D-76) — the Disclosure on the wizard's Confirm step, closed and open, and the last-N choice's live words

Design critique (D-57, AJ-E2814). First critique of this family, written from the 2026-11-24 captures as they are.

## What reads well

The choice hides itself until it's worth offering: in the closed captures the quiet Disclosure ("Very long conversation? Leave out its start") sits under the "What happens" facts without shouting, and the confirm step's hierarchy is untouched — the solid-violet primary still dominates, the choice never competes with it. Open, the three radios read in order (Everything → before-a-date → last-N) with the date field and the number field inline beside their own radios, one control to a label; the number field is a sensible width and the live words line under them names the trade in plain language ("Carries the last 200 turns of 4,213"). At 390×844 the radios stack without the fields colliding, and the fieldset legend is screen-reader-only so sighted users get no duplicated heading. In dark the quiet tone keeps the panel off the canvas without a second border colour.

## Weak

- warned-*.png: with "Keep only the last" picked, the number field still shows the default 200 while the words say "of 4,213" — the pair reads fine, but nothing previews the bytes the task's Req.2 names (the size line lives on the switch's space line, not here; a future pass could echo "≈ 7.1 GB → 180 MB" beside the words).
- open-390x844-*.png: the date field's fixed w-40 and the number field's w-20 leave the "Leave out the turns before" radio row tighter than the others at the narrow width; wrapping lands the field under the radio label.
- closed-*.png: the Disclosure's chevron is the only affordance marking it expandable; at the quiet tone it can read as a footnote rather than an option. (Noted, not acted: the shared Disclosure is used identically elsewhere; changing it is a family-wide pass.)

None of these block the captures' purpose (the task's "the advanced options with each choice, the size line, the warning" — the choice's own states are complete and legible in both themes and both widths).
