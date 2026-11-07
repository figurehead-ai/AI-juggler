# Maya — the solo developer on several subscriptions (primary, weight ×2)

The brief a tester gets is the **Brief** and one flow's **Goal**, nothing else. Sources are named per line; `[assumption]` marks what isn't sourced.

## Brief

- **Who:** 34, runs a small software business on her own; ships most days. (07-personas §1)
- **Setup:** two Claude subscriptions and a ChatGPT one with Codex; she works in the terminal and in Claude's desktop app; 4–8 coding sessions a day across 2–3 projects. (07-personas §1; the maintainer's own use: several Claude profiles in the desktop app's Code tab)
- **Goals (outcomes):** keep shipping when one subscription runs out mid-feature; know before she hits a wall; never have to re-explain work to a fresh session. (07-personas §1 JTBD)
- **Daily triggers:** a session stops because its account is out; one sits waiting for her answer while she's in another window; at night she wonders what the second subscription is worth. (07-personas §1 day-in-life)
- **What she knows:** git, the terminal, Claude Code and Codex well. Her words: "account", "subscription", "session", "limit", "project", "my other Claude". (07-personas §1; the maintainer's reports)
- **Frustrations:** a limit she didn't see coming; a session that waited 25 minutes for her and she never knew; a hand-off that forgot what was agreed in the morning. (07-personas §1 pains 1–3)
- **Doesn't care about:** charts for their own sake; team features. (07-personas §1, anti-features) `[assumption]` on team features
- **Today, without AI Juggler:** shell aliases per account, a cron'd usage script, sticky notes with reset times, copying files by hand. (07-personas §1 workarounds)

## Flows

| # | Start state | Goal, in her words | Success, observable | Abandon if |
|---|---|---|---|---|
| M1 First run | AI Juggler just opened for the first time on her Mac (lab: an empty data folder; her tools' folders seeded) | "I just installed this. Set it up so it knows my accounts and shows what I've got running." | Her accounts and the apps she uses are listed, signed in, without her typing a path | 10 minutes without seeing her accounts |
| M2 What needs me | Several sessions open; one waits for an answer, one is working | "Something's waiting on me somewhere. Find it and get to it." | The waiting session is open in front of her | She can't tell which one waits after 2 minutes |
| M3 Room left | Two Claude accounts, one nearly out | "Before I start the next feature: which account has room, and when does the tired one come back?" | She can say which has room and the reset time, and starts on the right one | She ends up guessing |
| M4 Out mid-feature | A session working on an account that's just run out | "My account just ran out in the middle of this. Keep going on the other one without losing what we've done." | The same conversation carries on in another app or account, its history there | The work restarts from scratch, or she can't tell what came along |
| M5 Back after reset | The first account has room again | "The first account's back. Move my work back to it." | The session continues on the first account | — |
