<!-- captures: c8ffb3e632ec -->
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

## 0.5.1

Re-captured on 0.5.1; only the shared chrome changed in these screens. The rail's head now starts past the traffic lights, its rows hold a fixed dot column (tools on hover), cards and panels separate by tone instead of a border, a chosen item is a fill, and the top bar's bottom line is calmer. The When to tell me card is unchanged; the page above it gained rows for watches and the digest.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: The lens switch's chosen segment is a solid fill. Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.

## 0.5.2 G52 (2026-11-27, Opus, both themes)

Re-captured from the worktree at HEAD (ea6bda3e2) with the family's own spec; each frame compared pixel by pixel with the one before, and every changed frame opened by eye, light and dark side by side. All four `section-*` frames changed.

**What changed.** The folded line under When to tell me read "More: quiet hours 22:00–07:00, all accounts"; it now reads "More: quiet hours 10:00 PM–7:00 AM, all accounts", the times by the system's clock as the Watches form and Bot's cards write them (G52, ea6bda3e2), each time kept whole with its AM/PM. The last committed frames were from before AJ-E1558, so these also show the section as it ships since then: each When-to-tell row led by its icon chip with its help line under it, the pickers beside the words, Send test under the rows; the alert rows above with their switches ("Watches and reminders", "The digest").

**Judged.** The longer times keep the fold's line to one line at 1024 and 1280. Nothing else moved because of G52.

| # | Line | Score | Note |
|---|---|---|---|
| 11 | Plain words | 4 | One time style across Settings, the Watches form and Bot. |
| 14 | Consistency | 4 | The fold's line uses the app's one time rule. |
| 19 | Rich-mode colour | 4 | Every row its icon chip; switches on in the accent; the fold calm on the surface; no bright edge. |
