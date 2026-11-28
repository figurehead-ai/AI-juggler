<!-- captures: 3d4269307027 -->
# 58-updates: About & updates in Settings across every update state, and the update banners on Home

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The About & updates card stands off both canvases, and its release-notes box (New / Fixed, bold feature names) reads well. On Home the update banners use the slim style: "AI Juggler 0.2.0 is available · Beta" with a solid violet Review, and "0.2.0 is installed" with a solid violet "Restart to update". In Settings, though, Install 0.2.0, Try again and Restart to update are still violet-edged outlines, so the same action is primary on Home and secondary here. The re-framing fixed most of the cut headings. available, downloading and failed now open with "About & updates" whole at the top, and going-back-confirm now shows the whole "Go back to 0.0.7?" box down to its Go back and quit / Cancel buttons. restart-ready and restart-waiting still slice the "Open projects in" heading at the top edge, and check-failed opens on that card with its top cut off. The section that matters is whole in all three. In downloading the disabled radios are still faint dashed rings, and the Stable one nearly vanishes on white. In failed, "⚠ Not installed — the download didn't check out" is plain text with no amber or red. The development capture still shows a "Beta · beta channel" badge on a build its own text calls a development build. In dark the lighter violet shows on the checked radio and checkbox, the Try again outline and the bar's New session, and the rail's ai-juggler row now reads "Stopped with an error" whole.

## Weak

- available, nightly-edition, failed (dark/light): "Install 0.2.0" and "Try again" are violet-edged outlines, while Home's banner gives the same step a solid violet Review.
- restart-ready-1280x800-dark/light: "Restart to update" is an outline in Settings but solid violet in home-restart-ready.
- downloading-1280x800-dark/light: the disabled Stable radio is a faint dashed ring that almost disappears in light.
- restart-ready, restart-waiting (dark/light): the "Open projects in" heading is sliced at the top edge; check-failed opens on that card with its top cut off.
- failed-1280x800-dark/light: the "⚠ Not installed" warning has no amber or red treatment and reads like an ordinary note.
- development-1280x800-dark/light: a "Beta" badge and "beta channel" on a build described as a development build that never updates.

## 0.5.1

Re-captured on 0.5.1; only the shared chrome changed in these screens. The rail's head now starts past the traffic lights, its rows hold a fixed dot column (tools on hover), cards and panels separate by tone instead of a border, a chosen item is a fill, and the top bar's bottom line is calmer. About and updates are unchanged; the settings list gained AI and Models.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Needs you's cards are tone fills with their level's mark (a stopped one in the broken tone), the most urgent first (F53, F66, F80). Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.

## 0.5.2 (2026-11-26, AJ-E3713, Opus, both themes)

Re-captured on the host; every changed capture opened (contact sheets, four to a sheet) beside its last capture and the approved story-feed mock (`AIJuggler-design-refs/story-feed/`). Captures: home-available and home-restart-ready (dark, light).

**What changed (AJ-E3713).** Home's feed below the pinned needs is the story feed now: the tabs All · Needs you · Sessions · Tasks · Pools · Switches & limits · This Mac (the chosen one the accent's soft fill), the state chips Running / Finished / Failed in their soft tints with today's counts, Showing: Every app at that row's end, and posts in place of log rows: a tone card each with the App's own icon (or AI Juggler's mark), the calm line "**Claude Code CLI** · Work Max · 10 min", the story with its thing as an accent-soft chip, a quiet gesture bar (↑ ↓ + … Mark reviewed · ↗ Open). Nothing else on these screens moved. The update banner is unchanged; the feed under it is posts.

Scores unchanged but for: line 5 (Mock-up) 4: the feed follows the approved story mock (68-home-feed's 0.5.2 section lists the deviations); line 19 (Rich-mode colour) **4**: the feed's filters are choices, the chosen tab a soft accent fill, the state chips their own hues with a ✓ when on, the Showing picker's options with their icons; nothing chosen by a ring.
