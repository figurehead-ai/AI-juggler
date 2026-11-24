<!-- captures: c822700e4c96 -->
# 37-flows: the first-run "bring in your accounts" detour and the New project dialog's two steps

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

The detour page (b-detour) has a clear order: a two-step tracker, a large title, a brighter lead sentence, the setups table grouped "The default setup" and "Other Claude setups", the two radio cards with the recommended one ringed, then one solid violet "Bring in 1 account". In light the table and option cards are white on the grey canvas; in dark the table is a step lighter than the page, and the lighter dark accent now shows on the step-1 dot, the checked Default box, the ringed "Use them where they are" card, the "Bring in 1 account" fill and the terms links. The New project dialog (b1-tiles, b2-account) is a step lighter than the dimmed page, the chosen ai-juggler tile is tinted, and "Start session →" is a solid violet fill that clearly outranks Change and Close. At 390 the blurred page behind the dialog changed only in its top bar (the cleanup pill is now the short broom form) and, in the second pass, below the dialog, where the Home page's blocks moved down a line; the dialog itself is as before. Phone width is still where the dialogs and the detour break: in b-detour-390x844 the setups table runs off the right edge, cutting "Claude Code 2.1.296 inst…", "Sign…" and "Needs…" and shrinking the names to "De…" and "cl…", and in b2-account-390x844 the account row squeezes "Work Max", "Deep work" and "last used here" into two- and three-line stacks. The tile hints are cut too ("last used with Wor…") at 390. All captures were re-rendered.

## Weak

- b-detour-390x844-*: the setups table overflows the page; the version, "Signed in" and "Needs sign-in" are cut at the right edge and the names shrink to "De…" and "cl…".
- b-detour-390x844-*: the step tracker wraps, leaving a stray connector line before "2 Sign in where needed".
- b2-account-390x844-*: the Work Max row wraps its name, profile and "last used here" into narrow stacks beside the "quota unknown" chip.
- b1-tiles-390x844-*, b2-account-390x844-*: the tile hints are cut ("last used with Wor…", "Fak…", "Per…").

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B2)

Re-captured after F53, 1280, 1024 and 390, both themes. b-detour is the setup wizard's "You already have 2 Claude Code apps on this computer." (Bring in your apps): the default app's group and Other Claude Code apps with their checkboxes, More options open (Use them where they are · Recommended, chosen; Make separate copies), Where we looked, then Set up a new app instead, Add from another folder…, Look again and the one primary "Bring in 1 app", Providers' terms folded (PAT-SER-7). b1-tiles and b2-account are the New project dialog.

| # | Line | Detour | New project | Note |
|---|---|---|---|---|
| 1 | Job | 5 | 4 | |
| 2 | Prominence | 4 | 4 | |
| 3 | Default | 5 | 4 | The chosen folder tile ringed; the App last used. |
| 4 | Mode | 4 | 3 | New project predates New session's stated choice: tiles, a bare App row, the primary mid-body: F68. |
| 5 | Mock-up | 4 | 3 | F68. |
| 6 | Areas | 4 | 4 | |
| 7 | Colour | 4 | 4 | "Recommended" plain words, not the accent tag (PAT-SURF-5). |
| 8 | Type | 4 | 4 | |
| 9 | Actions | 5 | 3 | "Start session →" sits mid-body under Step 2, Close alone in the footer (BTN-1): F68. |
| 10 | Serious | 4 | n/a | |
| 11 | Doors | 4 | 4 | |
| 12 | Words | 4 | 3 | The App named by its profile alone ("Work Max"), a "quota unknown" chip (unknown room says nothing elsewhere), "Deep work": F68. |
| 13 | States | 4 | 4 | |
| 14 | Consistency | 4 | 3 | F68. |
| 15 | Accessibility | 4 | 4 | |
| 16 | Hand walk | 4 | 4 | Preview. |
| 17 | System prompts | 5 | 5 | None here (the wizard's Mac access step primes its own). |
| 18 | Usability | 4 | 4 | |

Lowest: 3 (New project, lines 4, 5, 9, 12, 14), logged as F68.
