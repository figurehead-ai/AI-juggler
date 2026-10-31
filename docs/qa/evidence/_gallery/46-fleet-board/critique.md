<!-- captures: a64fdca6868f -->
# 46-fleet-board: Home with other Macs in view: the remote limit banner, the View menu's Mac filter, and the access sheet and "ask each time" prompt on Fleet

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

In all-machines the "Mac mini · Work Max hit its 5-hour limit — resets at 14:00." banner is slim, with a small warning mark, Open and a close button, and the Needs you and Runway cards stand off the canvas in both themes; Open its page is the one solid violet action in the first card. The View menu in alert and one-machine is a tidy grouped list (Show, then Only › Account / Project / Mac) with the Mac submenu showing All, This Mac and Mac mini and a clear check, and in one-machine the rail switches to "Mac mini ● live" with its two sessions. access-sheet is a readable four-option radio sheet with the chosen View only highlighted; in dark the selected radio, its outline and the Link on switch now use the lighter violet. ask-each-time is short and clear, with focus on Deny and Allow tinted red. The top bar fix is the big change here: at 1024×768 all-machines, alert and one-machine now keep the bar on one row (nav, a broom pill with 7, New session, bell, avatar), so the banner and page start right under it instead of a second row of actions. At 1440 the rail's error rows read "Stopped with an error" whole. Every capture was re-rendered.

## Weak

- ask-each-time: Allow is red-tinted like a destructive button although it is the "yes" answer; the colour reads as a warning rather than a choice.
