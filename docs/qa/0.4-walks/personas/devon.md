# Devon — the staff engineer running ten sessions in parallel

The brief a tester gets is the **Brief** and one flow's **Goal**, nothing else. Sources are named per line; `[assumption]` marks what isn't sourced.

## Brief

- **Who:** 41, staff engineer at an 80-person startup; judges tools like infrastructure. (07-personas §2)
- **Setup:** employer-paid Claude (two accounts) and Codex; a big laptop; 10–20 agent sessions a day across four services. (07-personas §2)
- **Goals (outcomes):** triage agents like CI, not babysit them; see at a glance which of fifteen needs him; keep his machine from grinding. (07-personas §2 JTBD)
- **Daily triggers:** the morning after an overnight run; a burst of parallel starts; the laptop fan spinning; end of day with work still open. (07-personas §2 day-in-life)
- **What he knows:** everything about git, terminals, worktrees, process tables. His words: "session", "agent", "repo", "service", "stuck", "process", "my laptop". (07-personas §2)
- **Frustrations:** an agent that asked a question at 01:14 and waited seven hours; tab sprawl with no search; leftover processes eating memory. (07-personas §2 pains 1, 4, 6)
- **Doesn't care about:** cost per token (his employer pays); hand-holding. (07-personas §2 WTP)
- **Today, without AI Juggler:** tmux and worktrees, `ps` and notify scripts, Slack messages to himself as an inbox. (07-personas §2 workarounds)

## Flows

| # | Start state | Goal, in his words | Success, observable | Abandon if |
|---|---|---|---|---|
| D1 First run | First open, several tools and accounts on the Mac (lab) | "Point it at what I've got and show me everything that's running." | His accounts and apps listed; the open sessions shown | He has to configure each one by hand |
| D2 Morning triage | Overnight: some finished, one stuck, one waiting, many ended | "What happened overnight, and what needs me first?" | He opens the one that waits on him first, then the stuck one | He scrolls the full history to work it out |
| D3 Find one of fifty | Fifty-plus sessions over four projects | "Find the session where I was fixing the login flow in the api repo." | That session is open | 2 minutes without it |
| D4 The laptop is struggling | Sessions working the Mac hard; a leftover process from an ended one | "Something's eating my laptop. Find it and stop what's left over, without killing live work." | The leftover is stopped, the live sessions still running | He can't tell what's safe to stop |
| D5 Start a batch | Two repos ready | "Start sessions on api and web on whichever account has room." | Both start, on an account with room | It picks an account that's out |
