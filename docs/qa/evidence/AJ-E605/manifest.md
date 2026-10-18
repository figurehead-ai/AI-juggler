# Evidence manifest — AJ-E605

Task: `docs/tasks/E6-quota-cost-and-billing-research.md` AJ-E605.
Branch: `stage2/e605-e606-batch`. Evidence:
`docs/research/gemini.md` + `docs/research/evidence/billing/gemini-captures.md`
(+ the billing README row). Zero product code. 2026-10-16, reviewer
zcode-primary. No Gemini install exists on the machine (`which gemini`
empty, no `~/.gemini`) — the task budget's documented PASSING outcome
(docs-only packet, live items `unverified — no install`).

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| `docs/research/gemini.md` in the codex.md house format | surfaces table, [V]/[U] markers, dated URLs | 8 sections; every [V] carries a dated official URL (geminicli.com docs 2026-10-07/08-17, ai.google.dev rate-limits 2026-10-15, developers.googleblog.com 2026-05-19, antigravity.google/docs) | PASS | 2026-10-16 | zcode-primary |
| Documented surfaces only; undocumented = prohibited with citation | no scraping, no undocumented endpoints probed | only official docs + first-party announcements read; the community local-state reader marked [U]/prohibited-until-captured (protocol §6) | PASS | 2026-10-16 | zcode-primary |
| D-12 renewal answer explicit | found-graded or absent-with-search-log | ABSENT — 5 searched surfaces logged (developers.google.com, Play Developer API own-app-only, myaccount/Play web UIs, one.google.com marketing, ~/.gemini documented keys) → user-entered or Unknown | PASS | 2026-10-16 | zcode-primary |
| No parity assumptions (pass2/09 U-4) | every parity-shaped claim without capture is [U]/inferred | CLI has NO 5h/weekly window (daily caps only — the 5h/weekly model is the Gemini app's and Antigravity's, both cited); sunset 2026-06-18 verified — individual login paths dead | PASS | 2026-10-16 | zcode-primary |
| Ceiling per field + fallback for AJ-E607 | ceiling table with fallbacks | §6 table: session tokens documented-sanctioned shape (live unverified), account % NONE (unavailable-with-reason, D-06), reset rule midnight-PT (API path), renewal user-entered, Antigravity /usage documented candidate | PASS | 2026-10-16 | zcode-primary |
| Credential hygiene | nothing read | no install existed; zero credential stores touched; grep gate zero | PASS | 2026-10-16 | zcode-primary |

## Implementation-mapping

- `docs/research/gemini.md` (new) — the AJ-E605 baseline incl. §7
  Antigravity successor section (add-a-section, don't fork the
  protocol — handover note honored).
- `docs/research/evidence/billing/gemini-captures.md` (new) — the
  AJ-E601 §2-templated packet (no-install statement, doc-based
  grades, renewal negative log, verdict table).
- `docs/research/evidence/billing/README.md` — capture table row.
- Consequences recorded: AJ-E607 emits NO Gemini adapter (stub renders
  unavailable-with-reason); AJ-E610 manual renewal entry; AJ-E611
  headroom unknown.
