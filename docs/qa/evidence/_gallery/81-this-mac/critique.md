<!-- captures: de6bf3b4b3ed -->
# 81-this-mac: the This Mac page in both densities and under memory pressure

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

page-1280x800 (comfortable and compact, both themes) splits the page into Now (Memory, Processor, Swap, AI Juggler cards) and What's using it (a grouped list with sizes and CPU right-aligned); the cards stand off the canvas in both themes and the large figures ("22 GB", "18%") lead each card. The Memory bar's violet AI-tools share and hatched compressed segment are easy to read against its legend; in dark the AI-tools bar, its legend square and the logo mark now use the lighter accent. In the compact captures the rail rows still squeeze their titles to make room for the figures, so "Refactor the settings screen" becomes "Refa…". page-other-apps shows the memory-pressure state with a red mark and a "What it means" fold, and its header now stays on one row: the status reads "This Mac · Under pressure" with a red dot, beside "7 to clean up" and New session, so the tabs, status and actions line up as on every other page. The Claude app group's line is cut at "6 windows · not started by AI Jug…" in every capture, which is acceptable since its windows are listed below.

## Weak

- page-1280x800-dark-compact, page-1280x800-light-compact: the compact rail cuts a session title to "Refa…".
