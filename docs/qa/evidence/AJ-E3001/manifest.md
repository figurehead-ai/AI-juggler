# Evidence manifest — AJ-E3001

Task: `docs/tasks/E30-pools-redesign.md` AJ-E3001, pools from real use:
research, the model and a proposal for review. Research done 2026-10-30,
approved as D-62. Checked on 2026-11-06 (release/0.4.1). R4 was amended
on 2026-11-06: the wireframes must live in the repo beside their
approvals. Status: **DONE, 1 gap open** (agent self-check, 2026-11-06).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | `docs/research/pools-model.md` §1.1 lists 13 sources (vendor docs on agent teams, cross-session messaging, worktrees, Codex subagents; Anthropic's engineering posts; a public parallel-agent workflow). §1.3 writes 8 use cases, each with who, setup, pool and value. All six named ones are there: feature team (1), across repositories (2), a reviewer for everything (3), rules by account (4), a build Mac through Fleet (5), a sealed client project (6). Note: the maintainer's own projects appear only as his reported cases (§0, §3), not as read examples. |
| R2 | PASS | `pools-model.md` §2.1 (facts T1–T19, each with a file:line, a doc link or the 2026-10-26 lab), §2.2 (capability matrix per tool and runtime: see, message, tasks, be asked, bound), §2.3 (what it means for the user: "Watched only", Continue in AI Juggler, Claude's own channel per receiving session). |
| R3 | PASS | `pools-model.md` §4.1 scope, §4.2 member types and precedence, §4.4 roles and D-28's levels per member, §4.5 talk (allowed, ask first, blocked; per role pair, one pair overridable), §4.2–4.3 a session covered twice (most specific wins inside a pool; several pools), §4.7 today's pools moved with their ids, nothing lost. Approved as D-62 decisions 8–16 (`docs/review/decision-log.md`). |
| R4 | GAP | The creation flow is proposed in words (`pools-model.md` §5: the wizard's four steps, the manual setup, the add list with reasons). The amended text needs its wireframes in `docs/wireframes/` with approvals in `wireframe-approvals.json`. They are not there: that folder holds only the 2026-10-13 screens 01–27, and nothing in it names pools. The proposal page with the wireframes was handed over outside the repo (`pools-model.md` line 6). |
| R5 | PASS | D-62 (`docs/review/decision-log.md`, "The 0.4 designs are approved", 2026-10-30) records the maintainer's review and approval of all 24 recommendations, listed one by one (decisions 8–18 for pools). The build tasks AJ-E3002 to AJ-E3006 in `docs/tasks/E30-pools-redesign.md` are written from them. |

Performance: n/a (research). The budgets the build holds are stated in
`pools-model.md` §4.8.
