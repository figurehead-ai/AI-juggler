# 0.5.1 F98: Quit… for a desktop App (usability reasoning, PAT-USE)

F-PL1-15 (a): a running desktop App (a Claude app profile, the Codex app) had no quit-only action: Restart… quits it and starts it at once, Forget asks the user to quit it first. Written by Opus before building (PAT-USE-1); each answer is re-checked against the built screens below it.

**UI**: yes

## Usability reasoning

1. **Who, and what they expect from the name and icon.** Someone running several Claude app profiles or the Codex app who wants one closed: to free the Mac, to end one that misbehaves, or because Keep alive keeps bringing it back. "Quit…" (the menus' stop square, beside Restart…'s arrow) reads as the app's own ⌘Q: it closes and stays closed. The "…" says a confirm comes first, as Restart…'s.
2. **What a newcomer sees.** "Quit…" right under "Restart…" wherever an App's actions are (its rail header's ⋯, an inactive App's row menu, Accounts › Apps' ⋯, Settings › Desktop apps' ⋯) and a "Quit…" button beside "Restart…" on its page. The confirm, "Quit Claude app · Work?", says what closes in counts ("1 session is working in it. Its turn stops now; its conversation is kept and can be resumed."), that AI Juggler asks it to quit and waits for it to close, and that it stays closed until they start it again, Keep alive too when it is on. No other term, size, download or choice.
3. **Its value, in one place.** The only way to close a desktop App from AI Juggler without starting it again. It is not a new surface: it sits where Restart… already is (one menu per object, D-71), and goes through the same quit as Restart and Move (the process checked by its identity right before the signal, `ownership::signal::quit_checked`; `juggler_platform::apps::quit`), so a quit means one thing everywhere.
4. **When it is unavailable or empty.** Shown only while the App runs, as Restart… is (D-127: a closed App has nothing to quit; a CLI App has no process of its own, its sessions stop with Stop). While a move holds the App, or a restart or quit of it is under way, the quit isn't sent and its toast says why in words.
5. **System prompts it can cause (PAT-OSP).** None: AI Juggler asks the App's own process to end (a signal to a process the user runs), with no Apple Events, Accessibility or Automation, so macOS shows nothing. The App itself may ask its own "quit while working?" question; then it isn't closed within 10 s and the toast says so, and nothing else happens.
6. **What would surprise a sensible person.** (a) Keep alive starting it again a few seconds later: a quit from AI Juggler pauses Keep alive for that App until it is opened or started again, and its line says "Keep alive paused: you quit it in AI Juggler". (b) Losing work: the confirm says what is working before anything happens, and conversations are kept. (c) Quit quietly failing: the toast says "Quit Claude app · Work" when it closed, or why not ("didn't quit within 10 seconds…").

## Re-checked against the build

Opus, against f1abaaa41 and the 99-app-page / 99-keep-alive captures (1f51d6ec3):

1. Match: "Quit…" with the stop mark sits right under "Restart…" in the rail menu (`99-keep-alive/rail-menu-keep`), and as a secondary button before Restart… on the App's page (`99-app-page/app-overview`).
2. Match: the confirm reads "Quit Claude app · work?", the counts line, then "AI Juggler quits it and waits for it to close. It stays closed until you start it again, Keep alive too." (vitest `quit-app.test.tsx`); no other term.
3. Match: one quit for Restart, Quit and Move (`places::quit::ask_to_quit`; `places::restart` calls it); no new surface, every Restart… door has Quit… beside it.
4. Match: shown only while the App runs (vitest: Accounts › Apps' ⋯ and the rail header have none for a closed App; `99-app-page/app-closed` shows none); a held or restarting App answers why in the toast (`places::quit` refuses through `why_held` and the restart guard).
5. Match: a signal to the App's own process only, no Apple Events; one that doesn't close within 10 s is said in words (cargo `one_that_doesnt_quit_in_time_is_said_and_stays_paused`).
6. Match: Keep alive pauses until the next Open (cargo `keep_alive_never_starts_an_app_quit_here_until_its_next_open`, fails with the pause removed); a quit never signalled leaves it as it was (`a_quit_that_wasnt_sent_leaves_keep_alive_as_it_was`).
