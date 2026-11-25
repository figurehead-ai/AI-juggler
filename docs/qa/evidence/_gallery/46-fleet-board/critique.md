<!-- captures: d2918c26a58c -->
# 46-fleet-board: Home with other Macs in view: the remote limit banner, the View menu's Mac filter, and the access sheet and "ask each time" prompt on Fleet

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

In all-machines the "Mac mini · Work Max hit its 5-hour limit — resets at 14:00." banner is slim, with a small warning mark, Open and a close button, and the Needs you and Runway cards stand off the canvas in both themes; Open its page is the one solid violet action in the first card. The View menu in alert and one-machine is a tidy grouped list (Show, then Only › Account / Project / Mac) with the Mac submenu showing All, This Mac and Mac mini and a clear check, and in one-machine the rail switches to "Mac mini ● live" with its two sessions. access-sheet is a readable four-option radio sheet with the chosen View only highlighted; in dark the selected radio, its outline and the Link on switch now use the lighter violet. ask-each-time is short and clear, with focus on Deny and Allow tinted red. The top bar fix is the big change here: at 1024×768 all-machines, alert and one-machine now keep the bar on one row (nav, a broom pill with 7, New session, bell, avatar), so the banner and page start right under it instead of a second row of actions. At 1440 the rail's error rows read "Stopped with an error" whole. Every capture was re-rendered.

## Weak

- ask-each-time: Allow is red-tinted like a destructive button although it is the "yes" answer; the colour reads as a warning rather than a choice.

## AJ-E710 retake (2026-11-07)

Retaken on release/0.4.1 for the Fleet gate's screenshot set (AJ-E710), both themes, every size the family has. First pass by a Sonnet agent against a written sheet; the points marked (checked) were confirmed by eye by Opus.

Home across machines: the banner now reads "Mac mini · Claude Max 20x has used up its 5-hour window; it resets in 1 h 30 min". The 1440 capture showed two filled buttons, Open its page and the permission reminder's Allow (checked); the reminder's Allow is tinted now (7a2b75aea) and this family is retaken after it (checked: one filled button).

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: all-machines-1440x900 dark, access-sheet-1024x768 light and ask-each-time-1024x768 dark, beside the AJ-E710 retake. Home is D-118's two columns in every capture: Needs you now as one card per need (the first card's primary the one filled button), the feed with its chips and Showing:, then Your apps and Runway by account on the right. The top bar gains Fleet.

- **all-machines:** "Mac mini · Claude Max 20x has used up its 5-hour window; it resets in 1 h 30 min" as the shared Banner (amber bar, bell, Open, ×); the rail starts with "This Mac".
- **access-sheet:** "What may Mac mini do on this Mac?" four choice cards, the chosen one (View only) a fill; "Change on this Mac" stays greyed until another is chosen; "Only this Mac can change this. Mac mini sees the change within half a minute."
- **ask-each-time:** "Mac mini asks to act on this Mac — It wants to stop Work Max (claude-code). Nothing happens unless you allow it. If you don't answer, it's denied in 18 s." Deny is the focused default; Allow is red-tinted because allowing stops work there (the risky answer), kept.
- **Weak:** at 1024 with Fleet in the tabs the top bar wraps onto a second row (logged **F57**); the prompt names the session by the provider's id, "(claude-code)" (logged **F58**, the backend's `describe_action`).

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Other Macs' alerts at home; what a paired Mac may do, asked or set here. |
| 2 | Prominence | 4 | The remote alert over Home; access in Fleet. |
| 3 | Default | 4 | Deny by default and on timeout; View only preselected. |
| 4 | Mode | 4 | Calm; the sheet and prompt on the overlay tone. |
| 5 | Mock-up | 4 | No mock; D-126's sheet and the Banner. |
| 6 | Areas | 4 | F57 fixed: one row at 1024 with Fleet; History and Fleet behind More (lit while Fleet is the page). |
| 7 | Colour | 4 | Allow in the danger tint as the risky answer; amber the banner's bar. |
| 8 | Type | 4 | Choice titles 13 px semibold, consequences 12 px ink-2. |
| 9 | Actions | 4 | "Change on this Mac" filled and last; the prompt has no filled default (Deny focused). |
| 10 | Serious | 4 | Each access level says what it allows; the prompt says it's denied by itself in 18 s. |
| 11 | Doors | 4 | Open goes to the Mac's board; "change it in Fleet" says where. |
| 12 | Words | 4 | F58 fixed: the ask names the session and its App, "Fix the flaky login test" in Claude Code CLI · Work Max. |
| 13 | States | 4 | The countdown is said; the sheet's change is said to land within half a minute. |
| 14 | Consistency | 4 | Choice cards as the pool setup's; the Banner shared. |
| 15 | Accessibility | 4 | alertdialog with Deny focused. Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | Not walked here (needs two Macs). |
| 17 | System prompts | 5 | None of the OS's; the prompt is ours and primed by the access setting. |
| 18 | Usability | 4 | A paired Mac can't act unseen, and the ask says which session in the words every list uses. |

Lowest as first scored: 3 (lines 6 and 12: F57, F58 logged; the first is the top bar's layout across every screen, the second a backend sentence).

**Re-captured after F57 and F58 (2026-11-24, Opus):** all twenty re-rendered; opened access-sheet-1024x768 light, ask-each-time-1024x768 dark and all-machines-1440x900. At 1024 the top bar is one row (it took two): Home · Projects · Pools · Tasks · Accounts · Usage · More ▾, More lit in semibold as the Fleet page is behind it, then 7 to clean up, New session, Bot, the bell, the Floor and the avatar; the page moves up about 45 px. The prompt reads "It wants to stop “Fix the flaky login test” in Claude Code CLI · Work Max." (it said "Work Max (claude-code)"). 1440 keeps every tab, Fleet included.

Lowest: 4 (lines 6 and 12 were 3: F57, F58 fixed).

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Needs you's cards are tone fills with their level's mark (a stopped one in the broken tone), the most urgent first (F53, F66, F80). Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
