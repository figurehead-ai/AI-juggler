# AJ-E3013 lab proof on aij-test, runs 2 and 3 (2026-11-16)

The real Claude app, the sanctioned test profile **aij-test**, and AI Juggler Dev on the real HOME (condition 7b), as in run 1 (`proof-aijtest-run1.md`). A Sonnet driver used `scripts/qa/floor_drive` on Dev's pid and `scripts/qa/app_drive` (AIJ_APP_PROFILE=aij-test). Opus checked each verdict against the driver's commands, outputs and captures. The captures stay out of the repo: Dev on the real HOME shows the maintainer's own sessions in its rail.

## Run 2 (Dev built from f7435d2d1)

| Step | Verdict | Evidence |
|---|---|---|
| B0: run 1's leftover pool deleted | PASS | "Delete pool…": the confirmation was fully in view (run 1's was clipped), "Keep it" focused; "Deleted aij46 scratch" |
| B1: snapshot before | PASS | settings.json `ca3d163b…`; no `skills/` |
| B2: Add the connector | PASS | only `skills/ai-juggler/.claude-plugin/plugin.json` and `skills/ai-juggler/skills/pool-tasks/SKILL.md` appeared; settings.json and .claude.json unchanged. "Show the files" didn't open on an accessibility press (fixed in round 7) |
| B3: a Code session, whoami at once | PARTIAL | The first call was attested (run 1's start race is fixed). But it named the default "No project" pool, "A session in another app", and Claude app without the profile |
| B4–B10 | NOT RUN | The session wasn't linked to its project, so it was neither a node nor a recipient |

**The cause** (connector builder, 6adf9ba30): the start-race fix let the first call arrive before AI Juggler's conversation index had read the session. The attestation took the project from that index only and kept "none" for as long as the session called. The fix reads the session's folder and title from the Claude app's own record (already read to confirm it; the folder must be an absolute path with no `..`, the title is one bounded line). A project is kept once found, "none" never is, and an App with no label is named by its profile folder. A test reproducing run 2 with no index fails on the old code.

The route note: the Claude app's native folder panel can't be driven, so the sessions run in aij-test's remembered `~/lab/web`, as in run 1.

## Run 3 (Dev built from 5795ee1ce)

| Step | Verdict | Evidence |
|---|---|---|
| C0–C2 | PASS | "Claude app · aij-test"; settings.json unchanged since run 2; the connector card read "added · joins at its next start" |
| C3: whoami at once | PASS | Registry entry claude-desktop with cwd `~/lab/web`; `juggler-agent --mcp --plugin` its child. The first call was attested in ~12 s with no prompt; it named the pool "aij46 run3", member via its App, and the place "Claude app · aij-test" |
| C4: node and recipient | PASS, after a reopen | The session is its own node under the aij-test App's card and its own recipient in Messages' To list. The page, open since before the session started, didn't show it until reopened (round 7, item 6) |
| C5 (A6): messages, a task, a Blocked rule | PARTIAL | "hello from lab" read and answered "received" (Messages shows both); the Lab session created "lab task"; aij-test claimed it, **but Tasks shows the taker as "An agent, whose session has ended"** (sent to the connector builder). No line was drawn between the two session nodes (round 7, item 7), so the Blocked rule was set role-wide (Member → Members): the Lab session got "This pool doesn't let you message that member." and nothing reached aij-test |
| C6 (A9, an agent at Full) | PASS | At "Work together and wake each other", with aij-test idle and nothing typed: the Lab member's message woke it. Its registry went busy within 8 s, and it read its inbox by itself (transcript: "Message from another session… read_inbox"). Run 1 failed this |
| C7 (A9, a person's Deliver) | PASS | At Work together: a Deliver from the person woke it within 4 s (D-119 item 5) |
| C8 (A7) | PASS | "reads messages on its next turn"; the inspector says a message sent with Deliver wakes it |
| C9 (A11) | PASS | With SKILL.md changed by one byte, Remove refused, named it and took nothing out (the listing unchanged); Remove anyway (Cancel focused) took `skills/ai-juggler` out whole; an unrelated `skills/aij-unrelated/` was untouched by Add and Remove. "Show the files" opened. The refusal's text still listed plugin.json under "Deletes" (round 7, item 8) |
| C10 (A10) | PASS for settings.json; PARTIAL for .claude.json | settings.json byte-identical. .claude.json's keys are all Claude's own; Claude had rewritten it between runs, so it was compared by key names only. The card's "1 open now" stayed after aij-test quit (round 7, item 6) |

**Clean-up:** the Lab session stopped, the pool deleted, Dev and aij-test quit by pid; no juggler-agent left; aij-test has no `skills/` folder. The installed AI Juggler wasn't touched.

**Also seen:** the Lab session sent "hello from lab" pool-wide, and a pool scoped to `web` holds about 35 old ended lab sessions, whose inboxes got copies.

## Where it stands

- A5 (attestation, at once), A9 (waking at Full and by Deliver), A10/A11 (exact and all-or-nothing removal): proven live.
- A6: messages and the Blocked rule are proven. The claim's taker shows as ended (a fix is in progress); it reruns with round 7.
- C2 (Codex app threads) waits for the maintainer's lab Codex sign-in in the 0.5 sitting.
