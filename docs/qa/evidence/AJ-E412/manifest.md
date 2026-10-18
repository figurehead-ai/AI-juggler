# Evidence manifest — AJ-E412 (Running-model telemetry)

Task: `docs/tasks/E4-indicators-and-inspector.md` AJ-E412.
Branch: `stage2/e4-running-model`. Local runs (macOS arm64) 2026-10-14,
reviewer zcode-primary. Rust suites run in CI (verify job); component tests
in the ui job.

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 read model split | `cargo test -p juggler-app --test running_model` (3 tests) + wire | launch vs running vs `model_source` vocabulary | `InstanceRecord.running: Option<RunningModel{model, source}>` (schema v9 JSON blob, additive nullable; never filled from the overlay); wire adds `launch_model`/`running_model`/`model_source` (`launch\|in-session\|transcript\|unknown`) and `model` = running ?? launch ?? null — asserted for drift, launch-only, and unknown rows | PASS | 2026-10-14 | zcode-primary |
| Req.2 detection + <300ms | same (`running_model_observed_classified_and_incremental`) | transcript watch; no config polling; emitted on `sessions_updated` | detection rides the existing 200 ms activity drain (the event path) + the 2 s tick as a safety net; incremental byte-offset tail scans (one metadata read when unchanged — asserted); on change the record persists and `sessions_updated` emits. NO new file watcher, NO Provider config polling, NO network (code-reviewed against the guardrail) | PASS | 2026-10-14 | zcode-primary |
| Req.3 drift surfacing | `pnpm --dir ui test` (running-model) + captures | `Launched with X - now running Y` on Overview + I-03 marker with that text as tooltip/aria | Overview dual-model line renders the exact copy + `(observed in transcript)`; rail I-03 gains a keyboard-focusable `⇄` marker whose `aria-label` is the full drift text (asserted); plain/launch-only rows show no marker (asserted) | PASS | 2026-10-14 | zcode-primary |
| Req.4 per-turn cost | `cargo test -p juggler-core --test quality_signals` (`per_turn_pricing_prices_only_turns_with_a_recorded_model`) + `pnpm --dir ui test` | price by observed per-turn model; unrecorded = excluded from $, tokens counted | mixed session (opus + unrecorded + sonnet) → $0.1575 with models `[opus, sonnet]` exact; all-unrecorded → `None` (tokens only); unpriced model → `None` (never $0.00); `instance_quality` prices via `session_cost_per_turn`; cumulative tokens keep every turn | PASS | 2026-10-14 | zcode-primary |
| Req.5 unobservable providers | UI tests + capture | `Model: not reported by {Provider}`; `not verified live` for launch-only | inspector renders the exact copy when neither model exists (asserted); launch-only shows the overlay chip + `not verified live` tag, never posing as running (asserted); the row keeps `default model` only when no launch model exists | PASS | 2026-10-14 | zcode-primary |
| Req.6 verbatim identifiers | fixtures + tests (no mapping table exists) | vendor strings displayed as-is; no normalisation | `opus-4.8`, `sonnet-5`, `gpt-5-codex` flow verbatim through parser→storage→wire→UI; no mapping/normalisation table anywhere in the path (code-reviewed); only display filters may group later | PASS | 2026-10-14 | zcode-primary |
| Req.7 local-only | code review (guardrail) | no telemetry egress | detection reads local transcript files only; no network code in the path; egress only possible via the future D-13 Fleet link (not present) | PASS | 2026-10-14 | zcode-primary |
| Parser fixtures (4/4) | `cargo test -p juggler-adapters --test claude_transcript running_model` | launch-only, /model change, per-turn mixed, no-model | 4/4 green — `ParsedMessage.model` reads top-level `model` AND nested `message.model` (both shapes asserted in the mixed fixture); fixtures committed at `crates/juggler-adapters/tests/fixtures/model_*.jsonl` beside the parser tests | PASS | 2026-10-14 | zcode-primary |
| Budget | detection tests (incremental asserts) | <1 ms/event; no extra watchers | unchanged transcripts cost ONE metadata stat per tick (early return asserted); new bytes parse line-wise without re-reading the head (seek-based tail read — the initial implementation's re-read bug caught by the incremental test and fixed) | PASS | 2026-10-14 | zcode-primary |
| Storage v9 | `cargo test -p juggler-core --test storage_roundtrip` (19/19) | additive; missing field loads None | v4→v9 exactly once on open; running-model round-trip set/clear; malformed blob degrades to `None` (same rule as attention) | PASS | 2026-10-14 | zcode-primary |
| Visual | `pnpm --dir ui visual:capture` | 4 states | 90/90 incl. new `12-model/`: rail drift marker (hover, aria asserted), rail plain (exactly one marker on the fixture board), Overview dual-model line, `Model: not reported` state; a11y 3/3 | PASS | 2026-10-14 | zcode-primary |
| Regression | full suites | prior gates intact | `just verify` green (fmt, clippy `-D warnings`, workspace tests, docs, evidence-check, terminology); 125/125 component (6 new); `pnpm --dir ui build` green | PASS | 2026-10-14 | zcode-primary |

## Notes for the next agent

- The indicator/inspector core of E4 (E401–E412) is DONE-EVIDENCED;
  the desktop track remains: AJ-E413 (desktop adapter contract),
  AJ-E414 (Focus Window), AJ-E415 (live preview) — then E5/E2/E3 per
  the catalog's dependency order.
- The detection engine is split: `detect_running_models_core` (storage)
  + the emit wrapper — tests drive the core; the 200 ms activity drain
  calls the wrapper for the event-budget path.
- `ParsedMessage.model` accepts BOTH transcript shapes (top-level and
  `message.model`); the app-side `line_model` mirrors this — keep them
  in sync when a new provider shape appears.
- Pricing precedence changed with this task: per-turn observed model
  pricing REPLACED the E409 launch-overlay fallback in
  `instance_quality` (overlay = intent, never priced as fact — the
  handover-note trap, honoured).
- A live `/model` dogfood against a real Claude session remains a
  maintainer-side validation (the same class as prior live-dogfood
  items); the deterministic evidence path (fixtures → detection → wire
  → UI) is fully covered above.
