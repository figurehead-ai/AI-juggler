# DW1 The digest, a burst of finishes, and a watch that fires — walk sheet

Generated from `docs/qa/scenarios/` by `scripts/qa/scenario.py sheet DW1`: the steps and expectations are the automation's own.

- **Validates:** D-132
- **Exercises:** the digest (AJ-E3712), a burst is one notification, a digest item opened, watches (AJ-E3814), a watch alert (AJ-E3817)
- **Runs on:** lab; layers L4
- **Build:** _(version and commit walked)_ · **Walked by / on:** _(who, date)_

Pass: every step does what it promised; nothing under 4; feedback ≤ 100 ms; nothing cut off or overlapping. A failure is a finding on its task, and the whole scenario is walked again from the start.

| # | Do | Expect | Seen | ms (feedback / done) | Works | Speed | Clarity | Layout | Captures (light · dark) |
|---|---|---|---|---|---|---|---|---|---|
| 1 | Settings › Notifications: turn on The digest (off by default); leave Watches you set on and quiet hours unset | Both switches read on; nothing else changed |  |  |  |  |  |  |  |
| 2 | New session three times within a minute: cc-work in web, cc-personal in api, cc-work in api | Each opens its session page with the stand-in at its prompt (Welcome to Claude Code (lab stand-in)); Home lists three working agents |  |  |  |  |  |  |  |
| 3 | In each of the three terminals type /exit and Return, all three within 30 seconds | Each session ends Finished (not Stopped or Failed); no notification yet, none per session _(measure: the three ends within 30 s of the first)_ |  |  |  |  |  |  |  |
| 4 | Wait about a minute after the first /exit | ONE notification for the burst (The digest: 3 agents finished), and one digest item on Home's feed naming the three sessions; never three notifications _(measure: first /exit to the notification ≈ 60 s)_ |  |  |  |  |  |  |  |
| 5 | Open the digest item (from the notification, then from Home's feed) | The notification opens Home with the digest item on its feed; each session name in the item opens that session's page |  |  |  |  |  |  |  |
| 6 | Watches › New watch: project web, Asks you, Always; save | Toast Watching.; the list reads Watching 1 active, the row Waiting · never told you |  |  |  |  |  |  |  |
| 7 | New session: cc-work in web; at its prompt type run the tests and Return | The stand-in asks (Bash: npm test); the session reads Asks you, Needs you counts it |  |  |  |  |  |  |  |
| 8 | Watch the notifications and the watch's row | One watch alert: the session in web asks you; the row says when it told you; opening the alert opens that session's page _(measure: the ask to the alert ≤ 10 s)_ |  |  |  |  |  |  |  |
| 9 | Answer 1 in its terminal, then type /exit | Back at its prompt, then Finished; the watch stays active (Always) and tells nothing more |  |  |  |  |  |  |  |

## Findings

_(each with its task, fix commit and the re-walk's date)_
