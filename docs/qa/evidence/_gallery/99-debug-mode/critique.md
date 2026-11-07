<!-- captures: b9af13cf39dc -->
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

## 0.4.1 re-score (2026-11-07, AJ-E3305)

The whole family was retaken on release/0.4.1 for the 14-line re-score (AJ-E3305, `docs/qa/0.4-walks/design.md`) and opened on contact sheets, both themes, 1280 and 1024. Shared with every screen since the last shoot: the header's New session is tinted (one filled button per view), the Floor button is a split control with its own menu, This Mac's line is its dot below 1120 px, and the rail's App headers put the profile first.

Unchanged in design: "Keep a detailed log" under Debug mode, the log empty with Show and Clear the log disabled.
