<!-- captures: bfdfb5e29506 -->
# 99-account-page: an Account's own page, and the Accounts screen that leads to it (0.4, D-62, AJ-E3205)

Design critique (D-57, AJ-E2814). First critique of this family, written at 0.4 from its captures.

## 0.4

The audit: an account (a subscription) had no page; its name was its folders' names ("Work, Work 2 and 1 more"), every card carried a lone "C" that told Claude Code from Codex for no one, and limits, renewal and fee were set per folder. Four screens, both themes, on the places desktop: the Work Max folder signed in to a Max 20x plan, used by Claude Code in a terminal and by the Claude app's Work profile; a Personal folder on another sign-in.

**accounts-screen.** Each card's title is the Account's own name: "Claude Max 20x" (its plan; no team, no name given), "p…@example.com" (no plan either). Under it one line with what the name doesn't already say (the masked email, the renewal, the fee). No tool letter in the header: the colour dot (its colour in the charts) is the only mark. With accounts on one tool the line doesn't name the tool; on two tools it would. The title is a button with "Open Claude Max 20x's page". Under the limits, one row per App (AJ-E3206 req. 2): "Claude Code · Work Max", its folder's CLI App, marked with the app's icon and the profile's dot as the rail marks it (no "WM" initials), with where its sessions are open and its folder; then "Claude app · Work", the desktop App signed in to the same plan, "3 sessions · 1 h ago" and a chevron to its page.

**account-overview.** "Account" over the name, a tile with the account's chart colour, "Claude Max 20x", then "w…@example.com · 2 apps · 5 open", and New session at the right. Tabs: Overview, Apps 2, Sessions 6, Projects 2, Usage, Settings. Limits first (the account is where limits live), then "Its apps, they share these limits": Claude Code · Work Max and Claude app · Work, each with "3 open · 3 in all" and a chevron to the App's page. Then Open now and Last used across both Apps; each row names its App ("in Claude app · Work", "in Terminal") and not the account. Then Projects. At 1024 the cards stack the same, the rows cut with an ellipsis.

**account-apps.** The two Apps in a card, as rows with icon, app, profile dot and name, counts and a chevron; an App with no session yet would sit below, on a tinted surface under "No sessions yet".

**account-settings.** Name ("Claude Max 20x", "The name AI Juggler gives it: its team, else its plan, else its email.", Rename), then "Plan, renewal and fee" for the whole account: Plan (read from the sign-in, "Max 20x"), Renewal date (a date field), Monthly fee (a field and Save), and the footer "Entered by you, for the whole account: AI Juggler doesn't read your billing."

## Fixed while critiquing

- A session that ended in the Claude app said "in Claude Code" (its tool) on the account's page: rows there name the App the rail puts it under.
- Rows said "in Claude" where every other place says "Claude app": a row's app is its app's name on screen.
- The Apps tab's rows sat bare on the page; they're in a card like every other list.

## Weak

- The Sessions tab, read a page at a time from the backend, still names an ended desktop session by its tool; the Overview's rows are right. With AJ-E3206.
- The renewal date field is the browser's own date control (mm/dd/yyyy in this locale), not the app's styled one.
- An App's Settings (Claude Code · Work Max) still shows its folder's plan and renewal; they belong to the Account now and should point there (AJ-E3202 req. 4, with the choice where folders disagree).
- The header tile is only a colour; an account has no mark of its own.
