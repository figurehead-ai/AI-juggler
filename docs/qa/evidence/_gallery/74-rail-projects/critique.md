<!-- captures: ee66f72806a0 -->
# 74-rail-projects: the rail grouped by project, on a first-run home

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

history-1280x800 shows the rail's Project view (Projects heading, six projects, the shopfront folder opened to two children, "Show all 6") beside a home that says "1 thing needs you". The cards stand off the page in both themes: dark cards sit a step above the near-black canvas, and in light the white Start your first session, Needs you and Runway cards read clearly on the grey canvas. The primary actions (New session in the card and in the header, Sign in, Allow) are one solid violet, so the eye goes to New session first; the rail's own New session stays a quiet outline, which is right next to it. After the design fixes the dark accent is lighter on the logo mark, the active filter button beside the search and the rail's New session outline; the top bar stays one row and nothing moved in light. The intro sentence ("Pick a project folder…") is brighter than the labels under it. After the second pass the Needs you card's detail is whole on two lines ("…AI Juggler never / sees your password.") where it was cut to "AI Juggler nev…", so the card is about 16 px taller. The one weak spot is still the rail rows' second line, cut to "41 conversat…" and "W…", so the account at the end of each line never shows at this width.

## Weak

- history-1280x800 (both themes): the rail rows' second line is cut ("20 min ago · 41 conversat…", "9 conversations · W…"), so the account never shows.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: history-1280x800 dark (and light), beside the 0.3.2 one. The Project lens on a first-run Home: "Projects 6" with each project's mark, its last use · conversations · account, the shopfront group folded open with shopfront3 and shopfront4, "Show all 6". Home behind: "1 needs you" with Work Max's sign-in card (Sign in ↵ the one filled button), the feed's empty state, the permission card, Your apps (two not running, Start) and Runway.

- **Since 0.3.2:** the calm rail rows and a project group's fold; the Home behind is D-118's.
- **Weak, kept:** each project's second line still cuts before its account ("20 min ago · 41 conversatio…"); the account is one hover away (title). The empty feed under it shows its five chips and Showing: with nothing to filter (**F56**, logged from 55-triage). Runway's "Not signed in · Not measured yet · How it's measured ›" explains measuring for an account whose real need is its sign-in (said in Needs you above).

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Every project, newest first, one click to its page. |
| 2 | Prominence | 4 | Needs you first; the projects list in the rail, the account last on its line. |
| 3 | Default | 4 | Six shown, the rest behind Show all. |
| 4 | Mode | 4 | Calm. |
| 5 | Mock-up | 4 | Matches calm-ui's project lens. |
| 6 | Areas | 4 | Rows by tone; the cut meta line is the row's own truncation (title on hover). |
| 7 | Colour | 4 | The project marks' hues once per row; amber only on the sign-in count. |
| 8 | Type | 4 | Project names 13 px, meta 12 px muted. |
| 9 | Actions | 5 | Sign in ↵ the one filled button. |
| 10 | Serious | n/a | Nothing destructive. |
| 11 | Doors | 4 | Each project opens its page; Show all 6 opens the rest. |
| 12 | Words | 4 | Plain; the empty feed has no chips since F56. |
| 13 | States | 4 | First run: the feed's empty state with New session and Open an app. |
| 14 | Consistency | 4 | The rail's row pattern. |
| 15 | Accessibility | 4 | Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | The PL1 walk (AJ-E2323) drove Home and the rail in Dev; not re-walked here. |
| 17 | System prompts | 4 | The permission card primes Accessibility and Automation before any prompt (PAT-OSP). |
| 18 | Usability | 4 | A newcomer sees their projects and the one thing to do (sign in). |

Lowest: 4. F56 is Home's (scored 3 in 55-triage and 67-home).

**Re-captured after F56 (2026-11-24, Opus):** both re-rendered and opened. The Home behind the Project lens shows the empty feed as its card alone, with Include my actions in it; its chips, Showing: and ⋯ are gone (F56). The projects' lines are unchanged. Lowest stays 4.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: The system-prompt card's Allow is restyled (one look for a secondary action). Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
