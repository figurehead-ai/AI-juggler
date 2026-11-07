<!-- captures: f4bbe9e8cb42 -->
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

## 0.4.1 re-score (2026-11-07, AJ-E3305)

The whole family was retaken on release/0.4.1 for the 14-line re-score (AJ-E3305, `docs/qa/0.4-walks/design.md`) and opened on contact sheets, both themes, 1280 and 1024. Shared with every screen since the last shoot: the header's New session is tinted (one filled button per view), the Floor button is a split control with its own menu, This Mac's line is its dot below 1120 px, and the rail's App headers put the profile first.

Unchanged in design: "5 go along · the first 800 turns stay behind" in amber, the fold open, "Switch to CLI" (design.md, under 4, item 4, on "CLI").
