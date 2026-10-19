# Evidence manifest — AJ-E807

Task: `docs/tasks/E8-validation-visual-performance-quality.md` AJ-E807
(milestone gate verification and sign-off). 2026-10-19, reviewer
claude-opus.

Evidence: the beta gate packet [`gate-beta.md`](../../gate-beta.md)
(from [`gates/beta.json`](../../gates/beta.json)), its index page
[`packet-beta.html`](packet-beta.html) and capture
[`packet-beta-1280x800.png`](packet-beta-1280x800.png), and the
rehearsal [`rehearsal.txt`](rehearsal.txt).

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Req.1: every criterion lists evidence paths under `docs/qa/evidence/`; a criterion without a resolving path is FAIL or DEFERRED with a task ID, never PASS | Req.1 | `scripts/gate_packet.py build`: PASS only when every path resolves and every owning task is DONE-EVIDENCED; otherwise DEFERRED (open task IDs), or FAIL when tasks claim done without proof. `check` rejects a PASS row with no resolving link. Today's beta packet: 1 PASS · 9 DEFERRED · 0 FAIL | PASS | 2026-10-19 | claude-opus |
| Req.2: the review includes the gallery delta, the axe summary, the measured performance table and the unresolved-items register | Req.2 | Packet §2: gallery delta (since the last signed gate; none yet, so the whole gallery); the AJ-E804 summary lines; the AJ-E805 measured table, copied from its manifest; unresolved items, covering every unsigned gate, every task not DONE-EVIDENCED and every manifest with PENDING/PARTIAL/FAIL rows | PASS | 2026-10-19 | claude-opus |
| Req.3: beta-exit includes the fidelity matrix per pair (AJ-E808), zero false cleanup kills, A→B→A continuity and the Fleet security review | Req.3 | Criteria B1 (register), B3 (cleanup safety), B2 (dogfood incl. continuity), B6 (STRIDE review). Plus B4/B5/B7/B8/B9/B10 from `beta-exit.md` | PASS | 2026-10-19 | claude-opus |
| Req.4: sign-off recorded with date, commit SHA and reviewer; unsigned gates block the next step | Req.4 | Every gate file now has a Sign-off section (`gate-M0/M1/M3/E5/E7/beta`). `gate_packet.py signed` requires Decision APPROVED, a reviewer, an ISO date and a commit SHA. `promote.yml` runs it before any promotion to stable, so an unsigned beta gate keeps the stable channel closed (decision D-20, `docs/RELEASING.md` § Each release). The signatures themselves are the maintainer's | PASS (mechanism) · signatures PENDING (maintainer) | 2026-10-19 | claude-opus |
| Quality gate: a deliberately unlinked criterion in a rehearsal gate is caught | gate | `rehearsal.txt`: an unlinked criterion with a done task builds as FAIL. Hand-edited to PASS, the check exits 1. The unsigned beta gate is refused (exit 1) and a filled copy accepted (exit 0) | PASS | 2026-10-19 | claude-opus |
| Performance: packet generation < 60 s | budget | 0.06 s (`time python3 scripts/gate_packet.py build docs/qa/gates/beta.json`) | PASS | 2026-10-19 | claude-opus |
| Visual: the packet index at 1280x800; every row shows verdict and evidence without horizontal scrolling | visual | `packet-beta-1280x800.png`: 10 rows, each with verdict and evidence; `scrollWidth − innerWidth = 0` | PASS | 2026-10-19 | claude-opus |

## Found on the way

- **Gate files' relative links were never checked.** `evidence_check.sh`
  only resolved links written as `docs/qa/evidence/…`, but gate files
  link relatively (`evidence/AJ-E702/manifest.md`). `just evidence` now
  runs `gate_packet.py check` over every `docs/qa/gate-*.md`.
- **Gate E7 had two PASS rows with no evidence link** (rows 3 and 6 cited
  test names only). They now link the AJ-E708 and AJ-E706 manifests and
  the `50-cross-switch` gallery.
- **The M1 gate (and M0, M3, E5, E7) was never signed** (the handover
  note). Each now has a Sign-off section and appears under the beta
  packet's unresolved items until the maintainer signs it.

## Status

IN-PROGRESS. The mechanism is evidenced. What remains is the
maintainer's: sign M0, M1, M3, E5 and E7, then review and sign the beta
packet once its criteria are PASS.
