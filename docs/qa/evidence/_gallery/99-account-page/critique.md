<!-- captures: 91098976e67d -->
# 99-account-page: an Account's own page, and the Accounts screen that leads to it (0.4, D-62, AJ-E3205)

Design critique (D-57, AJ-E2814). First critique of this family, written at 0.4 from its captures; a second section for AJ-E3202 req. 4 (plan, renewal and fee kept on the Account).

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
- The header tile is only a colour; an account has no mark of its own.

## 0.4, AJ-E3202 req. 4: plan, renewal and fee set once, on the Account

The audit: plan, renewal date and monthly fee were kept per folder, and the Account's Settings wrote the same value into each of its folders. Two folders on one plan could say two renewal dates, and an App's Settings had its own editors for them. Now the three are stored on the Account. Where an Account's folders agree, the value moves there on the next read. Where they disagree, each folder keeps its own until the user picks one, once. Four new captures per theme, on a second fixture: Work Max and Work 2 on one Max 20x plan, set up apart in 0.3. Their plan (max-20x) and fee ($200) agree, and their renewal dates (12 Nov and 3 Dec) don't. The six older captures were re-shot with the standard fixture.

**accounts-choose.** The Claude Max 20x card's line reads "w…@example.com · $200/month". The fee moved to the Account, and the renewal is left out because it isn't chosen yet. In the header, between the line and "2 apps share this account", is a warning chip, "⚠ Choose its renewal date", with the same shape and tone as the app's other warning chips. It's a button that opens the Account's Settings. Under the limits, each App row shows its own date until the choice: "Claude Code · Work Max … Renews in 11 days" and "Claude Code · Work 2 … Renews 3 Dec", so nothing is hidden. Light: the chip is the warning-soft fill with dark amber text and passes axe contrast. (A first version with a hand-made 10% amber fill failed contrast in light and was swapped for the Chip primitive.)

**account-choice.** The Account page opens on Settings with the same chip beside the name. The header facts are "w…@example.com · $200/month · 3 apps · 5 open". Under Name, "Plan, renewal and fee" starts with the choice as the first row of its card, on a faint amber tint:
- The question, after a warning icon: "Work Max and Work 2 say different renewal dates: 12 Nov or 3 Dec?" It uses the maintainer's own wording, with the Apps' profile names and short dates.
- One line on what the choice changes: "An account has one renewal date: renewal reminders and its card use it, for every app on it. Until you choose, each app keeps its own."
- Two option buttons, each with two lines: the value in body type ("12 Nov", "3 Dec") over the App that says it in muted type ("in Claude Code · Work Max", "in Claude Code · Work 2").

Then the three rows:
- Plan: "Max 20x", said as words and not as the stored "max-20x". Save stays off until the text changes. Plans are compared by their words, so "Max 20x" and "max-20x" never ask.
- Renewal date: an empty date field, described as "Its apps say different renewal dates: choose above, or set one here."
- Monthly fee: 200 and Save.

The footer reads "Entered by you, once for the whole account and every app on it: AI Juggler doesn't read your billing."

**account-chosen.** After a click on 3 Dec, the choice row and the header chip are gone. The facts now read "Renews 3 Dec · $200/month". The date field holds 12/03/2026 and its description is back to "When the plan renews: reminders come before it." Nothing asks again. Each folder's own date stays stored, but the folders aren't written any more (the Rust tests pin this).

**app-settings-terms.** The App page for Claude Code · Work Max, Settings tab. Under Name, a new "Plan, renewal and fee" section built from the same rows as every Settings pane:
- A first row, "On Claude Max 20x" / "They're its account's, set once for every app on it.", with **Set on its account** at the right. It opens the Account's Settings tab (the test clicks it).
- Read-only rows below: Plan "Max 20x", Renewal date "3 Dec", Monthly fee "$200/month". The values are right-aligned in ink.

The old "Plan and renewal" cell, with its date field, Clear and "Flat fee $/mo" editor, is gone from the App's Settings card. That card now starts with Tidying up. An App whose sign-in isn't read yet has no Account and keeps those editors (vitest pins this).

