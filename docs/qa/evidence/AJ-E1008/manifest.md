# Evidence manifest — AJ-E1008

Task: AJ-E1008 (provider outage failover day, `[inferred — falsify
first]`). Status: **REJECTED (D-27, 2026-10-20)**. Real provider-side
incidents are now logged from the status pages (`outage-log.md`): about
3 whole-tool outages per tool in 8 weeks, 1 to 2½ hours where stated,
never both tools on the same day. No user has reported project-level
pain, and moving a session by hand is 5 clicks. No detector or failover
flow is built; the log stays open through beta, and 3 user-reported
events with project-level pain reopen it. 2026-10-18, updated
2026-10-20, reviewer claude-opus.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Real outage events are logged during beta (detection source, duration, workaround); nothing is built on speculation | Req.1 | `outage-log.md`: the user-report table (no reports yet) and, new, the provider-side incidents from status.claude.com and status.openai.com (6 whole-tool incidents, 2026-08-24 to 2026-10-20, with impact windows where stated) | PASS (provider side) · user reports: none yet | 2026-10-20 | claude-opus |
| ≥ 3 events showing project-level pain open a build task | Req.2 | No event shows it: none is user-reported, the outages were 1–2½ h, the other tool was up each time, and moving by hand is 5 clicks per session (walked by hand) | NOT MET → REJECTED; the log stays open | 2026-10-20 | claude-opus |
| No auto-detection of an "outage" from a single failed request | Req.3 | No detector exists in the code; the guide says so | PASS | 2026-10-18 | claude-opus |
| The manual path is documented with honest steps | Req.4 | `docs/guide/when-a-provider-is-down.md`, re-walked against today's app (2026-10-20): Switch account… → destination → Continue ▸ → Continue ▸ → Switch ▸, then Resume the original conversation ▸ (the switch-back card, shown while the original account has room); and, new, the single-model case (`/model`, no move) | PASS | 2026-10-20 | claude-opus |
| The outcome is a new decision-log entry | evidence | D-27 in `docs/review/decision-log.md` | PASS | 2026-10-20 | claude-opus |
