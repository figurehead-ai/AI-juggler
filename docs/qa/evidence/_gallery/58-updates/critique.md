<!-- captures: 0b2debeb5f8d -->
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
