<!-- captures: 6338a0cb44e5 -->
# 42-fleet: the Fleet page for pairing other Macs, in its link, trust and pairing states

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The Fleet page reads in clear blocks: title with a Beta chip, a brighter intro sentence, the Link card, Paired machines, This Mac, and the Figurehead "Cloud" line; in dark every card stands off the canvas, and in light they are white on grey. Status chips carry the state ("Connected · mTLS · v1.0" green, "Degraded · v1.1" amber, "Offline" grey), the connection banners in connection-banners are ringed in amber, trust-broken puts its warning in its own amber-edged card with "Reset trust…", and the unpair confirm uses a red-tinted Unpair. With Link on (unfinished) "Add a machine…" is the solid violet primary, now in the lighter dark accent along with the Link toggle, the Beta and Cloud chips and New session; with Link off it is still a pale violet that looks pressed rather than disabled. The rail at 1280 now shows "Stopped with an error" whole, where the first pass cut the age. The machine rows are unchanged: the two-column layout at 1280 is too narrow for them, so in paired and unfinished "Office iMac" wraps onto two lines and its fingerprint stacks one group per line into a tall column, and at 1024 Unpair drops to its own line. The layout also still changes between states: This Mac sits in the right column in connection-banners, paired, trust-broken and unpair-confirm but falls below everything in this-mac, and the top bar shows the Fleet item in some captures and the "This Mac · Plenty of room" line instead in this-mac and trust-broken. All captures were re-rendered.

## Weak

- paired-1280x800-*, unfinished-1280x800-*: Office iMac's name wraps and its fingerprint stacks one group per line in a tall narrow column.
- paired-1024x680-*: Office iMac wraps and its Unpair button drops to a line of its own.
- unfinished-1024x680-*: Office iMac's fingerprint is on one line while Mac mini's wraps into three, so the two rows don't line up.
- this-mac-1280x800-*: This Mac falls below the list instead of the right column used in the other states.
- this-mac-*, trust-broken-*: the top bar has no Fleet item while the other states show it as the current page.
- connection-banners-*, paired-*, this-mac-*, trust-broken-*: the disabled "Add a machine…" is a pale violet that reads as pressed, not unavailable.