**accounts-screen, account-overview(-1024), account-apps, account-settings (re-shot).** The standard fixture with nothing to choose. account-settings now has the editable Plan row ("Max 20x", Save) where it used to say "as its sign-in says", which was wrong: a plan is entered, never read. It also has the empty fee field and the new footer. The re-shoot also picks up two earlier 0.4 changes that this family hadn't been shot with: the Claude app profile is now named by its Claude Code folder ("Claude app · Work Max", AJ-E3203), and the Account's dot, its chart line and both App dots share one colour (AJ-E3206's one mark per App). Before, the card was orange and the Claude app's dot blue.

### Fixed while critiquing

- The choice chip was a hand-made amber pill that failed axe contrast in light. It's the Chip primitive now, wrapped in a button, and both the card and the page use one `ChooseChip`.
- The Plan field showed the stored "max-20x" under a description that said "Max 20x". It shows the words now, and the description no longer repeats them.
- The fee read "$200 a month" in the choice and the App rows but "$200/month" on the cards and the header. It's "/month" everywhere.

### Weak

- "2 apps share this account" on the card counts folders, while the card lists three App rows (with the Claude app). That's AJ-E3206's count, not this task's.
- The choice offers only the values its Apps said. Another date is set in the row below it ("or set one here"), which is one more step than a third option would be.
- The renewal field is still the browser's own date control (mm/dd/yyyy).

## 0.4: a renewal said as a date

The weak point above: the Account page's header and the Accounts card's App rows said a renewal as the stored ISO string ("Renews 2026-12-03"), while the choice and the App's Settings said "3 Dec". Every renewal label now comes from one rule (`renewalLabel`), and its date is said by the same words the choice uses (`dateWords`: "3 Dec", with the year only when it isn't this one). The day before, it says "Renews tomorrow" (it said "Renews in 1 days"). Four captures changed; the other fourteen are as they were.

**accounts-choose (both themes).** The Work 2 row under the Claude Max 20x card reads "Claude Code · Renews 3 Dec · ~/.claude-claude-work-2" (it read "Renews 2026-12-03"). Work Max's row still says "Renews in 11 days": inside two weeks a renewal counts down, as before.

**account-chosen (both themes).** After 3 Dec is chosen, the header facts read "w…@example.com · Renews 3 Dec · $200/month · 3 apps · 5 open". The header, the card's rows, the choice ("12 Nov or 3 Dec?") and the App's Settings ("3 Dec") now say a date one way. The row is no wider than before: "Renews 3 Dec" is shorter than the ISO string it replaces.

### Weak

- The renewal date field itself is still the browser's control and shows 12/03/2026 beside "Renews 3 Dec" in the header: the same day, written two ways on one screen, until the field is the app's own.

## 0.4, AJ-E3206 req. 2: one mark and one colour per App

Re-shot after each App and each Account got one stored colour. All eighteen captures changed.

**account-overview, account-overview-1024, account-apps (both themes).** Before, the round mark by "Claude Max 20x", the dot by Claude Code · Work Max and the dot by Claude app · Work Max were one blue, because all three were coloured by the name "Work Max". Now the Account's round mark is rose, Claude Code · Work Max's dot is magenta and Claude app · Work Max's is orange: two Apps, two colours, and an Account in a third colour and a different shape. The Limits line is the Account's rose. The rail's dots and row edges match the Apps' rows here.

**account-settings, account-choice, account-chosen (both themes).** The header's round mark is rose. The rail on the left shows each App's own colour on its header, rows and edges.

**accounts-screen, accounts-choose (both themes).** Each Account card's dot is its own: Claude Max 20x rose, the personal account teal (before: blue and violet, its first folder's colours). The card's limits line is the same colour. The App rows below it carry their App's dot: Claude Code · Work Max magenta on its icon's corner, Claude Code · Work 2 orange, Claude app · Work Max orange or rose depending on how many folders the capture has.

### Weak

- In accounts-choose, with three folders, Claude app · Work Max takes the slot whose hue is the Account's rose. With eight hues, the fifth App always shares a hue with the first Account, so App and Account are told apart by shape: a dot on the icon's corner against the round mark by the Account's name.
- The personal account's teal is 27° from the live colour. It sits by the Account's name, never by a session's state.

## 0.4, AJ-E3103: the one related list and the one row

Recaptured: account-overview and account-overview-1024, in both themes. The rest were left as they were.

**account-overview.** The Open now and Last used cards use the one row. Line 2 reads "◆ ai-juggler · ● Claude Code · Work Max · just now", ending in the state word. The row names its App with the App's colour dot, as the rail does. The Account is the page's own and gets no colour there, following the 2026-11-03 ruling. In a half-width card the App's name truncates before the time does.

### Weak

- In a half-width card the App's name is cut to "Claude Code · …" or "Claude…". The icon says which app it is, but not which profile.

## 0.4.1 re-score (2026-11-07, AJ-E3305)

The whole family was retaken on release/0.4.1 for the 14-line re-score (AJ-E3305, `docs/qa/0.4-walks/design.md`) and opened on contact sheets, both themes, 1280 and 1024. Shared with every screen since the last shoot: the header's New session is tinted (one filled button per view), the Floor button is a split control with its own menu, This Mac's line is its dot below 1120 px, and the rail's App headers put the profile first.

The Account page's line names it without its e-mail ("2 apps · 5 open"; 059ff2db9), the window says its name, and with nothing measured its Limits end in "Measure its limits ›" where measuring can be switched on (d8dd6b4a1). The Accounts cards' link reads "Measure its limits ›" and opens the Account. Renewal: the newer date chosen, ⌘↩ (account-choice).
