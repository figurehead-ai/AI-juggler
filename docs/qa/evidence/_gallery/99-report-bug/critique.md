<!-- captures: 02dcc9a5ccff -->
<!-- captures: pending -->
# 99-report-bug: Report a bug

Design critique (D-57) against the shared sheet and Settings › Advanced › Diagnostics. New family (AJ-E2911).

## Checklist

| Check | Finding |
|---|---|
| Grid | One sheet: title, what happened, then each part that goes with it as a ticked card with Show. |
| Hierarchy | The promise first ("Nothing leaves this computer until you open the form or copy the report."), then the user's words, then the parts. |
| Spacing | Card rhythm; the footer's Copy and Open the bug form at the right. |
| Icons | None beyond the checkboxes and chevrons. |
| Shared controls | The shared sheet, fields, checkboxes and Buttons. |
| Disabled reasons | n/a. |
| Plain words | "Scrubbed on this computer: no messages, transcripts, credentials, names or home folder." |
| Type sizes | 14 px fields, 12 px card details. |
| Both themes | Checked dark and light. |
| 1280×800 and 1024×700 | Both. |

## Good

- Every part that goes is readable in full and can be left out.
- Nothing is sent by AI Juggler itself.

## Weak

- At 1024 the parts list scrolls inside the sheet; the last part is below the fold.

## Fixed

- Nothing on first render.

## 0.4.1 re-score (2026-11-07, AJ-E3305)

The whole family was retaken on release/0.4.1 for the 14-line re-score (AJ-E3305, `docs/qa/0.4-walks/design.md`) and opened on contact sheets, both themes, 1280 and 1024. Shared with every screen since the last shoot: the header's New session is tinted (one filled button per view), the Floor button is a split control with its own menu, This Mac's line is its dot below 1120 px, and the rail's App headers put the profile first.

Unchanged in design: one "Goes with it… · Change" fold, "The whole report" fold, Copy and Open the bug form.
