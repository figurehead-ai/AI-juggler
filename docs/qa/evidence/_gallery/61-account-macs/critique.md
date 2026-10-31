<!-- captures: c09c12ac4c46 -->
# 61-account-macs: Fleet: your other Macs found through your Figurehead account, and comparing fingerprints to pair one

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

On Fleet, the Link off, Paired machines, Your Macs on Figurehead and This Mac cards stand off both canvases. The two-column layout puts this Mac's identity on the right and the other machines on the left. The Studio row in Paired machines is crowded: its fingerprint is squeezed into a narrow four-line column beside the green "Connected · mTLS · v1.0" and "✓ Synced" chips. In light, the disabled "Add a machine…" is still a pale violet block with white text that is hard to read. The compare dialog lays out the two fingerprints in side-by-side boxes. Each fingerprint still wraps in the middle of a group ("8C3 / E 0D91", "9C7 / E 2D40"), which makes them harder to compare by eye, the one job of this dialog. "Pair (View only)" is still a violet-edged outline, not solid violet. The design fixes show only as the lighter dark accent (the Beta chip, the ticked checkbox, Pair's edge, the bar's New session) and the rail's ai-juggler row, which now reads "Stopped with an error" whole.

## Weak

- compare-1280x800-dark/light: both fingerprints wrap in the middle of a four-character group, so they are hard to compare group by group.
- list-1280x800-dark/light: the Studio fingerprint is squeezed into a narrow four-line column inside the Paired machines row.
- list-1280x800-light: the disabled "Add a machine…" is pale violet with white text, low contrast.
- compare-1280x800-dark/light: "Pair (View only)" is an outline, not the solid primary.
