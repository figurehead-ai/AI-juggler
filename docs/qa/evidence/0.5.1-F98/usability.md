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

(Filled in with the commit.)
