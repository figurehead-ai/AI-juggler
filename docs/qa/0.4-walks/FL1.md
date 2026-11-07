# FL1 The Floor tells a newcomer where to look — walk sheet

Generated from `docs/qa/scenarios/` by `scripts/qa/scenario.py sheet FL1`: the steps and expectations are the automation's own.

- **Validates:** D-68
- **Exercises:** the Floor, the Floor list, Settings › The Floor
- **Runs on:** lab; layers L4
- **Build:** _(version and commit walked)_ · **Walked by / on:** _(who, date)_

Pass: every step does what it promised; nothing under 4; feedback ≤ 100 ms; nothing cut off or overlapping. A failure is a finding on its task, and the whole scenario is walked again from the start.

| # | Do | Expect | Seen | ms (feedback / done) | Works | Speed | Clarity | Layout | Captures (light · dark) |
|---|---|---|---|---|---|---|---|---|---|
| 1 | Open the Floor | It opens on the floor with the App that most needs you |  |  |  |  |  |  |  |
| 2 | Find who waits on you, and answer them | The waiting session is found in seconds and answered |  |  |  |  |  |  |  |
| 3 | Spot a crash and an App working the Mac too hard | Both are seen and opened without reading a list |  |  |  |  |  |  |  |
| 4 | See a swarm of subagents | The helpers are seen beside their session |  |  |  |  |  |  |  |
| 5 | Find the Account running low | Its sign says so, and it opens |  |  |  |  |  |  |  |
| 6 | Add a new App and watch the floor make room | It gets an area; the others keep their places |  |  |  |  |  |  |  |
| 7 | Turn, tilt and reset the view | The view turns and tilts, and comes back |  |  |  |  |  |  |  |
| 8 | Change Apps per floor | Floors are made or merged; the areas keep their order |  |  |  |  |  |  |  |
| 9 | Re-create the office | It says what changes and what stays, then does it |  |  |  |  |  |  |  |
| 10 | Use only the Floor list | Every task above can be done from the list |  |  |  |  |  |  |  |

## Findings

_(each with its task, fix commit and the re-walk's date)_
