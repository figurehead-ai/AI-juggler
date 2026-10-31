<!-- captures: 81731aa52bc7 -->
# 71-add-account: adding a fresh account: name it, then its sign-in step

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

In fresh, the page reads top to bottom: stepper, heading, a brighter semibold intro, the focused Name field with a violet ring, then a styled "Start with the same settings as Work Max" chip in place of the native select, and one solid violet "Create and sign in". After the design fixes the dark accent is lighter on the focus ring, the current step's number, the checkbox and the Consumer Terms link, in fresh and fresh-signin at both widths; nothing else moved. The disclosure for "Its MCP servers, permissions and status line ▸" is still a tiny triangle at the end of the line that is easy to miss. In fresh-signin the one account row has a solid violet Sign in and a quiet "Sign in later" below. At 390 px the row's name is still cut to "Cla…" while "Not signed in yet" keeps its full width, and the stepper breaks after step 1, leaving the connector line dangling at the start of the second row. The sign-in step's stepper still says "Bring in your accounts" for step 1, while step 1 of this flow is called "Name the account".

## Weak

- fresh-signin-390x844-dark/light: the account name is cut to "Cla…", the one word that says which account it is.
- fresh-signin-390x844-dark/light: the stepper wraps after step 1; the connector line sits alone at the start of the second row, before "2 Sign in where needed".
- fresh-signin-*: the stepper names step 1 "Bring in your accounts", but this flow's first step is "Name the account".
- fresh-*: the expander for "Its MCP servers, permissions and status line" is a tiny trailing triangle.
