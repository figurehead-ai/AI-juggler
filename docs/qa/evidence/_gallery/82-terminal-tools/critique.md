<!-- captures: 70e7d1389876 -->
# 82-terminal-tools: the terminal's toolbar: find, maximize, own window and link hints

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

The terminal's toolbar (Find, Copy, A−, A+, Maximize, Own window) sits in a quiet row above the dark terminal in both themes, and the terminal text renders in a monospaced face with the account and session names in violet. link-hint underlines a file path and shows "⌘-click opens src/auth/session.test.ts:42" in the bottom-left corner, opposite the "^Tab leaves the terminal" hint. toolbar-find puts a focused search field with "1 of 1", arrows and Done above the terminal and boxes the match "Ask"; in dark the field's focus ring, the logo mark and New session now use the lighter accent, which reads a little brighter against the black terminal. maximized fills the window with the terminal and swaps Maximize for Restore; own-window-placeholder replaces the terminal with a centred note and two buttons, Show its window and Bring it back here. The light captures changed only by a sub-pixel shift of the header's "This Mac · Plenty of room" line; the header stays one row. After the second pass the terminal's black reaches the card's edge in link-hint and toolbar-find (the lighter or white frame is gone, so the text has a black margin), and the rail's "Refactor the settings screen" row reads "ai-ju… · viewed 2 min ago" where it was cut to "viewed 2 mi…". The session page around it has cards that stand off the canvas in both themes; nothing is clipped or overlapping.

## 0.5.1

Re-captured on 0.5.1; only the shared chrome changed in these screens. The rail's head now starts past the traffic lights, its rows hold a fixed dot column (tools on hover), cards and panels separate by tone instead of a border, a chosen item is a fill, and the top bar's bottom line is calmer. The terminal and its find bar are unchanged; the session header gained Watch… and Switch account… moved to the card.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: The lens switch's chosen segment is a solid fill. Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
