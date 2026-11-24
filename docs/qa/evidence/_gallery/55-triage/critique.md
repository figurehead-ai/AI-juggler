<!-- captures: 84ab5abe7f0d -->
# 55-triage: Home as a triage view: what needs you, what is parked, runway per account and what to pick up, plus the empty first-run Home

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

On the populated Home the Needs you, Parked until a reset, Runway by account and Pick up where you left off cards stand off both canvases. "Open its page" on the top Personal row is the only solid violet button in the main area, so the next step is obvious. The amber "Asks you" and "0% left" and the violet runway bar give the right column some colour. In dark the design fixes show as the lighter violet on Open its page, the Work Max runway bar, the Measure its limits links and the bar's New session. In the comfortable 1440 rail (both themes) the status lines now show their age whole or not at all: "Stopped with an error" and "Parked 2h 13m · 0% left" no longer end in a cut "· 2…". Comfortable and compact differ only in the rail. In the compact captures the rail's first rows are still cut to "Fa…" and "Fake D…", and their app badges and ages don't line up. The empty Home is clear: "Ready when you are", a Start your first session card with a solid violet New session, and the Jump to a session's own window prompt with a small solid Allow. But Runway by account still sits alone in the right column with an empty left half beside it, and the rail's project rows cut their meta ("1 min ago · 17 conversations · …", "1 h ago · 11 conversations · Pe…").

## Weak

- home-populated-1440x900-compact-dark/light: in the rail, the first Fake Desk rows are cut to "Fa…" and "Fake D…", and their badges and ages sit at different x positions.
- home-empty-1440x900-dark/light and home-empty-1024x680: Runway by account sits in the right column with nothing beside it, which leaves a large empty block on the left.
- home-empty (all four): three New session buttons compete (the bar, the card and the rail).
- home-empty-1440x900-dark/light: the rail's project rows cut their meta mid-word ("1 h ago · 11 conversations · Pe…" under storefront).

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: home-empty-1440x900 light and home-finished-1024x680-compact dark, beside the 0.3.2 ones. Home is D-118's two columns in every capture: Needs you now as one card per need (the first card's primary the one filled button), the feed with its chips and Showing:, then Your apps and Runway by account on the right.

- **home-empty:** "Ready when you are", the feed's empty state "Quiet so far. When your apps start, finish or ask you something, it shows here." with New session (filled) and Open an app, the permission card ("Jump to a session's own window … Not now · Allow"), Your apps' three not running with Start, Runway "Not measured yet · How it's measured ›" for each.
- **home-finished:** at 1024 compact, the Finished chip chosen with Parked until a reset and Needs you above.
- **Since 0.3.2:** the empty Home no longer leaves Runway alone in the right column (Your apps sits over it); the three New session buttons remain (the top bar's, the rail's, the empty card's), each in its place; the cut project meta stays as truncation.
- **Weak:** the empty feed shows its five chips, Showing: and ⋯ over "Quiet so far…": filters with nothing to filter (D-127). Hiding them also hides "Include my actions", so where that switch lives needs a call: logged **F56**.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | What needs you, what's parked, runway, the activity; on day one, how to start. |
| 2 | Prominence | 4 | Needs you first; the empty state's New session the one filled. |
| 3 | Default | 4 | New session filled when nothing waits (quiet). |
| 4 | Mode | 4 | Calm. |
| 5 | Mock-up | 4 | Matches the home-feed mock; its empty state is ours (R8). |
| 6 | Areas | 4 | Two columns, the permission card one card. |
| 7 | Colour | 4 | Amber and red only on needs; the empty Home has none. |
| 8 | Type | 4 | As Home. |
| 9 | Actions | 4 | One filled; three New session doors in three places (the global ones quiet). |
| 10 | Serious | n/a | Nothing destructive. |
| 11 | Doors | 4 | Open an app, Start, How it's measured each open their thing. |
| 12 | Words | 4 | F56 fixed: an empty feed shows no filters; its card offers Include my actions. |
| 13 | States | 4 | One empty state for the feed; Runway says "Not measured yet" once per account. |
| 14 | Consistency | 4 | Home's cards. |
| 15 | Accessibility | 4 | Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | The PL1 walk (AJ-E2323) drove Home and the rail in Dev; not re-walked here. |
| 17 | System prompts | 4 | The permission card primes Accessibility and Automation before any prompt (PAT-OSP). |
| 18 | Usability | 4 | A newcomer sees how to start, with no filters before there's anything to filter. |

Lowest as first scored: 3 (line 12, F56 logged).

**Re-captured after F56 (2026-11-24, Opus):** the family re-rendered; opened home-empty (1024 and 1440, both themes) and a populated one. The empty feed is its card alone: "Quiet so far…", New session and Open an app, then a muted line "Your own actions aren't shown. Include my actions" (the switch the hidden ⋯ held, the one thing that can change an empty feed). The chips, Showing: and ⋯ are gone above it; with anything in the feed, or a filter on, they are as before. The chosen segment is F53's solid fill.

Lowest: 4 (line 12 was 3: F56 fixed).
