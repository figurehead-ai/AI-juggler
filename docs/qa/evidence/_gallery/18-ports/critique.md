<!-- captures: ea64269a1f9f -->
# 18-ports: the Processes page's Ports tab (port inventory and automatic cleanup), and the sheet warning that a port is already held

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

inventory-* are clear: four raised count cards (7 to clean up in amber, 1 didn't stop in red), tabs with counts, and a Port inventory card with the port and PID in mono, the owner below, and a status chip on the right ("Running in a session", violet "Left running", "Picked up after restart", "Not from AI Juggler"); the Automatic cleanup card lists one card per account with labelled toggles. The cards stand well off the darker canvas, and in light the white cards sit on grey. warning-sheet-1440x900-dark and -light are broken: the sheet ":5173 is already held by an orphan from claude-code (s-1a2b3c4d) (2h)." with "Free this port" and "Launch anyway" now has no background, so its text runs over the header's Tasks, Accounts, Usage and History links and the two buttons float over the page; the previous render had a solid sheet. All captures were re-rendered.

## Weak

- warning-sheet-1440x900-dark, warning-sheet-1440x900-light: the port-held sheet is transparent; its text overlaps the header navigation.
