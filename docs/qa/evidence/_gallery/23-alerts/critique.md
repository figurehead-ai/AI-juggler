<!-- captures: 57dc9dd159f3 -->
# 23-alerts: alerts on the Processes page: a process using a lot of memory, an idle process, both at once, keyboard focus on an alerted row, and the stop sheet for the memory hog

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

idle-nudge-* and mixed-* still show the alerts as plain full-width rows above the table ("python3 has been idle for 2h — review it in Processes.", "node (vite) under Work Max holds 3.1 GiB — review it in Processes.") with a quiet "Show it" link; they carry no mark or colour, so two stacked alerts in mixed-* read like ordinary text rows, and "review it in Processes" is odd when the user is already on Processes. The hog alert says 3.1 GiB while the table shows 2.9 GB for the same node (vite) row. keyboard-pass-1440x900-dark draws a clear focus ring around the alerted row after "pid:8123" is searched, now in the lighter dark accent like the Work Max cleanup toggle below, but still leaves an empty band under it inside the table. hog-staged-* still show a "Won't stop it" sheet with "program ?" and "ports —" instead of the memory-hog stop sheet, which looks like a broken state. The 1024x700 captures are still the same images as the 1440x900 ones (1440 px wide), so the smaller width is not actually shown. The rail's error rows read "Stopped with an error" with no cut age. After the second design pass the wider Processes table re-spaces its columns: State moves right and CPU sits closer to it, which makes the long "Left behind while closed" label run into its own "0.2 %" CPU value in idle-nudge-* and mixed-*; the hog-staged-* sheet is unchanged, only the blurred table behind it moved.

## Weak

- idle-nudge-*, mixed-*: alerts have no mark or colour; "review it in Processes" on the Processes page.
- mixed-*, keyboard-pass-1440x900-dark: "3.1 GiB" in the alert vs "2.9 GB" in the table for the same process.
- hog-staged-*: a "Won't stop it" sheet with "program ?" and "ports —" instead of the hog's stop sheet.
- *-1024x700-*: identical to the 1440x900 captures; the narrower width is not captured.
- keyboard-pass-1440x900-dark: an empty band below the focused row inside the table.
- idle-nudge-*, mixed-*: in the python3 (http.server) row, "Left behind while closed" overlaps its CPU value ("close0.2 %") since the columns were re-spaced.
