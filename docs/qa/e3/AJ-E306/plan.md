# AJ-E306 WIP plan (started 2026-10-17, branch stage3/e3-e306-flows-cde)

Recon findings (verified this branch):
- `AdmissionVerdict` (Ok/Tight/No) EXISTS at
  `crates/juggler-core/src/quota_forecast.rs:299` (`admission_verdict()`,
  SPEC §5.2 language) — NOT yet wired to a command or bridge; flow (e)
  Req.7 needs a read command + bridge fn + inline render (explain, never
  hard-block).
- `verified_against` is an adapter trait method
  (`crates/juggler-adapter/src/adapter.rs:35`); flow (c) cards need it
  surfaced (extend providers_discover wire or a new command) as the
  "verified range" badge; unverified = badge states it honestly.
- Desktop kind: adapters expose desktop_launch + isolation claim
  (fake-desktop provider in fixtures carries `isolation_label`
  "Single-account app — Juggler tracks, cannot isolate"); Req.3 badge +
  pre-selection single-instance limit statement come from this.
- Flow (e) base = NewSessionModal (E611 ranking rows already there);
  E306 shapes it into the sheet spec: Launch settings collapsed under
  "Customise ▸" (already renamed from Profile in E105 — grep for the
  word "Profile" as a noun must stay clean in rail/sheet copy),
  logged-out rows disabled + "Sign in first" one-tap detour RETURNING
  to the sheet, unknown headroom = text never %.
- Flow (d) = wizard with provider LOCKED (skip step 0's multi-row gate
  → straight to provider-scoped scan/identity; pre-filled
  collision-suffixed label already exists from E303/E304; the
  already-imported "Open it" affordance is E307 provenance — render
  E303's managed "Already in AI Juggler" inline for now).
- Flow (c) = new AddProviderSheet: adapter cards (install state +
  verified badge, [Install →] for missing, desktop kind badge + limit
  note) → provider-scoped scan → [Import found]/[Add fresh] → login
  queue (E305 signins) → summary [Launch]/[Add another on this
  Provider]. Entries: Accounts screen + Settings-ish surface.
- Click budgets (quality gate): c ≤ 4, d ≤ 3, e ≤ 2 — Playwright specs
  like E305's (count primary activations + funnel seam b/c/d/e flows).
- Gallery 38-flows: provider cards 4 states (installed/not/unverified/
  desktop) + launch sheet (OK/Tight/No, logged-out row, unknown
  headroom) × 1280×800 + 390×844 × light+dark.
- a11y: sheet focus trap + return focus + axe 0 (NewSessionModal
  pattern exists).

Next steps (in order): (1) wire admission verdict command+bridge;
(2) AddProviderSheet (flow c); (3) wizard provider-lock mode (flow d);
(4) launch sheet reshape (flow e) + detour-return; (5) specs + gallery
+ a11y; (6) gates → evidence docs/qa/evidence/AJ-E306/manifest.md →
flip → PR.
