# AJ-E3011 newcomer walk, before the redesign: run 1 (2026-11-15)

- **Build:** release/0.4.6 at 4528771d2, the P10 lab (`scripts/qa/lab_scenario.sh setup P10`, not imported) through the lab write-mode bridge in the built-in browser, 1280×800.
- **Tester:** a fresh Sonnet agent, browser tools only (firewall checked: 60 calls, all `mcp__Claude_Browser__*`). It plays the persona "Sam", who has never heard of pools, agents, roles, leads, reviewers, observers or permission levels.
- **Goal:** make the two assistants on "web" work together, one in charge and one checking, and decide what they may do without asking, by hand, with no template.
- **Method limit:** the Agent tool gives every subagent the harness's project context, so the persona's ignorance is enforced by the brief, not by isolation.

## Path and scores

| # | Step | Worked | Value | Clarity | Feeling | Key question ("do I know what this does and who it affects?") |
|---|---|---|---|---|---|---|
| 1 | Pools › New pool | 4 | 3 | 3 | 3 | "Pools" no (guessed "a group") |
| 2 | "Set up by hand on the canvas" | 5 | 4 | 2 | 3 | the link is small grey text; the banner "Let agents … work together? Yes, let them / Not now — Asked once, for every pool": **no** (who it affects, what agents are, whether they see my code) |
| 3 | Choose projects › These projects › web | 5 | 4 | 4 | 4 | yes; the "Add a project" list's first row is itself "Add a project" and does nothing |
| 4 | Add both Apps from "Add a session" | 5 | 4 | 3 | 4 | "Add" yes; "Member" badge **no**; "none open now · new ones join" surprised: a pool of Apps starts empty |
| 5 | Codex card › Role › Reviewer | 5 | 4 | 4 | 4 | "Reviewer" guessed as "checks the work"; nothing on screen says what a role does; "Member", "Observer" **no** |
| 6 | "What its sessions may do": The pool's (Work together) / Read / Work together / Full → Read | 3 | 3 | 2 | 2 | **no**: "Read", "Work together", "Full" unexplained; nothing ties them to "without asking me"; unclear whether it is Codex's or the pool's |
| 7 | Make Claude the lead | 1 | 2 | 1 | 2 | **no**: "Lead is one session; pick one of this App's sessions to lead" with no session open and no picker; the canvas's step 3 ("Pick a lead and how they talk") had gone once members were added. Gave up here |

**Result:** F (incomplete end state: no lead; who may talk to whom never found). Ease 3/7, value 3/7, problem solved: partly. Feeling −1 ("unsure, patient, blocked").

**Words not understood:** pool, agents, lead, reviewer (guessed), observer, member, the App badge, session vs App, "Work together" (as a level and as a setting name), Full, Read, Tidy up, "its own level", "Take out of the pool", "new ones join".

**What the tester thinks the pool does:** Claude a plain member at the pool's Work together; Codex the reviewer at Read; nobody leads; who may talk to whom unknown; whether Codex will review Claude's work unknown.

## Opus verification (2026-11-15)

Checked against the lab's screen after the run (the pool "New pool", scope web): the banner's text, both App cards ("none open now · new ones join", Codex "Reviewer", "its own level"), the inspector headed "Member rule" with the App's name under it, Role "Member | Reviewer | Observer" with no explanation, "Lead is one session; pick one of this App's sessions to lead." with no picker, "What its sessions may do: The pool's (Work together)", and the selected card marked only by a dashed border — all as the tester said. **Confirmed.**

Not confirmed as a product finding: "1–2 s to reflect clicks". The Browser pane was hidden during the run, which throttles a page's frames; latency is measured on the Dev window instead (AJ-E3011's gates).

## Findings → the redesign (AJ-E3011 / AJ-E3012)

1. **Severity 3: the lead is a dead end** with Apps and no open session. The lead must be choosable as an App ("whichever session of Claude Code starts first leads") or deferred plainly, with an inline picker when sessions exist (R6).
2. **Severity 3: what members may do on their own is unexplained** (Read / Work together / Full) and not tied to "asking you" (R5).
3. **Severity 3: roles are unexplained** (R6: role cards with an icon, one line, who holds it).
4. **Severity 2: the agent-tools consent banner** reads as a global, unexplained switch above the work (R8: consequence and example; place it where it applies).
5. **Severity 2: the order disappears.** The three steps vanish once members are added, and talk rules were never found (R1: a flow in order).
6. **Severity 2: the manual path is a small grey link** beside the templates (R1–R3).
7. **Severity 2 (AJ-E3012): the inspector's heading is "Member rule"** rather than what is selected; the selection is a dashed border.
8. **Severity 1: the "Add a project" list's first row** repeats its label and does nothing.
