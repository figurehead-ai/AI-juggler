<!-- captures: df6f4ea62589 -->
# 48-remote-cleanup: Processes with Automatic cleanup and Your other Macs, cleaning a left-behind node on Mac mini and the refusal when its identity changed

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

The three blocks (process table, Automatic cleanup with one card per account, Your other Macs) stand off the canvas, and the Mac mini card is easy to scan: node pid 4242 with a green "Verified owned" chip and a red Clean…, python3 with "Not owned — never touched". confirm's "Clean node on Mac mini?" popover is short and explains the re-check, with a red Clean on Mac mini beside Cancel. The page is still captured scrolled: the first table row (webpack at 1024, Billing webhooks at 1440) is cut under the top bar, and at 1440 the node (astro) row is cut in half by the table's bottom edge. In light the off switches in Automatic cleanup are pale grey on white and hard to see. In identity-changed the refusal "Mac mini refused: Refused — identity changed…" still repeats "refused", is plain body text with no warning colour, and sits under a row that still shows "Verified owned" and an active Clean…. The design fixes show at the edges: in dark the Work Max "Tidy left-behind dev servers" switch and the logo mark take the lighter violet, and at 1440 the rail's error row reads "Stopped with an error" and the Personal row "Limit reached · 0% left" whole. The second pass only re-spaces the process table: at 1440 State is 11rem wide and starts further left, with CPU and Memory at even gaps, and at 1024 the cut top row now shows its Stop… and ⋯ whole at the right edge. Every capture was re-rendered.

## Weak

- grouped, identity-changed, confirm at 1440: the node (astro) row is cut in half by the process table's bottom edge.
- grouped, identity-changed, confirm (both sizes): the top row of the table is cut under the top bar.
- identity-changed: "Mac mini refused: Refused — …" repeats the word, has no warning style, and the row keeps "Verified owned" and Clean… next to it.
- grouped-light, identity-changed-light: the off switches in Automatic cleanup nearly vanish on the white cards.
