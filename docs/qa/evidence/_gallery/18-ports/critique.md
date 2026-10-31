<!-- captures: 0f9b4035e3d0 -->
# 18-ports: the Processes page's Ports tab (port inventory and automatic cleanup), and the sheet warning that a port is already held

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

inventory-* are clear: four raised count cards (7 to clean up in amber, 1 didn't stop in red), tabs with counts, and a Port inventory card with the port and PID in mono, the owner below, and a status chip on the right ("Running in a session", violet "Left running", "Picked up after restart", "Not from AI Juggler"); the Automatic cleanup card lists one card per account with labelled toggles. The cards stand well off the darker canvas, and in light the white cards sit on grey. In inventory-1024x700-dark the top bar keeps one row by leaving out the cleanup pill and This Mac line (the page's own count card shows the 7), and the rail rows in the light capture now end "Stopped with an error" and "Limit reached · 0% left" with nothing cut. The port warning in warning-sheet-1440x900-dark and -light is fixed: ":5173 is already held by an orphan from claude-code (s-1a2b3c4d) (2h)." with "Free this port" and "Launch anyway" now sits on a solid sheet with a violet edge, so its text no longer runs over the page. The sheet is anchored at the top and still covers the header's Tasks, Accounts, Usage and History links and the start of the This Mac line, and its two buttons have equal weight, so neither reads as the suggested choice. All captures were re-rendered.

## Weak

- warning-sheet-1440x900-dark, warning-sheet-1440x900-light: the sheet covers the header navigation from "Ta…" to "History"; "Free this port" and "Launch anyway" look the same.
