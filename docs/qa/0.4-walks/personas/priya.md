# Priya — the consultant with client accounts kept apart

The brief a tester gets is the **Brief** and one flow's **Goal**, nothing else. Sources are named per line; `[assumption]` marks what isn't sourced.

## Brief

- **Who:** 38, runs a six-person agency with four clients; she's the one clients audit. (07-personas §3)
- **Setup:** an agency Claude account plus a separate account per client; one repo per client; 6–14 sessions a day. (07-personas §3)
- **Goals (outcomes):** never run a client's work on another client's account; prove which account touched which repo; keep one client's work out of another's sight. (07-personas §3 JTBD)
- **Daily triggers:** a client standup; starting work for a client; a screen share; the monthly invoice. (07-personas §3 day-in-life)
- **What she knows:** git, the terminal, Claude Code; less patience for setup. Her words: "client", "account", "repo", "project", "billing", "who did what". (07-personas §3)
- **Frustrations:** launching on the wrong account because the labels look alike; another client's session visible while sharing her screen; working out per-client usage in a spreadsheet. (07-personas §3 pains 1–3, 5)
- **Doesn't care about:** running ten things at once. (07-personas §3) `[assumption]`
- **Today, without AI Juggler:** a separate macOS user per big client, a menu-bar switcher, usage scripts per config folder, a spreadsheet. (07-personas §3 workarounds)

## Flows

| # | Start state | Goal, in her words | Success, observable | Abandon if |
|---|---|---|---|---|
| R1 First run | First open; an agency account and two client accounts on the Mac (lab) | "Set this up so I can see each client's account separately." | Each account listed by its own name | She can't tell the accounts apart |
| R2 Right account for the client | Client A's repo; accounts with similar names | "Start work on Client A's repo — on Client A's account, nothing else." | A session starts in that repo on that account, and she can see which account it used | She isn't sure which account it used |
| R3 Who touched this repo | A week of sessions over three repos and three accounts | "Client B asks what AI did on their repo this week, and on which account." | She sees that repo's sessions with their accounts | 3 minutes without an answer |
| R4 Keep a client apart | A client project | "Make sure nothing from Client A's work can reach the others." | Client A's sessions are kept to themselves, and she can see that they are | She can't find a way, or can't tell it worked |
