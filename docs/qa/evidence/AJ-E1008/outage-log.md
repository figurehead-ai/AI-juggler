# AJ-E1008 outage event log

Real Provider outages during beta, as users report them through AJ-E809.
AI Juggler has no outage detector, and none will be built without a
documented signal source and a false-positive analysis (Req.3).

## How to report one

Add a row, or ask the maintainer to add one. Leave out names, account
emails, paths and anything from your work.

| Date | Provider | How you noticed | How long | What you did | Instances affected | Did moving them one by one hurt? |
|---|---|---|---|---|---|---|

## Decision rule

If 3 or more events show project-level pain, meaning moving Instances
one by one was the problem, a build task is opened. It covers a
project-level switch offer that reuses the E2 pack and switch machinery
per Instance, with per-Instance fidelity badges (D-11). Otherwise the
task closes as REJECTED with this log linked. The outcome goes into
`docs/review/decision-log.md`.

Until then, the supported path is the manual per-Instance switch:
`docs/guide/when-a-provider-is-down.md`.
