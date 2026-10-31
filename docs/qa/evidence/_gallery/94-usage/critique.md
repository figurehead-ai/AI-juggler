<!-- captures: a65bc5b7c44b -->
# 94-usage: usage on the Accounts page, a profile's page, a project's Overview and Usage tabs, and the Usage page

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

Five screens (accounts, profile, project-overview, project-usage, usage) at 1280, 1680 and 2000 wide in both themes, each as the first screen and as the whole page (-full), plus accounts and usage at 200 % zoom; all were re-rendered. The 0.3.2 surfaces work well on these chart-heavy pages: in dark every card (stat tiles, Usage over time, By account, the four runway cards, When you work, Top projects, Top sessions, Monthly caps) sits a clear step off the near-black canvas, and in light the white cards on grey keep the stacked bars, rings and heat map crisp; the big numbers (936M, ~$7,369, 460, 230 h) lead, with their "vs the 30 days before" lines and dim labels clearly below. New session is the one solid violet button, and Add an account / New session here stay the obvious entry points. The two-column card grid leaves holes when neighbours differ in height: under Codex Main on accounts-1680x1050 and -2000x1100, beside the tall Top sessions on profile and project-usage at 1680 (the Sessions card stretches with a large empty bottom), and to the right of the lone Agents card on project-overview, with Monthly caps alone at the foot of usage -full. In the project screens' rail, the session rows' second line is cut mid-word with no ellipsis ("Working · outside AI Juggl", project-overview-1280x800 and every project capture), while the row just below ends in a proper "…". At 200 % zoom (accounts-zoom200, usage-zoom200) the rail folds to icons and the top bar wraps onto two rows; everything stays readable and nothing is clipped beyond the expected scroll.

## Weak

- project-overview, project-usage (all sizes): rail rows' "Working · outside AI Juggl…" second line is clipped mid-letter with no ellipsis.
- accounts-1680x1050, accounts-2000x1100: a large empty block under the short Codex Main card beside the tall Work card.
- profile-1680x1050-full, project-usage-1680x1050-full: the Sessions card stretches to the height of Top sessions, leaving most of it empty.
- project-overview-*-full: the Agents card stands alone in its row with an empty column beside it.
- accounts (Personal card): "Not read since its reset · Last read 40% used, 13 h ago; it has reset since" wraps so that "since" sits alone on its own line.
