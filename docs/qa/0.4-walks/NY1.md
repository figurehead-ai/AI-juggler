# NY1 Archive takes a row out of Needs you at once — walk sheet

Generated from `docs/qa/scenarios/` by `scripts/qa/scenario.py sheet NY1`: the steps and expectations are the automation's own.

- **Validates:** D-49
- **Exercises:** Archive, Unarchive, Needs you, the bell
- **Runs on:** lab, rc, live-claude; layers L4
- **Build:** _(version and commit walked)_ · **Walked by / on:** _(who, date)_

Pass: every step does what it promised; nothing under 4; feedback ≤ 100 ms; nothing cut off or overlapping. A failure is a finding on its task, and the whole scenario is walked again from the start.

| # | Do | Expect | Seen | ms (feedback / done) | Works | Speed | Clarity | Layout | Captures (light · dark) |
|---|---|---|---|---|---|---|---|---|---|
| 1 | Archive the session that stopped with an error (Untitled session · web), timing from the click | Its row leaves Needs you and the bell's count drops by one, at once _(measure: click to row gone and bell dropped: at once, at most one 5 s tick)_ |  |  |  |  |  |  |  |
| 2 | Unarchive it | Its row comes back to Needs you and the bell's count rises by one _(measure: at most one 5 s tick)_ |  |  |  |  |  |  |  |
| 3 | Archive, in AI Juggler, another app's conversation that finished and wasn't looked at (the Lab Claude App, 7b) | Its row leaves Needs you and the bell's count drops by one, at once _(measure: click to row gone: at once, at most one 5 s tick)_ |  |  |  |  |  |  |  |

## Findings

_(each with its task, fix commit and the re-walk's date)_
