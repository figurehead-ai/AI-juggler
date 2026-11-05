<!-- captures: f8ca38178bb9 -->
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
