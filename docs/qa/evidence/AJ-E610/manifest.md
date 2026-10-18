# Evidence manifest — AJ-E610

Task: `docs/tasks/E6-quota-cost-and-billing-research.md` AJ-E610.
Branch: `stage2/e610-e611-batch`. Gate: `docs/qa/gate-M3.md` §AJ-E610.
2026-10-16, reviewer zcode-primary.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Every quota number carries its badge adjacently; stale Measured demotes | grep gate + state tests | WindowRing/QuotaChip render % + Measured/Computed/Estimate adjacent (16 UI tests); stale shows "last measured Nh ago" + opacity demotion; USD badge from E609 unchanged | PASS | 2026-10-16 | zcode-primary |
| Window clock = ring + % text + reset; subscription clock = calendar line; never merged | structural test | rings carry no "Renew"; renewal line carries no %; two separate components | PASS | 2026-10-16 | zcode-primary |
| Renewal entry per Account, states none/set/approaching/overdue, user-tunable 1–30d, user-provenance | explicit action only | E411 editor + new FeeEditor (explicit save); backend states with warn-days clamp tested (3d threshold: 6d=set, 2d=approaching exact copy); entered_by_user wired | PASS | 2026-10-16 | zcode-primary |
| Typed states on every surface incl. no-data/learning/blocked/unavailable-with-reason | never `percent={0}` for unknown | UI matrix 16/16 + backend 5/5; Gemini renders "No verified quota source for this Provider yet" (D-06); blocked shows reset CTA | PASS | 2026-10-16 | zcode-primary |
| Pro renders weekly; plans without 5h omit ring with reason | no empty rings | Codex-Pro: five_hour=None + reason "Pro plans have no five-hour limit — weekly only", weekly rendered (backend test + omitted-reason UI test) | PASS | 2026-10-16 | zcode-primary |
| Keyboard/SR access; rings role=img with text equivalent | aria labels exact | aria-label "82% of 5-hour window used…, Measured" asserted; add-date/save buttons focusable; no new global chords | PASS | 2026-10-16 | zcode-primary |
| Visual matrix 1440×900 + 1280×800 × light+dark | state/copy conformance | 16 new captures (26-quota) + reseeded account-overview; full suite 262/262 | PASS | 2026-10-16 | zcode-primary |
| Tray aggregate | ≤10 ms query | NOT CAPTURED — no tray exists in the app (deep/10 §19 later surface); recorded in gate-M3, not silently omitted | N/A | 2026-10-16 | zcode-primary |
| Live dogfoods (±5% /usage; renewal-entry restart persistence) | live data | PENDING the authenticated Claude session (gate-E5 blocker); fixture suites cover the state + persistence paths meanwhile | PENDING | 2026-10-16 | zcode-primary |

## Implementation-mapping

- `crates/juggler-app/src/commands.rs` — quota_display_inner +
  quota_display/quota_display_all/account_set_fee commands (the §Q8
  machine → wire states; forecast pill data; D-12 renewal with
  user-tunable threshold; provider window-shape rules).
- `ui/src/components/QuotaRings.tsx` (new) — WindowRing (SVG ring +
  % + badge + reset + stale note + typed states, role=img),
  RenewalLine, TwoClocks, ForecastPill, QuotaChip; + test (16).
- `ui/src/components/RightInspector.tsx` — Usage tab's two-clock card
  (AccountTwoClocks), account card engine rings (AccountWindowRings),
  FeeEditor.
- `ui/src/screens/NewSession.tsx` — per-row QuotaChip via
  quota_display_all (one query).
- `crates/juggler-app/tests/quota_display.rs` (new, 5) + gallery
  26-quota + the reseeded 08 account-overview capture.
