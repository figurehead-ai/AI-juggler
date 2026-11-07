# Codex → Claude Code: subagents and file history, live (2026-11-07)

Switch lab, outside suite, journey J2 (`switch_lab_outside_live`, `AIJ_LAB_JOURNEYS=j2`), harness at f48fd95f1 + ba78cd501, run in the clean lab environment (`scripts/qa/lab_env.sh`). Codex 0.163.0 on the Mac's Codex home, used where it is; Claude Code 2.1.310 on the Lab CLI App's folder (AI Juggler Dev's own copy, ADR-0027 7b), with `AIJ_LAB_FILE_HISTORY=1` (Claude Code keeps file history in its headless turns, as the Claude app's sessions do). A new throwaway project with a git worktree. Run by a Sonnet agent against the written sheet; every value below checked against the raw report by Opus.

| Step | What happened | Result |
|---|---|---|
| J2.1 | A Codex task outside AI Juggler (`codex exec`) in a worktree: codename TERN, a plan (add, sub, mul), `calc.py` with a test, a fake key. Then Codex spawned a subagent (`spawn_agent`) to count `calc.py`'s lines: "LINES=2" | ok; 1 subagent thread on Codex (its rollout names the task's thread as its parent) |
| J2.2 | Switch Codex → Claude Code. Claude answered: "Codename TERN; open items: sub, mul; test method names must end with `_lab`; dummy key: REDACTED". It then added `sub` and its test | ok: transcript-resume, todos, rules-carry, the worktree, secret stripped, source read-only, tests pass |
| J2.2, beside | The handover names the subagent's work; no subagent thread reached Claude Code (no `subagents` folder for its session). Codex has no file history (no `file-history` in its home). Claude Code's file history for the session held 3 files after its own edits to `calc.py` and its test, made after the switch | subagents: doesn't carry; file history: none to carry, Claude's starts after the switch |
| J2.3 | Switch back Claude Code → Codex: "Codename: TERN; release: Quiet Harbor; open: mul" | ok: switch-back, todos, one file, the worktree, tests |

**Cells decided:** codex→claude-code `subagents` FAIL and `file-history` FAIL, each with a drop note (the register). The rest of the pair's cells (lab 14) are on the same versions and stay PASS.

**Snapshot:** the lab's own sessions and Codex's housekeeping only (the run's report, kept outside the repository, lists every path changed).
