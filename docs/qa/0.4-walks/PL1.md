# PL1 Persona lab: what runs, and what each App holds — walk sheet

Generated from `docs/qa/scenarios/` by `scripts/qa/scenario.py sheet PL1`: the steps and expectations are the automation's own.

- **Validates:** D-100, D-102, D-84
- **Exercises:** persona walk (AJ-E3307), Open now, Inactive apps, out of room, the Codex app and Codex CLI, scripts' runs
- **Runs on:** lab; layers L4
- **Build:** _(version and commit walked)_ · **Walked by / on:** _(who, date)_

Pass: every step does what it promised; nothing under 4; feedback ≤ 100 ms; nothing cut off or overlapping. A failure is a finding on its task, and the whole scenario is walked again from the start.

| # | Do | Expect | Seen | ms (feedback / done) | Works | Speed | Clarity | Layout | Captures (light · dark) |
|---|---|---|---|---|---|---|---|---|---|
| 1 | Group the rail by App; unfold Open now | Open now holds exactly Claude app · personal and Claude app · work; nothing of the lab's is under Running, not added |  |  |  |  |  |  |  |
| 2 | Look at Claude app · personal | Its header reads nothing open, with no chevron and no row under it; no Claude Code CLI · personal App anywhere; its three claude -p runs are in History only |  |  |  |  |  |  |  |
| 3 | Unfold Claude app · work | Its last 3 sessions, then Show all 5; its header carries the Out mark with when it resets |  |  |  |  |  |  |  |
| 4 | Look under Inactive apps | Claude app · client (2 sessions, Out), Codex app (4 sessions), Claude Code CLI · Personal · terminal (4 sessions) and Codex CLI · default (3 sessions); none says Running |  |  |  |  |  |  |  |
| 5 | Unfold Codex app, then Codex CLI · default | The Codex app's are the 4 threads it started, Codex CLI's the 3 a terminal started; neither lists the codex exec run (History has it) |  |  |  |  |  |  |  |
| 6 | Open Home's apps, then Accounts | The Out mark on work's and client's tiles and on Work's and Client's Apps rows; Personal and ChatGPT have none |  |  |  |  |  |  |  |
| 7 | Run scripts/qa/presence_check.py --pid <the build> --home <the lab> | PASS: Open now is exactly the running Apps, each unfolds to its own records, every Out Account is marked, the Codex counts are the store's |  |  |  |  |  |  |  |

## Findings

_(each with its task, fix commit and the re-walk's date)_
