# Evidence manifest — AJ-E609

Task: `docs/tasks/E6-quota-cost-and-billing-research.md` AJ-E609.
Branch: `stage2/e608-e609-batch`. Gate section: `docs/qa/gate-M3.md`
§AJ-E609. 2026-10-16, reviewer zcode-primary.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Versioned pricing table with effective_from + source_url; unpriced ⇒ `—` | bundled, user-overridable, auditable | seed v2 (adds cache rates/effective_from/source_url over AJ-E409 v1) + V19 `pricing_overrides` repo; unpriced ⇒ USD None rendered `—` (tested, never $0.00) | PASS | 2026-10-16 | zcode-primary |
| Query-time join; event-time rate selection; repricing retroactive | stored rows never change | override flips the rollup; raw `usage_events` SQL identical before/after (byte-proof); `effective_from ≤ ts` per day bucket | PASS | 2026-10-16 | zcode-primary |
| Rollups per session/account/project/day, tokens by class + USD + confidence | USD always Estimate | CostRollup shape for all dimensions (Project via session→project map, unmapped keyed by session id — never fabricated); 5 token classes carried | PASS | 2026-10-16 | zcode-primary |
| Dual subscription view; ratio only with a fee | no guessed fee | subscription_view: fee-less ⇒ no ratio (tested); fee entry is AJ-E610's form (KV `subscription_fee:<account>`) | PASS | 2026-10-16 | zcode-primary |
| Model-switch attribution survives; reconciles to the cent with CSV | (session, model) grouping | switch fixture ⇒ 2 rows summing to session total; CSV↔rollup reconciliation test to the cent | PASS | 2026-10-16 | zcode-primary |
| CSV streams, reuses the exact rollup query | ≤50 MB peak | chunked month-window `events_between` fetch + day-bucketed merge; CSV writer streams rows (`write_csv`) | PASS | 2026-10-16 | zcode-primary |
| ccusage cross-check on shared corpus | corpus hash + both totals + $0.00 delta | oracle `ccusage-oracle.py`: sha256 143941e1…, 2 priced messages (uuid-deduped), $0.0083100 = Rust total; delta $0.0000000 (gate-M3) | PASS | 2026-10-16 | zcode-primary |
| ZCode credits alongside USD, never converted | separate column always | credits ride their own column in rollup + CSV (tested with NO USD rows present) | PASS | 2026-10-16 | zcode-primary |
| Label discipline + grep gate | no "you paid $" on API value | permanent badge "Estimated — list pricing, not your subscription fee"; vitest grep gate green | PASS | 2026-10-16 | zcode-primary |
| Visual: priced/unpriced/stale × viewports | badge visible without hover; `—` not $0.00; both numbers labelled | 24 captures in `_gallery/25-costs/` (3 states × 4 viewports × 2 themes); th scope + totals row asserted | PASS | 2026-10-16 | zcode-primary |
| Dogfood export reconciles for a real week | live data | PENDING the authenticated session (same blocker as the E607 dogfood; the export-vs-screen reconciliation is structurally pinned by the shared rollup query + tests) | PENDING | 2026-10-16 | zcode-primary |

## Implementation-mapping

- `crates/juggler-core/src/cost_ledger.rs` (new) + `cost_ledger_seed.json`
  — LedgerPriceRow/PricingCatalog (event-time selection, overrides,
  staleness), rollup (all dimensions), subscription view, streaming
  CSV writer, UTC day buckets.
- `crates/juggler-core/src/storage/usage.rs` — events_between (chunked),
  usage_accounts, pricing_override repo; storage/mod.rs V19.
- `crates/juggler-app/src/commands.rs` — cost_rollups, cost_csv_export,
  pricing_override_set/clear, pricing_catalog_info.
- `ui/src/components/CostMatrix.tsx` (+ test) — the matrix, dual view,
  stale warning, a11y table semantics; App.tsx mount; bridge seams;
  visual states in `ui/tests/visual/capture.spec.ts` (gallery 25-costs).
- `crates/juggler-core/tests/cost_ledger.rs` (new, 12 tests) +
  `docs/research/evidence/billing/ccusage-oracle.py`.
