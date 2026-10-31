<!-- captures: 4e8c9775a33e -->
# 37-flows: the first-run "bring in your accounts" detour and the New project dialog's two steps

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The detour page (b-detour) has a clear order: a two-step tracker, a large title, a brighter lead sentence, the setups table grouped "The default setup" and "Other Claude setups", the two radio cards with the recommended one ringed, then one solid violet "Bring in 1 account". In light the table and option cards are white on the grey canvas; in dark the table is a step lighter than the page. The New project dialog (b1-tiles, b2-account) is a step lighter than the dimmed page, the chosen ai-juggler tile is tinted, and "Start session →" is now a solid violet fill that clearly outranks Change and Close. Phone width is where it breaks: in b-detour-390x844 the setups table runs off the right edge, cutting "Claude Code 2.1.296 inst…", "Sign…" and "Needs…" and shrinking the names to "De…" and "cl…", and in b2-account-390x844 the account row squeezes "Work Max", "Deep work" and "last used here" into two- and three-line stacks. The tile hints are cut too ("last used with Wor…") at 390. All captures were re-rendered.

## Weak

- b-detour-390x844-*: the setups table overflows the page; the version, "Signed in" and "Needs sign-in" are cut at the right edge and the names shrink to "De…" and "cl…".
- b-detour-390x844-*: the step tracker wraps, leaving a stray connector line before "2 Sign in where needed".
- b2-account-390x844-*: the Work Max row wraps its name, profile and "last used here" into narrow stacks beside the "quota unknown" chip.
- b1-tiles-390x844-*, b2-account-390x844-*: the tile hints are cut ("last used with Wor…", "Fak…", "Per…").
