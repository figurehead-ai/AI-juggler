# Evidence manifest — AJ-E1004

Task: `docs/tasks/E10-derived-workflows-and-beta.md` AJ-E1004 (overnight
and unattended run supervision). 2026-10-18, reviewer claude-opus.

## How it works

Every Instance status change is written to a journal (`instance_transitions`,
migration V25) inside the same storage call that changes the status:
`insert` records the first status, `update` records a change only when
the status actually differs. The supervisor's existing state machine (E4/E5)
stays the single source; the journal only records what it decided, so
there is no second state machine.

The morning summary (`overnight_summary`) reads that journal for the
window since the user last looked (setting `summary_seen_at_ms`; 12 h
before the first look). It groups every Instance that changed in the
window, plus anything still running, by severity:
crashed → needs you → still running → finished → ended. Each row carries
its timestamped transitions, tokens where measured (otherwise
"Unavailable"), cost "Unavailable" (no measured cost source, D-06/D-07),
and its leftover processes from the E5 cleanup inventory. There is no
composite health score.

On the home view the summary sits above the fleet-at-a-glance cards.
Each row is one button that opens its Instance. "Mark all seen" moves
the window forward.

The Stuck rule names the threshold it was judged against
(`threshold 5m`). A Provider can have its own no-output threshold through
an optional `providers` map in `heuristics.json`
(`docs/qa/e4-attention-fixtures.md`).

## Notifications (Req.3)

Instance state changes do not interrupt. They reach the user through the
rail badges, the polite screen-reader live region (`AttentionAnnouncer`,
WCAG 4.1.3; it is not a notification) and this summary.

The one interrupting surface for an Instance is the wrong-account dialog
(AJ-E1001), raised before a launch. Crashed is allowed to interrupt but
does not; that is the more conservative choice, and it is recorded here
rather than left unexplained. Quota alerts (AJ-E611) keep their own
batching and quiet hours.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Needs-attention states (Stuck, WaitingUser, QuotaBlocked, Crashed) reach the rail and Inspector; no output past a Provider-configured threshold flags Stuck with the threshold shown | Req.1 | The E4 heuristics already emit the states (badges, tab count `!5`, Inspector quality line). The Stuck reason now includes `threshold 5m`. `heuristics :: a_provider_threshold_overrides_the_global_one_and_is_shown`: 400 s of silence is not Stuck under a 15 min Provider threshold; past it, the reason reads `threshold 15m`. `state::tests::provider_thresholds_ride_alongside_the_global_ones` parses the `providers` map | PASS | 2026-10-18 | claude-opus |
| The morning summary aggregates finished, needs attention, errors, tokens/cost where measurable and leftover processes, each row linking to its Inspector | Req.2 | `overnight_summary :: every_state_lands_in_its_group…`: 7 fixture Instances, one per state, land in their groups in severity order. vitest `morning-summary.test.tsx`: headline counts, `cost: Unavailable`, measured tokens shown, "2 leftover processes", a row click opens its Instance, "Mark all seen" clears it, nothing renders when nothing happened | PASS | 2026-10-18 | claude-opus |
| Notifications batched into the summary; only Crashed and wrong-account may interrupt | Req.3 | See "Notifications" above: no Instance state change raises an interrupting notification; the wrong-account dialog is the only interruption | PASS | 2026-10-18 | claude-opus |
| Every transition timestamped in session history, so the summary is auditable, not reconstructed | Req.4 | The crashed row's history is `starting → running → failed`, each with its `at_ms`, read from the journal. `storage_roundtrip :: status_transitions_are_journalled_and_survive_a_reopen` | PASS | 2026-10-18 | claude-opus |
| No transition lost across an app restart (journal replay) | quality gate | Same test: after a restart every earlier transition is still there, and the start-up crash recovery is journalled too (the Instance whose process was gone gains `running → stopped`). Log: `summary-and-replay.log` (`[summary]` before, `[replay]` after) | PASS | 2026-10-18 | claude-opus |
| Summary < 2 s for 50 Instances; no new poll under 5 s | perf | **1.8 ms** for 50 Instances (`fifty_instances_summarise_inside_two_seconds`). The summary is read when the home view opens; journal writes ride the existing status updates. No new timer | PASS | 2026-10-18 | claude-opus |
| Visual: the morning summary and rail attention badges, 1440×900 and 1024×680, both themes; severity readable as text and icon; rows keyboard navigable | visual / a11y | Gallery `54-summary` (home, opened: 8 captures). Playwright `morning summary — axe 0` (both themes): a labelled region, Tab moves from "Mark all seen" through each row. The 1024×680 capture found a layout bug: a tall summary was clipped under the header and could not be scrolled to (the glance used `justify-center`). Fixed: it is centred when it fits and scrolls from the top when it does not | PASS | 2026-10-18 | claude-opus |
| Dogfood overnight run (with AJ-E1003) | evidence | `docs/qa/dogfood-backlog.md` | DEFERRED-TO-DOGFOOD | 2026-10-18 | claude-opus |

## Test counts

- `cargo test -p juggler-app --test overnight_summary`: 2/2; stable over
  5 runs after a test-only race was fixed. The race: the "seen" mark
  could land in the same millisecond as the start-up recovery
  transition. The test now looks later, as a person would.
- `cargo test -p juggler-supervisor --lib heuristics`: 10/10.
- `cargo test -p juggler-app --lib state::tests`: 1/1.
- vitest `morning-summary.test.tsx` 2/2. Playwright `54 summary` 4,
  `morning summary — axe 0` 2.
