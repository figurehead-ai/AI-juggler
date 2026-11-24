<!-- captures: 4421de98e86c -->
# 74-overload: the Mac-under-strain states: the memory and CPU banner above a session, Home with the pressure item, and This Mac's advice

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

The banner is one bordered strip with an amber triangle, two lines of plain text and a close mark. It sits well above the session page, and in light its pale tint reads as a warning without shouting. After the design fixes the top bar stays one row everywhere here: the status reads "This Mac · Under pressure" in banner-above-session and memory-pressure, so the actions no longer drop under the tabs, and at 1024 the cleanup pill is a broom with "7". busy-cpu now says "Close the Claude app windows you aren't using", with no doubled "the". banner-above-session-1024x800 (dark and light) is re-rendered in the 0.3.2 shell with the same two-line banner. The dark accent is lighter on the logo mark, the rail's New session outline and the memory bar's AI tools segment in recommendations. The session page still has two solid violet buttons in view, "Go to it" in the header and "Open / Focus window" in the preview card, so neither clearly leads. In memory-pressure and recommendations the cards stand off the canvas, and "See what's using it" is the one strong action in the Needs you card. The new memory-pressure-other-apps shows Home with that same card for pressure from other apps. After the second pass the card's detail runs to two lines, so the cause is now read on Home: "…most of the memory: next-server 6.0 GB, Docker about 3.7 GB, fseventsd 1.4 GB." The advice after it is still cut ("To make room, quit next-serve…"), as is memory-pressure's ("…everything is slower until it…"), and both break a name at its hyphen across the lines ("next-" / "server", "ai-" / "juggler").

## Weak

- banner-above-session-*, busy-cpu-*: two solid violet buttons (Go to it, Open / Focus window) on one page.
- memory-pressure-1280x800-*, memory-pressure-other-apps-1280x800-*: the Needs you detail now names the apps but its second line still ends cut ("…quit next-serve…", "…slower until it…"), and the names break at their hyphen across the two lines ("next-" / "server", "ai-" / "juggler"); the rail's copy is cut even earlier ("macOS reports memory pressure…").

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: banner-above-session-1280x800 dark (before the fix) and -1024x800 light (after), memory-pressure-other-apps-1280x800 light, beside the 0.3.2 ones. Home is D-118's two columns in every capture: Needs you now as one card per need (the first card's primary the one filled button), the feed with its chips and Showing:, then Your apps and Runway by account on the right.

- **memory-pressure:** "Your Mac is under memory pressure" as an L3 card (red square), "macOS reports memory pressure. Other apps are using most of the memory: next-server 6.0 GB, Docker about 3.7 GB, fseventsd 1.4 GB. To make room, quit next-server or another app…" with See what's using it ↵; the rail's Needs you says it once with "See what's using it". The 0.3.2 weak point (names split at hyphens, the line cut) is gone; the detail ends on "…" after two lines with its whole text on hover.
- **banner-above-session:** no overload banner over a session's page (D-87), as the test asserts. **Before the fix** the page had two filled buttons: the head's "Go to it ↗ ↵" and the desktop surface's "Open / Focus window" (the 0.3.2 weak point, still there). **Fixed** (e5f9f70f0): Open / Focus window is tinted; Go to it is the page's one filled button. Re-captured (all four) and checked by eye; the gate's 13-desktop, 14-focus and 15-preview re-rendered for it.
- The empty feed's chips (F56, Home's) show behind memory-pressure-other-apps.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Says the Mac is under strain, by what, and where to act. |
| 2 | Prominence | 4 | The Mac's need pinned first; nothing over a session's page (D-87). |
| 3 | Default | 4 | See what's using it ↵ the default. |
| 4 | Mode | 4 | Calm; the L3 mark only. |
| 5 | Mock-up | 4 | No mock; Needs you's card. |
| 6 | Areas | 4 | One card, one rail row. |
| 7 | Colour | 4 | Red mark for a broken state; no red fill. |
| 8 | Type | 4 | As Home. |
| 9 | Actions | 4 | After the fix: one filled button on the session's page (was 3). |
| 10 | Serious | n/a | Nothing destructive; quitting another app is the user's. |
| 11 | Doors | 4 | See what's using it opens This Mac's list. |
| 12 | Words | 4 | Names the apps and sizes; "about 3.7 GB" where it's estimated. |
| 13 | States | 4 | Said once in Needs you and the bell, never as a banner. |
| 14 | Consistency | 4 | Needs you's card pattern. |
| 15 | Accessibility | 4 | Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | The PL1 walk (AJ-E2323) drove Home and the rail in Dev; not re-walked here. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | What to quit, in a sentence. |

Lowest: 4 (line 9 was 3 before the fix).
