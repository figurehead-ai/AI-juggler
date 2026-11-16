# AJ-E3013 lab proof on aij-test, run 4 (2026-11-16)

AI Juggler Dev built from e8f0e2061 on the real HOME (condition 7b), the sanctioned profile aij-test, and the Lab App as the CLI member, in `~/lab/web`. A Sonnet driver (`floor_drive` on Dev's pid, `app_drive` with AIJ_APP_PROFILE=aij-test); Opus checked each verdict against its captures. The captures stay out of the repo: Dev on the real HOME shows the maintainer's own sessions in its rail. This run proves the fixes since run 3 (`proof-aijtest-run2-3.md`).

| Step | Verdict | Evidence |
|---|---|---|
| D0: snapshot before | PASS | settings.json `ca3d163b…`; no `skills/` |
| D1: the connector from Who's in | PASS | The aij-test row said "Watched: can't talk in the pool yet" with Add on the row; "Show the files" opened and listed the two files; after Add only `skills/ai-juggler/.claude-plugin/plugin.json` and `skills/ai-juggler/skills/pool-tasks/SKILL.md` existed; settings.json and .claude.json's key names unchanged |
| D2: the page follows a new session | PASS | The pool's diagram, open since before aij-test started and never reloaded: the new Code session showed as its own row under "Claude app · aij-test" with "1 open now" within 3 s, and in Messages' To list. whoami (typed at once) named the pool "aij46 run4" and the place "Claude app" on the aij-test profile, with no refusal |
| D3 (A6): messages and a task claim | PASS | "hello from lab" read and answered "received" (Messages shows both). The Lab session created "lab task"; after aij-test claimed it, Tasks reads "In progress with AI Juggler pool whoami result, running now", "took it · now · running", with Open its session and no "Free it" (run 3 showed "a session that ended") |
| D4: lines and a per-line rule | PASS | Two lines join the Lab session's node and the aij-test session's row, "1 message" each way. Blocked set on the Lab → aij-test line itself (the inspector: "This line has its own rule"): the Lab session got "This pool doesn't let you message that member." for a direct send and a pool-wide one, and nothing reached aij-test; the reverse line still sent ("reply from aij-test" in Messages). Set back to Allowed |
| D5 (D-119 item 6): pool-wide to open members only | PASS (what the screens show) | "hello everyone" from the Lab session: Messages names one recipient, the open aij-test session ("Waiting in its inbox"), none of the pool's ~38 ended web sessions; the Lab agent was told only one member was open. The ended sessions' inboxes weren't opened (the bridge test `messages_open_members` covers that copies aren't made) |
| D6: open follows the process | PASS | After aij-test quit, the App's card read "No session open yet" and the session's row was gone within 2 s, the page not reloaded (run 3: "1 open now" stayed) |
| D7 (A10): a clean Remove | PASS | The preview named only the two files; `skills/` gone; settings.json byte-identical to D0; .claude.json rewritten by the Claude app with the same key names as D0 |

**Clean-up:** the Lab session stopped, the pool deleted, Dev and aij-test quit by pid; no juggler-agent and no lab `claude` process left; aij-test has no `skills/`; `~/lab/web` unchanged. The installed AI Juggler wasn't touched.

**Found and fixed in this run:** a message sent without a subject took its body's first 60 characters cut mid-word ("…titled 'la"); it is now cut at a word and marked "…" (`messaging::subject_from`).

## AJ-E3013's live proofs, together

Runs 1–4 on aij-test prove, on the real Claude app: exact consent and Add (A2, A3), the session's own registry and child process (A4), attestation from the first call (A5), messages, a task claim and per-line talk rules (A6), the idle line's words (A7), waking at Full and by a person's Deliver (A9), exact and all-or-nothing Remove (A10, A11), and the pool page following sessions live. Codex app threads (C2) wait for the maintainer's lab Codex sign-in in the 0.5 sitting.
