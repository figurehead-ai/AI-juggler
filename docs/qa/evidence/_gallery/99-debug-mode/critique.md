<!-- captures: 9c969a011e06 -->
<!-- captures: pending -->
# 99-debug-mode: Debug mode

Design critique (D-57) against Settings › Advanced (65-settings-sections). New family (AJ-E2912).

## Checklist

| Check | Finding |
|---|---|
| Grid | A settings card: the switch row, then the log row with Show and Clear. |
| Hierarchy | Section title, the switch's one-line promise, the log's state. |
| Spacing | The settings cards' rhythm. |
| Icons | None. |
| Shared controls | The shared switch and Buttons. |
| Disabled reasons | Show and Clear are greyed while the log is empty, and the row says "Empty". |
| Plain words | "Writes down each step of every switch and launch: sizes, timings and the tools' own errors. Never what's in a conversation." |
| Type sizes | 14 px names, 12 px explanations. |
| Both themes | Checked dark and light. |
| 1280×800 and 1024×700 | Both. |

## Good

- What it records and what it never records are both said beside the switch.

## Weak

- The log's size limit (16 MB) and its seven-day off aren't said on the card.

## Fixed

- Nothing on first render.
