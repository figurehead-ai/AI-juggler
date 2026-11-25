<!-- captures: dcf90343dca0 -->
# 71-add-account: adding a fresh account: name it, then its sign-in step

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## rc31 (0.4)

fresh-* now has "Add, sign in later" as a secondary button left of the one violet "Create and sign in", the pair right-aligned at 1280 and a row of their own under Import and Look again at 390, both readable in full. The intro says "Then you sign in with Claude Code's own sign-in, now or later". Later adds the App signed out, as first run's Sign in later does; its card says "Not signed in · Sign in". fresh-signin-* re-rendered from HEAD: the stepper reads "Bring in your apps", and the row's name is still cut to "Clau…" at 390 px.

## 0.3.2

In fresh, the page reads top to bottom: stepper, heading, a brighter semibold intro, the focused Name field with a violet ring, then a styled "Start with the same settings as Work Max" chip in place of the native select, and one solid violet "Create and sign in". After the design fixes the dark accent is lighter on the focus ring, the current step's number, the checkbox and the Consumer Terms link, in fresh and fresh-signin at both widths; nothing else moved. The disclosure for "Its MCP servers, permissions and status line ▸" is still a tiny triangle at the end of the line that is easy to miss. In fresh-signin the one account row has a solid violet Sign in and a quiet "Sign in later" below. At 390 px the row's name is still cut to "Cla…" while "Not signed in yet" keeps its full width, and the stepper breaks after step 1, leaving the connector line dangling at the start of the second row. The sign-in step's stepper still says "Bring in your accounts" for step 1, while step 1 of this flow is called "Name the account".

## Weak

- fresh-signin-390x844-dark/light: the account name is cut to "Cla…", the one word that says which account it is.
- fresh-signin-390x844-dark/light: the stepper wraps after step 1; the connector line sits alone at the start of the second row, before "2 Sign in where needed".
- fresh-signin-*: the stepper names step 1 "Bring in your accounts", but this flow's first step is "Name the account".
- fresh-*: the expander for "Its MCP servers, permissions and status line" is a tiny trailing triangle.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B2)

The captures from d86510acc (this batch's capture left them byte-identical), both themes, 1280 and 390. fresh: "Name the app · Sign in where needed", "Add a fresh app.", the Name field, "Start with the same settings as Work Max", Add, sign in later beside the one primary "Create and sign in", Providers' terms folded. fresh-signin: the stepper says "Name the app" (the 0.3.2 mismatch is gone), one row "Claude Code · Terminal · Not signed in yet · Sign in".

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 5 | |
| 2 | Prominence | 5 | |
| 3 | Default | 5 | Same settings as the App in use, ticked. |
| 4 | Mode | 4 | The setup wizard's frame (D-126). |
| 5 | Mock-up | 4 | The d126 add-account reference. |
| 6 | Areas | 4 | |
| 7 | Colour | 5 | |
| 8 | Type | 4 | |
| 9 | Actions | 5 | |
| 10 | Serious | 4 | What is copied and never copied said. |
| 11 | Doors | 4 | |
| 12 | Words | 4 | "Its MCP servers, permissions and status line ▸" still ends on a small triangle. |
| 13 | States | 4 | |
| 14 | Consistency | 4 | |
| 15 | Accessibility | 4 | |
| 16 | Hand walk | 4 | PL1 D2 added an App in the Dev app. |
| 17 | System prompts | 5 | Sign-in is the tool's own. |
| 18 | Usability | 4 | |

Lowest: 4.
