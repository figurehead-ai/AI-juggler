<!-- captures: 459572f1bfa4 -->
# 74-overload: the Mac-under-strain states: the memory and CPU banner above a session, Home with the pressure item, and This Mac's advice

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The banner is one bordered strip with an amber triangle, two lines of plain text and a close mark. It sits well above the session page, and in light its pale tint reads as a warning without shouting. After the design fixes the top bar stays one row everywhere here: the status reads "This Mac · Under pressure" in banner-above-session and memory-pressure, so the actions no longer drop under the tabs, and at 1024 the cleanup pill is a broom with "7". busy-cpu now says "Close the Claude app windows you aren't using", with no doubled "the". banner-above-session-1024x800 (dark and light) is re-rendered in the 0.3.2 shell with the same two-line banner. The dark accent is lighter on the logo mark, the rail's New session outline and the memory bar's AI tools segment in recommendations. The session page still has two solid violet buttons in view, "Go to it" in the header and "Open / Focus window" in the preview card, so neither clearly leads. In memory-pressure and recommendations the cards stand off the canvas, and "See what's using it" is the one strong action in the Needs you card. The new memory-pressure-other-apps shows Home with that same card for pressure from other apps; its sentence ("Other apps are using most of the memory: next-se…") is cut just where it names the apps, so the cause is never read on Home.

## Weak

- banner-above-session-*, busy-cpu-*: two solid violet buttons (Go to it, Open / Focus window) on one page.
- memory-pressure-other-apps-1280x800-dark/light: the Needs you sentence is cut at "next-se…", hiding the names of the apps using the memory; the rail's copy is cut even earlier ("macOS reports memory pressure…").
