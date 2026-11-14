# Persona runs, 0.4.5 (AJ-E3307)

0.4.5 carries the naming the 0.4.4 runs asked for (runs-0.4.3.md, "For 0.4.5"): an untitled session is named by the first words of its first ask, as `claude --resume` and Codex's picker show it, and "<folder> · started …" only when there is none. D3's first screen was walked again by a fresh Sonnet tester, the same way as the 0.4.4 runs: only the main window, through `floor_drive main` (shot, click, type, scroll, by pid). Every run passed the firewall check: only `floor_drive main` commands, only its own pictures read, and no leak words.

## The lab, before the run

PD3's 52 bulk ended sessions had no conversation at all, so every row fell back to "<folder> · started …" whatever the naming did (run 2 on 0.4.4 scored the first screen 2 / 2 / 2 / 2 for that reason). `ended_conversations = true` (bcfbe68d3) now gives each of them a short conversation in its tool's own format. Its first ask is one of 25 varied ones. Two in three of each Claude Code App's sessions are untitled, so the walk sees names taken from first asks; Codex's thread store names its own.

Found while setting up, and fixed before the run:

- Home's "In the last 12 hours" card named no session, only each row's App ("Claude Code CLI · Platform · platform · api · Finished"), so the new naming never reached the card that heads Home. Its rows now lead with the session's name by the rule every list uses, the App after it, the name on a line of its own (55ed1073a, a201fe90a).

## The gate scores (R4: per step, 1–5: worked / value / clarity / feeling)

Local Dev build at a201fe90a (a clean export of the commit, its own target), lab PD3.

| Flow | Step | W | V | C | F | Reason (short) |
|---|---|---|---|---|---|---|
| D3 Find one of fifty (run 3) | 1 Home | 4 | 3 | 3 | 3 | every row readable with its repo; "Add retries with backoff…", "Write a migration that splits the address column" (untitled, named by its first ask); the top two "Pick up" rows read "infra · started 3:50 PM" and "web · started 3:50 PM" (the lab's live two had no conversation: setup); a triage view, nothing says "login" (run 2: 2 / 2 / 2 / 2) |
| | 2 rail search "login flow api" | 5 | 5 | 5 | 5 | one row at once (~200 ms), "Fix the login flow in the api"; matched across title and repo |
| | 3 its page | 5 | 4 | 4 | 4 | the conversation with the cookie line, folder, branch and account in one place; Resume, Relaunch and Continue in another app there |
| | solved | yes | | | | one search and one click, about 40 s (run 2: 3 steps, partly) |

Fixed at once: the rail row's time read "api · 2 d a…" with half the line free (step 2). Beside a project narrower than 2.5ch, the time's cap (a percentage of a line sized by its own content) equalled its own width, and the app's WebKit cut it while rounding. Neither Playwright engine reproduces that rounding, so a unit test holds the cause and the fix was checked by hand in the real app: "api · 2 d ago" (0b88e4ba6).

Step 1 still scores under 4 on value, clarity and feeling. Part of that is setup: the lab's working and needs-you sessions had no conversation, so they read alike. PD3 now gives both a first ask (`first_ask`), and D3 runs again on the final 0.4.5 build. The rest is by design and goes to the maintainer: Home is the triage view (D2), so a session from three days ago isn't on it. The tester went to the rail's "Search ⌘K" at once and found the session in one search. It also said the first screen gives no hint that search exists.

## D3 again, with the lab's live two named (run 4)

A local Dev build at cfed9f0f4 (a clean export), lab PD3 with `first_ask` on its working and needs-you sessions. A fresh Sonnet tester; the firewall check and the leak lint are clean. An earlier attempt at this run was void: the desktop app's tool hooks stopped answering, and the tester never saw a picture.

| Flow | Step | W | V | C | F | Reason (short) |
|---|---|---|---|---|---|---|
| D3 Find one of fifty (run 4) | 1 Home | 3 | 3 | 3 | 3 | "All clear, 35 ended today"; the rows are told apart by name, app, repo and time. A triage view: the three-day-old session isn't on it, and only "Search ⌘K" leads further (run 3: 4 / 3 / 3 / 3) |
| | 2 rail search "login" | 5 | 5 | 5 | 4 | one match in 0.76 s, "Fix the login flow in the api", with its repo and age |
| | 3 its page | 4 | 4 | 4 | 3 | title, account, folder and branch; the Conversation panel showed only its loading bars (setup: see below) |
| | solved | yes | | | | one search and one click, about 40 s |

Step 3's loading bars were the lab window, not the build. The tester's window was fully covered by other apps (it ran behind them, as a walk's window does), and a page in a fully covered window counts as hidden, so the conversation feed waits until the window is shown (that has been so since 0238b1bda). With the same build's window brought forward, the page showed the whole conversation ("From its transcript"). A person looking at the window always sees it shown. The backend's read of this transcript was also checked by a test: every turn, the cookie line included.

Step 1 holds at 3 on value, clarity and feeling across runs 3 and 4. This is the design note for the maintainer: Home is the triage view (D2), so it doesn't list sessions older than the last 12 hours, and it gives no cue that search finds them. Both testers went to "Search ⌘K" at once and found the session in one search; neither saw the first screen as a way in.

Resolved by cb02b7f0f: Home stays the triage view, and its "In the last 12 hours" card now gives the cue. When the rail holds a session older than the card's window, the card ends with "Older sessions: search ⌘K", in the card's own link style. A click does what ⌘K does: the rail's search takes the caret and the search palette opens.
