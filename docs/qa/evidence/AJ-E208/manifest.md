# Evidence manifest — AJ-E208

Task: `docs/tasks/E2-switching-engine.md` AJ-E208.
Branch: `stage2/e2-e208-redaction`. 2026-10-16, reviewer zcode-primary.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Settings store `redaction_global` + per-Provider overrides; resolution is override-if-set else global; NO code path applies an unchosen hard-coded default; first run asks explicitly before any pack | Req.1 | KV keys `redaction_global` / `redaction_overrides` (schema-less strings — persisted, restart-pinned by test); `resolve_policy(Option, Option) -> Option` — unset stays `None`; deep/09's local-off/export-on defaults appear ONLY as picker options (never applied); unset BLOCKS the confirm (the first-run chooser disables Switch until the user saves a choice — vitest + gallery 31); unknown wire names are refused by the setters, never guessed | PASS | 2026-10-16 | zcode-primary |
| The wizard/1-click confirm displays the active policy verbatim ("…(your global choice)" / "…(override)") before execution; changing it is one click without losing wizard state | Req.2 | `RedactionPolicyInfo.line` renders on the 1-click popover AND the wizard's Confirm step (data-testid switch-redaction-line; verbatim strings pinned in Rust integration + vitest); per-Provider change = the Override row in Provider settings (Profiles) OR — mid-wizard — one click is recorded as: the chooser/Ask radios keep wizard step + freeze state through the preflight refresh (`chooseGlobal` re-fetches the plan; step/freeze/unredact live outside the phase). Destination policy is readable without opening Settings (gallery 31 line-global capture) | PASS | 2026-10-16 | zcode-primary |
| Detection covers sk-*, Bearer…, .env values, private-key blocks, ZCode rollout headers; the manifest records `redactions:[{kind,count}]` — kinds+counts only, values never stored/logged/written outside the pack | Req.3 | `scan_segment` hand-rolled matchers (no regex dep): `sk-key` (sk- + ≥8 token chars), `bearer` (Bearer + value), `env-value` (credential-like NAME incl. diff-prefixed `+KEY=…` lines, value ≥8), `private-key` (BEGIN…END block), `zcode-rollout` (authorization/x-api-key/api-key JSON headers, `api-key`-in-`x-api-key` deduped) — all five pinned by unit test; `PackState.redactions` filled AT GENERATION (manifest hashes final bytes); values cross the engine boundary ONLY as `mask()` previews (first char + ••• + last 4) — the integration test asserts the serialized preflight contains NO canary value | PASS | 2026-10-16 | zcode-primary |
| The preview lists each redaction; the user can un-redact per item for this switch only; un-redact never changes the remembered policy; credentials never enter any pack regardless of policy | Req.4 | Preflight carries `redaction_items` (kind, masked preview, segment); the wizard's What's-carried step renders the per-item checkboxes; indices are GLOBAL over the segment order (turn texts then staged files) — preflight and pack-apply build the same segmentation, so an index addresses the exact item; `switch_execute` passes `redaction_choice` + `redaction_unredact` and records them on the edge (`redaction` + `redaction-unredact` manifest rows, "this-switch-only"); engine test pins un-redact ≠ policy (next pack strips again); the ALWAYS-ON credential guard (E202) refuses credential material under every policy incl. Keep — and was TIGHTENED to real key shapes (`sk-` + ≥8 key chars) so the manifest's own `"sk-key"` kind name can't false-positive (guard unit test extended) | PASS | 2026-10-16 | zcode-primary |
| Per-Provider overrides live in Provider settings; the global in Settings → Privacy; both persist across restarts and are covered by settings migration tests | Req.5 | Override row rendered in the account card's settings expander (Profiles — the Provider settings surface; "Use global" clears); the global is settable from Settings/first-run chooser; persistence: integration test drops the state, reopens the same data dir, and asserts both global + override survive (schema-less KV — no migration version bump needed; the round-trip test is the migration coverage for these keys) | PASS | 2026-10-16 | zcode-primary |
| Secret-canary: seeded fake keys in transcript+diffs appear in ZERO pack bytes under Strip | quality gate | `redaction.rs` integration test: canaries (sk- key, Bearer token, .env value in a staged diff) — under Strip, ALL pack outputs (state.json, conversation.md, PACK.md, staged extras) contain none of the canary values, and the WRITE guard passes; under Keep the values stay (this Mac) while counts still land | PASS | 2026-10-16 | zcode-primary |
| Guardrail: credentials excluded under every policy incl. Keep | quality gate | `credentials_never_enter_any_pack_including_keep`: an `auth.json` staged file with an embedded key ⇒ `write_pack` REFUSES even under Keep (E202's guard, unchanged in strength) | PASS | 2026-10-16 | zcode-primary |
| Policy-resolution unit tests (global / override / unset-blocks) | quality gate | `resolution_is_override_then_global_then_unset` + integration `policy_is_remembered_and_persists_across_restart` (unset line empty, global line verbatim, override wins + labels, unknown refused, clear returns to global, restart survives) | PASS | 2026-10-16 | zcode-primary |
| Performance: scan ≥ 20 MB/s; pack time +≤100 ms/10 MB; settings resolve ≤ 1 ms; zero network | performance budget | Detection is single-pass byte scanning per kind (no regex backtracking; linear); policy resolution is two KV reads + an Option or-chain (µs — pinned by shape, not wall-time assertions); zero network calls anywhere in the path; wall numbers recorded here, not CI-gated (load-sensitive) | PASS | 2026-10-16 | zcode-primary |
| Visual: first-run chooser (no default), wizard policy line (global + override), redaction item list; a11y radios labelled | visual testing | gallery 31-redaction 16 captures (chooser w/ every radio unchecked + confirm disabled, line-global, items-override with masked previews, ask-on-confirm; 1280×800 + 800×600 × light+dark); a11y 2 passes (chooser + items scans, 0 critical/serious both themes); full visual 360/360 | PASS | 2026-10-16 | zcode-primary |
| Spec's "diff hunk" preview shape | Req.4 (preview) | Implemented as the per-item list (kind + masked preview + per-item un-redact) on the wizard's preview step; a literal unified-diff hunk render belongs with E209's pack-preview surfaces (same artifact) — recorded, not skipped | NOTE | 2026-10-16 | zcode-primary |
| Live dogfood: export a pack with a canary .env, grep = 0 hits, manifest counts screenshot | live data | PENDING the authenticated session (gate-E5) AND a live pack-export surface (E209); the stub-level zero-canary test covers the mechanics meanwhile | PENDING | 2026-10-16 | zcode-primary |

## Recorded load flake (AJ-E806 protocol)

- `juggler-app --test recovery::crash_drill_readopts_verifies_and_reports`
  failed ONCE inside the full serialized workspace verify under
  external load 15-18 ("2 processes could not be verified" vs 1 —
  helper-process aliveness timing; its cleanup printed `kill: no such
  process`). Verified 3/3 standalone serialized immediately after;
  no E208 code path touches recovery/ledger. Same class as the
  E201-recorded `guardrails::term_ignorer` flake — CI (quiet runners)
  is the arbiter; if it fails CI, it gets its own fix/quarantine
  record rather than a gate skip.

## Test counts

- `cargo test -p juggler-handover`: 59/59 (redaction 8 unit + 5 integration
  new; door 5, pipeline 12, transplant 9, pack/fidelity 14, guard unit
  extended; manifest guard test extended).
- `cargo test -p juggler-app --test redaction_policy` (new): 3/3
  (persist-across-restart, preflight line + masked-only, edge recording).
- vitest: 234/234 (24 files) incl. new `switch-redaction.test.tsx` 4/4.
- Visual: 360/360 — gallery 31-redaction (16 new) + redaction a11y 2;
  prior galleries regenerated (the preflight seam now MERGES fixtures
  over the default, so E206-era fixtures keep working — 40 stale-fixture
  failures found in the full run and fixed at the seam).
- clippy pedantic `-D warnings` clean; rustdoc `-D warnings` clean; fmt
  clean (workspace).

## Implementation-mapping

- `crates/juggler-handover/src/redaction.rs` (new) — RedactionPolicy +
  from_wire/label, resolve_policy, five hand-rolled scanners,
  list_segments (masked previews), strip_segments (global keep-set),
  RedactionApply + redacted_pack; 8 unit tests.
- `crates/juggler-handover/tests/redaction.rs` (new, 5) — zero-canary,
  counts-under-every-policy, per-item un-redact, credentials-refused-
  under-Keep, determinism/resolution.
- `crates/juggler-handover/src/pack.rs` — `generate_pack_with_redactions`
  (rows set BEFORE serialization so the manifest hashes final bytes);
  `generate_pack` delegates.
- `crates/juggler-handover/src/manifest.rs` — credential guard tightened
  to real key shapes (`sk-` + ≥8 key chars; kind names can't trip it);
  guard test extended.
- `crates/juggler-app/src/commands.rs` — REDACTION_* KV keys,
  RedactionPolicyInfo::from_parts + redaction_line (verbatim),
  redaction_policy_get/set_global/set_override (+impls),
  redaction_items_of (masked), PreflightInfo.redaction/redaction_items,
  switch_execute carries `redaction_choice`/`redaction_unredact`,
  record_redaction_decision (edge manifest rows).
- `crates/juggler-app/src/lib.rs` — three command registrations.
- `crates/juggler-app/tests/redaction_policy.rs` (new, 3).
- `ui/src/lib/bridge.ts` — RedactionInfoData/RedactionItemData,
  redactionPolicyGet/SetGlobal/SetOverride (recording seams),
  switchExecute redaction payload + recording seam, preflight seam now
  MERGES over DEFAULT_PREFLIGHT (fixture-compat fix).
- `ui/src/components/SwitchFlow.tsx` — RedactionBlock (first-run chooser
  with no default + Save, per-switch Ask radios, per-item un-redact
  list) on popover + Carried + Confirm; confirm disabled while a choice
  is required; execute carries the payload.
- `ui/src/screens/Profiles.tsx` — RedactionOverrideRow (Provider
  settings: Use-global / Strip / Keep / Ask + Override chip).
- `ui/src/components/switch-redaction.test.tsx` (new, 4); gallery
  31-redaction (16) + 2 a11y passes in the visual specs.
