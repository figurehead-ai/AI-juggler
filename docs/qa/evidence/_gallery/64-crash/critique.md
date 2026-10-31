<!-- captures: e622223035d6 -->
# 64-crash: the crash-report ask after a crash, and the crash-report setting in Privacy

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

In ask, the dialog puts the whole report in a dark code box under a short plain sentence, and the solid violet Send, with its focus ring, clearly leads Don't send and Always send; in dark, Send now uses the lighter violet. The report lines now wrap inside the box, so the long frame line ends whole on a second line ("app/src/commands.rs:4211:41") instead of losing ":41" at the right edge. It breaks after the hyphen in "juggler-", which is acceptable for a path. The report still stops mid-way, with the "app_version" line half cut at the bottom of the box and no fade or scroll cue. setting is re-framed on its subject: the "Crash reports · Ask me each time" row now sits near the top of the view with its scrub explanation and "1 kept on this computer" line, followed by the What it reads list. It still opens on the tail of the line above ("days, then only totals."), sliced at the top edge, with no Privacy section heading in view. The rail's ai-juggler row now reads "Stopped with an error" whole in both captures.

## Weak

- ask-1280x800-dark/light: the report box ends mid-report, with the "app_version" line half cut and no scroll hint.
- setting-1280x800-dark/light: the view opens on a sliced line ("days, then only totals.") above Crash reports, with no section heading in view.
