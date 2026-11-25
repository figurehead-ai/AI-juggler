<!-- captures: 5933d5d787a9 -->
# 67-home: Home in its states (welcome, no sessions, all clear, sessions that need you) with the legend, the account menu and the rail's View menu

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

The Needs you, Runway by account and Pick up where you left off cards stand off the canvas, and "Open its page" is the one solid violet button in the Needs you card. The welcome and no-accounts screens have a strong headline and a solid "Find my accounts", but with the header's New session and the solid Allow there are still three violet fills competing on one screen. After the design fixes the dark accent is lighter on the logo mark, the welcome icon, the rail's New session outline and the Work Max runway bar; the 1280 and 1440 top bars stay one row in both themes. The biggest change is at 390 px: the cleanup pill is now a broom with "7", so the header drops from three rows to two (tabs, then Usage and History beside the broom, New session, bell and avatar), and the page starts about 35 px higher. After the second pass the first need's detail at 390 takes two lines ("storefront · AI Juggler · / Personal") where it was cut to "storefront · AI Juggler · …", pushing the card down about 16 px; the Needs you rows below still cut their status to "Limit reach…" and "Stopped wi…", because each row's button takes half the card. The legend popover and the account menu (more-menu) read well in both themes. In rail-filters-open the View menu still covers the lower half of the "5 sessions need you" heading and gives no on/off mark for its Show items. welcome-how still opens with the first card's title cut under the top bar. In the rail's Open now group the third row's meta now cuts the project instead of the time: "ai-j… · 3 h ago" where it read "ai-jugg… · 3 …" (legend, legend-home, more-menu, needs-you and rail-filters-open at 1280).

## Weak

- rail-filters-open-1280x800-dark/light: the View menu covers the lower half of the "5 sessions need you" heading; it shows no checked state for its Show items.
- no-accounts-*, welcome-*: three solid violet buttons (header New session, Find my accounts, Allow) on one screen.
- needs-you-390x844-*: the Needs you rows cut their status to a few letters ("Limit reach…", "Stopped wi…").
- all-clear-390x844-*: the permission card's title wraps word by word ("Jump to a / session's own / window").
- welcome-how-1280x800-*: the first card's title ("Jump to a session's own window") is cut under the top bar.

## 0.4, AJ-E3103: the one row on Home

Recaptured: needs-you at 1280, in both themes. The other states were left as they were (their changes since the last shoot aren't this task's).

**needs-you.** "Pick up where you left off" uses the one session row:
- the app's icon with a grey dot, where the project's mark was;
- the title;
- "◆ ai-juggler · 5 h ago" on line 2, ending in "Ended";
- "Mark reviewed" at the end of the title line.

Before, the row read "where · account · N ago" with the action in a column. The rail's Open now rows now say why a session needs you in the rows' one vocabulary: "Limit reached" where they said "Asks you", "Probably stuck" where they said "Probably asks you".

### Weak

- Two sessions with the same first words ("Keep the old shortcuts w…") truncate before the suffix that told them apart. Their projects on line 2 still do.

## 0.5, AJ-E3602/E3603: Home rounds 1-3 against D-118's mock

The Visual-gate baselines of Home (`ui/tests/visual/__baselines__/linux/`: home-*, home-scrolled-*, loading-home-*, each in both themes, comfortable and compact, 1024 and 1280) were re-rendered in the pinned container. The Playwright fixture is generic (p…@example.com, "Fake Desk", storefront). The approved side-by-sides with the mock were checked first; every image was opened by eye.

