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

## Runs

### M3 Room left — run 1 (pilot), 2026-11-06

A fresh tester agent with only the brief and the goal, browser tools only (transcript checked: 18 screen actions and one page load, no file, search or shell; no leaked words in its report). Lab `PM3` (Work 95% used, Personal 30%, ChatGPT not signed in; 60 ended sessions), 0.4.1 Dev at 5a9b21ee5, the Browser pane at 800×600.

| Step | Did | Saw | Met |
|---|---|---|---|
| 1 | Wheel over the right-hand Runway panel | The page didn't scroll | no |
| 2 | Wheel over the main column | Runway by account: Work 5% left, Personal 70% left, both "resets at 10:42 PM"; ChatGPT not measured | yes |
| 3 | New session | The dialog: "Start in shop on Claude Code · Work · work", a warning that Work has 5% left, "Use Claude Code · Personal · personal (70% left)" | yes (seemed slow) |
| 4 | Use Claude Code · Personal · personal | The dialog moved to Personal, the warning gone | yes (seemed slow) |
| 5 | Start in shop | The session's page: Claude Code · Personal · personal, project shop | partly (the terminal said "Reconnecting") |

**Scores:** success S · ease 6 · value 6 · feeling +1 (in control, relieved, slow) · solved: partly-to-yes · confusion: Home top 1, runway 0, New session 1, session page 2.

**Triage (Builder, by hand):**
- *Home leads with rows of "tokens: Unavailable · cost: Unavailable"*: the app's. Fixed: a row says its tokens when known, else nothing (58ea681ec).
- *2–3 s with no feedback on New session, the switch and Start*: not reproduced. Measured headless on the same lab: the dialog in 86–107 ms, the switch in 36–43 ms, Start's button "Starting…" and disabled within 50 ms. The pause is the Browser pane's own screenshot lag (setup); later runs are told it.
- *"Reconnecting" over a black terminal*: setup (the lab bridge doesn't serve the terminal's attach).
- *Both accounts reset at the same minute*: setup (the seeder gives each account the same window).
- *The wheel over the Runway panel didn't scroll the page*: to check by hand (2, minor).
- *"Change app" when she meant the account*: wording, to the maintainer with the scores (clarity 1).

Runs 2 and 3 of M3, the critic, and the other flows follow.
