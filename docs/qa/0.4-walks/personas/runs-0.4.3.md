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