What changed in each screenshot:
- **home-\* (1280):** the top bar shows the current page, Home, in bold with no box beside regular muted tabs; the Floor is a borderless icon and arrow; "Plenty of room" is in words; the title is one count, "5 need you · 6 open"; Needs you now is one card per need, longest waiting first (2 h, 3 h, 4 h), with the question inline ("asks you — open it to see what it asks") and one filled button on the first card only; Your apps lists each running app with its count, a needs mark and Open; Runway by account has a disc, a figure ("58% left", "Out"), a thin meter and one facts line ("Resets at 7:13 AM · 3 sessions on it"); "How it's measured ›" sits under the account that isn't measured yet; the rail groups are named account then app, Open now rows are one line, and the rail's foot fades its last 20 px while more is below.
- **home-\* (1024):** the rail as icons, the same cards and columns, the room words hidden.
- **home-scrolled-\*:** the left column scrolled to its end (parked "storefront" with Resume, the feed's Today and Yesterday, the permission card, the keyboard line); the columns scroll, the page does not.
- **loading-home-\*:** the skeleton behind the new top bar and rail.

**The feed is not done; it waits for E3708/E3703/E3706.** The filter row, the item lines and the review meta shown under Today and Yesterday are the build as it stands, not the finished design: the review meta and the Open and Mark reviewed actions are missing. Do not read the feed region as finished.

Accepted deviations from the mock, with reasons:
- "Everything ▾" in place of Project and App filters: one combined filter (E37 R7).
- A "Needs you" tab in the feed's chips: the feed's one filter row carries it.
- "Claude Code CLI" and "Codex CLI" as names, not the mock's short names (D-100).
- "How it's measured ›" under an unmeasured account (D-103.4).
- "↺ 1" beside the room words (D-87), the runway count of accounts running low; it shows only when one is low, so these captures omit it.
- The Floor keeps its menu, so its arrow stays (D-68, D-78).
- A session need whose app doesn't say what it asks reads "— open it to see what it asks" (E3703: no adapter reports the ask yet).

Not re-baselined, left as they were: chrome-nav-button-* (the top bar's History button crop) and loading-accounts-*, which the re-render also changes; they belong to other screens and wait for their own review.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: welcome-1440x900 light, no-sessions-1280x800 dark, apps-out-1192x800 light and more-menu-1280x800 dark (52d7fa7b2's and d86510acc's renders), beside the calm references (`calm-ui/shots/67-home_*`, `home-feed/mock-home-feed-*`) and the 0.5 rounds above. The approved target is the home-feed design (2026-11-14): pinned needs over a narrated, chronological, paged feed on the left; Your apps then Runway by account on the right, each column scrolling on its own.

- **Since 0.5:** the calm surfaces (cards by tone, no borders), F22's rail head, AJ-E1554's rows; Your apps lists running Apps then "Not running" with Start; Runway's out account has a red "Out" chip over a full red bar with its reset said once under it (F43: at 1192 the name shows whole, "w…@example.com"); the feed's Showing: picker under its chips.
- **welcome:** "Let's get you set up" with what AI Juggler does, "We found Claude Code on this computer.", Get started (filled) and Add a fresh app, then the What it does for you cards (Bot first); the rail says "No apps open yet" and how to start one (F20, F49's re-render).
- **more-menu:** the account menu with an icon on every line, Appearance as a segmented Light · Dark · Auto, the version and channel at its foot.
- **Weak:** no-sessions shows the feed's chips, Showing: and ⋯ over "Quiet so far…" with nothing to filter: **F56** (logged in 55-triage). The feed's "Needs you" chip and "Everything ▾" stay the accepted deviations listed above.
- These captures predate F53 (1432a68d9); the chosen lens and chip are a solid neutral fill since, re-rendered with F53's round.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 5 | What needs you, what happened, what runs and how much room is left, in one place (the approved design). |
| 2 | Prominence | 4 | Pinned needs first, the feed under them; apps and runway on the right. |
| 3 | Default | 4 | The first need's primary is Enter; the feed's All chip by default. |
| 4 | Mode | 4 | Calm dashboard (D-117): cards by tone, one accent. |
| 5 | Mock-up | 4 | Matches the home-feed mock; deviations reasoned in the 0.5 section above. |
| 6 | Areas | 4 | Two columns, each scrolling on its own; areas obvious when blurred. |
| 7 | Colour | 4 | Status hues only on needs; runway bars by meaning (green, amber low, red out). |
| 8 | Type | 4 | Heading 22 px, card titles 13 px semibold, facts 12 px muted; tabular percentages. |
| 9 | Actions | 5 | One filled button per state (Open its page, Get started, New session when quiet). |
| 10 | Serious | n/a | Nothing destructive on Home. |
| 11 | Doors | 4 | Every card, App and account opens its page; How it's measured › explains. |
| 12 | Words | 4 | F56 fixed: the empty feed's card stands alone, with Include my actions in it. |
| 13 | States | 4 | Welcome, no sessions, all clear, needs: each said once; skeleton aside (F64, 75-feedback). |
| 14 | Consistency | 4 | Home's cards are the kit's; the menus carry icons. |
| 15 | Accessibility | 4 | Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | The PL1 walk (AJ-E2323) drove Home and the rail in Dev; not re-walked here. |
| 17 | System prompts | 4 | The permission card primes Accessibility and Automation (PAT-OSP); Welcome asks nothing of the OS. |
| 18 | Usability | 4 | A newcomer starts from Get started; a busy user sees the one thing to do first. |

Lowest as first scored: 3 (line 12, F56 logged).

**Re-captured after F56 (2026-11-24, Opus):** the family re-rendered; opened no-sessions (1280 and 390, both themes) and a populated one. The empty feed is its card alone: "Quiet so far…", New session and Open an app, then a muted line "Your own actions aren't shown. Include my actions" (the switch the hidden ⋯ held, the one thing that can change an empty feed). The chips, Showing: and ⋯ are gone above it; with anything in the feed, or a filter on, they are as before. The chosen segment is F53's solid fill.

Lowest: 4 (line 12 was 3: F56 fixed).

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Needs you's cards are tone fills with their level's mark (a stopped one in the broken tone), the most urgent first (F53, F66, F80); at 390 px the words wrap cleanly. Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
