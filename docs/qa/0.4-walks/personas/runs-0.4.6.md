# 0.4 persona runs on 0.4.6 rc3a (AJ-E3307)

Build: 0.4.6 rc3a at 6c2f07f70 (AI Juggler Dev, a release build). Lab: PR2, which has two client repos (client-a, client-b), the Apps Client A, Client A (old) and Agency, eight ended sessions, and no pool beyond each project's own.

Each run used a fresh Sonnet tester with screen tools only: guarded presses, window shots and typed keys on the lab window's pid. The firewall check was clean on all three runs; the only commands were the screen tool, the tester's own shots and writing its log.

## R4 Keep a client apart (Priya): three runs

Goal, in her words: "Make sure nothing from Client A's work can reach the others." Success: Client A's sessions are kept to themselves, and she can see that they are.

| Run | Steps | Result | Ease /7 | Value /7 | Where it went |
|---|---|---|---|---|---|
| 1 | 7 | partly | 3 | 3 | Projects › client-a, then "Agent access" (the pool, "Work together"), then Accounts and the Client A account page. She found the proof of who touched what, and no way to enforce anything. |
| 2 | 4 | partly | 3 | 4 | The Client A account's Projects tab (it lists client-b too), then client-a's "Who did the work" (mixed accounts), then "Agent access" (the pool, about agents inside one repo). |
| 3 | 6 | not | 2 | 3 | Projects › client-a, "Agent access", the pool, Accounts, then the Client A "…" menu (Open its page, Rename account). |

Per-step scores (worked / value / clarity / feeling):
- Run 1: 2/2/4/3, 3/4/4/3, 1/1/2/2, 2/3/4/3, 1/1/4/3, 4/5/4/4, 1/1/5/2.
- Run 2: 3/3/3/2, 3/5/4/3, 1/2/2/2.
- Run 3: 4/3/5/4, 3/4/3/3, 2/2/2/2, 1/1/2/2, 1/2/3/2, 1/1/4/2.

**No run reached sealing.** The flow's path is "Keep it apart…" on the project, or a sealed pool; none of the three opened the project page's "…" menu, where "Keep it apart…" is. The redesigned New pool (D-126) was therefore not walked by this flow.

What every run said, in the testers' own words:
- **Her model is accounts and repos, not agents.** She wants "client-a only on Client A's account", and the proof of it for an audit. The product's "keep apart" seals a project's agents from other pools' agents: a different promise, offered in words she doesn't use ("pool", "lead", "agents").
- **The wording points the other way.** The project page's Agents card reads "on their own: Work together", and "Agent access" opens a pool page about sessions messaging each other.
- **The evidence shows mixing, with no verdict.** "Who did the work" and the account's Projects tab show Agency and Client A (old) on client-a, and Client A on client-b. Home says "All clear", with no flag and no fix.

Test-tool notes (not product findings):
- In run 3, a "…" menu ignored presses outside it. The tool's presses are accessibility presses, and an open menu makes the page under it inert; a mouse click outside, or Escape, closes it.
- The tool's scroll didn't move the pool panel.

## For the maintainer (R5: none is a bug, so nothing changes without your ruling)

1. **"Keep it apart" where she looks.** Show it on the project page's Agents card, as a line that reads "Kept apart: no · Keep it apart…" (sealed: "Kept apart: yes, its agents can't reach other pools"), not only in the "…" menu. Say it in her words: "Client A's agents can't see or message other clients' work."
2. **The Agents card's words.** "on their own: Work together" reads as the opposite of keeping a client apart. Proposal: lead with what's true of the project ("Its agents: they work together inside client-a") and keep the level's name second.
3. **The feature she asked for (a decision, not a fix):** a project tied to its client's accounts, a session started there on another account flagged, and a per-repo, per-day "which account touched it" export for audits. Today the data is there (Who did the work, the account's Projects tab), but there is no rule and no flag.
