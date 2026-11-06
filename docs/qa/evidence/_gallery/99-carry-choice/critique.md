<!-- captures: 790132cd0d5b -->
<!-- captures: pending -->
# 99-carry-choice: the switch's "leave out the start" choice

Design critique (D-57) against the switch's confirm step (28-switch, 96-switch-setup). New family (D-76, AJ-E211).

## Checklist

| Check | Finding |
|---|---|
| Grid | A fold under the carry summary on the confirm step, three radio rows, the outcome under them. |
| Hierarchy | The fold closed by default, its label a question; inside, the choices, then what it carries, then what's lost in amber. |
| Spacing | The confirm step's rhythm; the date and number fields sit inline in their rows. |
| Icons | The fold's chevron only. |
| Shared controls | The shared Disclosure, Radio and fields. |
| Disabled reasons | n/a: the fold isn't offered where the switch can't leave part out, or the conversation is short. |
| Plain words | "Carries the last 200 turns (8.4 MB) and leaves out 800 turns.", "The new session won't know what was said or decided before 1 October." |
| Type sizes | 14 px choices, 13 px outcome lines. |
| Both themes | Checked dark and light. |
| 1280×800 and 1024×700 | Both. |

## Good

- Everything stays the default; the choice is one fold away, never in the way.
- What's lost is said before the switch, in the reader's words.

## Weak

- The two dates read in two formats ("Sep 30, 2024" and "1 October").
- The date field shows the browser's mm/dd/yyyy placeholder.

## Fixed

- Two components shared the test id `switch-carry`; the choice is `switch-carry-choice` now.
- The gallery's fixture said 7.8 GB beside the dialog's 42 MB; it agrees now.
