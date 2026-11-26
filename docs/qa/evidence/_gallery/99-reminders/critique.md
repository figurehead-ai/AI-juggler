<!-- captures: 4fa3c920cd1c -->
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
