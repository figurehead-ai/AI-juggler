# The second-Mac check (0.4.1, D-89)

Your check of Fleet on two real Macs, after 0.4.1 ships. Before the
release it was proven as far as one Mac allows: two copies of AI Juggler
on loopback, each with its own data folder. This sheet is what one Mac
can't prove: pairing over a real network, a switch between two Macs, a
cleanup on the other Mac, and **Bring forward on <Mac>**. Anything that
fails here is fixed in a following 0.4.x (D-89).

About 30 minutes. Read each check's **Expect** before you do it.

## What you need

- **Two Macs** on the same network: **A** (this one) and **B**. Both
  signed in to macOS as you. B needs at least one tool signed in (Claude
  Code or Codex) and a project folder you don't mind a test session in.
- **AI Juggler 0.4.1 on both**, from the beta channel: on B, install it
  from the release page (docs/guide/install-and-updates.md), open it once
  and finish its first run. Check **Settings › About** says 0.4.1 on both.
- A timer (your phone's stopwatch).

## The checks

| # | Do | Expect |
|---|---|---|
| 1 | On both Macs: **More › Fleet** (⌘⇧F) › turn **Link on**. Allow **Local Network** when macOS asks. | Each Mac says Link is on. Nothing else changes. |
| 2 | Start the timer. On A: **Add a machine…**. On B: **Join a machine…**, pick A, type the 6-digit code. Compare the **fingerprints** on both screens, then **Fingerprints match**. Leave **View only**. Stop the timer. | Paired in **under 60 s**. Both Macs list each other under Fleet. The fingerprints matched. |
| 3 | On A: open **Fleet**, then **View › Show › other Macs** in the list. | B's sessions show, with their project, app and account names and states. No prompt text, no files. |
| 4 | On B: start a session in your test folder. Watch A. | It appears on A within a few seconds, and its state changes follow (working, then waiting). |
| 5 | On B: Fleet › A's row › **Access…** › **View + control**. On A, on B's session's page: **Focus on B** (Bring forward on B). | B's window with that session comes to the front on B. A says what it did, in words. |
| 6 | On A, on B's session's page: **Continue on another machine** › A › read what carries over › switch. Then from A, switch it back to B. | A→B→A: the conversation carries its history, tasks and context each way (the switch screen's checklist). Nothing is lost; the original stays archived where it was. |
| 7 | On B: run `sleep 600 &` in the test session's terminal, then end that session in its tool. On A: **Processes › Your other Macs** › B › clean up the leftover `sleep`. | Only that `sleep` is offered and stopped. Nothing else on B is touched. |
| 8 | On A: **Stop on B** for B's other session (if any), then **Undo** within 5 seconds. | Nothing is sent; the session runs on. |
| 9 | Turn Wi-Fi off on B for a minute, then on. | A shows B offline, then back, without a restart. Nothing on A breaks. |
| 10 | On B: **Unpair…** A. | A no longer shows B. Turning Link off on both leaves each Mac working on its own as before. |

## When something fails

Tell me in this session, or open an issue on GitHub
(github.com/figurehead-ai/AI-juggler/issues), with:

1. The check's number and what you saw instead of **Expect**.
2. A screenshot of each Mac's window at that moment.
3. From the Mac where it went wrong: **Settings › Advanced ›
   Diagnostics › Export diagnostics…**, look through it, **Save…** (a
   scrubbed zip: no sign-ins, keys or prompts).

Don't redo a failed check: a second try can hide what happened. Go on
with the next one; the fix comes in 0.4.x.

## What this covers

These are the live rows of the Fleet gate, deferred to this check (D-89).
On one Mac they're proven by the in-process loopback suites (`just
gate-e7`: two AI Juggler backends over real loopback connections), with
the Fleet gallery set and a keyboard-only sweep; this check is where they
run live, on two real Macs:

- AJ-E710 R1a–R1d: pairing (check 2, under 60 s), the board (checks 3
  and 4), A→B→A (check 6), a remote cleanup with nothing else touched
  (check 7), and R4 (measured on real Macs).
- AJ-E2102 R6 (**Bring forward on <Mac>**, check 5).
- The guide's promises: docs/guide/fleet.md.
