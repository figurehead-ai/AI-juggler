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
