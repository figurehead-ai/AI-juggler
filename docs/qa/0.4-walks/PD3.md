# PD3 Persona Devon: find one of fifty — walk sheet

Generated from `docs/qa/scenarios/` by `scripts/qa/scenario.py sheet PD3`: the steps and expectations are the automation's own.

- **Validates:** D-62, AJ-E1536
- **Exercises:** persona walk (AJ-E3307), rail search, History search
- **Runs on:** lab; layers L4
- **Build:** _(version and commit walked)_ · **Walked by / on:** _(who, date)_

Pass: every step does what it promised; nothing under 4; feedback ≤ 100 ms; nothing cut off or overlapping. A failure is a finding on its task, and the whole scenario is walked again from the start.

| # | Do | Expect | Seen | ms (feedback / done) | Works | Speed | Clarity | Layout | Captures (light · dark) |
|---|---|---|---|---|---|---|---|---|---|
| 1 | Find the session where I was fixing the login flow in the api repo. | Searching the rail for “login” says no open or recent session matches and offers “Search all history for “login””; History opens with “login” in its search and lists “Fix the login flow in the api” (api, Claude Code · Work, 3 days ago) _(measure: search to the row ≤ 2 min)_ |  |  |  |  |  |  |  |
| 2 | Open it. | Its page opens: the api project, Claude Code · Work · work, ended, with its conversation |  |  |  |  |  |  |  |
| 3 | Search History for what was said: “cookie”. | The same session, with the matching words shown under it |  |  |  |  |  |  |  |

## Findings

_(each with its task, fix commit and the re-walk's date)_
