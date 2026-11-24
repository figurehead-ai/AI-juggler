<!-- captures: a1b43ea2d8c3 -->
# 99-new-session: New session, the usual made and said (0.4, D-62, AJ-E3207)

Design critique (D-57, AJ-E2814). First critique of this family, written at 0.4 from its captures.

## 0.4

The principles audit (`docs/design/PRINCIPLES.md`, worked example): New session opened with "How it runs" and "Start from a saved handover…" on top, six folder cards with none chosen from the top bar, a "Ranked for you" list and a "Which account?" card that said the same pick twice, about 14 equal boxes before Customise. Now it opens on one stated choice, one fold and one button. Seven screens, both themes, opened from the top bar's New session on a Mac with ten projects and four Apps on three Accounts (Work, Claude Pro, Codex Plus; one App not signed in).

**usual** (N1). One tinted block, the only one: "Start in **web-shop** on **Claude Code · Work Max** — the last you used here". Under it two quiet lines, each with its link at the right: the folder ("~/Dev/web-shop · worked here 12 min ago", Change folder) and the App's Account ("Account **Work** · 58% left in the 5-hour window", Change app). Room is said once, for the Account. Then one line, "More options · Main · opus · runs as its launch settings say". The footer, on its own surface: Cancel and the one primary, "Start in web-shop ⌘↩", which has focus, so Return or ⌘↩ starts it. Three areas, one accent button.

**usual-1024.** The same at 1024: the sheet keeps its width, nothing wraps.

**change-folder** (N2). Change folder opens the shared picker on its search field ("Search projects…", ten projects): "Choose another folder…" first, then Recent (five, newest first, with path and when), Pinned (infra), then All projects. Typing "pay" and Return picks payments-api, and the sentence becomes "Start in payments-api on Codex · ~/.codex — the last you used here": the App follows the folder.

**change-app** (N2). "Which app": the four Apps by room, each with its Account and room ("Claude Pro · 82% left…"), the one in use ticked, the one not signed in last with "Not signed in". Four is under the picker's grouping threshold, so it's one list; with six or more it opens on Suggested and "Used in web-shop".

**more-options** (N3). Open, under a hairline indent: Launch settings (Work Max has two: Main, Quick fixes), How it runs, Start from a saved handover…, Expert setup… The fold's label still says the values, so a closed fold hides nothing.

**first-time** (N4). No projects, no sessions: "Start a session on **Claude Code · Personal** — the most room left"; the folder line says "None yet: it starts in a temporary folder" (what the backend does without one), and the button reads "Start on Claude Code · Personal".

**out-of-room** (N5). Work is at 0%: the sentence still names Work Max with its why, the room words go, and one warning line below a hairline says "Work is out of room until 4:20 PM: a session there would stop at its limit at once. Use Claude Code · Personal (82% left)". The button still starts on Work Max: nothing switches by itself (D-31).

## Fixed while critiquing

- The search field sat inside the picker's menu (axe: aria-required-children, serious); the shared RowMenu now puts a field above its menu, outside it, so every Picker with search passes.
- Change folder and Change app showed the chosen value's icon beside the link (a folder beside "Change folder" under the folder's own icon); a Picker with its own words shows none.
- "Work" alone on the second line didn't say what it was; it reads "Account Work".
- The out-of-room line said "on this account"; it names the Account.
- At a narrow window the folder's when-words ran under Change folder (seen in the flows gallery at 390 px); each line is one line that cuts at its end.
- The Change app list said "Not measured yet" where the stated line says nothing for unknown room; both are silent now.
- "Start from a saved handover…" kept its old bottom margin inside the fold, a gap above Expert setup.

## Weak

- The folder picker is tall (60 lines drawn) and opens upward over the sheet's title at 800 px; paging All projects, and Recent and Pinned not repeated in All, would shorten it.
- Apps show the terminal glyph, not their tool's own icon.
- ~~The picker has no "Always use for web-shop" (the project's default) yet; it's set on the project page.~~ Built 2026-11-10 (0.4.1): Change app opens with "Always use for web-shop" (the App chosen now, named under it) and, once set, "No default for web-shop"; New session then says "— your default here", and out of room still warns and offers (D-31). The change-app captures show the new first line from the next shoot.
- A fresh room reading never moves the pick (it's held once said), but nothing says "Personal now has more room · Use it" either.
- Paired Macs and a started handover aren't in this gallery (open-pack and the remote sheet have their own).

## 0.4.1 re-score (2026-11-07, AJ-E3305)

The whole family was retaken on release/0.4.1 for the 14-line re-score (AJ-E3305, `docs/qa/0.4-walks/design.md`) and opened on contact sheets, both themes, 1280 and 1024. Shared with every screen since the last shoot: the header's New session is tinted (one filled button per view), the Floor button is a split control with its own menu, This Mac's line is its dot below 1120 px, and the rail's App headers put the profile first.

Unchanged in design: the stated choice with its why and ⌘↩, More options folded with its values, the out-of-room line with the App that has room.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B2)

Re-captured after F53 and F49 (start-another), every state, both themes, opened as dark|light pairs. usual: one tinted block "Start in web-shop on Claude Code CLI · Work Max — used here before; 2 accounts worked here", the folder line with Change folder, the App line with its Suggested tag and "Account Work · 58% left in the 5-hour window" with Change app, "Always start web-shop on Work", More options · Main · Opus, and the one primary "Start in web-shop ⌘↩". out-of-room and skipped now start on Claude Code CLI · Personal with why ("Work is out of room until 4:20 PM", "Work is almost out (5% left)"); own-account says "App changed to Codex CLI · ~/.codex (was Claude Code CLI · Work Max): the only account used here. Undo". start-another (F49): after a start, the toast "Started in web-shop on Claude Code CLI · Personal" with "Start another: payments-api", which takes the keyboard.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 5 | Start the usual in one key. |
| 2 | Prominence | 5 | |
| 3 | Default | 5 | The stated choice with its why; ⌘↩. |
| 4 | Mode | 4 | |
| 5 | Mock-up | 4 | PRINCIPLES' worked example. |
| 6 | Areas | 5 | One block, one fold, one footer. |
| 7 | Colour | 4 | |
| 8 | Type | 4 | |
| 9 | Actions | 5 | |
| 10 | Serious | 4 | Nothing switches by itself; a changed App says so with Undo. |
| 11 | Doors | 4 | |
| 12 | Words | 4 | "Always start web-shop on Work" names the Account where the App is set; a default Codex profile shows as "~/.codex". |
| 13 | States | 4 | First time, out of room, almost out, own account, start another. |
| 14 | Consistency | 4 | Change app's rows still show the terminal glyph, not each tool's mark; the toast's "Start another" is an outlined button (F73). |
| 15 | Accessibility | 4 | The toast takes the keyboard after a start (F49). |
| 16 | Hand walk | 4 | Preview. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | |

Lowest: 4; F73 logged for the toast's action.
