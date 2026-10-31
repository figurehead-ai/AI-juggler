<!-- captures: f57d178d27d3 -->
# 48-remote-cleanup: Processes with Automatic cleanup and Your other Macs, cleaning a left-behind node on Mac mini and the refusal when its identity changed

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The three blocks (process table, Automatic cleanup with one card per account, Your other Macs) now stand off the canvas, and the Mac mini card is easy to scan: node pid 4242 with a green "Verified owned" chip and a red Clean…, python3 with "Not owned — never touched". confirm's "Clean node on Mac mini?" popover is short and explains the re-check, with a red Clean on Mac mini beside Cancel. The page is captured scrolled: the first table row (webpack at 1024, Billing webhooks at 1440) is cut under the top bar, and at 1440 the node (astro) row is cut in half by the table's bottom edge. In light the off switches in Automatic cleanup are pale grey on white and hard to see. In identity-changed the refusal "Mac mini refused: Refused — identity changed…" repeats "refused", is plain body text with no warning colour, and sits under a row that still shows "Verified owned" and an active Clean…. Every capture was re-rendered.

## Weak

- grouped, identity-changed, confirm at 1440: the node (astro) row is cut in half by the process table's bottom edge.
- grouped, identity-changed, confirm (both sizes): the top row of the table is cut under the top bar.
- identity-changed: "Mac mini refused: Refused — …" repeats the word, has no warning style, and the row keeps "Verified owned" and Clean… next to it.
- grouped-light, identity-changed-light: the off switches in Automatic cleanup nearly vanish on the white cards.
- Rail at 1440: "Stopped with an error · 7 …" cuts the age.
