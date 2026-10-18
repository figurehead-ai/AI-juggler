# Evidence manifest — AJ-E1003

Task: `docs/tasks/E10-derived-workflows-and-beta.md` AJ-E1003 (quota
reset queue and parked continuation). 2026-10-18, reviewer claude-opus.

## How it works

A quota-blocked Instance offers **Park until reset**. Parking stages its
portable pack, through the same redacted pack writer as a standalone
export (an unset secret policy blocks, never a default). It records the
reset the Instance waits for, taken from the quota engine: the latest
reset among the Account's exhausted windows. It also records how far
that reset can be trusted (`measured`, `estimated` or `unknown`).

Nothing resumes on its own. The reset is checked inside the existing
30 s alert/quota poll; there is no timer of its own. At the reset exactly one offer is
raised and logged (`parked/offers.jsonl`). It stays until acted on:
- **Resume** continues the original native session on the original
  Account (D-05), with the AJ-E1001 identity check first.
- **Remind me in 30 min** snoozes once; on the last offer that button
  becomes **Stop reminding me**, and the Instance then stays parked
  quietly.

Only a Measured reset gets a countdown. An Estimated one says so and
offers **Check now**.

AJ-E611's earlier reset-queue table (a bare passive timer that nothing
in the app queues into) stays for its own tests. Parking is the
user-facing reset queue now.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| A quota-blocked Instance offers "Park until reset": the pack is staged, the reset recorded from the quota engine, and a visible Parked state | Req.1 | `parked_reset :: a_parked_instance_is_offered_back_once…`: `PACK.md` staged, the Measured reset recorded, no offer yet. `only_a_quota_blocked_instance_parks…`: a Running Instance is refused. The rail shows `⏸ Parked 2h 13m` (full words in its label); the inspector's park card shows the same | PASS | 2026-10-18 | claude-opus |
| At reset: notify, never auto-launch; one-click resume of the original (D-05); dismissal snoozes once, then no repeated nagging | Req.2 | Before the reset there is no offer and nothing launches. At the reset, one offer appears (repeat polls show it but never raise it again — **the log has exactly one line**). Resume launches `--resume native-7` under the same Account. `not_now_snoozes_once_then_stays_parked_quietly`: one snooze, then a final offer, then silence (2 log lines in all); the Instance is still parked | PASS | 2026-10-18 | claude-opus |
| Parked Instances in the rail and Inspector with the reset countdown as text, sortable by reset time in triage | Req.3 | Countdown text on the rail and in the inspector (vitest `parked.test.tsx`); `InstanceInfo.parked_resets_at_ms` is the sort key the AJ-E1005 triage uses | PASS (triage sorting with AJ-E1005) | 2026-10-18 | claude-opus |
| Estimated quota: the copy says so and offers "check now" instead of a false countdown | Req.4 | `only_a_quota_blocked_instance_parks_and_an_estimated_reset_has_no_countdown` (no Measured sample → not `measured`; Check now re-reads the engine); vitest: no "resets in" text when estimated, and the Check now button appears | PASS | 2026-10-18 | claude-opus |
| Park < 2 s including pack staging for 500 turns; no extra timer under 60 s | perf | **23 ms** for a 500-turn transcript. Offers are evaluated in the existing ≥ 30 s alert poll | PASS | 2026-10-18 | claude-opus |
| Simulated reset fires exactly one notification; Estimated shows no countdown | quality gate | As above | PASS | 2026-10-18 | claude-opus |
| Visual: the Parked row, the reset notification and the resume offer — 1280×800, light/dark; the countdown as text; the one-click action keyboard reachable; dismiss and snooze distinct | visual / a11y | Gallery `53-parked` (rail-and-offer, inspector). Playwright `parked — row and reset offer — axe 0` (both themes: Resume, then Tab reaches the snooze). The snooze ("Remind me in 30 min") and the final dismissal ("Stop reminding me") are different words | PASS | 2026-10-18 | claude-opus |
| Dogfood overnight run (W1) | evidence | `docs/qa/dogfood-backlog.md` | DEFERRED-TO-DOGFOOD | 2026-10-18 | claude-opus |

## Test counts

- `cargo test -p juggler-app --test parked_reset`: 3/3.
- vitest `parked.test.tsx` 4/4. Playwright `53 parked` 2, `parked … axe 0` 2.
