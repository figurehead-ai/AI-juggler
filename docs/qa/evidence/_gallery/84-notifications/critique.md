<!-- captures: db318d8eac3f -->
# 84-notifications: Settings › Notifications, which alerts reach macOS, quiet hours and the digest's accounts

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

The alerts card stands a clear step off the page in both themes (a lighter card on the near-black canvas, a white card on the grey one), and the nine choices read as two levels: a bright label over a dimmer one-line hint, with the intro and footnote sentences a level brighter than the hints. The two captures differ only in their ticks (settings-chosen has "A session waits for you" and "This Mac is getting tight"; settings-off has none), and the violet tick is easy to spot in both themes; in dark the ticks, the digest's checkboxes, the Desktop apps and Switching tiles and New session now use the lighter accent. The only violet fill on the page is New session in the header, which is right for settings that apply at once. The page opens scrolled so the card starts at the top edge with no "Notifications" heading above it, and every capture ends half-way through the second row of "In the digest and notifications" ("Claude Code · Personal" is cut by the window edge), so the digest list is never seen whole. After the second pass the rail's "Refactor the settings screen" row keeps its time whole ("ai-ju… · viewed 2 min ago" where it read "ai-juggler · viewed 2 mi…"); nothing else changed.

## Weak

- settings-chosen-1280x800, settings-off-1280x800 (both themes): the capture cuts the digest list's "Claude Code · Personal" row in half at the bottom edge, so the end of the page is never shown.
- settings-chosen-1280x800, settings-off-1280x800 (both themes): the page's heading is scrolled out of view; the card starts at the top edge with nothing naming it.
