# Persona validation: the protocol (AJ-E3307)

How 0.4 is validated the way its users would judge it, before the announcement. The method and its evidence are in [`docs/research/persona-validation.md`](../research/persona-validation.md); this page is what to do, in order. The maintainer's ruling (2026-11-02): put yourself in the user's shoes, not the developer's; measure whether each action worked, gave value, was clear, and how it felt; rank it; fix bugs at once; bring anything without a high score to the maintainer.

**What the research says, in one line each:**
- An AI tester finds and ranks problems well; it is *not* a stand-in for real users' satisfaction. Every study found simulated users more successful, more positive and less lost than people. So behaviour is measured, self-ratings are checked against it, and the thresholds are stricter than human norms.
- The tester that built the product can't judge it: it knows the hidden paths and favours its own work. So the tester is always a fresh agent that has never seen the code.
- One run is never enough: three per flow, and a critic who checks every claim against the screenshots.
- The maintainer's own runs of a few of the same tasks are the calibration: if the agents score higher than the maintainer, every threshold moves up by the gap.

## Roles (separate contexts, never merged)

| Role | Who | Sees | Does | Never |
|---|---|---|---|---|
| Builder | this session | everything | writes personas, flows, fixtures; triages; fixes | scores, or lowers a score or severity |
| Tester | a fresh agent per run (×3 per flow) | the persona brief, the goal, the screen | walks the flow, records each step, scores | reads files, code, docs, memory, or earlier runs |
| Critic | a fresh agent | the run's log and screenshots | checks each claim against the pixels, classifies failures, rates severity | sees the code |
| Maintainer | the person | the app | 2–3 shared tasks for calibration; decides disputed severities and anything under the bar | — |

**The firewall.** A tester is started with only the browser tools allowed by its brief, in a clean scratch folder. After each run its transcript is checked: any file read, search, shell command or code tool voids the run. The **leak lint** greps the transcript for words a newcomer can't know (below); a hit voids the steps that used it.

Leak list (grows as runs show more): place, instance, harness, provider, recipe, launch recipe, folder page, lens, book, known place, D-, AJ-E, E3, ADR, SPEC, rail-, testid, Tauri, dev bridge, fixture, preview, seam, adapter, `cli:`, `app:`.

## Where the walks run

