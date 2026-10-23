# Evidence manifest — AJ-E1523

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1523, the north-star metric: weekly juggling loops completed (P2).
Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR). 2026-10-23, claude-opus.

## What exists now

1. **A juggling loop is a switch away from a session at its usage limit that succeeds.** A session reached its limit, and its work went on, on another account or tool.
2. **Counted with no change to the schema.** `stats/schema.v1.json` is closed, and its byte-identical copy on the stats server enforces it. A new metric would mean a new server schema.
   - Instead, `switch.completed`'s existing `trigger: quota_wall` now means "a switch away from a session at its limit", however the switch screen was opened (`commands::loop_trigger`, at both switch starts: transplant and replay).
   - A loop is `switch.completed{trigger=quota_wall, outcome=success}`. It's weekly like every counter, rounded and identifier-free as the rest.
   - Before, a switch the user opened by hand from a session at its limit counted as `manual`, so the loop was undercounted.
3. **In the user's own stats:** with "Keep stats on this Mac only", the count shows in Settings ▸ Privacy's local report, like every metric.
4. **Documented in the public metrics document:** `docs/usage-stats.md` § "The one number we watch", generated from `scripts/gen_usage_stats_doc.py`.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 count a juggling loop in opt-in statistics and the user's own stats | `commands` `a_switch_from_a_session_at_its_limit_counts_as_a_juggling_loop`: a session at its limit is counted `quota_wall` whatever the screen's own reason (manual, suggestion, quota_wall); a session in any other state keeps its reason. Both switch starts use it. The local and shared modes count the same events (AJ-E1103). | PASS |
| Req.2 documented in the public metrics document | `docs/usage-stats.md` § "The one number we watch"; `just stats-doc` up to date | PASS |
| No schema change; the server's copy stays identical | `stats/schema.v1.json` untouched; `scripts/stats_schema_diff.sh` unaffected | PASS |
