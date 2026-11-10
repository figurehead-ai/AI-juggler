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

## The gate scores (AJ-E3307 R4: per step, 1–5), 2026-11-10

Re-scored from the three lab runs' notes by a fresh critic (the runs weren't repeated; the notes carry each step). The earlier 1–7 line above is extra evidence, not the gate. Steps whose only problem was the setup are "setup, not scored".

| Flow | Step | Worked | Value | Clarity | Feeling | Solved |
|---|---|---|---|---|---|---|
| M2 What needs me | 1 Home | 5 | 4 | 3 | 3 | **partly** |
| | 2 The waiting row | 4 | 4 | 4 | 3 | |
| | 3 Its page | 3 | 3 | 3 | 2 | |
| D2 Morning triage | 1 Landing | setup | setup | setup | setup | **partly** |
| | 2 Home | 4 | 4 | 4 | 4 | |
| | 3 Reading the rows | 3 | 2 | 2 | 2 | |
| | 4 The first "Asks you" | 3 | 2 | 2 | 2 | |
| D5 Start a batch | 1 Home | 5 | 5 | 4 | 4 | yes |
| | 2 New session | 4 | 4 | 3 | 3 | |
| | 3 Start in billing | 4 | 3 | 3 | 3 | |
| | 4 New session for shop | 3 | 3 | 3 | 3 | |
| | 5 Start in shop | 4 | 3 | 4 | 3 | |

Every score under 4 and both "partly" flows are R5 items: reasoned, a better design proposed, and brought to the maintainer before anything that isn't a bug changes. The bugs among them (the two disagreeing wait times, the counts that don't reconcile, New session's text cut off at a narrow window) are being fixed.

## The native runs on rc29 (c07ce83d2), 2026-11-10 — the R4 gate, per step

A fresh Sonnet tester per flow, with only screen tools (guarded clicks, shadow-free window captures, typed keys): no files, code or docs. Labs: PF1 (first run, reset per flow) and PM3 (live sessions staged through New session: one idle, one made to ask with `lab_ask.py`, one at Claude's first-run screen). Scores are worked / value / clarity / feeling, 1–5. The lab-bridge M2 steps 2–3 earlier the same day are recorded as setup-blocked (the bridge serves no terminal), not as product scores.

| Flow | Step | W | V | C | F | Reason (short) |
|---|---|---|---|---|---|---|
| M1 first run | 1 Home | 5 | 4 | 4 | 4 | "Find my apps" obvious; it already named her tools |
| | 2 Find my apps | 4 | 4 | 3 | 3 | both Claude setups found signed in; raw paths and "where they are / copies" needed a re-read |
| | 3 Bring in 2 apps | 5 | 4 | 4 | 4 | straight to the optional permission |
| | 4 Figurehead account | 5 | 3 | 4 | 3 | an account she doesn't need (→ D-97 item 2) |
| | 5 Home after setup | 3 | 3 | 3 | 3 | "Not measured yet", Codex gone without a word (→ D-97 item 1) |
| | 6 Accounts | 4 | 3 | 4 | 4 | clean list, opened on the Apps tab |
| | solved | partly | | | | |
| D1 first run | 1 Home | 3 | 3 | 4 | 3 | a hero, "Nothing open yet" |
| | 2 Find my apps | 5 | 4 | 4 | 4 | paths, versions, sign-ins in seconds |
| | 3 tick Codex, bring in 3 | 4 | 4 | 3 | 3 | Codex unticked by default |
| | 4 skip permission and account | 5 | 3 | 5 | 3 | four steps is many |
| | 5 Home | 2 | 2 | 3 | 2 | nothing running shown (the lab had none: setup) |
| | 6 This Mac | 4 | 4 | 3 | 4 | real numbers; "Plenty of room" with swap 39/40 GB (→ bb46b57d1) |
| | 7 Sessions tab | 1 | 1 | 3 | 2 | empty (setup) |
| | 8 Processes | 1 | 1 | 2 | 1 | "Everything running" empty while Claude ran at 782% (→ dcb7a5c9f) |
| | 9 Apps tab | 3 | 2 | 4 | 3 | all "not used yet" |
| | solved | no | | | | |
| R1 first run | 1 Accounts tab | 3 | 3 | 2 | 3 | empty page in "apps" words (→ D-97 item 3) |
| | 2 Add an app | 3 | 3 | 3 | 3 | a tool list, not her question |
| | 3 Claude Code | 5 | 5 | 4 | 4 | both accounts found, no sign-in |
| | 4 tick both | 5 | 4 | 5 | 4 | clear |
| | 5 Bring in | 5 | 4 | 4 | 4 | quick |
| | 6 Accounts 2 | 5 | 5 | 4 | 4 | one card per account |
| | 7 card ⋯ Rename | 2 | 2 | 1 | 2 | renamed the App, the card stayed "Default" (→ 202236141) |
| | 8 Account page | 5 | 5 | 4 | 4 | per-account Sessions and Projects |
| | 9 Settings › Rename | 5 | 5 | 4 | 4 | worked; a detour to find |
| | solved | partly | | | | |
| M2 what needs me | 1 Home | 5 | 5 | 4 | 4 | headline and rows with wait times |
| | 2 Open its page | 5 | 4 | 4 | 4 | live terminal with the question |
| | 3 back to Home | 5 | 3 | 5 | 4 | — |
| | solved | yes | | | | |
| D2 morning triage | 1 Home | 5 | 4 | 4 | 4 | what needs me in ~5 s; lists repeat |
| | 2 first waiting session | 4 | 3 | 3 | 3 | no summary of the question (→ 70b49190b, df94da4a1) |
| | 3 back to Home | 5 | 3 | 5 | 4 | — |
| | solved | partly | | | | |
| D5 start a batch | 1 New session | 5 | 4 | 5 | 3 | usual = Work at 5%, warned (→ D-97 item 4) |
| | 2 Use Personal | 5 | 5 | 5 | 5 | one click |
| | 3 Start (folder in use) | 4 | 5 | 5 | 4 | asks: worktree or share |
| | 4 Create the worktree | 1 | 1 | 1 | 1 | "Choose that place again" (→ b3243573c) |
| | 5 Share the checkout | 3 | 3 | 3 | 2 | started on Personal |
| | 6 Start another toast | 1 | 3 | 3 | 3 | gone before the click (→ 89d7ac576) |
| | 7 New session for shop | 4 | 4 | 4 | 3 | Change folder, stayed Personal |
| | 8 Home shows both | 4 | 4 | 3 | 3 | — |
| | solved | partly | | | | |

Every score under 4 and every "partly"/"no" went to the maintainer under R5 (D-96, D-97) or was a bug fixed at once (the commits named). The fixed steps are walked again by fresh testers on rc31 below.
