<!-- captures: 86ae824c4319 -->
# 44-settings: Settings › Advanced, the Figurehead & Cloud card from signed out to pairing code, linking, connected and unlink

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The cards stand off the page in both themes (white on the grey canvas in light, a step lighter in dark). In pairing-code the solid violet Connect account is the one clear primary next to the outlined Use a pairing code and the dim, disabled Link this Mac; in dark the violet is now the lighter accent, and the Cloud backup and Find my Macs switches in connected pick it up too. linking shows the QVM3-GABM code large in a mono box with Open the page again and Cancel, and connected lays out the three Cloud features with bold "Sends:" lines that make each one's cost easy to scan. unlink-confirm is a small, readable popover with a red-tinted Unlink beside Cancel. The design fixes show in the chrome: the rail's error rows now read "Stopped with an error" whole instead of "Stopped with an error · 7 …", and the top bar stays on one row at both sizes ("This Mac · Plenty of room" and "7 to clean up" at 1280, the pill alone at 1024). What the fixes did not touch: consent, paid-locked-offline, signed-out and unlinked still show the top of Advanced (the Cleanup card with its per-account checkboxes) and none of the Figurehead & Cloud card, so these four captures don't show the state they are named for. connected and unlink-confirm still open scrolled, with the card starting at "Cloud backup" under the top bar, and in light the off switch for Sync usage stats is a white track on a white card that almost disappears. Every capture was re-rendered.

## Weak

- consent, paid-locked-offline, signed-out, unlinked (all sizes, both themes): the capture stops at the Cleanup section and never reaches the Figurehead & Cloud card, so the named state isn't visible.
- connected-light, unlink-confirm-light: the off switch for "Sync usage stats to my dashboard" is nearly invisible on the white card.
- connected, unlink-confirm: the page is scrolled so the section heading is gone and the card starts cut under the top bar.
