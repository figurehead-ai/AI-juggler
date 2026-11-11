<!-- captures: 1dd82353557d -->
<!-- captures: pending -->
# 99-when-to-tell: Notifications › When to tell me

Design critique (D-57) against Settings › Notifications (84-notifications). New family (AJ-E2904).

## Checklist

| Check | Finding |
|---|---|
| Grid | Under the kinds of notice: the wait, the two quota levels and Send test, each a line with its picker inline. |
| Hierarchy | The sub-heading, each line a sentence with its choice in it. |
| Spacing | The notifications card's rhythm. |
| Icons | The pickers' chevrons. |
| Shared controls | The shared Picker and Button. |
| Disabled reasons | n/a. |
| Plain words | "Tell me when a session waits longer than 5 minutes", "Needs you and the bell show a wait at once either way." |
| Type sizes | 13 px lines. |
| Both themes | Checked dark and light. |
| 1280×800 and 1024×700 | Both. |

## Good

- Each setting reads as the sentence it changes.
- It says the in-app signals don't wait.

## Weak

- Send test sits alone under the quota line; its result is said only after a click.

## Fixed

- Nothing on first render.

## 0.4.1 re-score (2026-11-07, AJ-E3305)

The whole family was retaken on release/0.4.1 for the 14-line re-score (AJ-E3305, `docs/qa/0.4-walks/design.md`) and opened on contact sheets, both themes, 1280 and 1024. Shared with every screen since the last shoot: the header's New session is tinted (one filled button per view), the Floor button is a split control with its own menu, This Mac's line is its dot below 1120 px, and the rail's App headers put the profile first.

Unchanged in design: When to tell me in its own card, quiet hours and the digest folded with their values ("More: quiet hours 22:00–07:00, all accounts").
