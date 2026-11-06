# The Floor, 0.4.1 fixes — walked by hand in AI Juggler Dev

2026-11-06. AI Juggler Dev from `release/0.4.1` (debug build), on the
replica home (`/tmp/aij-replica`, made-up folders only), one external
display at scale 1. The replica's nine folders were brought in through
the first run ("Use them where they are"); a real Claude profile the scan
also offered was left unticked. Apps per floor set to 4 for the
multi-floor rows (three floors). Input was real pointer events (CGEvent
to the HID tap), the cursor read from `NSCursor.currentSystem`, each step
against the Dev build's own pid only.

Findings come from the maintainer's 0.4.0 test (`docs/research/floor-04x.md` §8).

| # | Step | Expected | Result | Time | Score (works / speed / clarity / layout) |
|---|---|---|---|---|---|
| 1 | Hover the Building tab | Hand cursor | Hand | — | 5 / 5 / 5 / 5 |
| 2 | Hover each floor tab (Floor 3, 2, 1) | Hand | Hand on each | — | 5 / 5 / 5 / 5 |
| 3 | Hover a desk area, the lift, a beanbag | Hand, outline, tooltip by the pointer | Hand; outline on the thing under the pointer; tooltip beside it (`hover-area.png`, `hover-lift.png`) | — | 5 / 5 / 5 / 4 |
| 4 | Hover empty floor and the sky | Arrow | Arrow | — | 5 / 5 / 5 / 5 |
| 5 | Click Building, Floor 3, Floor 2, Floor 1 | Each view opens | Each opened; the title says the view | ≈ 0.4 s to the new title (my tool's polling included) | 5 / 4 / 5 / 5 |
| 6 | A press that wobbles 2 points on a tab | Still a click | Clicked | — | 5 / 5 / 5 / 5 |
| 7 | Zoom far out on Floor 1 of 3 (25 ⌘-scrolls) | Stays on Floor 1, no other floor appears | Stayed on Floor 1, alone, at 1.6× its framing (`floor-zoomed-out.png`) | — | 5 / 5 / 5 / 5 |
| 8 | The building view's chips | Beside their own floor, no long lines | Each chip by its area on its floor; no line crosses a floor (`building.png`) | — | 5 / 5 / 4 / 4 |
| 9 | Window › The Floor, once | One Floor window | One (it made two before the fix) | ≈ 3 s on a debug build | 5 / 4 / 5 / 5 |
| 10 | Apps on the Floor | Only Apps the user added (D-84) | Before the import: 0 (the seven desktop profiles only seen running stayed offers); after: the 9 brought in | — | 5 / 5 / 5 / 5 |

Two bugs found on this walk and fixed in the same change:

- **Clicks landed a title bar's height below what they aimed at.** The
  event reader measured from the window's content-layout rect, not from
  the view the Floor draws into; the tabs' hit boxes sat about 28 points
  under the drawn tabs (this, more than the trackpad wobble, is why the
  maintainer's tab clicks did nothing). Events are now converted into the
  drawing view's own coordinates (`juggler-window-events`).
- **One Window › The Floor made two Floor windows.** Tauri hands a menu
  event to the app menu's handler and to the tray's, and both carry the
  Floor's id; two opens raced. A press seen twice within 400 ms now
  toggles once, and a second open waits for the first
  (`features/floor/window.rs`, tested).

Not walked here: the desktop App name ("Claude app · …") needs a desktop
profile the user added; the replica has none. It is covered by
`a_desktop_app_is_named_by_its_app_never_its_tool` and read from the same
places book the rail uses.
