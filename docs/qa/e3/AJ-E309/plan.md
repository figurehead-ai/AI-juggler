> **Superseded (2026-10-18).** This was the inherited WIP plan. Its
> "core policy storage" step was reverted (it duplicated AJ-E208's
> store) — see `docs/qa/evidence/AJ-E309/manifest.md` "Course
> corrections" for what shipped instead. Kept as the historical record.

# AJ-E309 WIP plan (started 2026-10-18, branch stage3/e3-e309-policy-metrics)

DONE + pushed (core + app):
- `juggler_core::policy` — PackPolicy{Strip,KeepWarn,KeepSilently},
  global + per-Provider override storage (empty-set = delete =
  inherit), active_policy resolution, NO default ever written; tests
  4/4 (no-choice-is-none, roundtrip+restatement, override wins +
  clear inherits, corrupt→none). Restatement copy lives there.
- `juggler_core::funnel_events` — FunnelEvent{kind,at_ms,fields},
  FORBIDDEN_KEYS enforced at insert (path/label/token/dir/source/
  account/email/key), JSONL store w/ 10MB rotation, corrupt-line
  skip, override_rates (edits/(shown+edits), >30% flagged); tests
  3/3. CoreError gained Io + Validation variants (display attrs).
- App commands (registered): policy_global / policy_global_set
  (parse-validated) / funnel_event / funnel_export (markdown table +
  override rates; local only).

REMAINING (UI + docs + gates):
1. Research note `docs/research/billing-renewal-sources.md`
   (per-Provider: sanctioned API/SDK evidence URL + access date or
   "no source found"; NO auto-populate code until verified live).
2. Bridge: policyGlobal/policyGlobalSet/funnelEvent/funnelExport +
   `__AIJ_POLICY__` seam for specs.
3. Wizard UI:
   - Policy panel MERGED into the Confirm/identity step (inside
     E305's ≤5 budget — it rides the accept, no extra step): radio
     group, NO pre-selected option, "Decide later" (stages no pack,
     persists nothing), consequence-per-option copy; restatement on
     the set Summary ("Packs: strip secrets (all Providers)").
     NO ⌘↩ on policy (deliberate choice). Write ONLY on explicit
     click (the accept click counts as the explicit click).
   - Renewal date (optional) on the Confirm row: native date input +
     Skip; empty persists None → "expiry unknown" (D-12; wire through
     account create/import params — extend account_import +
     create_account impls or an account_set_renewal command after
     creation; check existing renewal set command in Profiles).
   - [Expert setup…] hatch on the identity step: read-only state-dir
     path + [Reveal in Finder] (opener), env overlay editor (stub
     honestly if Profiles has one), Launch-settings CRUD pointer,
     [Import from folder…] (E307 entry exists on Detect — link it),
     copyable shell snippet (CLAUDE_CONFIG_DIR=... claude).
   - Event wiring: extend ui/src/lib/funnel.ts — when hasBackend,
     funnelStep also invokes funnel_event(kind, safe fields); emit
     scan_started/completed{providers,found}, proposal_shown/
     accepted/customised, import_started/completed/failed{reason_code},
     login_queued/completed/skipped/expired, expert_opened,
     first_launch{seconds_since_install}. inference_shown/edit from
     the E303 edit paths (field only). NO paths/labels ever.
4. Provider settings surface: the policy override control in
   Profiles provider view (schema exists; simple select+clear).
5. Tests: vitest (policy panel no-preselect + decide-later stages
   nothing + restatement; renewal optional→unknown; expert hatch
   contents), Playwright first-run spec (choose each option →
   Summary restatement; Decide-later), copy lint (no checked radio
   in markup), event-schema test = core's forbidden-keys (done).
6. Funnel dry-run: run the flow with backend? No backend in specs —
   the export fixture: call funnel_export against a temp state in a
   Rust test and store its output as docs/qa/e3/AJ-E309/
   funnel-dry-run.md (honest fixture note).
7. Gallery 41-policy (unchosen/chosen/decide-later, confirm row w/
   + w/o renewal, expert panel, summary restatement × 1280×800 +
   390×844 × light+dark) + a11y (radio group labelled, date input
   labelled, axe 0).
8. Gates → evidence docs/qa/evidence/AJ-E309/manifest.md → flip →
   indexes → PR → CI → merge.

Live-provider verification of the research note URLs + the <5min
fresh-install two-account metric = maintainer dogfood moments
(record PARTIAL-BY-DESIGN like E307/E308).
