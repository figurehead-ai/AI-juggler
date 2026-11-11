# PD4 Persona Devon: the laptop is struggling — walk sheet

Generated from `docs/qa/scenarios/` by `scripts/qa/scenario.py sheet PD4`: the steps and expectations are the automation's own.

- **Validates:** AJ-E504, AJ-E2401, D-25
- **Exercises:** persona walk (AJ-E3307), leftover processes, This Mac, Cleanup
- **Runs on:** lab; layers L4
- **Build:** _(version and commit walked)_ · **Walked by / on:** _(who, date)_

Pass: every step does what it promised; nothing under 4; feedback ≤ 100 ms; nothing cut off or overlapping. A failure is a finding on its task, and the whole scenario is walked again from the start.

| # | Do | Expect | Seen | ms (feedback / done) | Works | Speed | Clarity | Layout | Captures (light · dark) |
|---|---|---|---|---|---|---|---|---|---|
| 1 | Something's eating my laptop. Find it and stop what's left over, without killing live work. | This Mac (or Cleanup) lists `sleep 1800` as still running after its session (billing, ended) closed, apart from the two working sessions |  |  |  |  |  |  |  |
| 2 | Stop the leftover. | It stops and leaves the list; api-tests and web-refactor are still working |  |  |  |  |  |  |  |

## Findings

_(each with its task, fix commit and the re-walk's date)_
