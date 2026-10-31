<!-- captures: 6ece0838fcbc -->
# 37-flows: the first-run "bring in your accounts" detour and the New project dialog's two steps

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

The detour page (b-detour) has a clear order: a two-step tracker, a large title, a brighter lead sentence, the setups table grouped "The default setup" and "Other Claude setups", the two radio cards with the recommended one ringed, then one solid violet "Bring in 1 account". In light the table and option cards are white on the grey canvas; in dark the table is a step lighter than the page, and the lighter dark accent now shows on the step-1 dot, the checked Default box, the ringed "Use them where they are" card, the "Bring in 1 account" fill and the terms links. The New project dialog (b1-tiles, b2-account) is a step lighter than the dimmed page, the chosen ai-juggler tile is tinted, and "Start session →" is a solid violet fill that clearly outranks Change and Close. At 390 the blurred page behind the dialog changed only in its top bar (the cleanup pill is now the short broom form) and, in the second pass, below the dialog, where the Home page's blocks moved down a line; the dialog itself is as before. Phone width is still where the dialogs and the detour break: in b-detour-390x844 the setups table runs off the right edge, cutting "Claude Code 2.1.296 inst…", "Sign…" and "Needs…" and shrinking the names to "De…" and "cl…", and in b2-account-390x844 the account row squeezes "Work Max", "Deep work" and "last used here" into two- and three-line stacks. The tile hints are cut too ("last used with Wor…") at 390. All captures were re-rendered.

## Weak

- b-detour-390x844-*: the setups table overflows the page; the version, "Signed in" and "Needs sign-in" are cut at the right edge and the names shrink to "De…" and "cl…".
- b-detour-390x844-*: the step tracker wraps, leaving a stray connector line before "2 Sign in where needed".
- b2-account-390x844-*: the Work Max row wraps its name, profile and "last used here" into narrow stacks beside the "quota unknown" chip.
- b1-tiles-390x844-*, b2-account-390x844-*: the tile hints are cut ("last used with Wor…", "Fak…", "Per…").
