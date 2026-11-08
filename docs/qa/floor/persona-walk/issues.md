# The Floor walk's issues (AJ-E3514)

## The setup: a window manager on the walk Mac (2026-11-08)

The walk Mac runs BetterStage (a third-party window manager that stages an app's windows one at a time and tiles them). With it, opening the Floor puts it in the main window's frame and stashes the main window off screen; moving the Floor away swaps them back. Nothing in AI Juggler places the Floor (it asks for 1278 × 798 and no position). Reproduced on a lab with the same build (78bf9c22a + floor-harness, bundled):

- A door works: Enter on a selected area brought the main window forward at the App's page.
- `floor_drive open` on a Floor that's already open but stashed didn't bring it back while another app was in front (an app in the background may not bring itself forward; the Floor button, clicked in the app, is the real path).

So the plan's check "the main window stays on screen beside the Floor" can't hold on this Mac. The check is now: **a door brings the main window forward at that page**, and testers are told that one window shows at a time (`main …` verbs need the main window shown: a door, or the main window's own controls, bring it back). A walk on a Mac without a window manager keeps the original check.

## F1 (Maya), run 1 — the 0.4.1 session's run, 2026-11-08

Failed, mostly on the setup above. Product findings, triaged:

| # | Finding | Severity | Lands in |
|---|---|---|---|
| 1 | The waiting desk isn't marked apart from CRASHED and the sleeper's Zzz | 2 | 0.4.x: check its `!` balloon and spotlight against the crash and limit marks at the opening view (captures) |
| 2 | The title's "1 session waiting here" can't be clicked to fly to the desk | 1 | Not a bug: a native title can't be a button; N jumps to the next one waiting (SPEC §9.7). 0.4.x: say N in the hint line |
| 3 | View › Labels › "Attention only" seemed to change nothing; the menu draws past the window's edge | 2 | 0.4.x: check by hand (may be a missed click); the menu's placement is the system's |
| 4 | The red memory-pressure banner competes with the goal | 1 | Lab artefact (the walk Mac's own memory pressure); not a Floor issue |

## F1 (Maya), run 2 — critic-checked, 2026-11-08

| # | Finding | Severity | Cause, and what's done |
|---|---|---|---|
| 1 | No waiting mark in the 3D view: only the tab's "!1" and the title; Maya didn't find who waits in 13 steps | 3 | **The lab, and the title's word.** FL1's "asks" session has no live process, so a start closes it: nothing on FL1 asks. The title's "1 session waiting" was the desk out of usage (asleep, Zzz). The title now says waits by kind: "1 asking", "1 stuck", "1 out of usage" (this commit). For F1 to test an asking session, one must really ask: `scripts/qa/lab_ask.py` on a live Lab CLI App session (`docs/qa/persona-validation.md`). Until then F1 is a lab gap, not a pass or a fail of the Floor's waiting mark |
| 2 | "5 need you" while the pictures show at most 4 | 2 | Needs you counts an Account's needs too (a sign-in, almost out of room), which aren't desks. Home says the same number. 0.4.x: the title could say how many are sessions ("5 need you · 1 asking here" already does where one asks) |
| 3 | Two rooms both labelled "Claude Code · Client · Client" (one "1 crashed", one "No sessions open") | 2 | 0.4.x: the Client desktop App should read "Claude app · Client"; check its word on a lab whose places book names it |

## The View menu in this setup

Opened through the driver while another app is in front (and with the window manager), the View menu can hang: the app's main thread stays in the menu's tracking, the picture never changes, and TERM isn't answered (the 0.4.1 session had to stop its lab instance by KILL, watcher first). On a lab instance here no menu appeared at all, so a driver verb for it can't be checked on this Mac. **Testers don't open View.** F7 (Apps per floor) and F8 (Re-create the office) are done from Settings › The Floor in the main window, which the SPEC lists too.

## F2 (Devon) and F4 (Maya), run 1 — 2026-11-08 (the 0.4.1 session's runs)

| Flow | Finding | Severity | Triage |
|---|---|---|---|
| F2 | The This Mac rack ("load and memory", a striped gauge with no number) opens nothing on a click | 3 | **The window manager.** The rack is a door (`Target::ThisMac` → the This Mac page, `floor/render.rs`); the page opens in the main window, which BetterStage keeps stashed while the Floor shows. To check with the window manager off. The gauge without a number: to the maintainer's list (a better design: "62 % memory · load 3.1" on hover, as the Floor list says) |
| F2 | The hover target just under the rack answers as the lift | 2 | 0.4.x: check picking at the rack's foot (Pixel art picks at the scene's pixel size) |
| F2 | No live hog on FL1 | — | Setup: F2's overload part needs the bounded load in a Lab CLI App session (the plan's live part) |
| F4 | In the building view, four "LOW · 8% LEFT" flags (one per App, all Personal's), the Account never named, two names cut ("Personal.."), no reset time | 3 | To the maintainer's list, with a better design: when a flag says only an Account's room, it names the Account, not the App ("PERSONAL · LOW · 8% LEFT · RESETS 9:31 PM"), and its Apps share one flag. Not changed silently (persona-validation: a design under the bar goes to the maintainer) |
| F4 | A hover near the flags shows another room's tooltip over them | 2 | 0.4.x: the tooltip keeps clear of flags (it's placed at the pointer) |
| F4 | Floor 2's Apps show no usage, so "fine" and "unknown" look the same | 2 | To the maintainer's list: an App whose Account isn't measured could say "not measured" on its sign, as the Account page does |

## Where the 0.4.1 walk stopped (2026-11-08, token budget)

Run by the 0.4.1 session on floorrc and floorrc2 (859150589 and later, lab FL1, BetterStage on); its records and captures are in that session's scratchpad (`floor-runs/`), not copied here (not yet checked by eye for the repo).

| Flow | Result | Note |
|---|---|---|
| F1 | lab gap | Nothing on FL1 asks; needs `lab_ask.py` on a live Lab CLI session |
| F2 | partly (×2) | The crash found every time; the Mac's load not found (the rack's gauge has no number: the maintainer's list) |
| F3 | not run | Needs a live swarm (a Lab CLI session with subagents) |
| F4 | partly (×2) | The OUT desk found ("Limit reached · shop"); "about to run dry" and the reset time not said (the maintainer's list) |
| F5 | not run | Needs an App added first |
| F6 | pass | — |
| F7 | blocked | The View menu off limits; the main window's Settings didn't come forward from a room click |
| F8 | not run | Same as F7 |
| F9 | partly | — |

New from these runs, for 0.4.x:
- `floor_drive open` still failed twice before it worked on F4 run 2, and after a click the window went from 3416 to 1798 wide (the window manager re-tiling it).
- A drag on a room does nothing and says nothing.
- Floor 1's opening view shows about 5 of its 10 Apps; a room click's view shows all 10. The opening view should fit the floor.

(Superseded by "The closing runs on the release build" below: every flow was walked and AJ-E3514 is done.) AJ-E3514 was not done then: the bar (each flow's success) isn't met for F1, F3, F5, F7 and F8, which weren't walked, and F2, F4 and F9 passed only in part. Whether it waits for 0.4.x or holds the release is the maintainer's call (with the BetterStage question in `docs/qa/0.4-completion.md`).

## The rerun on the release build (2026-11-08, after 32083e36d, bebc0bc96, bde1e8a80)

The bundled `floor-harness` build of HEAD, lab FL1, BetterStage on, one Sonnet tester per flow, every transcript checked (only `floor_drive`, only its own pictures). A real ask for F1: a session AI Juggler launched (a stand-in `claude` that waits) made to ask through `scripts/qa/lab_ask.py`, the product's own hook path.

| Flow | Result | What happened |
|---|---|---|
| F1 | partly | Found who waits on the Floor (the yellow ASKS mark, the area's "1 asking") and on Home; couldn't answer: the stand-in session has no question to read (the lab) |
| F2 | partly | The crash found every way; the Mac's load not looked for on the rack (now "Plenty of room · CPU 23%"), and no live hog in the lab |
| F4 | partly | Answered from the building view: "LOW · 8% LEFT · RESETS 2:24 PM"; the same flag once per App, and the floor view doesn't say it |
| F7 | pass (run 3) | View › Settings › The Floor…, Apps per floor 10 → 8, Floor 1 went to 8 Apps; runs 1–2 were the driver's (fixed) |
| F8 | not reached | The Floor stayed in the window manager's stash: neither `open` nor the Floor button brought it back from the driver |
| F3, F5 | not walked | F3 needs a real swarm (a signed-in Claude, outside the lab); F5 needs an App added first |

Findings for 0.4.x (under 4, the maintainer's list):
- F4: the room flag repeats per App; one per Account ("PERSONAL · 8% LEFT · RESETS 2:24 PM") is the better design (already proposed).
- F7: "Apps per floor" is only in Settings; the testers looked for it in the View menu or on the building view.
- F1: a click on a person changes the main window's page but the office doesn't say so ("opened in the main window").
- Home's header "Plenty of room" beside "1 account low" read as a contradiction (one is the Mac, one an Account).

## The closing runs on the release build (2026-11-08)

The bundled `floor-harness` build of release/0.4.1 HEAD, lab FL1, BetterStage on. A swarm made through the product's hook path: a stand-in `claude` session writing Claude Code's own subagent files (`scripts/qa/lab_ask.py` style, no real tokens). For F5, a new client app: a stand-in Claude app "Acme" profile, running. One Sonnet tester per run, every transcript checked (only the driver, only its own pictures).

| Flow | Result | What happened |
|---|---|---|
| F1, F2, F4 | partly | Earlier runs (above). F4's answer ("LOW · 8% LEFT · RESETS 2:24 PM") and the building view's one line a floor (eaec93546) close its finding |
| F3 | partly (run 3) | Runs 1 and 2: the helpers weren't counted as running in the first fixture, then counted but drawn too small to find. Fixed: an area's words count its sessions' subagents ("1 working · 3 subagents", acd1f7d9c, 91e61e1df). Run 3 found the session from the office ("1 WORKING · 3 HELPERS" then, "3 subagents working for this session" on the helpers). What each helper does is on the session's Subagents tab in the main window; a refused off-screen capture kept the tester from it (the driver now retries, 91e61e1df) |
| F5 | reached (run 2) | Run 1 failed: the import pre-ticked all 8 setups ("Bring in 8 apps"). Fixed (b1777b643, 24cf889e2): adding one tool's app with several setups finds ticks none. Run 2: "Bring in 1 app", the new app on Floor 2, Floor 1 and the other accounts unchanged. Two look-alike "acme" setups and a "Replica" account name come from the lab's fixtures |
| F7 | pass (run 3, earlier) | Apps per floor is now also in the View menu (e1e33025f) |
| F8 | pass | Re-created through Settings › The Floor; the confirm's "Changes / Stays" list was the best moment. "Re-create the office…" is now in the View menu too (d49f3a34e) |

R3, the hand walk (Sonnet, every element):
- Run 1 was blocked: the Floor stayed in the window manager's stash. Fixed in the product (142eccb5a): a Floor that's open but unseen is made fresh on open.
- Run 2: tabs, floor lines, areas, people, the subagents' desk, the notice board, keys (h, 0, b, 1, 2, n, tab, return), pan, turn and tilt, zoom and the View menu all worked, in 2 to 113 ms.
- Lows for 0.4.x: the rooftop rack is hard to find; the whiteboard doesn't say whose it is; the lift click does nothing visible; tab and escape focus isn't shown clearly; a hover outline can stay after the pointer leaves.

Design lows decided and fixed in this round:
- The header's "Plenty of room" next to "1 account low": a computer icon now says whose room (fb7b5c374).
- Per-App LOW flags: one line a floor in the building view.
- Helpers unfindable: the area's words now count them.
- Import pre-ticks: it ticks none.
- Re-create and Apps per floor were only in Settings: both are in the View menu now.

Left for the maintainer: none that needs a credential or a product-direction ruling.
