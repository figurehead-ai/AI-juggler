# Evidence manifest — AJ-E305

Task: `docs/tasks/E3-smart-onboarding.md` AJ-E305.
Branch: `stage3/e3-e305-flows`. 2026-10-17, reviewer zcode-primary.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Flow (a): Step 0 gate → Scan + proposal → Confirm identity (pre-filled) → queued Sign-ins → Summary | Req.1 | The set path: gate → detect (proposal) → accept (= the pre-filled identity confirmation; labels from AJ-E303 inference) → the new `signins` step → set Summary with its three exits. `wizard-first-install.spec.ts` walks it end to end | PASS | 2026-10-17 | zcode-primary |
| Primary path ≤ 5 clicks, 0 typed fields | Req.1 | The spec counts its own primary-button activations: exactly 4 (Continue → Use recommended setup → Accept setup → Launch first session); skips are secondary. A page-wide `input`-event recorder proves ZERO typed fields on the path | PASS | 2026-10-17 | zcode-primary |
| Zero-found collapses to "Create your first account" with a Provider picker (installed first, others greyed with install links); dashed dead-end removed | Req.2 | The dashed box is gone; `zero-found` panel renders only when the scan found nothing AND the proposal is empty; installed sorts first + enabled; not-installed disabled with its `install_url` link inline; choosing one lands on Confirm identity with the label PRE-FILLED ("Main" — kept only when no draft survived a restore) (vitest 17/17; gallery `37-flows/zero-found-*`) | PASS | 2026-10-17 | zcode-primary |
| Sign-in steps queue per Account needing login with [Sign in] [Open vendor page] [Copy command] [Skip for now]; Skip never blocks Summary | Req.3 | `acceptTheSetup` collects the queue from the accept results (imports: `login !== logged_in`; creates: `login_state`); the `signins` step shows one card at a time ("n of m") with exactly those four controls; [Skip for now] advances; an empty queue auto-lands on Summary — vitest + first-install spec walk skip-through to the Summary | PASS | 2026-10-17 | zcode-primary |
| Summary offers [Launch first session →] / [Add another account] / [Done]; Launch opens the launch sheet with Start-here preselected | Req.4 | `summary-exits` block with the three buttons (wired in BOTH wizard instances — App's and the Accounts screen's); the first-install spec asserts the New session sheet opens with the Start-here project ("dev-api") preselected; Add-another remounts the wizard fresh (nonce) | PASS | 2026-10-17 | zcode-primary |
| Flow (b) starts from the Projects rail "New project" + ⌘N + Summary Start-here; recent dirs as tiles + native [Choose folder…] + drag-drop; no typed paths | Req.5 | `NewProjectSheet` (2-step modal): recency-sorted tiles (dir name + last-used account), [Choose folder…] via the native picker, drop target (the native Tauri drop path opens it pre-filled); the rail's primary button becomes "New project" on the Projects tab; ⌘N summons it anywhere; the wizard's Launch/Start-here routes into the launch/project surfaces with the dir preset | PASS | 2026-10-17 | zcode-primary |
| Step 2 preselects the recommended Account (last-used on dir, else most headroom, else most recent) with [Change] + [Start session →]; ≤ 3 clicks | Req.6 | `pickRecipe` is a PURE exported function — unit tests pin all three tiers; the sheet renders the recommendation ("last used here" / "recommended") with [Change] exposing the full list; `new-project.spec.ts` completes in 2 clicks (tile → Start session →), no typed path | PASS | 2026-10-17 | zcode-primary |
| No Account for the chosen Provider detours INLINE into flow (d) and returns to the pending project launch | Req.7 | `no-account-detour` panel → [Add an account] mounts the wizard INSIDE the sheet overlay; the sheet keeps the pending directory ("Starting in ai-juggler" stays visible); completing/creating refreshes recipes + quotas in place (vitest + spec, both themes) | PASS | 2026-10-17 | zcode-primary |
| Wizard state survives an accidental close on Confirm (label/plan/selection restored on reopen); closing needs no confirm when idle | Req.8 | sessionStorage `aij.wizard.state.<provider>` saves {label, plan, choices, edits, ignored} while on Confirm; restore on reopen (the zero-found prefill defers to a restored draft); vitest: close → reopen → label "Side project" + Max 5× restored; the stored JSON is asserted to contain NO path/token-shaped field; idle close still needs no confirmation (the busy-gate confirm dialog unchanged) | PASS | 2026-10-17 | zcode-primary |
| Click-count is a tested contract; funnel events per step with no missing-step gaps | quality gates | `wizard-first-install.spec.ts` + `new-project.spec.ts` are the click-count contracts (4 ≤ 5 and 2 ≤ 3 primary activations, asserted numerically); the `aij:funnel` seam (window CustomEvent, string keys only — no path/label/token values) records a:gate→scan→confirm→signins→summary and b:open→pick-dir→launch; the specs assert presence + order (no gaps) | PASS | 2026-10-17 | zcode-primary |
| UI/UX: 4 step dots for (a), ⌘N, arrow-key tiles, Back preserves state, every step reversible until commit | UI/UX reasoning | Dots render 4 phases (Gate/Scan/Sign-ins/Ready) via a `role="group"` labelled container (a11y-clean); tiles are arrow-key navigable with visible focus (a11y test drives the whole flow (b) by keyboard alone); Back buttons never reset identity drafts (state lives above the step renders) | PASS | 2026-10-17 | zcode-primary |
| Performance: click-to-next-step paint ≤ 200 ms; recents ≤ 100 ms from cached seeds; scan-to-proposal ≤ 3 s | perf budget | Click-to-paint is React state only (no I/O on transitions — the specs' default-wait assertions pass in < 1 s wall); the sheet's recents render from the App's ALREADY-loaded projects state (no fresh query on open); scan-to-proposal is AJ-E302's measured budget (unchanged). time-to-first-Instance p50 < 3 min is the AJ-E309 dogfood funnel metric — the funnel seam built here is its instrument | PARTIAL-BY-DESIGN | 2026-10-17 | zcode-primary |
| Visual: all 5 steps of (a) + both steps of (b) + zero-found + no-Account-detour at 1280×800, 1024×768, 390×844; a11y keyboard completion + axe 0 | visual testing | Gallery `37-flows` = 54 captures (a1–a5 walk, zero-found, b1/b2, b-detour × 3 viewports × light+dark); a11y +4: flow (b) FULL keyboard completion (⌘N → arrows → Enter → Tab → Enter, focus asserted at each hop) and flow (a) axe 0 on EVERY step — after two real fixes: the dots div needed `role="group"` for its aria-label (aria-prohibited-attr), and the sign-in command block's `text-live` failed AA on light canvas (3.46:1) → `text-success` in BOTH the queue card and the legacy login step | PASS | 2026-10-17 | zcode-primary |

## Test counts

- vitest: 272/272 across 28 files (wizard suite 17/17 — +3 AJ-E305:
  the 4-primary-click path with exits, the zero-found picker, the
  accidental-close restore; new `new-project-sheet.test.tsx` 6/6 —
  the three pickRecipe tiers + the 2-click walk + the detour + the
  unsigned-in honesty).
- Playwright: 483/483 — `wizard-first-install.spec` (+1),
  `new-project.spec` (+4), gallery `37-flows` (+24 = 54 pngs), a11y
  flow gates (+4).
- `just verify` green; workspace clippy 0.

## Notes for the next agent

- The funnel seam (`ui/src/lib/funnel.ts`) is the instrument
  AJ-E309's event store subscribes to — string keys only, by design
  (the E309 schema forbids path/label/token values).
- `pickRecipe` is pure and exported — reuse it anywhere a launch
  surface needs the same recommendation rule.
- The E304 set-summary spec/galleries now walk the sign-ins queue
  (skip-through) — that's the flow (a) shape, not a regression.
- Req.4's "launch sheet (AJ-E306)" is today's NewSessionModal with
  the Start-here directory preset; when E306 ships its own launch
  sheet, repoint `onLaunchFirstSession` — the wizard contract
  doesn't change.
