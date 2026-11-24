<!-- captures: 253be5c69f62 -->
# 06-inspector: the main panel for an account's sessions, a project's sessions and a session's terminal, and Home with nothing opened

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

account-sessions-* and project-sessions-* show the same five-row session list in a raised card: a name, a coloured status line, the account and an age per row, with clear tabs above ("Overview / Sessions", and on the project "Sessions 3" plus "1 failed / 1 stuck / 1 working" chips). The list is short, so about half the panel below it is empty canvas; the account page's breadcrumb still reads "Claude Code › Account", placing the account under a tool. session-terminal-* matches the session page of 05-session-panel, with its cards a clear step off the canvas and no solid violet competing in the panel; after the second pass the terminal's black fills its card to the edges, with no lighter frame around it, and the prompt text sits inside a black margin instead of at the black's edge. empty-1280x800 and empty-1440x900 are still just Home with no panel opened, so "empty" shows no empty panel state. The design fixes show here as the lighter accent on the logo mark, the selected Sessions tab, "Open its page" and the rail's New session button, a one-row top bar, and rail error rows that read "Stopped with an error" with no cut age. All captures were re-rendered.

## Weak

- empty-1280x800, empty-1440x900: Home with no panel; no empty panel state is shown.
- account-sessions-*, project-sessions-*: the list status "Stopped with an error" is amber, while the session header shows the same state in red.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B3)

Re-captured (eight states at 1280 and 1440); opened at full size: account-sessions-1280x800. Only the rail moved since the last shoot (F53's lens, F55's row ends). An account's Sessions tab groups its sessions under Needs you, Working and Last 7 days, each row the App's icon with its state dot, the title, project · App · time, and the state word at the end in its colour; session-terminal matches 05-session-panel.

- **Closed since 0.3.2:** "Stopped with an error" is red in the list and the header alike (it was amber in the list); the account page's breadcrumb reads "Account", no longer under a tool.
- **Weak, kept:** empty-* is Home with nothing opened (no empty panel state exists to show); the list's rows are split by hairlines inside the card, a rule inside an area that PAT-SURF-1 allows.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | An account's or project's sessions, and a session's terminal. |
| 2 | Prominence | 4 | Needs you first, then Working, then the last 7 days. |
| 3 | Default | 4 | New session is the page's one primary. |
| 4 | Mode | 4 | Calm. |
| 5 | Mock-up | 4 | SPEC §9's account and project pages. |
| 6 | Areas | 4 | Header, tabs, one grouped card. |
| 7 | Colour | 4 | State words in their hue only; violet only on New session. |
| 8 | Type | 4 | Titles ink, meta muted, the group heads quieter. |
| 9 | Actions | 4 | One filled primary. |
| 10 | Serious | 4 | n/a. |
| 11 | Doors | 4 | Every row opens its session; the project and App in each row are named. |
| 12 | Words | 4 | The rail's words ("Probably stuck", "Stopped with an error"). |
| 13 | States | 4 | Grouped by state; empty groups not drawn. |
| 14 | Consistency | 4 | The rows of History and the project page. |
| 15 | Accessibility | 4 | axe clean in the capture runs (no serious or critical); keyboard and zoom not re-walked this round. |
| 16 | Hand walk | 4 | Not re-walked by hand here; captured from the fixtures, both themes. |
| 17 | System prompts | 5 | Causes no system prompt. |
| 18 | Usability | 4 | AJ-E2327 (6): nothing contradicts the rail (the same states and counts). Match. |

Lowest: 4 (every line 4 or 5).

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Needs you's cards are tone fills with their level's mark (a stopped one in the broken tone), the most urgent first (F53, F66, F80); the lens switch's chosen segment is a solid fill; rail rows carry a model mark where the model changed (F77). Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
