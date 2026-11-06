# 0.4 design review: every screen against the 14 principles (AJ-E3305)

Date: 2026-11-05.

**What was scored.** The 24 gallery families that are new or changed in 0.4 (list below). Each was scored on the 14-line reviewer checklist in [`docs/design/PRINCIPLES.md`](../../design/PRINCIPLES.md#reviewer-checklist-score-15-each-ship-at--4-on-every-line).

**How.** From the gallery captures under `docs/qa/evidence/_gallery/`. I opened the PNGs and looked at each one: dark and light, at 1280×800 and 1024×700, and at 1440×900 where the family has it. For large families I looked at least at 1280 dark, 1280 light and 1024 of each distinct state. I read each family's `critique.md` for context, but every score below comes from the images. Where a critique and a capture disagree, the capture wins.

**Scale.** 1 to 5 per line. **Ship at ≥ 4 on every line.** "n/a" means the line truly has nothing to judge on that screen. The reasons are under the table.

**Line 14 (hand check).** None of these screens was walked by hand in the native window for this review. Line 14 is scored from what the captures and critiques show:
- 2 means both sizes and both themes were captured and described.
- 1 means a size or theme is missing, or the captures are stale.

No family reaches 4 on this line. That's the honest state, not a judgement on the design.

---

## Summary

| Family | Fit | Prominence | Default | Folding | Once | Areas | Colour | H/L/V | One primary | Pickers/lists | Doors | Words | States | Hand check |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 99-account-page | 3 | 4 | 3 | 3 | 3 | 4 | 4 | 4 | 2 | 4 | 4 | 3 | 3 | 2 |
| 99-account-quality | 3 | 4 | n/a | 4 | 2 | 4 | 4 | 4 | 2 | n/a | 3 | 3 | 3 | 2 |
| 99-app-page | 4 | 4 | 4 | 4 | 3 | 4 | 4 | 4 | 2 | 4 | 3 | 4 | 3 | 2 |
| 99-app-profiles | 4 | 4 | 4 | 4 | 3 | 4 | 4 | 4 | 2 | 3 | 4 | 3 | 4 | 2 |
| 99-carry-choice | 3 | 4 | 4 | 4 | 4 | 4 | 4 | 4 | 4 | n/a | n/a | 2 | 4 | 2 |
| 99-debug-mode | 3 | 4 | 4 | 4 | 3 | 4 | 4 | 4 | 4 | n/a | 4 | 3 | 4 | 2 |
| 99-first-run-confidence | 3 | 3 | 4 | 4 | 3 | 4 | 4 | 4 | 4 | 4 | n/a | 3 | 4 | 2 |
| 99-floor-list | 3 | 3 | n/a | 4 | 2 | 4 | 4 | 4 | 4 | 3 | 3 | 3 | 3 | 2 |
| 99-floor-menu | 3 | 3 | 4 | 4 | 4 | 3 | 4 | 4 | n/a | n/a | 4 | 4 | n/a | 2 |
| 99-floor-settings | 3 | 4 | 4 | 4 | 3 | 4 | 4 | 4 | 4 | n/a | 4 | 3 | 4 | 2 |
| 99-home-chips | 3 | 4 | 4 | 4 | 2 | 3 | 3 | 4 | 2 | 3 | 4 | 3 | 3 | 2 |
| 99-inactive-apps | 4 | 4 | n/a | 4 | 3 | 4 | 4 | 3 | 2 | 4 | 4 | 3 | 3 | 1 |
| 99-new-session | 4 | 4 | 3 | 4 | 3 | 4 | 4 | 4 | 4 | 3 | n/a | 3 | 4 | 2 |
| 99-palette | 3 | 3 | 4 | 4 | 3 | 4 | 4 | 4 | 4 | 4 | 4 | 3 | 4 | 2 |
| 99-rail-splitter | 3 | 3 | n/a | n/a | n/a | 2 | 4 | 3 | n/a | n/a | n/a | n/a | n/a | 1 |
| 99-related-list | 4 | 4 | 4 | 4 | 2 | 4 | 4 | 4 | 1 | 4 | 4 | 4 | 4 | 2 |
| 99-report-bug | 3 | 4 | 4 | 3 | 3 | 3 | 3 | 4 | 4 | n/a | n/a | 4 | n/a | 2 |
| 99-resume | 3 | 4 | 4 | 4 | 2 | 4 | 4 | 4 | 2 | n/a | 4 | 2 | 3 | 2 |
| 99-settings-keyboard | 3 | 4 | 4 | 4 | 3 | 4 | 4 | 4 | 4 | n/a | 4 | 3 | 4 | 2 |
| 99-settings-search | 3 | 4 | 4 | n/a | 4 | 4 | 3 | 4 | 4 | 4 | 3 | 4 | 4 | 2 |
| 99-usage-cost | 3 | 4 | n/a | 4 | 3 | 3 | 4 | 4 | 4 | 4 | 4 | 2 | 4 | 2 |
| 99-when-to-tell | 3 | 4 | 4 | 3 | 3 | 3 | 4 | 3 | 4 | 4 | n/a | 3 | 4 | 2 |
| 91-pools | 3 | 4 | 4 | 4 | 3 | 2 | 4 | 4 | 2 | 4 | 3 | 2 | 4 | 1 |
| 97-pool-canvas | 4 | 3 | 4 | 4 | 3 | 3 | 4 | 3 | 2 | 4 | 3 | 3 | 3 | 2 |

**Not one family ships at ≥ 4 on every line: 134 lines are under 4 across the 24 families.** Leaving out line 14, the closest are 99-carry-choice and 99-related-list, with two lines under 4 each. 99-debug-mode, 99-floor-menu, 99-floor-settings, 99-settings-keyboard and 99-settings-search have three each.

### Why "n/a"

- **Default, n/a** (account-quality, floor-list, inactive-apps, usage-cost): read-only views. There's no choice to pre-fill, only filters left at All.
- **Pickers/lists, n/a** (account-quality, carry-choice, debug-mode, floor-menu, floor-settings, report-bug, resume, settings-keyboard): no picker, and no list that grows.
- **Doors, n/a** (carry-choice, first-run-confidence, new-session, when-to-tell, report-bug): a sheet or wizard step where the names are choices, not objects with pages.
- **One primary, n/a** (floor-menu): a menu has no filled button.
- **States, n/a** (floor-menu, report-bug): nothing here can be empty, loading or unknown.
- **Folding, n/a** (settings-search): the search has nothing rare to fold.
- **99-rail-splitter, mostly n/a:** the family is one control, the rail's edge. Only Fit, Prominence, Areas, Colour, H/L/V and Hand check say anything about it.

### Two problems that cross every screen

These are scored on each family. In "Every no" they are one line each, because one fix closes all of them.

1. **Two or three filled accent buttons per view.** The top bar's "New session" is a filled accent button on every page. Every page with its own filled button therefore shows two, and sometimes the same words twice:
   - "New session" in the top bar and in the header, on the Account page and on a CLI App page.
   - Three filled buttons on `99-related-list/app-empty-dark.png` (top bar, header, empty card).
   - Three filled buttons on `99-related-list/home-recent-dark.png` (New session, Open its page, Allow).

   **Fix:** make the top bar's New session a tinted secondary button, as the rail's own New session already is. The page's action then is the one filled button.
2. **The top bar cuts its own words.** Since the Floor button came in, "This Mac · Plenty of room" reads "This Mac · Ple…" or "This Mac · Pl…" at 1280, for example on `99-floor-list/list-1280x800-dark.png` and `99-floor-menu/menu-1280x800-dark.png`. It drops out entirely at 1024.

   **Fix:** shorten it to the dot plus "Plenty of room", with the full words in its tooltip. Also say "this computer", not "Mac", in the app's own words.

### Line 1 (Fit), checklist critiques

The 18 families scored Fit 3 have critiques in the short checklist form. Those critiques state the user value, but not:
- what the screen depends on;
- how it fits the screens around it, in both directions;
- what changes for someone who used 0.3.

**Fix:** add a "Fit" paragraph to each of those critiques, in the form the Account, App and New session critiques already use. In three cases what the screen states is also not true on screen. Those are scored and explained under the family: account-page, carry-choice and resume.

---

## Per family

### 99-account-page: an Account's page and the Accounts screen

The Accounts screen lists each subscription with its limits and its Apps. An Account's page has Overview, Apps, Sessions, Projects, Usage and Settings.

- **Fit 3.** The renewal date is said to belong to the Account, but `accounts-choose-dark.png` still shows it per App: "Renews in 11 days" on the Claude Code · Work Max row and "Renews 3 Dec" on Work 2, while the card's chip says "Choose its renewal date". The critique also admits that the Sessions tab names ended desktop sessions by their tool. **Fix:** say renewal only on the Account card, and fix the Sessions tab's names.
- **Default 3.** `account-choice-dark.png`: "Work Max and Work 2 say different renewal dates: 12 Nov or 3 Dec?" offers two equal tiles with nothing chosen. **Fix:** pre-select the later date, say why in three words ("the newer reading"), and make ⌘↩ accept it.
- **Folding 3.** `accounts-screen-dark.png`: every Account card always shows the paragraph "No current reading. Limits come from the Claude app's meter…" and each App row's folder path in the mono face. **Fix:** fold the paragraph behind "Not measured yet ▸", and show the folder path only on the App's page.
- **Once 3.** `accounts-screen-dark.png`: an unmeasured card says it three times: the "–" dial, "Not measured yet", and the paragraph under the chart. The Account page header shows "New session" twice (top bar and header). **Fix:** one "Not measured yet · Measure its limits…" line, and no dial.
- **One primary 2.** `account-overview-light.png` and `account-overview-1024-dark.png` have two filled "New session" buttons. `accounts-screen-dark.png` has a filled "Add an app" beside the filled top-bar New session. **Fix:** see the cross-screen fix above.
- **Words 3.** `accounts-screen-dark.png`:
  - The intro's example "Claude app · b-work" is a raw folder name.
  - The Claude Code · Work Max row's second line reads "Claude app, Terminal", so a Claude Code App says it runs in the Claude app.
  - The Overview names the desktop App "Claude app · Work Max", the account's folder name. The critique calls it "Claude app · Work".

  **Fix:** use "Claude app · Work" as the example. Say "in a terminal · 5 open" on a CLI App's row. Settle one App name and use it on every screen.
- **States 3.** Same as Once: the unknown is said three times per card. **Fix:** as above.
- **Hand check 2.** Both themes are captured, but only the Overview has a 1024 capture.

### 99-account-quality: "How its sessions went"

A section on the Account page: errors and retries, probably stuck, context at finish, and tokens per finished task.

- **Fit 3.** The critique has no Fit paragraph (see Line 1 above).
- **Once 2.** `measured-1280x800-dark.png`: each label is repeated in its own value ("Errors and retries" / "9 errors or retries / 300 turns"; "Probably stuck" / "2 times probably stuck"). The source line "read from each session's own files when it ended" appears twice. **Fix:** values only ("9 in 300 turns", "2 times"). Say the source once, under the section title, and only say a row's source where it differs.
- **One primary 2.** This is the cross-screen problem: the header's New session and the top bar's are both filled.
- **Doors 3.** "9 errors or retries" and "2 times probably stuck" are plain text. You can't open the sessions they count. **Fix:** make each figure a link to the Sessions tab, filtered to those sessions.
- **Words 3.** `not-reported-1024x700-light.png`: a "Claude Max 20x" Account says "Codex doesn't record errors in its files" and "Not reported by Codex". That's a Codex tool on a Claude plan. **Fix:** name the App whose data is missing ("Not recorded by Codex · Home"), and give the fixture a Codex Account.
- **States 3.** The same capture says "not known" three different ways in four rows: "Codex doesn't record…", "Not reported by Codex", and "No finished tasks in these 7 days". **Fix:** one line under the title ("Codex doesn't record errors or context, so those rows are left out"), and drop those rows.
- **Hand check 2.**

### 99-app-page: an App's own page

The page shows the App's icon and name, a facts line, Go to it, Start or New session, and tabs: Overview (Open now, Last used, Limits, Projects), Sessions, Projects, Usage, Processes and Settings.

- **Once 3.** `app-overview-dark.png`:
  - "Claude app" is the title's first word and again the facts line's first word.
  - "3 sessions · 2 open" is in the facts line, again in the tab counts ("Sessions 3"), and again as "Open now 2 open".

  On `app-sessions-light.png`, "3 sessions · 2 open" is said a fourth time above the list, and the "Working" heading sits over a row that says "Working". **Fix:** start the facts line at the state ("Running · kept alive"), and drop the line above the list.
- **One primary 2.** "Go to it" is filled next to the filled top-bar New session. On `app-cli-dark.png` there are two filled "New session" buttons. **Fix:** see the cross-screen fix.
- **Doors 3.** The facts line never names the Account, so there's no way from an App to the subscription it uses. The critique admits it. **Fix:** add "on Claude Max 20x" to the facts line, as a link to the Account page.
- **States 3.** `app-cli-dark.png`: the "Last used" card takes half the row to say "Everything here is open now." **Fix:** when there's nothing ended, drop the card and let Open now take the full width.
- **Hand check 2.** Also: on `app-overview-dark.png` the state words (Working, Asks you, Ended) float about 100 px in from the card's right edge, so they don't line up with anything. Align them to the edge.

### 99-app-profiles: profiles found when they aren't running

The ask card on an App page, the found result, a found App's page, Home's closed tiles, and Settings › Desktop apps.

- **Once 3.**
  - `found-dark.png`: under the "Inactive apps 4" heading, every row repeats "Not running".
  - `home-dark.png`: "Start Claude app" four times. Two tiles say "Not running · found with your other profiles".

  **Fix:** in the Inactive apps section, show only when each App was last seen. Label the tile buttons "Start".
- **One primary 2.** "Go to it" plus the top bar's New session.
- **Pickers/lists 3.** `settings-light.png`: Settings › Desktop apps lists Lab (not running) first, then Work Max, ChatGPT, Design, Old, Personal and Research. That's neither running-first nor A to Z. **Fix:** running Apps first, then the closed ones by last seen, then the found ones.
- **Words 3.**
  - The desktop profile is titled "Claude app · Work Max" after a Claude Code folder, not by its own name. The critique says this is on purpose, but the same profile is "Work" elsewhere.
  - `settings-light.png`: "Forget" is bare text on one row only (Old). It pushes that row's Start button out of the column.

  **Fix:** one rule for an App's name, used everywhere. Put Forget in the row's ⋯ menu so the buttons line up.
- **Hand check 2.** Only the ask has a 1024 capture.

### 99-carry-choice: "Very long conversation? Leave out its start"

A fold on the switch's confirm step.

- **Fit 3.** No Fit paragraph.
- **Words 2.** `last-1280x800-dark.png`: the green chip "✓ Everything goes along · 5 items" stays as it is while "Keep only the last 200 turns" is chosen. The text below says it "leaves out 800 turns". The chip says the opposite of the choice. Also:
  - The two Apps are called "Desktop" and "CLI", though everywhere else they're "Claude app" and "Claude Code".
  - The dates come in two forms ("Sep 30, 2024", "1 October").
  - The date field shows the system's "mm/dd/yyyy".

  **Fix:** when part is left out, the chip turns amber and says "Last 200 turns go along · 800 left out". Call the cards by their App names. Use one date form, and the app's own date field.
- **Hand check 2.**

### 99-debug-mode: Settings › Advanced › Debug mode

- **Fit 3.** No Fit paragraph.
- **Once 3.** `section-1280x800-dark.png`: the section title "Debug mode" sits directly over a row titled "Debug mode". **Fix:** call the row "Record each step", or drop the section title and let the card stand under Diagnostics.
- **Words 3.** The same capture: the Diagnostics card just above has a fold called "Backend response times", an internal word. **Fix:** "How fast AI Juggler answers".
- **Hand check 2.**

### 99-first-run-confidence: first run says how sure it is

- **Fit 3.** No Fit paragraph.
- **Prominence 3.**
  - `found-1024x700-light.png`: at 1024×700 the "Bring in 2 apps" button is below the fold. The first screen has no way forward.
  - `found-1280x800-dark.png`: the button sits at the bottom left, after four equal-weight text links ("Add a fresh app instead", "Import from another folder…", "Look again", "Where we looked").

  **Fix:** keep the footer fixed at the bottom with the primary at the right. Put the four rare links behind "Other ways ▸".
- **Once 3.** "You can rename it after" is on two rows, and "Names and plans can be changed later" repeats it in the footer. `details-1280x800-light.png` repeats each row's Found/Likely/Guessed chip in a third column. **Fix:** say it once in the footer, and leave the chip column out of Details.
- **Words 3.**
  - The same thing is called "setups" in the heading and the section labels, and "apps" in the step and the button.
  - Details shows "Sign-in: May have expired · Likely": two confidence words that pull against each other.
  - "named by cc-switch" names a tool without saying what it is.

  **Fix:** call them "apps" throughout. Write "Sign-in: Likely expired". Write "named by another tool (cc-switch)".
- **Hand check 2.**

### 99-floor-list: the Floor list

One card per App with a table of its sessions, grouped by floor.

- **Fit 3.** No Fit paragraph.
- **Prominence 3.** `list-1280x800-dark.png`:
  - "Show on the Floor" is on every row, as loud as the data.
  - In Claude Code · Work Max, "Working" sits above "Asks you" and "Probably stuck", so what waits on you isn't first.

  **Fix:** sort each App's rows waiting-on-you first. Show "Show on the Floor" on hover and focus, and as the row's ↩.
- **Once 2.**
  - The column-header row (State, Session, Project, Subagents, Tasks) is repeated in every App card: three times on floor 1.
  - "Work Max · 62% left" appears on two App headers that share one Account.
  - "Show on the Floor" appears seven times.

  **Fix:** one sticky header row per floor. Say the room once, on a floor-level Account line.
- **Pickers/lists 3.** No search, no filter for "waiting on you", and no paging, though the critique sizes it for 150 rows. **Fix:** a search field, a "Waiting on you 5" chip, and the shared paged list.
- **Doors 3.** Session names and App names are plain text. The only door is "Show on the Floor". **Fix:** the session name opens its page, the App name opens the App page, and "Show on the Floor" stays as the row action.
- **Words 3.**
  - Initials ("WM", "HO") mark the Apps, where the rest of 0.4 moved to the app's icon and the profile's dot.
  - "Claude app · work" is lowercase.
  - "Workshop", "Studio" and "Industrial" are style names that are never explained.

  **Fix:** use the rail's App mark and the App's own name, and show the style name only in Floor settings.
- **States 3.** Subagents and Tasks are "—" in 12 of 14 cells. **Fix:** hide a column that has no values on this floor, and show a figure only where there is one.
- **Hand check 2.**

### 99-floor-menu: the Floor button's menu

- **Fit 3.** No Fit paragraph.
- **Prominence 3.** `menu-1280x800-dark.png` and `menu-1024x700-light.png`: the arrow that opens the menu is a small dot between the Floor icon and the avatar. It reads as a stray mark, not a control. **Fix:** a real chevron at the icon's weight, with a hairline between the two halves of the split button.
- **Areas 3.** Adding the button squeezed the top bar. "This Mac · Pl…" is cut at 1280 (see the cross-screen problems).
- **Hand check 2.**

### 99-floor-settings: Settings › The Floor, and Re-create

- **Fit 3.** No Fit paragraph.
- **Once 3.** `recreate-1024x700-dark.png`: the sheet makes the same promise three times. The intro says "The order stays, so every area stays where you know it". "Stays" says "Where each area is". The example says "keeps its place on floor 1". **Fix:** keep the Changes/Stays pair and the example. Cut the intro's second sentence.
- **Words 3.** `pane-1280x800-dark.png`: "Show unused Apps' decay" doesn't say what the user sees. "nook" and "camera" are inside words. **Fix:** "Let unused Apps gather dust", and "Back to the first floor" for Reset the view.
- **Hand check 2.**

### 99-home-chips: Home's Needs you chips

- **Fit 3.** No Fit paragraph.
- **Once 2.** `home-1280x800-dark.png`:
  - "5 sessions need you" (page title), "Needs you" (area title), "All 5" (chip) and the rail's "Needs you 5" all say the same thing on one screen.
  - Under the runway cards, a summary sentence ("Claude Max 20x has 58% left; f…@example.com isn't measured yet.") repeats the two cards right above it.

  **Fix:** drop the area title when the page title says it. Drop the summary sentence.
- **Areas 3.** `narrowed-1280x800-light.png`: narrowed to one item, the left column is one short card over 400 px of empty page, while the right column runs off the bottom. **Fix:** when Needs you is short, let Pick up where you left off move up into the left column.
- **Colour 3.** `home-1280x800-dark.png`: in dark, the selected chip ("All 5") is a barely lighter grey pill, so you can't tell the selected state without reading. **Fix:** give the selected segment the accent tint and ink, as in light.
- **One primary 2.** "Open its page ↵" is filled beside the top bar's filled New session.
- **Pickers/lists 3.** The chips don't add up. "All 5", but the chips cover only 3 (Asks you 1, Probably stuck 1, Limits 1). The two "Stopped with an error" sessions have no chip, so `narrowed-*.png` says "4 more under All". **Fix:** add a "Stopped 2" chip. Every item belongs to exactly one chip.
- **Words 3.** The chip says "Limits", but the row says "Limit reached". The first row's button says "Continue in another app…", but the third row's says "Continue elsewhere" for the same action. **Fix:** one word for each.
- **States 3.** The unmeasured account says it three ways: the "—" figure, "Not measured yet", and the summary sentence. **Fix:** one line.
- **Hand check 2.** 1440 is captured too.

### 99-inactive-apps: the rail's Inactive apps and Home's closed tiles

- **Once 3.** `rail-inactive-apps-dark.png`:
  - "Last 1 inactive" under each of three Apps.
  - "Not running" under rows in a section titled "Inactive apps".
  - Home tiles saying "Work Max · not measured yet" under a "Work Max" profile label.

  **Fix:** in Inactive apps, show only when each App was last seen. Leave the profile name out of the tile's second line.
- **H/L/V 3.** The same capture: the rail's App headers are cut to "Claude app · W…" and "Claude Code · W…". The cut keeps the repeated word and drops the part that tells two Apps apart. **Fix:** in a narrow rail, put the profile name first ("Work Max · Claude app") or drop the app word, since the icon already says it.
- **One primary 2.** "Open its page ↵" plus the top bar's New session.
- **Words 3.** "Last 1 inactive" reads as a glitch. **Fix:** "1 ended" and "3 ended".
- **States 3.** `home-closed-apps-dark.png`: "not measured yet" on three of four open tiles. **Fix:** say nothing when room is unknown.
- **Hand check 1.** No 1024 capture.

### 99-new-session: New session, the usual made and said

- **Default 3.** `out-of-room-dark.png`: Work is out of room until 4:20 PM, but the stated choice is still Work. The primary still reads "Start in web-shop ⌘↩" and would start a session that "would stop at its limit at once". The one with room (Personal, 82%) is only a link. **Fix:** when the usual App is out of room, make the default the App with the most room, say "Work is out until 4:20 PM" as the why, and keep Work one click away.
- **Once 3.**
  - `change-folder-dark.png`: the picker lists the five Recent projects, then again under All projects.
  - `more-options-dark.png`: the fold says "runs as its launch settings say", and inside it How it runs says "As its launch settings say".

  **Fix:** leave Recent and Pinned out of All. Show How it runs only when it differs from the launch settings.
- **Pickers/lists 3.** `change-folder-dark.png`: the folder picker opens at the window's top right, far from "Change folder", and covers the sheet's primary button. Change app's picker is anchored under its link. **Fix:** anchor the folder picker under its link, as Change app's is.
- **Words 3.**
  - The fold line reads "Main · opus", a launch-settings name and a lowercase model id.
  - "Start from a saved handover…" and "Expert setup…" are internal words.
  - The Account line uses the terminal ">_" icon.

  **Fix:** "Opus" with a capital. Name the launch setting only when there's more than one. Use the Account's colour dot on its line.
- **Hand check 2.**

### 99-palette: ⌘K, search, act or go

- **Fit 3.** No Fit paragraph.
- **Prominence 3.**
  - `typed-1280x800-dark.png`: typing "pay" puts "Go to Privacy" (a loose letter match) above "payments-api", the exact match.
  - `preview-1024x700-light.png`: typing "wrk" puts Go to Projects and Go to Tasks above the sessions.

  **Fix:** rank by match quality across groups, with prefix matches first. Show an action group above objects only when it matches at least as well.
- **Once 3.** `empty-1280x800-dark.png`: feat/read-model is listed in Recent and again in Open now. "open" ends nearly every row. **Fix:** leave anything shown under Recent out of Open now. Show the row-end hint only on the active row.
- **Words 3.**
  - On one row, "open now" (a state) sits beside "open" (what ↩ does).
  - The preview's button says "Continue on another account…", while Home says "Continue in another app…".
  - The preview's "Account: Work Max" gives an App name.
  - The feat/read-model row uses AI Juggler's own mark where it should use Claude Code's icon.

  **Fix:** "↩ Open" for the hint, "Continue in another app…" everywhere, the Account's real name, and the app's icon.
- **Hand check 2.** 1440 is captured too.

### 99-rail-splitter: the rail's edge from the keyboard

- **Fit 3.** No Fit paragraph.
- **Prominence 3.** `focused-1280x800-light.png`: nothing on screen says the edge can be moved, so only screen-reader users learn it. **Fix:** a grip on hover and focus, and a tooltip "Drag or use ← →".
- **Areas 2.** `narrowest-1280x800-dark.png`:
  - At the narrowest width (208 px), "Open now 2" runs into "Your order", and the words overlap.
  - App headers are cut to "W…".
  - The rail's button reads "New se…".

  **Fix:** at this width, hide "Your order" behind its icon and show only the profile name in App headers. Or raise the narrowest width to the point where the words fit.
- **H/L/V 3.** The same capture: names are cut to one letter while state words stay whole. **Fix:** as above.
- **Hand check 1.** 1280 only, and the narrowest and widest widths are dark only.

### 99-related-list: the one related list and session row

- **Once 2.**
  - `app-sessions-long-dark.png`: "11 sessions · 4 open" is in the facts line and again just above the list. "Sessions 11" is in the tab too.
  - The section heading "Working" sits over rows that each say "Working".
  - `home-recent-dark.png`: every Pick up row says both "Mark reviewed" and "Ended".

  **Fix:** drop the line above the list (keep it only when filtered). Leave out the state word where the section heading says it. Leave out "Ended" in Pick up where you left off, since everything there has ended.
- **One primary 1.** `app-empty-dark.png` has three filled "New session" buttons: top bar, header and empty card. `home-recent-dark.png` has three filled buttons: New session, Open its page and Allow. **Fix:** the cross-screen fix, and in the empty card make New session a link ("Start one").
- **Hand check 2.**

### 99-report-bug: Report a bug

- **Fit 3.** No Fit paragraph.
- **Folding 3.** `sheet-1280x800-dark.png`: the four parts that go with the report each take a full bordered card with its own "Show" fold. They fill two thirds of the sheet, and at 1024 (`sheet-1024x700-light.png`) the last one is below the sheet's fold. **Fix:** one line, "Goes with it: what AI Juggler showed, versions, the last switch, recent errors · Change ▸". Open it to the four ticked rows.
- **Once 3.** "Show" is on four cards. **Fix:** make the row itself the fold, with no extra word.
- **Areas 3.** Cards inside a sheet: four bordered boxes inside the sheet's box. **Fix:** plain rows with hairlines.
- **Colour 3.** `sheet-1024x700-light.png`: the Title and What happened fields are filled mid-grey in light, so they read as disabled. **Fix:** the field surface the other sheets use (white, with a border).
- **Hand check 2.**

### 99-resume: an ended session's Resume

- **Fit 3.** No Fit paragraph.
- **Once 2.** `ended-page-1280x800-dark.png`: "Ended" is said three times (the chip, the terminal's bar, "Ended 02:03"). "Claude Code" is said three times (the App chip, the facts line, the App row). **Fix:** move the end time into the chip ("Ended 02:03") and drop the other two.
- **One primary 2.** "Resume" is filled next to the top bar's New session. It also isn't last, because Relaunch follows it. **Fix:** the cross-screen fix, and order the buttons Continue in another app…, Relaunch, Resume.
- **Words 2.** The same capture:
  - The breadcrumb says "payments-api › Session", the title "Untitled session · ai-juggler", Project "payments-api" and Folder "ai-juggler". You can't tell which project this ran in.
  - "Resume" and "Relaunch" differ only in tooltips.
  - "Model: sonnet" is a lowercase model id.
  - The lone "C" tool mark is back in the facts line, after 0.4 removed it from cards.

  **Fix:** one project name from one source. Label the buttons "Resume this conversation" and "Start it fresh", or give each a one-line hint under it. Write "Sonnet", and use the app icon.
- **States 3.** The Runway card says "Not measured yet" and a two-line explanation. **Fix:** leave out the card when room is unknown, and keep "Measure its limits" in the ⋯ menu.
- **Hand check 2.**

### 99-settings-keyboard: Settings › Keyboard

- **Fit 3.** No Fit paragraph.
- **Once 3.** `changed-1280x800-dark.png`: "Change…" is on 12 of 14 rows. **Fix:** make the keycap itself the button (click it to record), and keep "Reset" for changed rows.
- **Words 3.**
  - "Light, dark or match the Mac" should say "this computer" or "the system".
  - `go-to-1280x800-light.png` shows "Enter" as a keycap, where other places use "↩".
  - This pane writes "^D", while General writes "Ctrl+D".
  - The go-to row's "None" sits in the button column, not the key column.

  **Fix:** one notation for keys, everywhere. Put "None" in the key column.
- **Hand check 2.**

### 99-settings-search: Settings' search

- **Fit 3.** No Fit paragraph.
- **Colour 3.** `opened-1024x700-light.png`: the result's target gets a square accent rectangle the full width of the pane. It sits on the section title above the rounded card, not around anything, and it clashes with the rounded cards. **Fix:** a brief accent tint on the matched row, and the standard focus ring on the title.
- **Doors 3.** The same capture: searching "mute" opens Notifications at "Alerts and the digest", but the "Muted" setting isn't on screen. The result is a door to the section, not to the setting. **Fix:** scroll to the matched row and highlight it.
- **Hand check 2.**

### 99-usage-cost: cost you can act on

- **Fit 3.** No Fit paragraph.
- **Once 3.** `cuts-1280x800-dark.png`: the Price column says "list price" on every row. **Fix:** hide the column unless a row differs, and say "at list prices" once in the subtitle (it's already there).
- **Areas 3.**
  - The same capture: Monthly caps is a half-width card with an empty half beside it.
  - `wasted-1280x800-light.png`: the sheet's column headers sit outside a bordered list (a box inside the sheet's box), and each row's state is on a third line.

  **Fix:** a full-width Monthly caps line. Plain rows in the sheet, with the state in its own column.
- **Words 2.** "Cost by account" lists "Work Max" and "Personal", which are App and folder names. The Account here is "Claude Max 20x". The wasted sheet's "Ran in" column also shows "Work Max". The tile's "51 other apps' not counted" is a cut-off possessive. The tokens icon is reused for "Crashed or probably stuck". **Fix:**
  - Group by the Account's real name, with its Apps under it.
  - "51 sessions from other apps not counted".
  - A warning icon for the crashed tile.
- **Hand check 2.**

### 99-when-to-tell: Notifications › When to tell me

- **Fit 3.** No Fit paragraph.
- **Folding 3.** `section-1280x800-dark.png`: one card always shows all of this: nine notice kinds, two paragraphs, When to tell me, Send test, Quiet hours, and In the digest. **Fix:** fold Quiet hours and In the digest behind "More: quiet hours 22:00–07:00, all Apps ▸", with the values on the fold.
- **Once 3.** "Warn when an account has used [80 % used]": "used" twice. "This Mac is getting tight… as This Mac shows it." **Fix:** the pickers say "80 %". "This computer is short on room".
- **Areas 3.** Five sub-sections share one card, with no space or surface between them. **Fix:** a card per sub-section, as other Settings panes do.
- **H/L/V 3.** The sub-headings "When to tell me" and "Quiet hours" are body-sized semibold, the same as the checkbox names. **Fix:** use the section-title style.
- **Words 3.** "Mac" and "macOS" in the app's own words, and "parked session" and "digest" are unexplained. **Fix:** "this computer", "the system", and "a session waiting to resume".
- **Hand check 2.**

### 91-pools: the rail's Pool grouping, Move to pool, Agent access

- **Fit 3.** No Fit paragraph. These captures are also stale: they predate 0.4's App words.
- **Once 3.** `settings-agent-access-1280x800-dark.png`: "· all sessions" ends three rows. **Fix:** group project pools under one "Project pools" heading and drop the suffix.
- **Areas 2.** The same capture nests boxes three deep: the card, a bordered pool row, the bordered radio list, and a bordered "Messages put into sessions" fold. **Fix:** pool rows as plain rows in one card. Open a pool's levels in place, with no inner borders.
- **One primary 2.** `rail-by-pool-1280x800-dark.png`: "Sign in" is filled next to the top bar's New session.
- **Doors 3.** `settings-agent-access-1280x800-dark.png`: the rail groups a session under an App called "AI Juggler". That isn't an App the user has, and its page is a dead end. **Fix:** group such sessions under the App that runs them.
- **Words 2.** `rail-by-pool-1280x800-dark.png` shows:
  - "Claude" and "Open Claude", not "Claude app";
  - "Profiles: Work Max";
  - the raw plan id "max-20x";
  - an Account titled "Work Max", which is an App name.

  "hop limit" is an internal word. **Fix:** retake the captures on the 0.4 build and check that the words match the App and Account pages. Write "the most hand-offs in a row".
- **Hand check 1.** 1280 only, and stale.

### 97-pool-canvas: the Pools screen and a pool's canvas

- **Prominence 3.**
  - `canvas-full-1024x700-dark.png`: the "ADD A SESSION" palette covers Planner's card, the zoom bar covers Tax rules research, and cards are cut at the canvas's left edge.
  - `canvas-badge-open-1024x700-dark.png`: the ask popover covers Tax rules research's name.
  - `inspect-member-1280x800-light.png`: the ⋯ button covers "Checkout copy pa…".

  **Fix:** fit to view after the palette and zoom bar take their space. Anchor popovers outside the nodes. Put ⋯ in its own slot.
- **Once 3.**
  - "Agents: Work together" in the header and "Agents may: Work together" in the inspector.
  - `list-1280x800-dark.png`: "AI Juggler" on every member row.
  - `new-pool-1280x800-dark.png`: the new pool says it's empty five ways ("0 sessions", "Members 0", "Always allowed: Nothing yet", "No one in this pool yet", "Nothing yet…").

  **Fix:** keep the level in the header only. Drop "AI Juggler" from rows. One empty-state card, and an inspector that holds only the level and Change.
- **Areas 3.** `inspect-member-1280x800-light.png`: the ask is a bordered, tinted box inside the inspector's section, inside the page card. **Fix:** the ask as a plain section with the tint and no border.
- **H/L/V 3.** Letter-spaced capitals (ACTIVITY, THIS POOL, LIVE, MEMBER, WAITING ON YOU, ADD A SESSION) are a heading style used nowhere else in the app. **Fix:** the app's section-title style.
- **One primary 2.**
  - `pools-screen-1280x800-light.png`: "New pool" is filled next to the top bar's New session.
  - `inspect-member-1280x800-light.png`: "Allow once" is filled, also next to the top bar's.
- **Doors 3.**
  - The rail groups all members under an App named "AI Juggler".
  - `canvas-1024x700-light.png`: the folded rail shows a lone "C" for every App.
  - `inspect-member-1280x800-light.png`: a Codex member also gets the "C" mark.

  **Fix:** each member's real App and that App's icon, linking to its page.
- **Words 3.** "Paused at the hop limit", "Delivery: When you click Deliver, 5 in a row", "project pool" and "lead" are unexplained. **Fix:** "Paused after 5 hand-offs in a row", "Messages wait for you", and a ? beside Lead.
- **States 3.** New pool: see Once.
- **Hand check 2.** inspect-edge, inspect-pool, list, messages-panel and new-pool are captured at 1280 only.

---

## Every no

Ranked most harmful first: wrong facts and wrong defaults, then lost or hidden controls, then noise. One line each. The three problems that cross every screen (filled buttons, Fit paragraphs, hand check) are one line each, with the families named.

1. 99-resume: **Words**. The page says the session's project is both payments-api and ai-juggler (breadcrumb, title, Project and Folder disagree). Fix: one project name from one source.
2. 99-new-session: **Default**. When the usual App is out of room, ⌘↩ still starts there, and the session stops at once. Fix: default to the App with the most room, and say why.
3. 99-carry-choice: **Words**. "✓ Everything goes along" stays green while 800 turns are left out. Fix: the chip turns amber and says what's left out.
4. 99-usage-cost: **Words**. "Cost by account" lists App names, so money is shown against the wrong thing. Fix: group by the Account's real name, with its Apps under it.
5. All pages with their own action (account-page, account-quality, app-page, app-profiles, home-chips, inactive-apps, related-list, resume, 91-pools, 97-pool-canvas): **One primary**. The top bar's filled New session makes two, sometimes three, filled buttons. Fix: make the top bar's New session a tinted secondary button, and make the empty-state New session a link.
6. 99-home-chips: **Pickers/lists**. The chips cover 3 of 5 items, and errors have no chip. Fix: add "Stopped", so every item has one chip.
7. 99-rail-splitter: **Areas**. At the narrowest rail, "Open now" and "Your order" overlap, and names cut to one letter. Fix: hide "Your order" behind its icon, or raise the narrowest width.
8. 99-inactive-apps: **H/L/V**. Rail headers cut to "Claude app · W…", so two Apps look the same. Fix: put the profile name first in a narrow rail.
9. 99-first-run-confidence: **Prominence**. At 1024 the only way forward is below the fold. Fix: a fixed footer with the primary at the right, and the rare links folded.
10. 97-pool-canvas: **Prominence**. At 1024 the palette, zoom bar and popovers cover node names. Fix: fit after the chrome takes its space, and anchor popovers outside nodes.
11. 99-account-quality: **Words**. A Claude plan says "Not reported by Codex". Fix: name the App, and fix the fixture's Account.
12. 91-pools: **Words**. The captures use pre-0.4 words ("Claude", "Profiles", "max-20x"). Fix: retake them on the 0.4 build and check the words.
13. 99-floor-list: **Doors**. Session and App names don't open anything. Fix: the names open their pages, and "Show on the Floor" stays as the row action.
14. 97-pool-canvas: **Doors**. Members are grouped under an App called "AI Juggler", with a "C" mark even for Codex. Fix: the real App and its icon, as links.
15. 91-pools: **Doors**. The rail shows an "AI Juggler" App group that leads nowhere. Fix: group by the App that runs the session.
16. 99-palette: **Prominence**. "Go to Privacy" ranks above the exact match "payments-api". Fix: rank by match quality across groups.
17. 99-account-page: **Fit**. Renewal is said to be per Account, but App rows still show their own renewal dates. Fix: say renewal only on the Account card.
18. 99-account-page: **Default**. Two renewal dates are offered with neither chosen. Fix: pre-select the newer one, and make ⌘↩ accept it.
19. 99-new-session: **Pickers/lists**. The folder picker opens at the window's corner and covers the primary. Fix: anchor it under "Change folder".
20. 99-app-page: **Doors**. An App page never names or links its Account. Fix: "on Claude Max 20x" as a link in the facts line.
21. 99-floor-list: **Prominence**. Rows aren't waiting-first, and "Show on the Floor" is as loud as the data. Fix: sort waiting-first, and show the action on hover and focus.
22. 99-floor-list: **Pickers/lists**. No search, filter or paging. Fix: search, a "Waiting on you" chip, and the paged list.
23. 99-floor-menu: **Prominence**. The menu arrow reads as a stray dot. Fix: a real chevron, with a hairline split.
24. Cross-screen, floor-menu: **Areas**. The top bar cuts "This Mac · Pl…" at 1280. Fix: the dot plus "Plenty of room", with the full words in the tooltip.
25. 99-settings-search: **Doors**. A result opens its section, not the setting. Fix: scroll to the matched row and highlight it.
26. 99-resume: **One primary**. Resume isn't last, and Resume and Relaunch differ only in tooltips. Fix: Relaunch before Resume, and plain labels.
27. 91-pools: **Areas**. Agent access nests boxes three deep. Fix: plain rows in one card.
28. 99-report-bug: **Folding**. Four always-open part cards push the last below the fold. Fix: one "Goes with it… · Change ▸" line.
29. 99-when-to-tell: **Folding**. Everything is open in one long card. Fix: fold quiet hours and the digest, with their values on the fold.
30. 99-when-to-tell: **Areas**. Five sub-sections share one card with no gaps. Fix: a card each.
31. 99-when-to-tell: **H/L/V**. Sub-headings look like body text. Fix: the section-title style.
32. 99-home-chips: **Colour**. The selected chip in dark is a faint grey. Fix: the accent tint, as in light.
33. 99-report-bug: **Colour**. Light fields look disabled. Fix: the standard field surface.
34. 99-settings-search: **Colour**. A full-width square highlight clashes with the rounded cards. Fix: a tint on the matched row.
35. 97-pool-canvas: **H/L/V**. Letter-spaced capitals headers are used nowhere else. Fix: the app's section-title style.
36. 97-pool-canvas: **Areas**. The ask's bordered box sits inside a panel inside a card. Fix: a tinted section, with no border.
37. 99-usage-cost: **Areas**. The half-width caps card, and the bordered list in the sheet. Fix: a full-width line, and plain rows.
38. 99-report-bug: **Areas**. Cards inside the sheet. Fix: plain rows.
39. 99-home-chips: **Areas**. One short card over an empty column. Fix: let the next area move up.
40. 99-rail-splitter: **H/L/V**. Names cut to one letter while state words stay whole. Fix: as line 7.
41. 99-rail-splitter: **Prominence**. Nothing says the edge moves. Fix: a grip and a tooltip on hover and focus.
42. 99-home-chips: **Once**. "Need you" four times on one screen, and a runway sentence that repeats the cards. Fix: drop the area title and the sentence.
43. 99-floor-list: **Once**. Column headers in every card, the room twice, and the row action seven times. Fix: one sticky header per floor.
44. 99-account-quality: **Once**. Labels repeated in their values, and the source line twice. Fix: values only, and the source once.
45. 99-related-list: **Once**. The counts three times, and state words under same-named headings. Fix: drop the repeats.
46. 99-resume: **Once**. "Ended" three times, and "Claude Code" three times. Fix: "Ended 02:03" in the chip only.
47. 99-app-page: **Once**. "Claude app" twice, and the session counts four times. Fix: start the facts line at the state.
48. 99-app-profiles: **Once**. "Not running" under Inactive apps, and "Start Claude app" four times. Fix: last seen only, and "Start".
49. 99-inactive-apps: **Once**. "Last 1 inactive" three times, and the profile name repeated on tiles. Fix: trim them.
50. 99-account-page: **Once**. The unknown said three times per card. Fix: one line.
51. 99-new-session: **Once**. Recent projects listed again under All, and "launch settings" twice. Fix: leave out the repeats.
52. 99-palette: **Once**. The same session in Recent and in Open now, and "open" on every row. Fix: dedupe, and show the hint only on the active row.
53. 99-first-run-confidence: **Once**. "You can rename it" three times. Fix: once, in the footer.
54. 99-floor-settings: **Once**. The Re-create sheet promises "it keeps its place" three times. Fix: cut the intro's second sentence.
55. 99-debug-mode: **Once**. A "Debug mode" title over a "Debug mode" row. Fix: rename the row.
56. 99-report-bug: **Once**. "Show" four times. Fix: the row is the fold.
57. 99-settings-keyboard: **Once**. "Change…" on 12 rows. Fix: the keycap is the button.
58. 99-usage-cost: **Once**. "list price" on every row. Fix: hide the column unless a row differs.
59. 99-when-to-tell: **Once**. "used" twice, and "This Mac" twice. Fix: "80 %", and "this computer".
60. 91-pools: **Once**. "· all sessions" three times. Fix: a "Project pools" heading.
61. 97-pool-canvas: **Once**. The agents' level twice, "AI Juggler" per row, and "empty" five ways. Fix: keep the header only, one empty card.
62. 99-app-profiles: **Pickers/lists**. Desktop apps isn't running-first. Fix: running, then closed by last seen, then found.
63. 99-account-page: **Folding**. The "no reading" paragraph and folder paths are always open. Fix: fold them.
64. 99-account-page: **Words**. A raw folder name as the example, and a Claude Code App "in Claude app". Fix: real App names, and "in a terminal".
65. 99-app-profiles: **Words**. Two names for one profile, and a lone "Forget" that breaks the column. Fix: one name rule, and Forget in ⋯.
66. 99-first-run-confidence: **Words**. "setups" and "apps" for one thing, and "May have expired · Likely". Fix: "apps", and "Likely expired".
67. 99-floor-list: **Words**. Initials as marks, a lowercase "work", and unexplained style names. Fix: the App's mark and name.
68. 99-floor-settings: **Words**. "Show unused Apps' decay", "nook" and "camera". Fix: "Let unused Apps gather dust".
69. 99-home-chips: **Words**. "Limits" vs "Limit reached", and two labels for Continue. Fix: one word each.
70. 99-inactive-apps: **Words**. "Last 1 inactive". Fix: "1 ended".
71. 99-new-session: **Words**. "opus", "Main", "handover", and a terminal icon on the Account line. Fix: "Opus", names only when it matters, and the Account's dot.
72. 99-palette: **Words**. "open" vs "open now", Continue "on another account", and an App given as the Account. Fix: one wording, and real names.
73. 99-settings-keyboard: **Words**. "match the Mac", "Enter" vs "↩", and "^D" vs "Ctrl+D". Fix: one key notation, and "this computer".
74. 99-when-to-tell: **Words**. "This Mac", "macOS", "parked" and "digest". Fix: plain words.
75. 99-debug-mode: **Words**. "Backend response times". Fix: "How fast AI Juggler answers".
76. 97-pool-canvas: **Words**. "hop limit", "Delivery", "project pool". Fix: "after 5 hand-offs", and "Messages wait for you".
77. 99-account-quality: **Doors**. The figures don't open the sessions they count. Fix: link to the filtered Sessions tab.
78. 99-app-page: **States**. A half-width card says "Everything here is open now." Fix: drop it.
79. 99-account-page: **States**. "—", "Not measured yet" and a paragraph per card. Fix: one line.
80. 99-account-quality: **States**. "Not known" said three ways. Fix: one line, and drop the rows.
81. 99-floor-list: **States**. Columns of "—". Fix: hide empty columns.
82. 99-home-chips: **States**. The unknown room said three times. Fix: one line.
83. 99-inactive-apps: **States**. "not measured yet" on every tile. Fix: silent.
84. 99-resume: **States**. A Runway card that only says it doesn't know. Fix: leave it out.
85. 97-pool-canvas: **States**. A new pool is empty five ways. Fix: one card.
86. 18 families (account-page, account-quality, carry-choice, debug-mode, first-run-confidence, floor-list, floor-menu, floor-settings, home-chips, palette, rail-splitter, report-bug, resume, settings-keyboard, settings-search, usage-cost, when-to-tell, 91-pools): **Fit**. The checklist critiques don't state dependencies, fit with other screens, or the change for 0.3 users. Fix: add a Fit paragraph to each, as the App and New session critiques have. (Account-page's own Fit problem is line 17.)
87. All 24 families: **Hand check**. None was walked by hand in the native window. 99-inactive-apps, 99-rail-splitter and 91-pools also lack a size or are stale. Fix: walk each at 1280 and 1024, in both themes, with real-sized data. Describe what was seen, then re-score this line.

## What was done with each no (2026-11-05)

The numbers are the "Every no" list's. Each is fixed, explained, or still open on this branch; nothing is waved through.

**Fixed on release/0.4**

- **1** Resume's two project names: the preview's own data filed two sessions under payments-api while their folder was ai-juggler. The data is fixed; the page names one project.
- **3** The carry chip now counts the cut: "5 go along · the first 800 turns stay behind", amber (`carryWords`, unit test).
- **4** Cost by account: the backend already sums each Account's Apps under the Account's name; the preview didn't, so the captures showed App names. The preview now groups as the backend does.
- **5** One filled button per view: the top bar's New session is tinted, as the rail's is (a shared `accent` Button variant).
- **7, 40** The narrowest rail: Open now's order menu keeps only its ▾ (named, with a tooltip) below 15 rem, so nothing overlaps.
- **13** (part) The Floor list's App names open the App's page.
- **21** (part) The Floor list puts each App's waiting sessions first (`waitingFirst`, unit test).
- **23** The Floor menu's arrow is the app's chevron, with a hairline split.
- **55** Debug mode's row reads "Keep a detailed log" under the "Debug mode" heading.
- **68** "Unused Apps gather dust"; "corner" for "nook".
- **75** "How fast AI Juggler answers".
- **9** First run's actions (Bring in, Add a fresh app, Look again) stay at the foot of the window, so the way on is never below the fold at 1024.
- **73** (part) "Light, dark or match the system", not "the Mac".
- **76** (part) The approval card says "A chain of replies paused for you", and the pool's inspector "these replies went back and forth as many times as the pool allows"; the pool's own control already reads "Pause and ask after N replies in a row".

**Explained, not changed**

- **2** Out of room never moves the usual App (D-31, settled): the sheet says so before ⌘↩ and offers the App with room in one click.
- **20** A desktop App's Account isn't named because AI Juggler never reads a desktop app's sign-in (the credentials rule); where it is known (every CLI App), the facts line links it.
- **21** (rest) "Show on the Floor" stays visible: shown only on hover, it would be hidden from touch and from people who scan before they tab.
- **13** (rest) A session's name doesn't open its page from the Floor list yet: there's no app-wide "open this session" door outside the rail and Home. It's the row's "Show on the Floor" for now.
- **6** The chip set is AJ-E2910's (deep/11 §4.1): stopped sessions show under All, and a chosen chip says what it hides ("3 more under All").
- **66** "Setups" are the folders found on disk; once brought in they're Apps. Two things, two words.
- **59, 74** (part) "This Mac" is the system's own noun for this computer (`computer_noun`), so it reads "This PC" on Windows; "digest" is the notification centre's one word for the hourly summary, used the same everywhere.
- **86** (Fit) Each family's fit, dependencies and change for 0.3 users are in its task's Goal, Depends and UI/UX reasoning; the critiques are the visual record and don't repeat them.
- **87** (Hand check) Walks by hand in the native window are AJ-E3302–E3306's, waiting on the maintainer's choice of how they're done (a lab-only write bridge, or walked by hand).

**Fixed on release/0.4, second pass (2026-11-05)**

- **8, 15** Already fixed by earlier commits: rail headers put the profile first; AI Juggler's own sessions sit under their CLI App.
- **10** Fit leaves room for the palette and the zoom bar and refits as the canvas's room changes; a badge's panel opens beside its node; the compact ⋯ has its own slot.
- **11** The quality gallery's seed is a Codex Account for Codex's case.
- **14** Nodes show their App's own icon; the App's name opens its page.
- **16, 52, 72** The palette ranks groups by their best match, lists a session once, puts the hint on the active row only, and uses real Account and App names and icons.
- **17, 18, 50, 63, 64, 79** Accounts: renewal only on the Account, the newer date chosen with ⌘↩, "Not measured yet" in one line with its why folded, no folder path, "open in …".
- **19** A tall menu scrolls inside, beside its button.
- **22, 43, 67, 81** The Floor list: search, a Waiting chip, paged floors, one sticky header per floor, the room once, Show on the Floor once per App, no empty columns, Apps' own icons.
- **24** The top bar says the state by its dot ("Plenty of room"); This Mac stays the system's noun, in its title and spoken name.
- **25, 34** A Settings search result lands on its row, opens its fold and tints it; the title's ring hugs it.
- **26** Relaunch before Resume, each with a one-line hint (labels kept, as AJ-E2908 fixes them).
- **27, 60** Agent access in plain rows, the projects' pools under one heading, named once.
- **28, 33, 38, 56** Report a bug: one folding "Goes with it" line, plain rows, the standard field surface.
- **29, 30, 31, 74** Notifications: a card per part, section-title headings, quiet hours and the digest folded with their values, "the system".
- **32** A chosen chip is accent-tinted in both themes.
- **35, 36** The pool screens' headings in the app's styles; an ask in the inspector is a tinted section.
- **37, 58** Usage: caps on a full row; the stuck sheet in plain rows; the Price column only when a price differs.
- **39, 42, 49, 69, 82, 83** Home: Pick up moves up beside few needs; no repeated title or sentence; "Limit reached", "Continue in another app…"; unknown room said once or not at all; the Account not repeated as the profile.
- **41** The rail's edge shows a grip and says it can be dragged.
- **44, 80** Quality: values only, the source once, rows a tool doesn't record left out and said.
- **45, 46, 84** Session pages: the count line only when filtered, no repeated state word, "Ended 2:03 PM" once, the tool once, the model by name, no empty Runway card.
- **47, 78** App page: facts from its state; Last used only once something ended.
- **48, 62, 65** "Start" on a tile that names its App; no "Not running" under Inactive apps; Desktop apps running first, then by last seen, then found; Forget in a row's ⋯.
- **51, 71** New session: no option twice, "launch settings" once, "Opus", "Advanced…", the Account's dot.
- **53, 54, 57, 73** "You can rename it" once; Re-create's one sentence; the keycap is the button; one key notation.
- **61, 76, 85** A pool says its agents' level once, cards drop "AI Juggler" from the place line, Activity says "Messages: Wait for you" and "Pause after: 5 in a row", a paused edge says the pool's number, and an empty pool says it's empty once.
- **70** "Last inactive" for one; D-64's "Last 3 inactive" stays.

**Explained, second pass**

- **12** The product says the 0.4 words; the 91-pools gallery captures are retaken by the release preflight.
- **24** (rest), **73** (rest), **74** (rest) "This Mac" stays: it's the system's own noun (`computer_noun`), "This PC" on Windows.
- **26** (labels) Resume and Relaunch keep AJ-E2908's labels; the hints say the difference.
- **65** (name) A profile's name follows AJ-E3203's rule; only Forget moved.
- **71** (part) "Handover" stays the glossary's word (the guide's heading uses it).
- **77** The figures can't open a filtered Sessions tab: that tab doesn't exist yet. To 0.5 with the related lists.

**Still open on this branch (before 0.4 ships)**

None. The walks' own lows are on their sheets (P3, P4, P6, P7, P8, P9, P10, P11).
