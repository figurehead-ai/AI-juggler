# 0.4 persona runs on 0.4.6 rc3a (AJ-E3307)

Build: 0.4.6 rc3a at 6c2f07f70 (AI Juggler Dev, a release build). Lab: PR2, which has two client repos (client-a, client-b), the Apps Client A, Client A (old) and Agency, eight ended sessions, and no pool beyond each project's own.

Each run used a fresh Sonnet tester with screen tools only: guarded presses, window shots and typed keys on the lab window's pid. The firewall check was clean on all three runs; the only commands were the screen tool, the tester's own shots and writing its log.

## R4 Keep a client apart (Priya): three runs

Goal, in her words: "Make sure nothing from Client A's work can reach the others." Success: Client A's sessions are kept to themselves, and she can see that they are.

| Run | Steps | Result | Ease /7 | Value /7 | Where it went |
|---|---|---|---|---|---|
| 1 | 7 | partly | 3 | 3 | Projects › client-a, then "Agent access" (the pool, "Work together"), then Accounts and the Client A account page. She found the proof of who touched what, and no way to enforce anything. |
| 2 | 4 | partly | 3 | 4 | The Client A account's Projects tab (it lists client-b too), then client-a's "Who did the work" (mixed accounts), then "Agent access" (the pool, about agents inside one repo). |
| 3 | 6 | not | 2 | 3 | Projects › client-a, "Agent access", the pool, Accounts, then the Client A "…" menu (Open its page, Rename account). |

Per-step scores (worked / value / clarity / feeling):
- Run 1: 2/2/4/3, 3/4/4/3, 1/1/2/2, 2/3/4/3, 1/1/4/3, 4/5/4/4, 1/1/5/2.
- Run 2: 3/3/3/2, 3/5/4/3, 1/2/2/2.
- Run 3: 4/3/5/4, 3/4/3/3, 2/2/2/2, 1/1/2/2, 1/2/3/2, 1/1/4/2.

**No run reached sealing.** The flow's path is "Keep it apart…" on the project, or a sealed pool; none of the three opened the project page's "…" menu, where "Keep it apart…" is. The redesigned New pool (D-126) was therefore not walked by this flow.

What every run said, in the testers' own words:
- **Her model is accounts and repos, not agents.** She wants "client-a only on Client A's account", and the proof of it for an audit. The product's "keep apart" seals a project's agents from other pools' agents: a different promise, offered in words she doesn't use ("pool", "lead", "agents").
- **The wording points the other way.** The project page's Agents card reads "on their own: Work together", and "Agent access" opens a pool page about sessions messaging each other.
- **The evidence shows mixing, with no verdict.** "Who did the work" and the account's Projects tab show Agency and Client A (old) on client-a, and Client A on client-b. Home says "All clear", with no flag and no fix.

Test-tool notes (not product findings):
- In run 3, a "…" menu ignored presses outside it. The tool's presses are accessibility presses, and an open menu makes the page under it inert; a mouse click outside, or Escape, closes it.
- The tool's scroll didn't move the pool panel.

## For the maintainer (R5: none is a bug, so nothing changes without your ruling)

1. **"Keep it apart" where she looks.** Show it on the project page's Agents card, as a line that reads "Kept apart: no · Keep it apart…" (sealed: "Kept apart: yes, its agents can't reach other pools"), not only in the "…" menu. Say it in her words: "Client A's agents can't see or message other clients' work."
2. **The Agents card's words.** "on their own: Work together" reads as the opposite of keeping a client apart. Proposal: lead with what's true of the project ("Its agents: they work together inside client-a") and keep the level's name second.
3. **The feature she asked for (a decision, not a fix):** a project tied to its client's accounts, a session started there on another account flagged, and a per-repo, per-day "which account touched it" export for audits. Today the data is there (Who did the work, the account's Projects tab), but there is no rule and no flag.

## R4 again on rc3f (e32d3de55), after main's rulings 1 and 2: three fresh runs, 2026-11-19

