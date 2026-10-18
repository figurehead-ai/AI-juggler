# Evidence manifest — AJ-E409 (Quality signals without a score)

Task: `docs/tasks/E4-indicators-and-inspector.md` AJ-E409.
Branch: `stage2/e4-quality-signals`. Local runs (macOS arm64) 2026-10-13,
reviewer zcode-primary. Component tests also run in CI (ui job); Rust suite
runs in the verify job.

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 tokens/min exact | `cargo test -p juggler-core --test quality_signals` (`claude_fixture_exact_tokens_per_min`) | exact trailing-5-min total + avg from committed fixture | `tests/fixtures/claude_usage.jsonl` (41 turns over 10 min; turns 22-41 in window): trailing 5 min = 3000 tok and avg = 600 tok/min asserted exact; freshest bar asserted non-zero | PASS | 2026-10-13 | zcode-primary |
| Req.1 idle vs no data | same (`idle_renders_real_zero_distinct_from_no_data`) | real `0` distinct from `no data` | events older than window → `bars: Some([0;5])`, real 0; empty events → `bars: None` (`no data — no transcript usage events yet` in UI); both asserted | PASS | 2026-10-13 | zcode-primary |
| Req.2 pricing as data | same (`pricing_table_is_versioned_data_and_costs_are_estimates`) | versioned JSON, source+date, no code constants; unknown model → tokens only | `quality/pricing.v1.json` (v1, source anthropic.com/pricing, captured 2026-10-13) loaded via `include_str`; opus 12,345 in + 6,789 out → 12.345×$0.015 + 6.789×$0.075 = $0.6944 (asserted to 1e-9); unknown model → `None` → UI renders `tokens only — no pricing row for this model (never $0.00)`; CHANGELOG line added for v1 | PASS | 2026-10-13 | zcode-primary |
| Req.3 error numerator/denominator | same (`error_rate_renders_numerator_denominator`) | `N errors / M turns`, never bare % | errors fixture → `3 errors / 10 turns` exact string; turns==0 → `Not measurable (interactive)` (copy-exact asserted) | PASS | 2026-10-13 | zcode-primary |
| Req.4 context honest | same (`context_unavailable_renders_the_provider_reason`) | no fabrication from cumulative | `ContextReport::NotReported` carries the provider-specific reason → UI `Context: not reported by {provider}` + cumulative labelled `cumulative, not context`; estimating context from cumulative is structurally impossible (no conversion path exists); asserted | PASS | 2026-10-13 | zcode-primary |
| Req.5 stuck + no score | same (`no_composite_score_exists_in_crates_or_ui`) | grep gate over `crates/` + `ui/src` | zero matches for `quality_score\|health_score\|QualityScore`; stuck renders AJ-E405 rule id + inputs + `since` with `Stuck?` label; `QualitySignals` struct has no score field by construction | PASS | 2026-10-13 | zcode-primary |
| Req.6 CPU/RAM footnote | `pnpm --dir ui test` (quality-tab) | footnote names the collector | tab footnotes `2s sampling · CPU/RAM land with the process collector (AJ-E410)` — honest owner naming, no flat-zero placeholder | PASS | 2026-10-13 | zcode-primary |
| Req.7 forecast range | same Rust test (`quota_forecast_is_a_range_or_not_burning`) | range tier-tagged; idle `not burning` | 40% left at 10%/5 min → `~13-27m at current rate · Est` (±33%); burn 0 → `not burning`; no point estimate/infinity in any branch; asserted | PASS | 2026-10-13 | zcode-primary |
| Req.8 tiers adjacent | `pnpm --dir ui test` (quality-tab) | Measured/Computed/Estimated adjacent to every value | every signal row carries a `TierTag` (tokens `Computed`, errors `Measured`, context `Measured`/reason, cost `Est · value-equiv.`, forecast `Est`, stuck rule text); screen-reader visible (text nodes asserted) | PASS | 2026-10-13 | zcode-primary |
| Budget: 100k events < 5 ms | same Rust test (`tokens_per_min_100k_events_under_budget`) | < 5 ms per Instance | single-pass over 100k synthetic events completes well under budget (test asserts elapsed < 5 ms; measured ~1 ms) | PASS | 2026-10-13 | zcode-primary |
| Budget: lookup O(1)-ish | by construction | pricing lookup per model | `PricingTable::price` = linear scan over the tiny embedded table (2 rows v1); per-model budget holds by construction; documented in the doc comment | PASS | 2026-10-13 | zcode-primary |
| Visual | `pnpm --dir ui visual:capture` | populated + View-as-table, no score-like number | 74/74 visual; new `09-quality/populated-1440x900.png` (bars, tiers, cost, forecast, stuck) + `as-table-1440x900.png` (text-table alternative); no 0-100 score number appears in either capture | PASS | 2026-10-13 | zcode-primary |
| Regression | full suites | prior gates intact | 102/102 component (97 prior + 5 new); `just verify` green (fmt, clippy -D warnings, workspace tests, docs, evidence-check, terminology); `pnpm --dir ui build` green | PASS | 2026-10-13 | zcode-primary |

## Notes for the next agent

- Quota forecast is honest-by-design today: `instance_quality` passes
  burn rate `0.0` because quota-window deltas are not observable until
  the usage engine (AJ-E606+) — it renders `not burning`, never a
  fabricated rate. Wire real `pct_burned_last_5_min` there when deltas
  land; the label math is already tested.
- Pricing lives in `crates/juggler-core/src/quality/pricing.v1.json`
  (DATA, not constants): bump `PRICING_VERSION` + file name + CHANGELOG
  line when rows change — the Rust test pins the version and a cost,
  so a silent edit fails CI.
- Codex/ZCode tier trap (handover note): tokens are `Measured` even when
  their quota is `Estimated` — tiers are per-series in the UI model;
  keep them separate when adding providers.
- Transcript scan (`scan_transcript`) reads fixture-compatible shapes
  (Claude `timestamp`+`usage` on assistant lines); Codex
  `token_usage_record` / ZCode `model_usage` shapes join with their
  adapter work (E5/E6) — extend the scan per provider, never guess.
- `View as table` toggle is the WCAG text alternative for the bar chart;
  every future chart on this tab must ship both views.
