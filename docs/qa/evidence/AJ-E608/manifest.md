# Evidence manifest — AJ-E608

Task: `docs/tasks/E6-quota-cost-and-billing-research.md` AJ-E608.
Branch: `stage2/e608-e609-batch`. Gate section: `docs/qa/gate-M3.md`
§AJ-E608. 2026-10-16, reviewer zcode-primary.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| EWMA 5/15/60 cone; headline 15-min | single-rate non-compliant | three α=2/(N+1) EWMAs over per-minute unit buckets; cone = 5↔60 spread; headline rate15 | PASS | 2026-10-16 | zcode-primary |
| Cold start <10 min ⇒ Learning, no number | no numeric output at any confidence | 9-min fixture ⇒ Learning; type carries no number | PASS | 2026-10-16 | zcode-primary |
| TTE = headroom/rate15 only inside the reset | never beyond reset; unknown reset ⇒ no promise | TTE ≥ remaining ⇒ WontExhaust; resets_at None ⇒ WontExhaust; non-finite guard | PASS | 2026-10-16 | zcode-primary |
| Confidence inherits lowest input (Q-2) | one Estimated ⇒ range output | one Estimated event ⇒ forecast Estimated + cone range (type-pinned) | PASS | 2026-10-16 | zcode-primary |
| Launch gate OK/Tight/No, never blocks | SPEC §237 verdict language | verdict fn + median_session_burn (empty sessions ignored); No is data, never a block | PASS | 2026-10-16 | zcode-primary |
| Calibration silent, >2× trips | note exposed to AJ-E611 | calibration_note pure fn: 2.5×/0.4× trip, 1.5× holds; pairing loop ships with the E611 alert FSM (boundary documented) | PASS | 2026-10-16 | zcode-primary |
| Fixture timelines reproduce known TTE | constant ±10%; accelerating MAPE ≤30% | constant 60 min ±10% green; accelerating oracle 20.6 min MAPE ≤30% green; 30-day evaluation ≤500 ms | PASS | 2026-10-16 | zcode-primary |
| ZCode on credits, units not interchangeable | shared cone, distinct units | credits fixture TTE ≈30 min green | PASS | 2026-10-16 | zcode-primary |

## Implementation-mapping

- `crates/juggler-core/src/quota_forecast.rs` (new) — burn_rate cone,
  headroom, forecast (Learning/WontExhaust/Exhausts/NoHeadroom —
  deep/10's four pill states), admission verdict, median session
  burn, calibration notes; signed-zero normalisation documented.
- `crates/juggler-core/tests/quota_forecast.rs` (new) — fixture
  generator + independent oracles, 13 tests.
