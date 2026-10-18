# Evidence manifest — AJ-E408 (Provider / Account / Project inspector content)

Task: `docs/tasks/E4-indicators-and-inspector.md` AJ-E408.
Branch: `stage2/e4-inspector-aggregate`. Local runs (macOS arm64) 2026-10-13,
reviewer zcode-primary. Component tests also run in CI (ui job).

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 Provider Overview | `pnpm --dir ui test` (inspector-aggregate) | counts, worst-first aggregate, honest badges | accounts/running counts real; aggregate lists accounts worst-quota-first (Personal 5% left above Work Max 45% — order asserted); `version: No data` + `capabilities: No data` honest badges | PASS | 2026-10-13 | zcode-primary |
| Req.2 Provider tabs + Actions | same | mixed labelling; never-auto-update reason | `mixed` appears exactly when confidence tiers differ (computed-vs-estimated fixture asserted); Add account → Accounts (real event); Check-for-update disabled with the SPEC §5.4 never-auto-update reason; Usage/History/Quality/Processes honest empties with owners | PASS | 2026-10-13 | zcode-primary |
| Req.3 Account Overview | same | rings with % text, reset, affinity, renewal, logged-out collapse | 5h ring + week ring render `% left`/`No data` text and `role="img"` aria labels (45% left asserted); reset countdown real; usual-projects from accounts_used; renewal `not set (optional, D-12 — AJ-E411)`; logged-out fixture collapses to the sign-in card (rings absent) | PASS | 2026-10-13 | zcode-primary |
| Req.4 Account History + Actions | same | Today/Yesterday/Earlier; D-10 label; managed-dir note | grouped timeline renders Today on the fixture; `New launch settings…` (no `New profile` anywhere — asserted); Delete keeps `the managed isolated directory is kept` adjacent | PASS | 2026-10-13 | zcode-primary |
| Req.5 Project Overview | same | On-it-now card, root copy, branches, counts | full SessionRow (I-01..I-11) inside On-it-now; copyable root; branch summary + dirty total; accounts-used; instance + imported counts | PASS | 2026-10-13 | zcode-primary |
| Req.6 Missing dir + empty project | same | danger banner + Relocate…; Start-first-session CTA | `ProjectInfo.missing` (backend root stat) drives a `role="alert"` banner with `Relocate…`; the zero-session project renders the CTA | PASS | 2026-10-13 | zcode-primary |
| Req.7 Traceability | same | every aggregate explains itself | provider-aggregate rows all name their account (loop-asserted) | PASS | 2026-10-13 | zcode-primary |
| WCAG 1.4.1 ring fix | same + axe | % text + role=img; no colour-only | rings carry aria (`45% quota left`) and visible `% left`/`No data` text; `QuotaRing` semantics documented (percent = fill/used) | PASS | 2026-10-13 | zcode-primary |
| Regression | full suites | prior gates intact | 97/97 component (86 prior + 11 new); 72/72 visual incl. three `08-inspector-aggregate/` captures (rings, logged-out collapse, On-it-now); axe both themes green | PASS | 2026-10-13 | zcode-primary |
| Full gates | `just verify` + `pnpm --dir ui build` | green | verify green; typecheck+bundle green | PASS | 2026-10-13 | zcode-primary |

## Notes for the next agent

- Ring semantics: `QuotaRing percent` = FILL (used %); the ring's own
  text/aria derive `% left`. Passing "% left" produces inverted rings —
  the test catches it, keep it that way.
- Honest `No data` badges name their wiring tasks: version/verified
  range/capabilities (adapter discovery), burn-down (AJ-E606+), window
  timeline (AJ-E606+), adapter history (E3+). Replace badges with real
  chips as sources land — never widen them silently.
- `aij:open-accounts` / `aij:new-session` window events route inspector
  actions to App surfaces; extend rather than fork.
- Project launch ranking = worst quota % left per account (headroom),
  `Usual` star from `project.accounts_used`; re-rank when AJ-E409
  burn-rate signals exist.
