<!-- captures: e821051a96d8 -->
# 99-inactive-apps: the rail grouped by App is about apps (0.3.4, D-64)

Design critique (D-57, AJ-E2814). First critique of this family, written at 0.3.4 from its captures.

## 0.3.4

The maintainer's report: a Claude app that quit overnight vanished from the rail with no way to start it, and "23 inactive" listed sessions from all over the place. Four screens, both themes, on the places desktop (two Claude profiles and ChatGPT running, a Lab profile kept alive that's down, an old one), plus a CLI app with nothing open: Claude Code on a "Side project" folder with two sessions that ended yesterday and two days ago.

**rail-inactive-apps.** Under Open now, each app with sessions that aren't open ends with a quiet link in the accent colour, "Last 1 inactive", with a chevron at the right like the section heads'. After the last open app comes a section head, "Inactive apps 3", with a clock icon and the secondary-text colour, so it sits below Open now's green in weight. Its rows are drawn like place headers: the app icon, the app name in secondary text (dimmer than an open app's), the profile badge and name, then a second line, "Not running · 20 min ago" or "20 h ago · 2 sessions". The rows are in most-recently-used order. Each has one 24 px action at the right, in the same outline as an open app's ↗: ▷ Start for a desktop app that's down, + New session for a CLI app. A row with sessions has the fold chevron at the left; one without keeps the space, so the icons line up. "Also:" no longer lists inactive sessions: "1 ended today · 1 background · History ›".

**rail-last-3.** "Last 1 inactive" unfolded under Claude app · Personal shows its ended session as a normal row ("Ended") under the link, its chevron turned down. The Side project app unfolded shows its two sessions with the folder's colour edge, newest first.

**rail-idle-menu.** A right-click on Claude app · Lab opens its menu: the name as the heading, Start, then Keep alive with the violet ✓, as the place header's menu draws it.

**home-closed-apps.** Your apps: the four open tiles, then Lab and Old as dashed tiles with "Start Claude app".

## Weak

- A row's second line can still be cut at the narrowest rail width when the profile name is long. The full words are its accessible name, but there's no tooltip.
- An inactive app with more than 3 sessions shows only its last 3; the rest are in History until the App page (0.4) lists them.
- The menus have no icons, matching the place header's menu. Icons for both menus come with 0.4's App page actions.