Build: 0.4.6 rc3f at e32d3de55. The project page's Agents card now reads "Kept apart: no · its sessions can be in your other pools too", with "Keep it apart…" beside it. Its line reads "Agents here may message each other (Work together)". Lab: PR2, one fresh lab per run. Each run had a fresh Sonnet tester and the same brief as before. The firewall check was clean on all three runs: only the screen tool, the tester's own pictures and its log. No leak words appeared in the logs.

| Run | Steps | Result | Ease /7 | Value /7 | Where it went |
|---|---|---|---|---|---|
| 1 | 8 | partly | 4 | 4 | Home, Projects, client-b. "Kept apart: no" was on the Agents card, and "Keep it apart…" opened "Sealed client project", preselected. Next, Next, Make pool now: a sealed pool on client-b. Then Accounts, looking for a repo-to-account rule. |
| 2 | 5 | partly | 5 | 4 | Home, Projects, client-b ("Who did the work" showed the mixing), "Keep it apart…", the wizard's project step. She stopped there: sealing doesn't say which account a repo may use. |
| 3 | 7 | partly | 4 | 4 | Home, Projects, client-a ("Kept apart: no" under "Who did the work"), "Keep it apart…", Next, Next, Make pool now: a pool with the "sealed" badge. |

Per-step scores (worked / value / clarity / feeling):
- Run 1: 4/4/3/2, 5/4/4/3, 5/5/5/2, 4/4/3/3, 4/4/4/3, 4/3/3/3, 4/3/3/3, 1/1/3/2.
- Run 2: 3/3/3/2, 3/3/4/3, 4/5/4/2, 3/3/2/3, 3/3/3/3.
- Run 3: 3/3/3/2, 4/3/4/3, 4/4/4/3, 3/3/2/3, 4/3/4/3, 4/3/3/3, 3/3/3/3.

**What changed.** On rc3a, no run found "Keep it apart" (ease 2–3). On rc3f, all three found it on the project page, within 3–4 steps, and two of them sealed the project. The 0.4.6 fix does what it was for.

**Recorded as partly solved: account binding deferred (D-134).** Every run's "partly" has one reason: she wants a repo tied to its client's account, with a session on another account flagged. That's the feature main's D-134 defers to 0.6 or later (opt-in, in project settings). Sealing keeps a client's sessions out of other pools, and the testers saw that. It doesn't stop a repo running on another account.

For the maintainer (R5, not bugs; nothing changes without a ruling):
1. **The wizard behind "Keep it apart…"** (clarity 2 on the step in all three runs; confusion 2): a 9-step pool setup where she expected one confirmation. Proposal: from the project page, "Keep it apart…" opens a one-screen confirmation that says what sealing blocks and what it doesn't, with "Set it up in detail…" for the full wizard.
2. **Home and Projects don't flag a repo run on another client's account** (run 1 step 1, run 3 step 1: "nothing flagged it"). This belongs with D-134's binding.

Test-tool note (not a product finding): the wizard's Cancel, at the window's bottom left, gives "nothing to press there" to the accessibility press (run 2, and again by hand at three points). Next, in the same footer, presses fine, and Cancel works on a click (new-pool.test). It's on the a11y follow-up list.

## R4 once more on rc3g (f9167f4fe), after main's R5 ruling: one fresh run, 2026-11-19

Build: 0.4.6 rc3g at f9167f4fe. "Keep it apart…" now opens one sheet in plain words, with Keep apart, Customise… and Cancel; Keep apart makes the sealed pool «<project> · kept apart» with the wizard's defaults (c35c088d7, 930905a33). Lab: PR2, fresh. The tester was a fresh Sonnet Priya with the screen only, behind the idle gate (the Mac is shared). A first attempt stopped after 3 steps because the Mac never went idle; it isn't counted.

| Run | Steps | Result | Ease /7 | Value /7 | Where it went |
|---|---|---|---|---|---|
| 1 | 5 | partly | 5 | 5 | Accounts, Projects, client-b ("Who did the work": Client A 2, Agency 1, Client A (old) 1, "the audit answer, and it beats my spreadsheet"), "Keep it apart…": the sheet, read in one step (clarity 5, confusion 0). She chose Cancel: it says plainly that it doesn't tie client-b to one account, and that is what she wanted. |

Per-step scores (worked / value / clarity / feeling): 3/3/3/3, 3/3/3/3, 5/5/4/4, 3/3/5/3, and Cancel.

