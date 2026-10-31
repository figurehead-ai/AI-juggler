<!-- captures: 395863d2d80b -->
# 22-recovery: after AI Juggler restarts: the summary of what kept running, a session picked up again, and the Processes list with the picked-up processes

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

summary-dialog-* say what happened in one bold line ("2 sessions picked up again, 1 helper left behind, 1 process could not be checked and was left alone.") followed by a grey paragraph and "Got it"; it is readable, though the paragraph is long and small for a dialog that greets the user after a crash. cleanup-readopted-* mark the picked-up rows with a violet "Picked up after restart" state and keep the Automatic cleanup card underneath, all well aligned. adopted-panel-* contradict themselves: the header says "Working · 2 more processes · picked up after restart" and the notice says "Stop it with Stop… on its row below", but the Processes table below reads "No sessions are running processes now." with no rows; the previous render listed the claude process there. All captures were re-rendered.

## Weak

- adopted-panel-1440x900-*: the Processes tab is empty ("No sessions are running processes now.") although the header and notice say processes were picked up and point to a row below.
- summary-dialog-*: a long small grey paragraph in the first dialog after a restart.
