<!-- captures: 02b5b9c189bf -->
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

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B2)

Re-captured after F53 (last-1280 changed only at the rail's lens). The fold "Very long conversation? Leave out its start" sits under What happens on the Confirm step, open: Everything (as usual), Leave out the turns before (Choose a day), Keep only the last 200 turns (chosen), then "Carries the last 200 turns (8.4 MB) and leaves out 800 turns. It starts on Sep 30, 2024." and the loss line in amber.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Leave out a long start, knowing what is lost. |
| 2 | Prominence | 5 | Folded by default. |
| 3 | Default | 5 | Everything. |
| 4 | Mode | 4 | Rich step, plain radios inside the fold. |
| 5 | Mock-up | 4 | No mock. |
| 6 | Areas | 4 | |
| 7 | Colour | 4 | The loss line is amber words (a caution, not an action). |
| 8 | Type | 4 | |
| 9 | Actions | 5 | |
| 10 | Serious | 5 | What is lost said before the switch. |
| 11 | Doors | n/a | |
| 12 | Words | 4 | "Sep 30, 2024" beside "before 1 October": the second is the gallery's fixture; the backend says "before then". |
| 13 | States | 4 | |
| 14 | Consistency | 4 | |
| 15 | Accessibility | 4 | The count field is labelled. |
| 16 | Hand walk | 4 | Preview. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | |

Lowest: 4.
