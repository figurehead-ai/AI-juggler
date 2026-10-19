# Evidence manifest — AJ-E810

Task: `docs/tasks/E8-validation-visual-performance-quality.md` AJ-E810
(external research evidence standard). 2026-10-18, reviewer claude-opus.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Every external claim carries a source URL, access date, evidence and one of four confidence labels; negative evidence is never worded as a vendor denial | Req.1 | `docs/research/EVIDENCE-STANDARD.md` (the four fields, the four labels, "not found by this search") | PASS | 2026-10-18 | claude-opus |
| Billing/renewal research tests live where possible; "no API found" has a search log | Req.2 | `billing-claims-register.md`: per Provider, every claim labelled with source and access date; the Anthropic sources re-opened on 2026-10-18; the search logs live in `docs/research/billing-apis.md`. No live API test: no candidate passed the bar | PASS | 2026-10-18 | claude-opus |
| Claims older than 90 days are marked stale and re-verified before use | Req.3 | `scripts/research_staleness.sh` (exit 1 on any research document older than 90 days; today: none) | PASS | 2026-10-18 | claude-opus |
| Secrets never test an API; sacrificial accounts only, with maintainer approval | Req.4 | Rule 3 of the standard; no account was used | PASS | 2026-10-18 | claude-opus |
| The register reads at 100 columns: labels and links readable, no table over 4 columns | visual | Every register table has exactly 4 columns; sources are short host/path forms | PASS | 2026-10-18 | claude-opus |
