# Evidence manifest — AJ-E807

Task: `docs/tasks/E8-validation-visual-performance-quality.md` AJ-E807
(release sign-off: the Stable approval, recorded with its evidence).
Rewritten 2026-11-13 for D-109, reviewer claude-opus.

D-109 replaced the per-milestone gate files: a release's sign-off is the
maintainer's approval of the `stable` environment in `promote.yml`, and the
workflow's `record` job writes it down with `scripts/signoff_record.py`.

Evidence: the dry run [`signoff-rehearsal.txt`](signoff-rehearsal.txt)
(the self-tests, the rehearsal against a real past Promote run, the
refusal without a Stable approval, the timing) and the record it wrote,
[`signoff-rehearsal-0.4.4.md`](signoff-rehearsal-0.4.4.md) (the approver's
login replaced by `maintainer-login` before it was committed).

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| R1: the record lists every task of the release's scope with its state and a link to its evidence manifest; a task its manifest doesn't prove is never shown as proven | R1 | `release_scope.py check X.Y.Z --channel stable --json` writes each named task's state (proven only when the task is DONE and its manifest has a PASS line for every R<N>; else open, unproven, deferred with its decision, or unknown); the record renders one row per task with a manifest link pinned to the commit the scope was checked on. The rehearsal: 139 rows, 135 proven, 4 open. Self-tests: `release_scope.py --selftest` (the JSON states), `signoff_record.py --selftest` (the rows and links) | PASS | 2026-11-13 | claude-opus |
| R2: the record carries the gallery delta, the accessibility result, the measured performance table and the unresolved items | R2 | The record's "Review inputs": the run's accessibility gate (the promote job runs only after it passed on the commit), the gallery delta against the previous `stable/v*` tag (no Stable yet: 1974 captures, review whole), the AJ-E805 measured budgets at the scope commit, and every open, unproven and deferred line of the scope | PASS | 2026-11-13 | claude-opus |
| R3: beta-exit includes the fidelity matrix per pair (AJ-E808), zero false cleanup kills, A→B→A continuity and the Fleet security review | R3 | `docs/qa/beta-exit.md` criteria 1 (fidelity register, AJ-E808), 3 (cleanup safety), 2 (dogfood, continuity) and 6 (STRIDE review, AJ-E710); criterion 7 now names the D-109 record. AJ-E808 and AJ-E710 are in 0.4's scope, so a 0.4 Stable can't move without them proven | PASS | 2026-11-13 | claude-opus |
| R4: sign-off recorded with date, commit SHA and reviewer; what blocks Stable | R4 | The sign-off is the `stable` environment approval (GitHub records reviewer and time). `promote.yml`'s `record` job (after the channel moves; `actions: read`, `contents: write`, no new secret) reads the run's approvals (`/actions/runs/{id}/approvals`) and jobs, writes `signoff-X.Y.Z.md` (version, commit, reviewer, approval comment, approved at, recorded at, run), attaches it to the release and the run's summary. No approval of `stable` → no record, exit 1. What blocks Stable now: `release_scope.py check X.Y.Z --channel stable` in the check job, before the approval is asked (with D-24's rules, the a11y gate and the CHANGELOG check); unsigned gate files no longer block anything. Dry run: `signoff-rehearsal.txt` against Promote run 43413923798's real approval | PASS (test and dry run) · first live run PENDING (the first Stable) | 2026-11-13 | claude-opus |
| Quality gate: no record without a Stable approval; no unproven task shown as proven | gate | `signoff_record.py --selftest`: only another environment's approval, a rejection, or none each exit with "no approval of the `stable` environment"; the rehearsal with the default environment against a beta run refuses the same way | PASS | 2026-11-13 | claude-opus |
| Performance: record generation < 60 s | budget | about 3.5 s for a rehearsal, most of it the two API reads and the scope check (`signoff-rehearsal.txt`) | PASS | 2026-11-13 | claude-opus |

## The route the record takes

A pull request opened by the workflow isn't possible here: the repository's
Actions setting "Allow GitHub Actions to create and approve pull requests" is
off (`GET /repos/{owner}/{repo}/actions/permissions/workflow` →
`can_approve_pull_request_reviews: false`), and a pull request opened with
the workflow's token starts no CI, so it could never merge green. The
record is therefore attached to the release (`signoff-X.Y.Z.md`) and written
to the run's summary; `python3 scripts/signoff_record.py pull X.Y.Z` copies
it into `docs/qa/signoffs/X.Y.Z.md`, committed on the release branch. Nothing
pushes to `main`.

## Retired with D-109

- `scripts/gate_packet.py`, `docs/qa/gates/beta.json` and the `just
  gate-packet` / `gate-signed` recipes. `promote.yml` no longer runs
  `gate_packet.py signed`. `just evidence` still resolves every relative
  link in the history gate files.
- The six gate files keep their content with a "Replaced by D-109" note.
  The first packet's evidence stays here as history:
  [`packet-beta.html`](packet-beta.html),
  [`packet-beta-1280x800.png`](packet-beta-1280x800.png),
  [`rehearsal.txt`](rehearsal.txt).

## Status

IN-PROGRESS. R1–R4 pass by self-test and dry run. Left: the `record` job's
first run on GitHub, at the first Stable promotion (a 0.4.x); then pull its
record into `docs/qa/signoffs/`, add the live row here and close the task.
