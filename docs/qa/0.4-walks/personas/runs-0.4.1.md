# Persona runs, 0.4.1 release candidate (AJ-E3307)

The run plan: each flow once (14), with a fresh sonnet tester for each, using only the browser tools. A flow runs again only when it fails or scores under 4.

Where they ran:
- The design preview with its generic fixture, served from the release branch at `localhost:5173`.
- Flows that need a live backend run on a lab instead (marked *lab*). The preview can't start a session or make one wait.

Scores are 1–7. Feeling is −2 to +2.

| Flow | Run | Success | Ease | Value | Feeling | Notes |
|---|---|---|---|---|---|---|
| M2 What needs me | 1 | — | — | — | — | Void: the preview has no session waiting on anyone. Moved to the lab (`lab_ask.py`). Two product findings, fixed: (a) a folded group's "!1" did nothing when clicked; (b) the session page's name differed from Home's (462309d47) |
| M3 Room left | 1 | partly | 5 | 4 | 0 | The New session dialog picked the account with room and said "58% left". The tired account's return time is "Not measured yet" in the preview |
| M4 Out mid-feature | 1 | partly | 5 | 4 | 0 | "What goes along" was the best moment. "Go to the new session" landed on "Unavailable" because the preview has no backend: re-walk live on the RC (P1 step 6, switch check) |
| M5 Back after reset | 1 | partly | 5 | 3 | −1 | The same preview dead end after the switch. The value score is under 4 only because nothing in the preview proves a move. Re-walked live (see the RC sheet) |
| D3 Find one of fifty | 2 | partly | 5 | 4 | 0 | No "login" session in the fixture. History search was the right tool, but the tester found it by guessing: no visible search entry in the header (⌘K only). **Recommendation for the maintainer.** The session opened on a remembered Pools tab: fixed (ce149525f) |
| D4 Laptop struggling | 2 | partly | 5 | 5 | 0 | Its best moment: the per-process proof before Stop. A refused stop ("belongs to a different program") gives no next step. The page didn't refresh after Stop because the preview fixture doesn't change |
| R2 Right account | 2 | partly | 6 | 5 | +1 | The start dialog names the account and folder. Wish: lock a repo to one account (0.5 idea) |
| R3 Who touched this repo | 2 | yes | 6 | 5 | +1 | Two clicks. Wish: dates and an export for audits (0.5 idea) |
| R4 Keep a client apart | 2 | no | 3 | 2 | −1 | Sealing lived only as a pool template, behind a word she didn't know. Fixed: the project menu has "Keep it apart…" (c299c500f) |
| R4 Keep a client apart | 3 (fresh tester) | partly | 5 | 5 | 0 | Her best moment: "Keep it apart…" opened "Sealed client project", already chosen. Found on the way: (a) after sealing, "Let agents in your projects see each other?" read as the opposite, so a sealed pool no longer asks it (c4a03e321); (b) her "Not now" exposed that the seal rode the agent tools, so it's now its own launch step (a3903836e); (c) the preview's project page still listed the default pool with the sessions, a preview-model limit, re-checked live |
| M2 What needs me | 2 (*lab*, PM3 on the lab bridge at 0c8a82136) | partly | 6 | 5 | +1 | Home's headline and Needs you found it in one click. Rows give no wait time or order ("last change 06:38" on all three); "2 more things" unexplained. The question itself wasn't visible: the lab bridge doesn't attach the terminal (setup). Fixes in flight (Home triage agent) |
| D2 Morning triage | 1 (*lab*) | partly | 5 | 5 | +1 | "Since you were away" plus Needs you answered "what needs me", not "what happened": every ended run reads "Finished" (no failed/crashed), "60 finished" beside "11 ended today", Home's and the session's wait times disagree. The landing page was the previous run's page (shared browser pane), not the product. No stuck session could be made (kit gap) |
| D5 Start a batch | 1 (*lab*) | yes | 5 | 4 | +1 | Both started on Personal, the account with room, without touching the choice. Lab repos weren't git, so "Its own worktree" was greyed (kit fix in flight); New session's text truncates at 800×600 (fix in flight). Wish: a batch start |

Run 1 of D3, D4, R2, R3 and R4 was void: a dev-server reload broke the preview, which was my fault. That run did find one product gap: a failed start left the boot screen up for good. Fixed in 0edb8e078: after 20 s, or on a load error, the screen says so and offers Reload.

To come:
- ~~M2, D2 and D5 on a lab.~~ Walked 2026-11-10 (above).
- M1, D1 and R1 (first run) on the native window with a lab HOME, as scripted persona walks.
