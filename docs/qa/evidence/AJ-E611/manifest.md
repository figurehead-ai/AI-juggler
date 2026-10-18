# Evidence manifest — AJ-E611

Task: `docs/tasks/E6-quota-cost-and-billing-research.md` AJ-E611.
Branch: `stage2/e610-e611-batch`. Gate: `docs/qa/gate-M3.md` §AJ-E611.
2026-10-16, reviewer zcode-primary.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Hard filters before scoring; filtered rows greyed with reason, never dropped | deep/09 §Q6 lines 449-452 | rank() applies logged-out / exhausted-no-reset first (wrong-provider stub false — switch-epic surface, documented); filtered rows stay in output with `filtered_reason`; UI renders them greyed (16 gallery captures assert the reason visible) | PASS | 2026-10-16 | zcode-primary |
| Score = 0.45·headroom + 0.20·affinity + 0.15·fidelity + 0.10·costFit + 0.10·calm; constants in ONE core function | unit-tested, not per call site | weights-sum test green; constants live only in ranking.rs; Estimated headroom ×0.85 with Measured winning exact ties (tie-break test) | PASS | 2026-10-16 | zcode-primary |
| Every row shows top-2 factors + confidence badge + TTE; "Why this order?" expander lists all five; unknown headroom says so | no row without explanation | explanation-completeness test (≥2 factors or a filter reason) + UI captures assert factors without hover; "headroom unknown — ranked without quota" renders in row + factors | PASS | 2026-10-16 | zcode-primary |
| §Q5 alert table verbatim; FSM Armed→Warned→Critical→Exhausted→(reset)→Armed at-most-once | fixture-day + property test | exact §Q5 sequence asserted (tray once TTE<45; actionable once TTE<15/≥90%; exhausted once; reset once; repeats silent); 256-case random-sequence proptest: never two notifications for one crossing; no backward transitions (Critical never decays) | PASS | 2026-10-16 | zcode-primary |
| Snooze 30 min; >2× forecast error silent-log | §Q5 lines 436-441 | snooze-holds test (30 min hold, no re-emit); forecast-miss never interrupts (test) | PASS | 2026-10-16 | zcode-primary |
| Non-actionable crossings batch into hourly digest; per-key mute + quiet hours 22:00–07:00 defer without dropping | Requirement 4 | digest mute/defer/flush tests + app-level: renewal-approaching joins ONCE per hour, quiet hour (23) defers, never triggers the window FSM; renewal renders visually distinct ("Digest ·", dashed) | PASS | 2026-10-16 | zcode-primary |
| Subscription-clock alerts separate: renewal NEVER triggers window FSM, handover suggestion, or "switch now" copy | Requirement 5 | renewal feeds ONLY the digest (structural: FSM sees window observations only); app test asserts zero actionable/tray-amber from a renewal-approaching account | PASS | 2026-10-16 | zcode-primary |
| Reset-queue: passive timer, notify at reset once, resume ONLY from click; states queued/ready/resumed/dismissed (+stale explained) | deep/09 §Q4 + deep/10 §9 | queue notifies once (state→notified), ghost session → stale with explained copy; NO resume command exists — Resume click opens the NewSession picker (structural suggest-only) | PASS | 2026-10-16 | zcode-primary |
| No switch/launch/resume without explicit click — guardrail test | SPEC.md:263-style invariant | 4 app guardrails: sessions digest byte-identical through alert ticks, ranking, and every queue operation; core purity test (no launch API reachable); UI: the only verbs are user clicks | PASS | 2026-10-16 | zcode-primary |
| Ranking compute ≤10 ms / 20 accounts on demand; FSM ≤1 ms/tick; digest ≤50 ms; queue one wakeup | performance budget | compute is a pure sort over ≤20 candidates on demand (no timer); FSM steps are O(1) compares; digest flush is a linear filter; queue is storage-driven per tick (no polling loop — the UI's 30 s poll is the only clock) | PASS | 2026-10-16 | zcode-primary |
| Visual matrix: ranked list + why + notification + queue × 1440×900 + 1280×800 × light+dark | state/copy conformance, a11y | 16 captures (27-ranking) with copy assertions before each shot; 2 new axe passes 0 critical/serious; full suite 280/280 | PASS | 2026-10-16 | zcode-primary |
| A11y: notifications polite live-region; ranking list labelled with arrow-key navigation | WCAG + task spec | NotificationRegion role=status aria-live=polite; RankedRows labelled list with ↑/↓ roving focus (fixed nested-interactive: list of buttons, not listbox-wrapped buttons) | PASS | 2026-10-16 | zcode-primary |
| Live dogfood: one real quota-wall event end-to-end (resume latency <15 min) | live data | PENDING the authenticated Claude session (gate-E5 blocker, same as E607/E609/E610); fixture-day sequence + guardrails cover the state machine meanwhile | PENDING | 2026-10-16 | zcode-primary |

## Implementation-mapping

- `crates/juggler-core/src/ranking.rs` (new) — hard filters, the
  weighted score (constants in one function), Estimated discount,
  tie-breaks, factor explanations, unknown-headroom honesty,
  fidelity/wrong-provider stubs documented for their epics.
- `crates/juggler-core/src/alerts.rs` (new) — the §Q5 FSM (per
  (account,window), at-most-once, no backward transitions), snooze,
  digest (mute/quiet-hours/hourly flush), thresholds as constants.
- `crates/juggler-core/src/storage/usage.rs` — window-scoped
  `samples_for_account` (the E606 pollution bug this task's guardrail
  exposed) + reset-queue states.
- `crates/juggler-core/src/quota.rs` — `AccountEvaluation` per-window
  sample slices (five_hour/weekly fed separately).
- `crates/juggler-app/src/commands.rs` — ranking_rows,
  alerts_tick (FSM stepping + reset-queue notify + hourly digest
  flush with the UI-supplied local hour), reset-queue commands
  (queue/list/dismiss — no resume by design).
- `ui/src/components/RankedRows.tsx` (new) + `NotificationRegion.tsx`
  (new) — the ranked list (Suggested, factors, TTE, greyed rows, why
  expander, arrow-key focus) and the polite live-region (actionable,
  digest-distinct renewal, queue rows, explicit Resume).
- `ui/src/screens/NewSession.tsx` — ranked rows above the recipe list;
  picking an account filters its recipes (suggest-only).
- `crates/juggler-core/tests/ranking_alerts.rs` (new, 11) +
  `crates/juggler-app/tests/ranking_alerts_guardrails.rs` (new, 4) +
  gallery 27-ranking (16) + 2 axe passes; the a11y fixes
  (nested-interactive, modal text-faint → text-muted).
