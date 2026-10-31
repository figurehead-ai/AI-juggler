<!-- captures: d206160d1155 -->
# 64-crash: the crash-report ask after a crash, and the crash-report setting in Privacy

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

In ask, the dialog puts the whole report in a dark code box under a short plain sentence, and the solid violet Send, with its focus ring, clearly leads Don't send and Always send. The code box now uses the wider SF Mono, so the longest frame line ("…/commands.rs:4211:41") runs into the box's right edge and loses ":41", which the 0.3.1 render showed whole. The report also stops mid-way ("channel": "beta",) with no fade or scroll cue. In setting, the Privacy card stands off the canvas and "Crash reports · Ask me each time" reads clearly, but the page opens scrolled, with the card's top rows cut under the top bar. The rail shows the "Stopped with an error · 7…" cut in both captures. All four captures were re-rendered.

## Weak

- ask-1280x800-dark/light: the frame line "at ~/Dev/…/commands.rs:4211:41" is clipped at the code box's right edge in SF Mono (it fit in 0.3.1).
- ask-1280x800-dark/light: the report box ends mid-report with no scroll hint.
- setting-1280x800-dark/light: the page opens scrolled, with the usage-statistics table's top rows cut under the bar and no section heading in view.
- All: rail row "Stopped with an error · 7…" loses its age.
