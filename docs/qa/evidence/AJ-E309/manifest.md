# Evidence manifest — AJ-E309

Task: `docs/tasks/E3-smart-onboarding.md` AJ-E309 (first-run policy
choice, renewal dates, expert hatches, onboarding metrics).
Branch: `stage3/e3-e309-policy-metrics`. 2026-10-18, reviewer
claude-opus (took over from zcode-primary's WIP; see "Course
corrections").

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Flow (a) asks the global pack policy inside the ≤ 5-click budget, merged with Confirm; no pre-selected option; "Decide later" stages no pack | Req.1 | The chooser rides the set-path Confirm (proposal block, above Accept) and the single-account Confirm (identity step) — no extra step. 4 radios (Strip / Keep / Ask each time / Decide later), each with a one-sentence consequence wired by `aria-describedby`; NOTHING checked until the user picks (copy lint renders the markup: no `checked`); Accept/Create stay disabled with a stated reason; ⌘↩ never answers it. Flow (a) = 4 primary clicks + 1 policy answer = 5 ≤ 5, 0 typed (`wizard-first-install.spec.ts`, `first-run-policy.spec.ts`). Asked only while NO global exists and only on the unlocked first-run flow — locked flows (c)/(d) keep their 4/3-click budgets and restate the active choice instead | PASS | 2026-10-18 | claude-opus |
| Global + per-Provider override storage; written only on an explicit click; changeable in Provider settings; never backfilled | Req.2 | Writes AJ-E208's store (`redaction_policy_set_global`) on the accept/create click only — picking a radio writes nothing (vitest + Playwright assert the write seam is empty until the click, then `["strip"]` exactly once; Decide later → never). `a_fresh_install_has_no_policy_and_onboarding_backfills_none`: onboarding writes accounts/Launch settings/funnel events and the global stays `None`/`unset`; restart keeps an explicit choice. Accounts → Pack secrets: the global radio group + one override select per Provider in use ("Use my choice" clears). Backend unset-blocks-the-first-pack pinned in `switch_graph_targets.rs` | PASS | 2026-10-18 | claude-opus |
| Optional renewal date per Confirm row; empty persists unknown and renders "expiry unknown"; never inferred | Req.3 | Each proposal item (set path) and the single-account Confirm carry `RenewalField`: collapsed "Expiry unknown · ＋ Renewal date", a labelled native `<input type=date>` (min = local today) with [Skip]; set → `account_set_renewal` after the account exists; empty → nothing written. Summary restates per Account ("Work: renews 2026-11-30 — we'll warn you before it" / "Side: expiry unknown"); a failed save is reported, never dropped | PASS | 2026-10-18 | claude-opus |
| D-12 research note filed; no auto-populate code | Req.4 | `docs/research/billing-renewal-sources.md`: per-Provider (Claude Code, Codex, ZCode, Gemini CLI) verdict "No source found" with vendor URLs + access dates (E602–E605 packets 2026-10-13…16, re-searched 2026-10-18), confidence labels, rejected candidate classes, revisit trigger. `UnknownRenewalSource` stays the only `RenewalSource`; the only writers are user clicks | PASS | 2026-10-18 | claude-opus |
| Every flow exposes [Expert setup…]: raw state dir (read-only + Reveal), env overlay editor, Launch-settings CRUD, [Import from folder…], copyable shell snippet | Req.5 | `ExpertSetup` (disclosure, `aria-expanded`/`aria-controls`) on the wizard's Detect, Confirm and Summary (flows a/c/d/f), the launch sheet (e), the New project sheet (b), and every Accounts card. Paths come from the backend only (`expert_setup_info`, `expert_reveal` — never caller-supplied; reveal falls back to the nearest existing folder INSIDE the data dir and creates nothing). The shell line comes from new adapter seams (`isolation_env_keys`, `shell_command`, quoted with `'\\''`) — the hard-coded `claude` login line is gone. Env overrides: `launch_recipe_env_get/set` with `validate_env_overlay` (shell-style names, one-line ≤ 1 KB values, ≤ 32 keys; isolation keys refused; credential-shaped keys refused by whole `_` segment — `…_API_KEY`/`…_TOKEN` no, `…_MAX_OUTPUT_TOKENS` yes); the Claude adapter applies isolation LAST so no overlay can repoint the config dir (backstop test). Launch-settings CRUD = [Manage Launch settings →] (launch/new-project sheets → Accounts) and the Accounts card itself | PASS | 2026-10-18 | claude-opus |
| Events emitted locally (opt-in only), Req.6 kinds, no paths/labels/credential-adjacent data | Req.6 | `juggler_core::funnel_events`: an ALLOW-LIST schema (18 kinds → exact keys → value shapes: counts, fixed vocabularies, lowercase codes) — free text cannot pass, so a path/label/email/token can't be smuggled through a permitted key; `FORBIDDEN_KEYS` asserted absent from the schema and rejected for every kind. JSONL in the app data dir, 2 × 5 MB segments (≤ 10 MB total). Wired: scan_started/completed{providers,found,health}, proposal_shown/accepted/customised, import_started/completed/failed{reason_code = the coded wire, never the message}, login_queued/completed/skipped/expired, expert_opened{surface}, first_launch (backend computes seconds_since_install, once per install), inference_shown/edit{field}, policy_chosen{choice}, and every flow step as flow_step. No network path exists | PASS | 2026-10-18 | claude-opus |
| Inference-override rate per field; > 30% flagged; local export | Req.7 | `override_rates` = edits ÷ shown (clamped), > 30% flagged; `report()` is deterministic markdown (kinds, steps in first-seen order, policy choices, time to first Instance, override rates). `funnel_export` → [Export report…] in the Expert panel saves it locally (Blob); dry-run stored at `docs/qa/e3/AJ-E309/funnel-dry-run.md` and regression-checked against the command layer (`funnel_dry_run_export_matches_the_stored_record`; `AIJ_BLESS=1` regenerates) | PASS | 2026-10-18 | claude-opus |
| Perf: policy step ≤ 1 s; settings write ≤ 50 ms; event ≤ 1 ms/event off the UI thread; store ≤ 10 MB | perf budget | The policy step is in-flow (no extra screen; the read is one settings lookup at mount). `funnel_event` is an async command on `spawn_blocking` — never the UI thread — and the UI never awaits it (fire-and-forget). Store cap pinned (`rotation_keeps_the_store_under_ten_megabytes`). Per-number wall-clock instrumentation not separately wired (bounded by construction) | PARTIAL-BY-DESIGN | 2026-10-18 | claude-opus |
| Quality gates: persistence round-trips (global + override + Decide-later = no staging); copy lint (no pre-selected radio); event-schema forbidden keys | quality gates | `redaction_policy` (E208, global/override/restart) + `onboarding_policy_metrics` 5/5 (no backfill, Decide-later writes nothing); vitest copy lint via `renderToStaticMarkup` (no `checked`); core `forbidden_keys_are_never_in_the_schema_and_always_rejected` + `values_cannot_carry_paths_labels_or_emails` + app-level refusal (coded `validation`, nothing written) + UI-level scan of every emitted event | PASS | 2026-10-18 | claude-opus |
| Playwright first-run spec choosing each option and asserting Summary restatement | evidence proof | `first-run-policy.spec.ts` 9/9: strip / keep / ask / decide_later → exact restatement + exact write seam; ≤ 5 clicks; renewal set/empty; Expert contents before/after; funnel events clean | PASS | 2026-10-18 | claude-opus |
| Visual: Policy step (unchosen/chosen/Decide later), Confirm with/without renewal, Expert open, Summary restatement at 1280×800 + 390×844; a11y radio group + date input labelled, axe 0 | visual testing | Gallery `41-policy` = 32 captures: 7 wizard states × 2 viewports × light+dark, plus Accounts → Pack secrets at 1280×800 + 1024×680 (the Tauri window minimum — the main shell has no phone layout) × 2 themes. a11y +4 (both themes): named radio group with 4 radios none checked, consequences as accessible descriptions, labelled date input, disclosure semantics, keyboard choice, axe 0 serious/critical | PASS | 2026-10-18 | claude-opus |
| Fresh install → two Accounts in parallel < 5 min with the policy step included | evidence proof | Every UI precondition is pinned (flow (a) 5 clicks, 0 typed; the funnel's first_launch records seconds_since_install). The wall-clock number is a real-machine measurement that needs a clean install and the maintainer's vendor sign-ins — it rides the maintainer dogfood window (live-user-data constraint), like AJ-E305/E306's PARTIAL-BY-DESIGN rows | PARTIAL-BY-DESIGN | 2026-10-18 | claude-opus |
| Research note reviewed against live Provider docs | evidence proof | Re-searched 2026-10-18 (Anthropic, OpenAI, Z.ai); Gemini from the 2026-10-16 packet. "Verified live" is n/a everywhere — there is no candidate source to verify | PASS | 2026-10-18 | claude-opus |

## Course corrections to the inherited WIP (recorded, not silent)

- **Duplicate policy store removed.** The WIP added
  `juggler_core::policy` (`strip|keep_warn|keep_silently`, its own
  settings keys) and `policy_global(_set)` commands — a second source
  of truth beside AJ-E208's store, which the switch flow actually
  reads. A first-run choice written there would never have reached a
  switch. Deleted; the wizard writes E208's store (task-text fix in
  the epic, same PR).
- The WIP's [Reveal in Finder] called `login_open_web` (the vendor's
  sign-in URL); the path and shell line were hard-coded to a Claude
  layout for every Provider; its copy used "Profiles" (D-10). All
  replaced by the backend-sourced Expert setup above.
- The WIP put the policy only on the manual single-account step; the
  primary flow (a) path (the set accept) never asked. Now both.

## Fixes made in passing (in-scope files, recorded)

- **AJ-E806 "light-theme first-scan contrast flash" — root cause
  fixed.** The reduced-motion rule set `transition-duration: 0.01ms`
  on every element; with the CSS default `transition-property: all`
  that made EVERY element transition inherited text colour after
  boot, so light theme briefly rendered dark-theme ink (CDP probe:
  dialog `color` = dark ink while `--color-ink` = light ink, zero
  web animations). Now `transition: none !important` under
  reduced motion; the dialog has the correct ink on the first frame.
  The confirm-rescan protocol stays in place as a guard, but this
  load-race class should no longer reproduce.
- Accounts screen: heading "Profiles" → "Accounts", "harness login"
  → "Provider sign-in", Launch-settings modal copy ("Profile name",
  "Create profile") → Launch settings + the native folder picker
  (its placeholder still said the picker "ships with AJ-E306");
  launch-sheet footer "the profile's default" → "the Launch
  settings' default folder" (D-10/D-02). The terminology lint now
  catches capitalised `Profile(s)` in JSX text (it was lowercase-only,
  which is how the heading slipped through).
- Accounts screen order: the user's accounts first, then Pack
  secrets, then the Beta-validation table (it used to lead the page).
- Wizard: when Accept is clicked before the scan finishes, the
  per-item accept progress now shows (it was hidden behind "Checking
  your machine…").
- Single-account Summary: "with a “Main” profile" → "with default
  Launch settings".

## Test counts

- `cargo test -p juggler-core --lib funnel_events`: 6/6.
- `cargo test -p juggler-app --test onboarding_policy_metrics`: 5/5
  (new); `--test redaction_policy` unchanged and green.
- vitest: 311/311 (new `first-run-policy.test.tsx` 20/20; wizard
  21/21 with the flow-(a) contract now counting the policy answer).
- Playwright: 596/596 full suite (new `first-run-policy.spec.ts` 9/9, gallery
  `41-policy` +32, a11y +4; 28 older specs that pass the unlocked
  wizard's Confirm now answer the policy via the shared
  `answerPolicy` fixture).
- `just verify`: green (fmt → clippy -D → test → doc → evidence →
  terminology). One environmental failure on the first run, recorded
  per AJ-E806: `juggler-app --test ports` binds real :5173/:8000 and
  my own Vite dev server (opened for visual review) held :5173; the
  panicked test then orphaned its two `http.server` helpers, which
  were identified by command line + start time and stopped. After
  that the test passed and the full verify ran green. Test-hygiene
  note: that test leaks its helpers when it panics.
