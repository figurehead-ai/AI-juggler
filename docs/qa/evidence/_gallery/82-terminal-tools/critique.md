<!-- captures: 116d261e757e -->
# 82-terminal-tools: the terminal's toolbar: find, maximize, own window and link hints

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The terminal's toolbar (Find, Copy, A−, A+, Maximize, Own window) sits in a quiet row above the dark terminal in both themes, and the terminal text renders in a monospaced face with the account and session names in violet. link-hint underlines a file path and shows "⌘-click opens src/auth/session.test.ts:42" in the bottom-left corner, opposite the "^Tab leaves the terminal" hint. toolbar-find puts a focused search field with "1 of 1", arrows and Done above the terminal and boxes the match "Ask"; in dark the field's focus ring, the logo mark and New session now use the lighter accent, which reads a little brighter against the black terminal. maximized fills the window with the terminal and swaps Maximize for Restore; own-window-placeholder replaces the terminal with a centred note and two buttons, Show its window and Bring it back here. The light captures changed only by a sub-pixel shift of the header's "This Mac · Plenty of room" line; the header stays one row. The session page around it has cards that stand off the canvas in both themes; nothing is clipped or overlapping.
