<!-- captures: 1cabf56d25e4 -->
<!-- captures: pending -->
# 99-resume: an ended session's Resume

Design critique (D-57) against the session page header (92-session-pages). New family (AJ-E2908).

## Checklist

| Check | Finding |
|---|---|
| Grid | The header's actions: Continue in another app…, Resume (primary), Relaunch. |
| Hierarchy | Resume is the one primary action on an ended session. |
| Spacing | The header's button rhythm. |
| Icons | None. |
| Shared controls | The shared Button. |
| Disabled reasons | When it can't resume, the button says why (unit-tested; not in this preview). |
| Plain words | "Resume", "Relaunch". |
| Type sizes | The header's. |
| Both themes | Checked dark and light. |
| 1280×800 and 1024×700 | Both. |

## Good

- The ended page leads with picking up where it left off.

## Weak

- Resume and Relaunch sit side by side; the difference (same conversation vs a fresh start) is only in their tooltips.

## Fixed

- Nothing on first render.

## 0.4.1 re-score (2026-11-07, AJ-E3305)

The whole family was retaken on release/0.4.1 for the 14-line re-score (AJ-E3305, `docs/qa/0.4-walks/design.md`) and opened on contact sheets, both themes, 1280 and 1024. Shared with every screen since the last shoot: the header's New session is tinted (one filled button per view), the Floor button is a split control with its own menu, This Mac's line is its dot below 1120 px, and the rail's App headers put the profile first.

Unchanged in design: Relaunch then Resume, each with its line under it, "Ended 2:03 AM" once, This Mac on the facts line (a session's door to This Mac since 9b6b9c96b).
