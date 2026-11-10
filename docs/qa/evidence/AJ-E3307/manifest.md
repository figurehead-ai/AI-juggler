# Evidence manifest: AJ-E3307, persona-based validation

Task: `docs/tasks/E33-validated-by-use.md` AJ-E3307. The protocol is
`docs/qa/persona-validation.md`; the personas, runs and scores are under
`docs/qa/0.4-walks/personas/`. The 0.4.1 runs: one per flow (the run plan of
2026-11-07, the maintainer's token budget), a fresh sonnet tester each, screen
tools only. The native runs on rc29 come from the release session's walk
results (2026-11-10), checked by Opus.

| Req | Result | Evidence | Commit | Date |
|---|---|---|---|---|
| R1 personas built from evidence, each with goals, context, skill, frustrations and daily triggers, and their day-to-day flows written before any walk | PASS | `personas/maya.md` (×2 weight, flows M1–M5), `devon.md` (D1–D5), `priya.md` (four flows, first run to keeping a client apart): each Brief names who, setup, goals, daily triggers, what they know, frustrations, what they don't care about and today's way, sourced per line to `docs/review/deep/07-personas-and-journeys.md`, the maintainer's use and the research, `[assumption]` where not; `paths.md` the shortest paths. Written as the protocol's step 1, before the first run | a69333327 | 2026-11-06 |
| R2 a written, research-based protocol that guards against the tester knowing the product: a newcomer's walk from what the screen shows, never the code or docs; the expectation written before each step | PASS | `docs/qa/persona-validation.md`, from `docs/research/persona-validation.md`: Builder, Tester and Critic in separate contexts; a fresh tester per run with the browser tools only; the firewall (any file read, search, shell or code tool voids the run) and the leak lint; a step record with "expect" written before "action". The scoring gate aligned to the task (96b133391) | 8755dfb12, 96b133391 | 2026-11-02, 2026-11-10 |
| R3 each persona walked from first run, flow by flow, by hand in the built app on the throwaway lab, each step evidenced (screens before and after, expected, happened, time) | PARTIAL | **Walked in the built app (rc29, native window, lab):** the three first runs (Maya's M1, Devon's D1, Priya's first run), and M2, D2, D5. **Not proven:** M3, M4, M5, D3, D4 and Priya's three other flows were walked only in the design preview (`runs-0.4.1.md`), not in the built app; M2 steps 2–3 were setup-blocked on the lab bridge (no terminal). The rc29 runs' step records (screens, expectations, times) are not in the repo: `runs-0.4.1.md` stops at the lab runs of 2026-11-10 and still lists the first runs as "to come" | 8b668d16d, b517543dc | 2026-11-10 |
| R4 each step scored 1–5 on worked, value, clarity and feeling, each flow on whether the problem was solved, every score with a one-line reason | PARTIAL | **Scored per step:** M2, D2, D5 (re-scored from the lab runs' notes by a fresh critic, cef7c755f; table in `runs-0.4.1.md`), and the rc29 native runs: M2 solved (4–5, one 3), D2 partly, D5 partly, M1 partly, D1 not solved (a lab with no running sessions, and two misleading labels, fixed bb46b57d1, dcb7a5c9f), Priya's first run partly (the Rename bug, fixed 202236141). **Not proven:** M3–M5, D3, D4 and Priya's other three flows carry only the older 1–7 flow scores; the committed per-step table has no one-line reason per score; the rc29 scores aren't in the repo | cef7c755f | 2026-11-10 |
| R5 findings ranked by severity and reach; bugs fixed at once and walked again; anything under 4 reasoned, a better design proposed, and brought to the maintainer | PARTIAL | **Done:** the under-4 items and partly-solved flows went to the maintainer with designs: A–F approved in D-96 and G in its design note (2548659f0, f5c4843a5, 21d1a2315, 388a793e7); the second round's five items approved (D-97) and built (08199ac60, 4049d6c56, e4c192799, f1efccf9c, 4f088e9a7), verified on rc30. Bugs fixed at once: 202236141, b3243573c, 89d7ac576, 70b49190b, bb46b57d1, dcb7a5c9f, 462309d47, c299c500f, c4a03e321, 0edb8e078. **Not proven:** no ranked list of findings (the protocol's `personas/issues.md` with the three severities and reach doesn't exist); the re-walk of each fixed step by a new tester isn't recorded for the rc29 fixes | 34c031f1c, 904544566 | 2026-11-10 |
| R6 a report for the maintainer: personas, flows, the score table, the ranked findings with evidence, what was fixed, and what waits on a decision | NOT DONE | No report written. The protocol's report also has the calibration (step 7: the maintainer does two or three of the same tasks and scores them), which was not done in this release, so the agents' scores aren't calibrated | — | 2026-11-10 |

## Still to do for DONE

- R3: M3–M5, D3, D4 and Priya's other three flows in the built app; the rc29 runs' step records committed.
- R4: per-step 1–5 scores with a reason each, for every flow.
- R5: the ranked findings (`personas/issues.md`); the fixed steps re-walked by a new tester.
- R6: the report, with the maintainer's calibration runs.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] No captures here
- [x] No conversation content
