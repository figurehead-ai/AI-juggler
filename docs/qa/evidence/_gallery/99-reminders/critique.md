<!-- captures: 200069cfaa93 -->
# 99-reminders: the Reminders list beside the watches, and Settings › Advanced › Scheduled work

Design critique (D-57, AJ-E2814), AJ-E2027 (ADR-0039 §6), Opus, 2026-11-26. Captured on the host from a clean export of HEAD (`watches-gallery.spec.ts`: one reminder set about "api", one made by Bot and paused, one told; the app's own two schedules), both themes, the list at 1280 and 1024, Scheduled work at 1280; every capture opened. Mock-ups: the Watches page's approved mock (its rows, state column and actions), `d126-rich-ui/kit-*.png` for the settings rows.

**The list.** Reminders sits under Watching and Ended with the same head (title, a quiet count "1 set · 1 paused", Pause all at the right) and the same card of rows. Each row leads with a clock in an accent-soft tile (muted when paused or done), the reminder's words as the title, "About api" and its day-and-time chip ("4:00 PM", "9:00 AM tomorrow") in the line under, the state column ("In 1 h · Once, then done", "Paused · Resume to be told", "Told 2:40 PM · Done"), then Change time, Pause or Resume, and Remove. The columns line up across rows whatever their actions (a first capture had a paused row's state column 55 px left of the others and "Change time" wrapping; fixed before these, 1c733f8e3 and the next commit). At 1024 the rows keep one line each.

**Scheduled work.** A settings section in Advanced, after Local cache and Diagnostics: two read-only rows in the kit's row form (an icon chip, the title, one line of when: "Every hour · next 3:23 PM · last ran 2:23 PM", "When this computer is quiet, about hourly · last ran 2:10 PM"), and a footer that says nothing runs while AI Juggler is closed, with Open reminders. A watch's own timer isn't listed (the Watches page shows the watch). The heading wears the landing ring because the capture opens Settings on this section by its anchor.

**Weak.** The state column says "In 1 h"; under a minute it says "Due now" (never "In just now"). Bot's reminder card isn't in this family (it's a Bot turn); its words and button are in vitest.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Hierarchy | 4 | Title, then about and when, then the state; the section head matches Watching's. |
| 2 | Alignment | 4 | One grid for every row; the actions column fixed so the state column lines up. |
| 3 | Spacing | 4 | The watch rows' 12 px padding and dividers. |
| 4 | Mode | 4 | A list you act on (calm card, rich lead chips); the settings section in the kit's rows. |
| 5 | Mock-up | 4 | The Watches mock's row anatomy; Scheduled work the kit's row. |
| 6 | Areas | 4 | Tone, no outlines; no box in a box. |
| 7 | Colour | 4 | The accent marks what is set (the clock tile, the time chip); paused and done go neutral; told keeps its green dot. |
| 8 | Type | 4 | 14 px titles, 13 px lines; tabular times. |
| 9 | Icons | 4 | The clock for a reminder and for a timed job, the moon for idle work. |
| 10 | Density | 4 | Three rows read at a glance; nothing truncated at 1024. |
| 11 | Motion | 4 | None beyond the shared menus. |
| 12 | Words | 4 | "In 1 h", "Resume to be told", "Once, then done", "When this computer is quiet". |
| 13 | States | 4 | Set, paused, told; the section hidden with no reminders (D-127). |
| 14 | Consistency | 4 | WatchChip, StateCell, RowMenu, SettingRow: no new pieces. |
| 15 | Accessibility | 4 | Every action named with its reminder ("Pause: Look at “api”"); axe clean on each capture. |
| 16 | Hand walk | 3 | Not walked in Dev yet (the coordinator's Sonnet walk follows). |
| 17 | System prompts | 4 | Nothing here asks; making a reminder primes Notifications at its click. |
| 18 | Usability | 4 | The six answers in AJ-E2027 match the screens. |
| 19 | Rich-mode colour | 4 | Every row leads with its icon; set is the accent's soft step (tile and chip); chosen and on are fills, never a ring; status keeps its hue. |

Lowest: 3 (line 16).

## 0.5.2 G23: Remind me's toast opens the reminder (Opus, 2026-11-26)

Four new captures, both themes, 1280, from a clean export of HEAD (`watches-gallery.spec.ts` "reminder opened from Remind me": the story Home, its first post's Remind me › In 1 hour, the toast, then its action); the list, Scheduled work and their 1024 captures re-ran byte for byte. Every capture opened.

**What changed.** `remind-toast-1280-*`: the toast after Remind me reads "Reminder set for 12:00 PM. About Login refactor." with **Open reminder** beside the words (it read "Reminders" and opened the Watches page at its top, the reminders under the fold: RW1 B7 saw no way to it). `list-opened-1280-*`: Open reminder lands on the Watches page with that reminder ("Look at “Login refactor”", In 1 h) brought into view, marked with the selected row's soft fill, the keyboard on it; the other three reminders as before. Bot's set card says Open reminder too (vitest: it opens the one just set).

**Weak.** The toast's two lines wrap "Reminder set for / 12:00 PM." at the toast's width (as before this change). The marked row keeps its fill until the page is left or another door opens it; it doesn't fade.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Hierarchy | 4 | The toast: result, then about, then its one action; the list unchanged. |
| 2 | Alignment | 4 | The marked row on the list's one grid; the fill inside the card's radius. |
| 3 | Spacing | 4 | Unchanged. |
| 4 | Mode | 4 | A list you act on; the arrival is the selected fill, not a ring (PAT-SURF-3, -5). |
| 5 | Mock-up | 4 | The rows' anatomy as the Watches mock; the toast the shared one. |
| 6 | Areas | 4 | Tone only. |
| 7 | Colour | 4 | Selected is the neutral soft fill; the accent stays on the clock tile and time chip. |
| 8 | Type | 4 | Unchanged. |
| 9 | Icons | 4 | Unchanged. |
| 10 | Density | 4 | Four rows at a glance. |
| 11 | Motion | 4 | The row scrolls into view, centred; reduced motion respected by the browser's own scroll. |
| 12 | Words | 4 | "Open reminder" says where it goes and what it opens. |
| 13 | States | 4 | Set, opened, paused, told. |
| 14 | Consistency | 4 | The toast's action pattern; the rows' own selected fill (as a row with its menu open). |
| 15 | Accessibility | 4 | The opened row takes focus (its words read); axe clean on both. |
| 16 | Hand walk | 3 | Driven by the spec in a built app; the re-walk walks it in Dev. |
| 17 | System prompts | 4 | Remind me primes Notifications at its click; after Not now it no longer asks again (G33). |
| 18 | Usability | 4 | Where did my reminder go: one click from the toast to it, by its id. |
| 19 | Rich-mode colour | 4 | Every row still leads with its clock tile; set is the accent's soft step; the arrival is the selected fill, never a ring; status keeps its hue. |

Lowest: 3 (line 16).
