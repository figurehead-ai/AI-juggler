<!-- captures: 1624b22a60d8 -->
# 76-keys: the keyboard shortcuts sheet and the folded rail on a session page

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

sheet-1280x800 lays the shortcuts out in two columns (Anywhere, Screens) with key caps right-aligned on thin rules; it is easy to scan, and the dark sheet sits a step above the dimmed page. After the design fixes the close button's focus ring is in the lighter dark accent (sheet, both sizes). After the second pass the sheet is wider and ends in a "More below ⌄" chip when its list runs past the window: at 1024x680 the list stops at "Close a sheet, menu or search" with the chip under it, so "Open a row's menu" and "Open a session's page" are signposted rather than silently hidden. At 1280x800 the chip shows too, under "Open a session's page (on its row)", although the window has room left below the sheet; in both sizes the last visible key cap (Esc, ↩) is still slightly clipped at the bottom of the list's scroll area. folded-1024x680 shows the session page with the rail folded to marks, and folded-1280x800 the full page; the "Where it runs" and Runway cards lift off the canvas in both themes, and the terminal's black now fills its card to the edges in both themes (no white frame in light, no lighter frame in dark), with the text inset from the black's edge. The rail in folded-1280x800 now reads "Stopped with an error" whole, without the cut-off age, and the logo mark and the rail's New session outline take the lighter accent.

## 0.4.1

The maintainer found the right column's key caps cut off at the sheet's edge in the app ("⌘0", "⌘⇧P", "⌘F" half hidden): WebKit's fonts and a visible scrollbar took the room Chromium left. The sheet is now 48rem wide (`Modal broad`), its columns are as many as fit at 19rem each (one when narrow), labels wrap and key caps never shrink, and the list keeps a stable scrollbar gutter, so a scrollbar never covers a key. sheet-1024x680 and sheet-1280x800 (both themes) show two columns with a clear gap right of every key cap; the list still scrolls and still ends in "More below ⌄". Checked in WebKit too (the app's engine): at 1024 × 680 two columns and at 720 × 680 one, 0 of 54 key caps outside the list either way.

## Weak

- sheet-1024x680, sheet-1280x800 (both themes): the last visible key cap (Esc at 1024, ↩ at 1280) is still slightly clipped at the bottom of the list above "More below".
- sheet-1280x800 (both themes): the list scrolls ("More below") although the window has room for the sheet to grow.
