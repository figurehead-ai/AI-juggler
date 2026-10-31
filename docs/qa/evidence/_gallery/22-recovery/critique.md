<!-- captures: e693d2c5e20e -->
# 22-recovery: after AI Juggler restarts: the summary of what kept running, a session picked up again, and the Processes list with the picked-up processes

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

summary-dialog-* say what happened in one bold line ("2 sessions picked up again, 1 helper left behind, 1 process could not be checked and was left alone.") followed by a grey paragraph and "Got it"; it is readable, though the paragraph is long and small for a dialog that greets the user after a crash. cleanup-readopted-* mark the picked-up rows with a violet "Picked up after restart" state and keep the Automatic cleanup card underneath, all well aligned; in dark the Work Max toggle that is on now shows the lighter accent. adopted-panel-* are framed on their subject again: under the "Running — picked up after restart" notice the Processes tab lists the session's tree (claude "Picked up after restart", node (mcp-server-filesystem), npm, node (vite) on :5180, esbuild, node (tsc)) with Stop… buttons and sparklines, so the header's "picked up after restart" chip is backed by rows. Two things still jar: the notice says "Stop it with Stop… on its row below", but the claude row itself has only a "…" menu and no Stop… button, and the header's "CPU 4% · 172 MB · 2 more processes" does not match the table's "6 processes · 1.4 GB · 8 %". The second pass re-spaced the Processes table at 1440: Process takes more width, and State, CPU, Memory, Ports and Age move right. That lets the node (vite) owner line show more ("probably started by subagent 'Run the f… · probably a dev server", only the subagent name still cut), and a later fix widened State to 11rem, so in cleanup-readopted "Left behind while closed" and "Picked up after restart" end with a clear gap before "0.2 %" / "0.7 %", and in adopted-panel before "5 %". Memory and Ports headers sit close together, and in cleanup-readopted the Clean up (4)… and Go to session buttons stick out right of the Stop… column. All captures were re-rendered.

## Weak

- adopted-panel-1440x900-*: the notice points to "Stop… on its row below", but the claude row has no Stop… button.
- adopted-panel-1440x900-*: header "172 MB · 2 more processes" vs the table's "6 processes · 1.4 GB"; the node (vite) owner line still cuts the subagent's name ("subagent 'Run the f…").
- cleanup-readopted-1440x900-*: the row actions don't line up: Clean up (4)… and Go to session end further right than the Stop… buttons.
- summary-dialog-*: a long small grey paragraph in the first dialog after a restart.
