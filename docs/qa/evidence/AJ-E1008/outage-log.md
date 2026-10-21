# AJ-E1008 outage event log

Real Provider outages during beta, as users report them through AJ-E809.
AI Juggler has no outage detector, and none will be built without a
documented signal source and a false-positive analysis (Req.3).

## How to report one

Add a row, or ask the maintainer to add one. Leave out names, account
emails, paths and anything from your work.

| Date | Provider | How you noticed | How long | What you did | Instances affected | Did moving them one by one hurt? |
|---|---|---|---|---|---|---|

No user has reported an outage yet (2026-10-20).

## Provider-side incidents (from the status pages)

These are what the providers published, not user reports, so the
columns above stay empty: nobody has told us what they did or whether
moving sessions one by one hurt. Read from
[status.claude.com](https://status.claude.com/history) and
[status.openai.com](https://status.openai.com/history) on 2026-10-20.
Only incidents that took the coding tool down as a whole are listed;
single-model errors are counted below.

| Date | Tool | Status-page title | Impact window | Workaround the tools allow |
|---|---|---|---|---|
| 2026-08-28 | Claude Code | Elevated errors on Claude Code and Claude Cowork | not stated (updates over ≈ 3 h) | wait, or switch the session to another provider |
| 2026-08-31 | Claude Code | Degraded performance on claude.ai and Claude Code | 16:55–19:16 UTC (2 h 21 min) | same |
| 2026-09-03 | Codex | Elevated errors across ChatGPT and Codex | not stated | same |
| 2026-09-25 | Codex | Issues with Codex | not stated | same |
| 2026-09-29 | Claude Code | Elevated errors on claude.ai, Claude Code, Claude Cowork and the Claude API | 14:00–14:59 UTC (59 min) | same |
| 2026-10-06 | Codex | Elevated errors across ChatGPT, Codex, and the API | not stated | same |

What they show:

- **Frequency and length.** Over 8 weeks, each tool was down as a whole
  about 3 times. Where the window is stated, it lasted 1 to 2½ hours.
- **No overlap.** Claude Code and Codex were never down on the same
  day, so a session could always have moved to the other tool.
- **Most incidents are narrower.** One model erroring (for example
  "Elevated errors for Claude Opus 5.5", 2026-10-06; 9 such Claude
  incidents in the period) is fixed inside the tool with `/model`, with
  no move at all.
- **Moving by hand costs 5 clicks per session** in the app today, walked
  2026-10-20: Switch account… → a destination (its fidelity badge and
  "Doesn't carry" list shown) → Continue ▸ → Continue ▸ → Switch ▸.
  A project with three sessions takes 15 clicks, against a wait of an
  hour or two. Whether that hurts is exactly what nobody has reported.

## Decision rule

If 3 or more events show project-level pain, meaning moving Instances
one by one was the problem, a build task is opened. It covers a
project-level switch offer that reuses the E2 pack and switch machinery
per Instance, with per-Instance fidelity badges (D-11). Otherwise the
task closes as REJECTED with this log linked. The outcome goes into
`docs/review/decision-log.md`.

Until then, the supported path is the manual per-Instance switch:
`docs/guide/when-a-provider-is-down.md`.
