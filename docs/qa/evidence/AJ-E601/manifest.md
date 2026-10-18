# Evidence manifest — AJ-E601 (Sanctioned-source proof protocol for Provider billing & renewal data)

Task: `docs/tasks/E6-quota-cost-and-billing-research.md` AJ-E601.
Branch: `stage2/e511-e601-batch` (batched with AJ-E511; E601 itself
changes ZERO product code — its files are docs/research/ only).
2026-10-13, reviewer zcode-primary.

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 four grades; only the first two feed product (after Req.2-3) | protocol §1 | grades defined with bite-rules | `documented-sanctioned` / `local-observable` / `inferred` / `prohibited`; marketing pages = inferred; third-party arithmetic = inferred (grade follows the METHOD); web-UI-only fields are NOT sanctioned for us; the §7 table maps grades 1:1 onto UI states (no UI state the protocol cannot produce) | PASS | 2026-10-13 | zcode-primary |
| Req.2 capture template + live verification | protocol §2 + `evidence/billing/claude-statusline-20261013.md` | every field mandatory; a capture missing one is not evidence | template complete on the worked capture (binary 2.1.300, OS, ISO-8601-with-tz date, exact field paths, redacted excerpt, redaction rationale); protocol ≤400 lines (169) | PASS | 2026-10-13 | zcode-primary |
| Req.3 read-only classification; ToS-ambiguous → prohibited | protocol §3 | clause cited; silence = ambiguity | read-only defined as vendor-written local surfaces; read-write/ambiguous → prohibited with the clause quoted; "we never try it and see" | PASS | 2026-10-13 | zcode-primary |
| Req.4 the two-capture-or-changelog gate (D-12) | protocol §4 | never softened | `verified` = two captures ≥24 h apart OR one LIVE capture + vendor changelog entry; single capture = `candidate` ("proves the field existed once") | PASS | 2026-10-13 | zcode-primary |
| Req.5 quota vs renewal graded separately | protocol §5 + capture verdicts | per-class verdicts, never blanket | `Data class` on every capture row; the worked capture: quota verdict recorded, renewal `unknown` (D-12's expected default — user-entered dates remain the product path) | PASS | 2026-10-13 | zcode-primary |
| Req.6 demotion rule | protocol §6 | demote within one release cycle, never coerced | shape change → `unknown` in product within one cycle; no `?? fallback`, no shape-guessing parse; the paper trail (old capture, new capture, demotion date) recorded | PASS | 2026-10-13 | zcode-primary |
| PASS THRESHOLD: second agent classifies blindly to the same grade | blind subagent review (protocol + capture only) | same classification | the blind reviewer AGREED on grade (`documented-sanctioned`), read-only, data class, renewal-unknown — and DISAGREED on verification: the capture machine's unauthenticated binary emits no `rate_limits`, so the live-observation leg of §4 did not exist; the initial `verified` claim was wrong. The capture and the worked example were honestly DOWNGRADED to `candidate / may-feed-product: no` (template gaps the reviewer also found — tz-less date, an unclassified `cost.total_cost_usd` path — fixed). The disagreement IS the protocol working; the gate governs over the worked example | PASS | 2026-10-13 | zcode-primary |
| Evidence hygiene | grep over `docs/research/evidence/billing/` | zero credential-marker hits | 0 hits (the README spells the markers without literals so the grep stays self-clean) | PASS | 2026-10-13 | zcode-primary |
| D-12 line-by-line review | decision-log.md:113-124 vs protocol | every clause implemented | "user-entered, optional" default → §5/§7 (`unknown` never inferred); "sanctioned, read-only, verified live" → §1/§3/§4; "unproven sources stay unknown" → candidate/prohibited produce no product state | PASS | 2026-10-13 | zcode-primary |
| Zero product code (E601's own diff) | file list | docs/research/ only | `billing-source-protocol.md`, `evidence/billing/README.md`, `evidence/billing/claude-statusline-20261013.md` — no crates/ or ui/ changes from E601 (the batch PR carries AJ-E511's code, separately evidenced) | PASS | 2026-10-13 | zcode-primary |

## Implementation-mapping

- `docs/research/billing-source-protocol.md` (new, 169 lines): the
  four grades + bite-rules, capture template, read-only law, the D-12
  verification gate, per-class grading, demotion rule, UI mapping,
  the worked Claude statusline example (honestly `candidate`), the
  per-Provider checklist for AJ-E602–E605.
- `docs/research/evidence/billing/README.md` + the Claude capture:
  the evidence store (redacted, credential-grep-clean, immutable
  captures; new shapes get NEW files for the §6 paper trail).
- Handover to E602–E605: the checklist is §9; Claude's quota verdict
  flips to `verified` only with an authenticated live capture.