**What changed.** The wizard's clarity 2 (every rc3f run) is gone: the sheet scored clarity 5 and confusion 0, and ease rose from 4–5 to 5. Her "partly" has the same one reason as before, a repo tied to its client's account, which D-134 defers. **Recorded as partly solved: account binding deferred (D-134).**

Firewall: her tool calls were the screen command, her own pictures and her log only. Opus checked her claims against her pictures (the client-b page's "Who did the work" and "Kept apart: no · Keep it apart…").

## On beta: the soak's persona runs (AJ-E3307 R3–R6), 2026-11-20

Build: the soak's Dev bundle, the same product code as 0.4.6 on beta (v0.4.6, 130fd935a). Each run was a fresh Sonnet tester with the screen only (window shots and accessibility presses on the lab window's pid), one flow, one log. Opus checked every transcript (only the screen tool, the tester's own shots and its log) and every claim against its pictures. The pictures were then deleted: lab screens, and one run's macOS folder picker showed the real home's folder names. The logs keep each step.

| Persona · flow | Lab | Steps | Solved | Ease /7 | Value /7 | Feeling |
|---|---|---|---|---|---|---|
| Devon · find one of fifty | PD3 | 4 | yes | 6 | 5 | calm, quick; +1 |
| Maya · room left | PM3 | 2 | partly–yes | 6 | 6 | relieved, informed; +1 |
| Maya · first run | PF1 | 7 | partly | 5 | 4 | calm, hopeful, slightly stuck; +1 |
| Devon · the laptop is struggling | PD4 | 7 | partly | 5 | 5 | capable, then doubtful; +1 |
| Maya · out mid-feature | PM4 | 6 | partly | 5 | 5 | hopeful, uneasy; +1 |
| Devon · start a batch | N5 | 7 | partly | 5 | 4 | hopeful, then blocked; 0 |
| Maya · what needs me | K3 | 4 | partly | 4 | 4 | hopeful, stuck; 0 |
| Devon · overnight | K3 | 2 | partly | 4 | 5 | hopeful, then stuck; 0 |
| Devon · first run | PF1 | 7 | partly | 4 | 3 | curious, mildly disappointed; 0 |
| Maya · back after a reset | — | — | not walked: no lab seeds an account coming back while work waits | | | |

### Ranked findings (severity, then how many personas and flows they touch)

1. **A stopped left-behind process stays listed** (Devon · struggling; the soak's PD4 walk too): the stop works, the row stays "Left behind" until AI Juggler next starts, and Clean up then says "already ended". A bug with its cause found (SK-10): 0.4.7.
2. **"What needs me" never shows the question** (Maya and Devon, both stopped there): Home says "Open it to see what it asks", but nothing quotes the ask. In K3 the seeded sessions print nothing, so the page was blank. Part lab (the lab must stage a readable ask), part product (show the tool's own notice text on the card and the page).
3. **A switch doesn't show its carried work on the new page** (Maya · out mid-feature; the PM4 walk): the title is lost and only History says "Arrived from …" (SK-11). Content carrying was proven live on 0.4 (the c2c journeys); what's missing is showing it.
4. **First run with nothing to show** (Devon and Maya · first run): set-up was smooth (Find my apps, Bring in 3 apps, under a minute), but PF1 has no running sessions and its accounts no plan, so "show me what's running" and "measure its limits" couldn't land. Mostly the lab. Product part: "Plenty of room" beside 72 % CPU never says what room measures.
5. **Starting work on several repos** (Devon · batch): one session at a time, the folder defaulting to the last; no "start these on whichever account has room". A feature question.
6. **Home's headline vs the question asked** (Maya · room left): the answer was in Runway, lower right, under a headline about something else.

### What waits on the maintainer (R5: scores under 4)

Every flow above scored under 4 somewhere. None of 2–6 is a bug in what the screen promises, so nothing changes without a ruling. 1 is a bug: its fix goes to 0.4.7 with the triage. The lab gaps (a readable ask in K3, running sessions in PF1, a "back after a reset" seed, both repos in N5, its own web storage per lab) are test work for 0.4.7.
