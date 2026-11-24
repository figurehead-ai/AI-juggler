<!-- captures: a1c276940d73 -->
# 94-usage: usage on the Accounts page, a profile's page, a project's Overview and Usage tabs, and the Usage page

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

Five screens (accounts, profile, project-overview, project-usage, usage) at 1280, 1680 and 2000 wide in both themes, each as the first screen and as the whole page (-full), plus accounts and usage at 200 % zoom. In dark every card (stat tiles, Usage over time, By account, the four runway cards, When you work, Top projects, Top sessions, Monthly caps) sits a clear step off the near-black canvas, and in light the white cards on grey keep the stacked bars, rings and heat map crisp; the big numbers (936M, ~$7,369, 460, 230 h) lead, with their "vs the 30 days before" lines and dim labels clearly below. New session is the one solid violet button, and Add an account / New session here stay the obvious entry points. The design fixes are most visible on the dark charts: the stat-tile sparklines, the runway rings (62 %, 59 %, 78 %, 88 %), the When you work heat map, the Sessions bar chart and the Opus share of By model now use the lighter violet, which separates the shades of the heat map better and lifts the bars off the near-black. The light captures changed only in the top bar, which stays one row with "This Mac · Plenty of room" and the whole "7 to clean up" pill, and on the accounts -full captures the Personal profile's folder now reads with a neutral fixture name (~/…/studio-main). At 200 % zoom (accounts-zoom200, usage-zoom200) the rail folds to icons and the cleanup pill is now a broom and "7", but the top bar still sits on two rows (the nav, then the pill, New session, bell and avatar); everything stays readable. The two-column card grid still leaves holes when neighbours differ in height: under Codex Main on accounts-1680x1050 and -2000x1100, beside the tall Top sessions on profile and project-usage at 1680 (the Sessions card stretches with a large empty bottom), and to the right of the lone Agents card on project-overview, with Monthly caps alone at the foot of usage -full. In the project screens' rail the session rows' second line is still cut mid-word with no ellipsis ("Working · outside AI Juggl", project-overview-1280x800 and every project capture), and on the accounts Personal card "it has reset since" still leaves "since" alone on its line. The second pass changes only the rail on accounts, profile and usage (every size, both themes). After its last fix every Open now row keeps its time whole and a short project sits right against its dot, with no gap: "site · viewed 2 min ago", "api · viewed 2 min ago", "web… · viewed 2 min ago".

## Weak

- project-overview, project-usage (all sizes): rail rows' "Working · outside AI Juggl" second line is clipped mid-letter with no ellipsis.
- accounts-1680x1050, accounts-2000x1100: a large empty block under the short Codex Main card beside the tall Work card.
- profile-1680x1050-full, project-usage-1680x1050-full: the Sessions card stretches to the height of Top sessions, leaving most of it empty.
- project-overview-*-full: the Agents card stands alone in its row with an empty column beside it.
- accounts (Personal card): "Not read since its reset · Last read 40% used, 13 h ago; it has reset since" wraps so that "since" sits alone on its own line.
- accounts-zoom200, usage-zoom200: the top bar still takes two rows at 200 % even with the broom pill; and on accounts-zoom200 "This week 59% left · resets Mon 10:40" leaves "AM" alone on the next line.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B3)

Re-captured (the five screens at 1280, 1680 and 2000, first screen and full, both themes, and the 200 % zoom pair); opened at full size: usage-1280x800-dark-full and profile-1280x800-light. Usage is the app's best dashboard: four stat tiles, Usage over time with Account / App / Tool / Model / Project and Tokens / Cost (F53's solid chosen segments), By account and By model rings, the four runway cards, When you work, Sessions, Top projects, Top sessions, Cost by account, Monthly caps.

- **Fixed** (51ea3254f): runway cards were stretched to the tallest card in their row, leaving an empty panel beside a long one (0.3.2's "large empty block under the short Codex Main card"); each card is as tall as what it holds.
- **Weak, logged F76:** a cost rise is an amber chip with a fill behind "▲ 28%" (the kit's amber chip).
- **Weak, kept:** at 200 % zoom the top bar folds its tabs behind More (F57) and stays one row; "Not read since its reset · Last read 40% used, 13 h ago; it has reset since" is long for a card line.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | How much was used, where, and how long the room lasts. |
| 2 | Prominence | 4 | Tiles, the over-time chart, then by account and model. |
| 3 | Default | 4 | 30 days, by account, tokens. |
| 4 | Mode | 4 | Calm dashboard, the dials of §2.2. |
| 5 | Mock-up | 4 | The Usage dashboard's design. |
| 6 | Areas | 4 | Bands by tone; no box in a box. |
| 7 | Colour | 3 | Amber change chips are fills behind words (F76); series colours by account. |
| 8 | Type | 4 | Big values lead, labels muted, tabular numbers. |
| 9 | Actions | 4 | New session here / Set a monthly cap: one filled per view. |
| 10 | Serious | 4 | "API-equivalent estimate — your subscription price doesn't change per token." |
| 11 | Doors | 4 | Every account, project and session opens its page. |
| 12 | Words | 4 | Honest numbers: estimates marked ~, unknown runways "Not measured yet". |
| 13 | States | 4 | First paint draws the frame, figures follow (usage-gallery's test). |
| 14 | Consistency | 4 | The reference for the other dashboards. |
| 15 | Accessibility | 4 | axe clean in the capture runs (no serious or critical); keyboard and zoom not re-walked this round. |
| 16 | Hand walk | 4 | Not re-walked by hand here; captured from the fixtures, both themes. |
| 17 | System prompts | 5 | Causes no system prompt. |
| 18 | Usability | 4 | Re-read against PAT-USE: one place for usage, cost and runway, each figure named with its source. Match. |

Lowest: 3, each line under 4 logged (F76); the rest 4 or 5.
