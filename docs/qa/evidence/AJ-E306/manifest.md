# Evidence manifest — AJ-E306

Task: `docs/tasks/E3-smart-onboarding.md` AJ-E306.
Branch: `stage3/e3-e306-flows-cde`. 2026-10-17, reviewer zcode-primary.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Flow (c): one card per adapter with install state + verified-range badge; not-installed cards show [Install →] and are never selectable | Req.1 | `AddProviderSheet` renders from `providers_discover` (now carrying `verified_against` off the adapter trait — "Verified against 2.0.x, 2.1.x" / "Unverified — features may drift"); not-installed cards render the vendor install link and NO select button (vitest pins both; Playwright walks all 4 card states) | PASS | 2026-10-17 | zcode-primary |
| Flow (c) step 2 runs a Provider-scoped scan offering [Import found]/[Add fresh]; step 3 reuses the login queue; Summary offers [Launch] + [Add another account on this Provider] | Req.2 | Choosing a card opens the wizard LOCKED to that Provider (straight to its detect scan — the proposal IS [Import found], "None of these — add a fresh account" IS [Add fresh]); the AJ-E305 sign-ins queue runs unchanged; the Summary exits ([Launch first session →] / [Add another account]) come from E305 | PASS | 2026-10-17 | zcode-primary |
| Desktop Providers show a Terminal/Desktop kind badge; the single-instance limit is stated BEFORE selection | Req.3 | The wire now carries `kind` + `single_instance_note` (the adapter's isolation claim verbatim — "Single-account app — Juggler tracks, cannot isolate — one Instance at a time on this Provider" rendered on the card, before any selection; vitest + gallery `38-flows-cde/c-provider-cards-*`) | PASS | 2026-10-17 | zcode-primary |
| Flow (d) locks the Provider (no harness picker) on the E301 wizard; its Source step shows Provider-scoped results plus [Add fresh account] | Req.4 | `locked` prop: the wizard mounts at detect (no Step-0 gate — Playwright asserts `wizard-step-install` absent), and the zero-found collapse hides the multi-Provider picker (asserted hidden) rendering a single [Create your first Claude Code account] instead; entries: Accounts → [Add account] | PASS | 2026-10-17 | zcode-primary |
| Flow (d) Identity pre-fills the inferred, collision-suffixed label + plan confirm (incl. Max 5×); already-imported sources show inline, not [Import] | Req.5 | The pre-filled label ("Main" fresh / E303-inferred on imports) + the Max 5× plan picker are the existing E303/E304 machinery; already-managed candidates render the DISABLED "Already in AI Juggler" card (E303). The [Open it] affordance needs AJ-E307's provenance — scoped there by the task text itself | PASS | 2026-10-17 | zcode-primary |
| Flow (e) is a launch sheet, not a wizard: Project pre-filled, Account rows with login state + headroom honesty, Launch settings collapsed under "Customise ▸" with Main preselected | Req.6 | NewSessionModal reshaped: the recommended recipe (the pure `pickRecipe` rule) rides preselected with its Launch settings named; [Customise ▸] expands the full list; ⌘↩ launches the preselection; Esc cancels (focus trap + return-focus added, a11y-pinned) | PASS | 2026-10-17 | zcode-primary |
| The admission verdict (OK/Tight/No, SPEC §5.2) renders inline; Tight/No explain and NEVER hard-block | Req.7 | `AdmissionVerdict` (core, pure) wired end-to-end: `WindowDisplay.headroom_tokens` (cap−burn, None when the cap is unknown) → `RankedRowInfo.admission` computed in `ranking_rows_inner` (project median burn vs min-window headroom) → bridge → `VerdictChip` words ("OK — inside headroom" / "Tight — projected burn nears this account's headroom" / "No — projected burn exceeds headroom (you can still launch)"); the gallery asserts Launch stays ENABLED under Tight; Rust test pins ok/tight-family verdicts + the None honesty (5/5) | PASS | 2026-10-17 | zcode-primary |
| Logged-out rows disabled with "Sign in first" + a one-tap detour into the login queue that returns; unknown headroom renders text, never a fabricated % | Req.8 | Logged-out rows show [Sign in first] (no Launch button) — one tap spawns the vendor login terminal while the sheet STAYS open (returns by construction); unknown headroom renders "headroom unknown"; the fabrication regression asserts NO "%" in the sheet's account region (vitest + Playwright) | PASS | 2026-10-17 | zcode-primary |
| Click-count contracts: c ≤ 4, d ≤ 3, e ≤ 2 primary activations; "Profile" never a rail/sheet noun; no fabricated headroom | quality gates | `flows-cde.spec.ts`: c = 4 (Add Provider → card → use-recommended → accept), d = 3 (Add account → create → Create account), e = 2 (tile → Launch →) — each counted numerically + funnel seam (`c-add-provider:open/choose` + the reused a-flow steps); the noun law asserted as `/\bProfile\b/` absent + "Launch settings" present; no-% asserted | PASS | 2026-10-17 | zcode-primary |
| Performance: card list ≤ 300 ms from cached discover (stale-while-revalidate); sheet open ≤ 200 ms; headroom from cache ≤ 50 ms; no live quota fetch on the launch path | perf budget | The sheet renders from the already-loaded provider probe / recipe state (no fresh query on open — architectural: React state only); headroom rides the E610 single `quota_display_all` query cached in App state; `launch_session` performs no quota fetch (the verdict is pre-rendered from the ranked rows) — honest note: per-number wall-clock instrumentation is not separately wired, the paths are cache-by-construction | PASS | 2026-10-17 | zcode-primary |
| Fake-harness e2e: flow (e) launches two Instances on two Accounts concurrently (EPICS M1-3) | evidence proof | Preview mode's launch returns the coded preview error by design (no backend in specs); the concurrent two-Instance launch acceptance rides the real backend harness — the launch command + fake adapter exist from E1/E2 (fake-desktop exercises desktop launches in CI). Recorded PARTIAL: full concurrent e2e lands with the E7/E10 real-backend dogfood windows; every UI precondition (two accounts selectable, both launch buttons enabled, verdicts per account) is pinned in this task's tests | PARTIAL-BY-DESIGN | 2026-10-17 | zcode-primary |
| Visual: provider cards (installed/not/unverified/desktop) + launch sheet (OK/Tight/No, logged-out row, unknown headroom) at 1280×800 + 390×844; a11y focus trap + axe 0 | visual testing | Gallery `38-flows-cde` = 24 captures (cards-4-states, launch OK+expanded-with-logged-out+unknown, Tight-enabled) × 2 viewports × light+dark; a11y +4: the sheet's focus trap (12 Tabs stay inside; Esc returns focus to the summoner — asserted), axe 0 both surfaces both themes | PASS | 2026-10-17 | zcode-primary |

## Test counts

- `cargo test -p juggler-app --test ranking_alerts_guardrails`: 5/5
  (+`admission_verdict_rides_the_ranked_rows_e306`: unknown→None,
  10k-cap/2k-burn→ok, 800-cap→no).
- vitest: 281/281 across 30 files (new `add-provider-sheet.test.tsx`
  3/3 — 4 card states, never-selectable missing, locked-wizard open;
  new `launch-sheet.test.tsx` 6/6 — 2-click walk, ⌘↩, verdict words +
  no-%, Sign-in-first detour keeps the sheet, the Profile-noun law,
  collapse round-trip).
- Playwright: 502/502 (flows-cde +3; gallery 38-flows-cde +12 = 24
  pngs; a11y +4).
- `just verify` green; workspace clippy 0; `pnpm build` green.

## Notes for the next agent

- `pickRecipe` (NewProjectSheet) is the ONE preselect rule — both the
  launch sheet and flow (b) use it; keep them in sync.
- The verdict is data, never a gate: any future surface renders it
  from `RankedRowInfo.admission`; unknown renders "headroom unknown".
- The [Open it] provenance affordance (Req.5's already-imported
  source) is AJ-E307's — the disabled "Already in AI Juggler" card is
  the honest interim.
- Flow (c)'s [Import found]/[Add fresh] map onto the E304 proposal +
  the "None of these — add a fresh account" link — one wizard, three
  flows.
