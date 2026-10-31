<!-- captures: 3569e50bdbe1 -->
# 58-updates: About & updates in Settings across every update state, and the update banners on Home

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The About & updates card now stands off both canvases, and its release-notes box (New / Fixed, bold feature names) reads well. On Home the update banners use the new slim style: "AI Juggler 0.2.0 is available · Beta" with a solid violet Review, and "0.2.0 is installed" with a solid violet "Restart to update". In Settings, though, Install 0.2.0 and Restart to update are plain outlines, so the same action is primary on Home and secondary here. Several captures open scrolled with a heading cut. In available, "About & updates" is sliced under the top edge. In downloading the heading is off-screen. In restart-ready and restart-waiting, "Open projects in" is cut. going-back-confirm is the worst case: the "Go back to 0.0.7?" box runs off the bottom, so its Go back and quit / Cancel buttons are not in the capture. In downloading the disabled radios are faint dashed rings, and the Stable one nearly vanishes on white. The development capture still shows a "Beta · beta channel" badge on a build its own text calls a development build. All 26 captures were re-rendered.

## Weak

- going-back-confirm-1280x800-dark/light: the confirm box is cut at the bottom; its actions are not visible at all.
- available, nightly-edition, failed (dark/light): "Install 0.2.0" is a plain outline, while Home's banner gives the same step a solid violet Review.
- restart-ready-1280x800-dark/light: "Restart to update" is an outline in Settings but solid violet in home-restart-ready.
- available-1280x800-dark/light: the "About & updates" heading is cut under the top edge.
- downloading-1280x800-dark/light: the section heading is off-screen, and the disabled Stable radio is a faint dashed ring that almost disappears in light.
- restart-ready and restart-waiting (dark/light): the "Open projects in" heading is cut at the top.
- development-1280x800-dark/light: a "Beta" badge and "beta channel" on a build described as a development build that never updates.
