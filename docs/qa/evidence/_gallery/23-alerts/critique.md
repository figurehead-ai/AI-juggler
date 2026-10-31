<!-- captures: 74a7a51292ae -->
# 23-alerts: alerts on the Processes page: a process using a lot of memory, an idle process, both at once, keyboard focus on an alerted row, and the stop sheet for the memory hog

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

idle-nudge-* and mixed-* show the alerts as plain full-width rows above the table ("python3 has been idle for 2h — review it in Processes.", "node (vite) under Work Max holds 3.1 GiB — review it in Processes.") with a quiet "Show it" link; they carry no mark or colour, so two stacked alerts in mixed-* read like ordinary text rows, and "review it in Processes" is odd when the user is already on Processes. The hog alert says 3.1 GiB while the table shows 2.9 GB for the same node (vite) row. keyboard-pass-1440x900-dark draws a clear violet focus ring around the alerted row after "pid:8123" is searched, but leaves an empty band under it inside the table. hog-staged-* show a "Won't stop it" sheet with "program ?" and "ports —" instead of the memory-hog stop sheet, which looks like a broken state. The 1024x700 captures are the same images as the 1440x900 ones (1440 px wide), so the smaller width is not actually shown. All captures were re-rendered.

## Weak

- idle-nudge-*, mixed-*: alerts have no mark or colour; "review it in Processes" on the Processes page.
- mixed-*, keyboard-pass-1440x900-dark: "3.1 GiB" in the alert vs "2.9 GB" in the table for the same process.
- hog-staged-*: a "Won't stop it" sheet with "program ?" and "ports —" instead of the hog's stop sheet.
- *-1024x700-*: identical to the 1440x900 captures; the narrower width is not captured.
- keyboard-pass-1440x900-dark: an empty band below the focused row inside the table.