- **The design preview with a fresh fixture**, in the built-in browser: deterministic, generic names, nothing real. It covers every screen and most flows, and first run with an empty fixture.
- **The built app on the throwaway lab** (AI Juggler Dev, lab accounts only; never a real account's data changed), through the dev bridge in the built-in browser: real tools, real sessions, real timing.
- **A session that asks you** (M2, D2): launch one in the lab, then `scripts/qa/lab_ask.py <its pid> <lab HOME>` makes it ask, through the product's own hook path, as a real permission prompt would. The lab's app must run from a folder only you can write (not under `/tmp`) with `juggler-agent` beside it, and the agents question answered yes in the lab; otherwise its sessions carry no hooks.
- **First run (M1, D1, R1) and other native windows** (the main session's ruling, 2026-11-06): the lab bridge doesn't serve first run's scan or in-place import, and isn't widened for it. These flows run on the release candidate's real AI Juggler Dev window, started with `HOME` set to a lab whose tools' folders are made-up fixtures (no keychain there), driven by `osascript` and `screencapture` on that process's pid only. Testers get a click-and-screenshot tool for that one window; where that's impractical, the Builder walks them as a scripted persona walk on the same sheet and protocol, and the report says so. The Floor's 3D window is the same kind of step. The lab is torn down after.
- **What an agent can't drive** is said, not skipped: the installer, macOS permission prompts, the menu bar, notifications. The maintainer's calibration runs cover those, and the report lists them as not walked by an agent.

## Never, in any walk or persona run

No connectors, no artifacts, no Docs, no sharing: a tester, a driver and every session it starts use the app, the lab's own folders and the pool's own tools only, never a claude.ai connector, an artifact, Docs or any way of sharing outside AI Juggler. Every brief says so, and every prompt that asks a session to message or share names the pool's tool (`send_message`, `share_doc`). The Lab App's sessions run with claude.ai's connectors off ([7b sheet](0.4-walks/7b-live-claude.md)). The rc5 walk's P8 scout made a private artifact on the maintainer's account; this rule is why.

## Step 1 — Personas and flows (Builder)

Three personas from the evidence ([`docs/review/deep/07-personas-and-journeys.md`](../review/deep/07-personas-and-journeys.md), the maintainer's own use and reports, the research): Maya, the solo developer on several subscriptions (primary, weighted ×2: the launch audience and the maintainer's own profile); Devon, a staff engineer running ten sessions in parallel; Priya, a consultant with client accounts kept apart. Each persona page ([`0.4-walks/personas/`](0.4-walks/personas/)) has, with a source or `[assumption]` on every line and **no feature names**: who and context; goals as outcomes; daily triggers; what they know and the words they use; at least two frustrations; one thing they don't care about; what they do today without AI Juggler.

Each persona has 4–6 flows, starting with **first run**. A flow is: start state; the goal in the persona's words (never the UI's); an observable success state; an "abandon if"; and the shortest path, kept from the tester. Before any walk, the briefs are run through the leak lint.

## Step 2 — First impressions (Tester)

- **Clarity test:** the first-run screen and the main window, once each. "What is this? What is it telling you? What would you do first?" Pass: it names the purpose and a right first action.
- **First click:** for each flow, the start screen and the goal: "Where do you click first?", before exploring. Right or wrong is recorded.

## Step 3 — The walk (Tester, three fresh runs per flow)

The brief: "You are *persona*. You're busy and sceptical; you manage this today without this app. Try to *goal*. Think aloud. Use only what you can see. Say plainly when something's unclear. You may give up."

Every step is one record:

| Field | |
|---|---|
| before | screenshot |
| looking for | the goal now, and what I'm looking for |
| candidates | the controls I can see that might do it, each rated 0–2 for fit; a tie is flagged |
| expect | what should happen |
| action | what I clicked or typed, by the words on screen |
| after | screenshot; how long until the screen settled |
| saw | what changed; expectation met yes / partly / no |
| four questions | did I know to try this? did I see the control? did I connect it to my goal? did I see progress after? (Y/N each) |

Stop rules: give up after 3 attempts with no progress, or after twice the shortest path plus 5 steps; the same screen and action twice in a row means re-plan or give up.

After each flow: success grade; ease 1–7; value 1–7; feeling, three words from the balanced list plus −2…+2; "was my problem solved?" yes / partly / no; each with a one-line reason citing a step. After the walk, a **confusion pass**: each screen visited rated 0–2 for "would someone like me be confused here?"

At the end of each persona: UMUX-Lite (two items, 1–7), likelihood to recommend (0–10, reported, not gated), the three best and three worst moments, and "what would make you stop using this?"

## Step 4 — Heuristic sweep (a fresh agent, screenshots only)

Nielsen's ten heuristics over every screen captured. The Builder checks consistency across screens, user control and undo by hand: the research found agents weakest there.

## Step 5 — Critic

Checks every claim against its screenshot; strikes misread icons and OS chrome; voids leaked steps; classifies each failure as the app's, the agent's or the setup's; rates severity 0–4 on its own; challenges any ease or value of 6–7 with no behaviour behind it.

## Step 6 — Triage and fix (Builder)

Every bug reproduced by hand in the real app, fixed at once, and its flow walked again by a **new** tester (never the one who reported it), with before/after captures. Severity is the mean of the tester's, the critic's and the Builder's. Anything under the bar (below) is not changed silently: the Builder writes why it scored low and a better design, and brings it to the maintainer.

## Step 7 — Calibrate

The maintainer does 2–3 of the same tasks and gives each an ease score and one sentence. The report shows the agents' ease against the maintainer's on those tasks, and which of the maintainer's own earlier complaints the agents found. If the agents rate more than 1 point above the maintainer, every threshold below rises by the gap, and the report says so.

## Scores and the bar

| Measure | Scale | Anchors |
|---|---|---|
| Success | S / SD / F / GU | S: reached the success state with no detour. SD: after a wrong click, backtrack or tie. F: wrong end state, including "thought it worked". GU: gave up |
| Ease | 1–7 | 7 did it without thinking, first click right; 6 one brief hesitation; 5 a search or one wrong click; 4 several tries, unsure it worked; 3 trial and error; 2 barely; 1 couldn't |
| Value | 1–7 | 7 clearly better than my way today, I'd do it daily; 5 useful, no better than my way; 3 not worth the effort; 1 worse or harmful |
| Clarity | 0–2 per step | 0 obvious; 1 had to look or compare (a tie, a hover); 2 wrong expectation, or didn't know what happened |
| Feeling | −2…+2 and 3 words | confident, in control, relieved, fast, calm / confused, anxious, annoyed, slow, overwhelmed, distrustful |
| Severity | 0–4 (NN/g) | 0 not a problem; 1 cosmetic; 2 minor; 3 major, fix before release; 4 catastrophe |
| Response | ms | ≤ 100 instant; ≤ 1 s keeps the flow; > 1 s needs a progress cue; > 10 s is a finding |

**A flow passes** when all three runs reach S or SD and at least two reach S, the first click is right in at least two, median ease ≥ 6, median value ≥ 5, the problem is solved in all three, no step is rated confusion 2 in two or more runs, and no issue of severity 3 or 4 is open.

**A persona is at high satisfaction** when every flow passes, median feeling ≥ +1, and UMUX-Lite ≥ 80, and no answer to "what would make you stop" points at something unfixed.

**0.4 is ready to announce** when all three personas are at high satisfaction, no severity-4 issue is open, no severity-3 issue is open on any flow, and the calibration gap is at most 1 point (or the thresholds have risen by it).

## Evidence

- `docs/qa/0.4-walks/personas/<persona>.md`: the persona, its flows, each run's step records with links to captures, the scores, the confusion pass.
- `docs/qa/0.4-walks/personas/issues.md`: every issue in the user's words, with personas, flows, evidence, heuristic, kind, the three severities and their mean, how many runs saw it, the fix and its release.
- Captures under `docs/qa/0.4-walks/personas/captures/`, opened by eye, from generic fixtures or the lab only.

## The report (for the maintainer)

1. A verdict per persona: ready, ready with fixes, or not ready, and the one reason.
2. The score grid: persona × flow, with success, median ease, value, feeling and behaviour flags against the bar.
3. The ten worst issues, ranked by severity, then reach (personas × flow weight), then how many runs saw it; each with before and after captures, its status and its release.
4. What was fixed during the walks, and the re-walks.
5. Calibration: the agents against the maintainer, and recall of the maintainer's own complaints.
6. What this can't tell: whether strangers with other setups will adopt it, how it feels after weeks, how it works with assistive technology. After the announcement, 3–5 real users per persona.

## The run plan for 0.4.1 (the maintainer's token budget, 2026-11-07)

The account's weekly tokens were nearly gone, so on 0.4.1's release candidate each flow runs **once** (14 runs, a sonnet tester each), not three times. A flow runs a second time only where it fails or scores under 4 on any line. The Opus critic reviews only the failed and low runs. The bar and the scores are unchanged; what's lost is the variance three runs would show, and the report says so.

## When it runs

On each piece of 0.4 as it lands (the App and Account pages first, a short walk of the flows they touch), and whole on the release candidate with AJ-E3306.
