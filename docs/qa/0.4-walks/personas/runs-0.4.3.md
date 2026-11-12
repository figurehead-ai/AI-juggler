# Persona runs, 0.4.3 (AJ-E3307)

The code the 0.4.3 beta ships (`b281d3ee6`, the same tree as the merge `a5c84290f`) was walked as CI built it (`rc-dev-app`). The installed app on the test Mac was still 0.4.2, and updating it is the maintainer's step. Run plan as in 0.4.1: each flow once with a fresh Sonnet tester, seeing only the main window (`floor_drive main`: shot, click, type, scroll, by pid); a flow runs again where it fails or scores under 4. Every run passed the firewall check: only `floor_drive main` commands, only its own pictures read, and no leak words.

## Presence first

- **Real Mac** (an act-free copy, `act_free_copy.sh`):
  - PASS. Open now is exactly the 5 running Apps; each unfolds to its own records; every Out Account is marked; Codex 15 + 33 = 48 as Codex lists them.
  - Ledger 0 → 0, no child process; the Dev folder was restored.
- **Lab PL1** ("what runs and what each App holds", new):
  - PASS (`presence_check.py --home`). Two Claude app profiles run: work, with 5 past sessions and nothing open, Out; personal, with none.
  - Inactive: client (closed, 2 sessions, Out), the Codex app (4), Codex CLI (3) and a Claude Code CLI (4).
  - Script runs are in no App. A build on a lab sees only the lab's processes, so the Mac's own ChatGPT.app reads closed there; the real-Mac check covers it.

## The gate scores (R4: per step, 1–5: worked / value / clarity / feeling)

| Flow | Step | W | V | C | F | Reason (short) |
|---|---|---|---|---|---|---|
| M6 Which can I use now (Maya, PL1) | 1 Home | 4 | 4 | 3 | 3 | Out marks clear; the same Account "resets 7:06 PM" (5-hour) on its tile and "Sun 5:06 PM" (weekly) in its notice: which frees me? |
| | 2 Usage | 5 | 5 | 4 | 4 | both windows' "back in" side by side; "Measure its limits" vague |
| | solved | partly | | | | Personal and ChatGPT "Not measured yet": can't tell whether they're safe |
| D6 What's up right now (Devon, PL1) | 1 Home | 5 | 4 | 4 | 4 | cards and rail agree on running / not; "seen this hour" vague |
| | 2 Show all 5 (work) | 4 | 3 | 3 | 3 | the rows under Open now are all Ended: is anything live? |
| | 3 personal | 3 | 2 | 3 | 3 | "No sessions … yet, once AI Juggler has seen them": empty, or unseen? |
| | solved | partly | | | | running vs not is clear; what's open in each reads as history |
| M7 Back to that conversation (Maya, PL1) | 1 Home | 4 | 3 | 3 | 3 | work's last 3 shown, the one she wants not among them; guessed "Show all 5" hides it; Home led by limits, not her sessions |
| | 2 Show all 5 | 5 | 4 | 4 | 4 | the App's Sessions tab named it; landing on an App page surprised |
| | 3 its page | 5 | 3 | 3 | 3 | read in place, no app started; "Work is out of room" and "Looking for room…" crowd a page she only reads. Its replies were the lab's placeholder text (setup: value partly) |
| | solved | partly | | | | found in 3 clicks; the text she wanted was the lab's placeholder |

Fixed at once: the empty desktop App's words (D6 step 3), eb49987ff.

## Held-over flows, re-walked on 0.4.4 (local Dev build at 318c31fa9, its fixes in)

