# Evidence manifest — AJ-E1008

Task: AJ-E1008 (provider outage failover day, `[inferred — falsify
first]`). Status: **IN-PROGRESS** — outage logging is set up and the
supported manual path is documented; the decision needs real beta
outage events. No detector or failover flow is built. 2026-10-18,
reviewer claude-opus.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Real outage events are logged during beta (detection source, duration, workaround); nothing is built on speculation | Req.1 | `outage-log.md`: the event table and how to report one | INSTRUMENT READY (events await beta) | 2026-10-18 | claude-opus |
| ≥ 3 events showing project-level pain open a build task | Req.2 | The decision rule is in the log, written before any events | PENDING EVIDENCE | 2026-10-18 | claude-opus |
| No auto-detection of an "outage" from a single failed request | Req.3 | No detector exists in the code; the guide says so | PASS | 2026-10-18 | claude-opus |
| Until graduation, the manual per-Instance switch is documented with honest steps | Req.4 | `docs/guide/when-a-provider-is-down.md`: the switch steps as they appear in the app (Continue on another account → destination → Switch ▸ → Switch back), and what the app does not do | PASS | 2026-10-18 | claude-opus |
| The outcome is a new decision-log entry | evidence | Written when the decision is made | PENDING | 2026-10-18 | claude-opus |
