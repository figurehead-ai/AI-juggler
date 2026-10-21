# When a tool is down

AI Juggler does not detect outages. One failed request isn't an outage,
and moving work on a false alarm does more harm than moving it yourself.
Check the tool's status page first:
[status.claude.com](https://status.claude.com) for Claude Code,
[status.openai.com](https://status.openai.com) for Codex.

## One model is failing

Most incidents are a single model erroring, for example "Elevated errors
for Claude Opus". The tool still works, so don't move the session. Switch
the model inside the tool instead: in Claude Code, `/model sonnet` (or
`/model` to pick one); in Codex, its model picker. Switch back the same
way once the status page says it's resolved.

## The whole tool is down

These usually last an hour or two. If you can wait, wait. If you need to
keep working, move each session yourself:

1. Open the session from the rail.
2. Click **Switch account…** at the top right of the session. It scrolls
   to **Continue on another account**, the destinations ranked for you.
   Each one shows its fidelity badge (for example "F1 · Beta") and what
   doesn't carry over. A destination on the other tool moves the work
   from Claude Code to Codex, or the other way round.
3. Pick a destination. The switch opens at **Target**: press
   **Continue ▸**, check **What's carried**, then **Continue ▸** again.
4. At **Confirm**, press **Switch ▸** (or Enter). The session is frozen,
   snapshotted, staged and checked before the new one starts. If a step
   fails, the switch tells you which step and why.
5. Repeat for each session you want to keep working on.

When the tool is back, open the moved session. Under **Continue on
another account** it offers **Resume the original conversation ▸**
(shown while the original account has room), which returns the work to
where it started.

There is no "move the whole project" button, and none is planned
(decision D-27). Outages have been short, and the other tool has been up
each time. If moving sessions one by one gets in your way, tell us: add a
row to `docs/qa/evidence/AJ-E1008/outage-log.md`, or ask the maintainer
to. Three such reports bring the question back.
