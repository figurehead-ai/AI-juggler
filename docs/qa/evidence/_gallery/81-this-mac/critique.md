<!-- captures: ef217971a521 -->
# 81-this-mac: the This Mac page in both densities and under memory pressure

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

page-1280x800 (comfortable and compact, both themes) splits the page into Now (Memory, Processor, Swap, AI Juggler cards) and What's using it (a grouped list with sizes and CPU right-aligned); the cards stand off the canvas in both themes and the large figures ("22 GB", "18%") lead each card. The Memory bar's violet AI-tools share and hatched compressed segment are easy to read against its legend. In the compact captures the rail rows squeeze their titles to make room for the figures, so "Refactor the settings screen" becomes "Refa…". page-other-apps shows the memory-pressure state with a red mark and a "What it means" fold, but its longer header status ("This Mac · Under memory pressure") pushes the status, "7 to clean up" and New session onto a second header line, breaking the header layout.

## Weak

- page-other-apps-1280x800 (both themes): the header wraps to two lines; the status, clean-up and New session drop below the tabs.
- page-1280x800-dark-compact, page-1280x800-light-compact: the compact rail cuts a session title to "Refa…".