| Flow | Step | W | V | C | F | Reason (short) |
|---|---|---|---|---|---|---|
| D3 Find one of fifty (Devon, PD3) | 1 Home | 3 | 3 | 2 | 2 | 32 rows alike, "api · Work · work · started …", cut off: nothing to scan (P10; the lab's sessions are mostly untitled) |
| | 2 rail search | setup | setup | setup | setup | the walk driver couldn't focus the search box (its placeholder was under the point); fixed in the driver, 55d923531; by hand, "login" there finds it at once |
| | 3 History | 4 | 4 | 4 | 4 | a search box with title, project, branch, date, what was said |
| | 4 "login" | 5 | 4 | 4 | 4 | one hit, exact |
| | 5 its page | 4 | 3 | 3 | 3 | "Finished Mon 7:07 PM" (P3 fixed); the conversation shown (P2 fixed); no files or diff named |
| | solved | partly | | | | found in 5 steps; what was done is a summary, not the change |

P1 (newest first) holds in the rail's "Ended today". Fixed on the way: "ended Thu 06:20 PM" on a Thursday in Home's summary (f56e254ac).
| R2 Right account for the client (Priya, PR2) | 1 Home | 4 | 3 | 2 | 2 | rows read "client-b · Client A · client-a": repo, account and profile in mixed places, her wrong-account fear (P5) |
| | 2 New session | 3 | 2 | 3 | 2 | "Start in client-a on … Agency · agency, the last you used here": the repo's own account isn't the default (P4) |
| | 3 Change app | 4 | 3 | 2 | 2 | "Client A" beside "Client A (old)"; the "Always use for client-a" line surprised |
| | 4 Client A · client-a | 5 | 3 | 4 | 4 | the dialog now names Client A and the folder |
| | 5 Start | 5 | 4 | 4 | 4 | "Where it runs": Account, project, signed-in address; toast and rail agree |
| | solved | partly | | | | right account, only because she caught the default |
| R3 Who touched this repo (Priya, PR3) | 1 Home | 2 | 2 | 3 | 3 | Home is today only; Client B "Not measured yet" |
| | 2 Projects | 4 | 3 | 5 | 3 | client-b with 14 sessions, "Tokens this week: None" (P6's label fixed; the lab's sessions carry no usage) |
| | 3 its Sessions | 5 | 4 | 5 | 4 | 14 rows, each naming account and repo: the "who did what" |
| | 4 its History | 3 | 3 | 4 | 3 | grouped by day; untitled names say nothing of the work |
| | 5 Overview | 3 | 3 | 2 | 2 | "Apps used 1 · Client B" proves one account; "0 tokens", "No usage yet" beside 14 sessions (P7) |
| | 6–8 scroll | setup | setup | setup | setup | the walk driver can't scroll that page's inner area (its scroll bar shows it scrolls) |
| | 9 its Usage | 1 | 1 | 2 | 2 | "No usage … once a session in its folder has run" beside 14: a false reason, fixed (9eb945506) |
| | solved | partly | | | | count, dates and account in two clicks; not what was done, no usage |

## Re-walks on 0.4.4 (fresh testers; PL1 presence PASS on 0.4.4 first)

| Flow | Step | W | V | C | F | Reason (short) |
|---|---|---|---|---|---|---|
| M6 Which can I use now (run 2) | 1 Home | 4 | 5 | 4 | 4 | one return time on every screen ("Out · resets Sun 6:46 PM"); was 4 / 4 / 3 / 3 |
| | 2 Accounts | 3 | 4 | 3 | 3 | opened on its Apps tab, not Accounts; the header says why it can't tell which has room |
| | solved | partly | | | | which is spent and when it's back: sure (90%); the one with room "not measured" (she didn't try Measure) |
| D6 What's up right now (run 2) | 1 Home | 5 | 4 | 4 | 4 | running vs not at a glance; the swap warning helps |
| | 2 Show all 5 (work) | 3 | 3 | 3 | 3 | "work nothing open" above three Ended rows reads as a contradiction (R5 item 1, now seen twice) |
| | 3 personal | 4 | 3 | 4 | 3 | "No sessions in Claude app · personal yet": trusted now; was 3 / 2 / 3 / 3 |
| | 4 the not-running apps | 3 | 3 | 2 | 3 | Inactive rows cut off with "…" |
| | solved | partly | | | | running vs not clear; what's open in each reads as history |
| M7 Back to that conversation (run 2) | 1 Home | 4 | 3 | 3 | 3 | two out-of-room notices lead Home; she came for a chat |
| | 2 rail search "stack trace" | 5 | 5 | 4 | 4 | one hit, right account and age; "work nothing open" above it confused |
| | 3 its page | 4 | 4 | 3 | 4 | read in place; "Work is out of room" shouts on a page she only reads (R5 item 3); "From its transcript · not saved" worried her |
| | solved | partly | | | | found in 2 actions (was 3); the lab's replies are short summaries, not a real explanation (setup) |

## Summary

- **Verdict: ready with fixes; not yet at high satisfaction.**
  - Every flow reached its goal. None was "solved" in full: 9 runs, all "partly".
  - The bugs the walks found are fixed in 0.4.4, and the re-walks rose where they touched (M6 step 1 clarity 3 → 4, D6's empty app 3 → 4, M7 3 steps → 2).
  - What stays under 4 is design, brought to the maintainer under R5: the ended rows under "nothing open", New session's default account in a client's repo, untitled names, usage fallbacks, the out notice on a page you only read.
- **Best moments:**
  - the rail's search finding a session in one step;
  - "Where it runs" naming account, project and sign-in;
  - Home's app tiles answering "what runs" at a glance.
- **Worst moments:**
  - "the last you used here" defaulting a client repo to the agency account;
  - rows that read "client-b · Client A · client-a";
  - "No usage yet" beside 14 sessions (fixed).
- **Setup, not scored:**
  - the walk driver couldn't focus the rail's search box (fixed in the driver) or scroll a page's inner area;
  - the labs' conversations are short summaries;
  - a build on a lab sees only the lab's processes, so ChatGPT.app reads closed there.
- **What this can't tell:** how strangers with other setups judge it, how it feels after weeks, and anything the maintainer's calibration (`calibration-0.4.3.md`) will show. One run per flow, so no run-to-run variance.
