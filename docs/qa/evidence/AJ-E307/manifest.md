# Evidence manifest — AJ-E307

Task: `docs/tasks/E3-smart-onboarding.md` AJ-E307.
Branch: `stage3/e3-e307-import-provenance`. 2026-10-17, reviewer zcode-primary.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Account persists import provenance (canonical source path + origin + imported-at); migration backfills as provenance-unknown, never guessed | Req.1 | `Account` gains the three fields; V21 migration adds the columns (existing rows stay NULL — the honest backfill); repo insert/update/map wired; `AccountInfo` wire carries them; round-trip test pins insert + update persistence (storage_roundtrip) | PASS | 2026-10-17 | zcode-primary |
| A provenance-matched source is blocked in Detect with "Already imported as «Work»" + [Open it] | Req.2 | `account_import` canonicalizes and rejects duplicates with coded `already_imported` (wire code + label pinned in test); the wizard renders the block at identity with [Open it] (role="alert", vitest + gallery); no second account is created (pinned) | PASS | 2026-10-17 | zcode-primary |
| Confirm-source shows path, origin badge, total size, per-folder breakdown before any copy | Req.3 | New `source_inspect` command → adapter `inspect_source` (metadata-only, bounded 50k-file walk, truncation flag) → the wizard's confirm-source panel: path, origin chip, file count + total, the breakdown as real list markup; vitest + a11y pin the list semantics | PASS | 2026-10-17 | zcode-primary |
| Over-2-GiB sources offer [Import settings only — skip history] + [Add fresh]; the raw ceiling string never reaches the UI | Req.4 | The UI computes oversize from `total_bytes > ceiling_bytes`; the footer REPLACES Import with exactly that pair; `import_settings_only` skips projects//todos/ with sized skip notes; vitest asserts "ceiling"/"2 GiB" NEVER appear — the copy says "The history in this folder is large" | PASS | 2026-10-17 | zcode-primary |
| Non-config folder → plain reason + [Pick a different folder]; vanished source → "That setup moved — [Rescan]" | Req.5 | `is_config: false` renders the rejection with role="alert" + the picker button (vitest + gallery); import-time NotFound maps to the moved state with [Rescan] returning to Detect (rescan = report reset) | PASS | 2026-10-17 | zcode-primary |
| Partial failure cleans up; orphan managed dirs never accumulate silently | Req.6 | Mid-copy failures (copy or parent-create) remove THIS session's partial managed dir before returning — safe by construction (the import refuses an existing dest, so anything there mid-call is ours; the plan's marker-first trap closed by session scoping); injected-permission test pins the recovery copy + the absent orphan | PASS | 2026-10-17 | zcode-primary |
| No adopt-in-place; copy-import is the only default path | Req.7 | No "just track it" affordance exists anywhere; `import_claude_config` documents the copy as the only default-path mode; no adopt code path added | PASS | 2026-10-17 | zcode-primary |
| Keychain re-login note at Confirm-source time | Req.8 | `keychain_bound` (signed in + no credentials file) rides `source_inspect`; the panel renders the keychain note BEFORE any copy (vitest + gallery show it on the normal panel) | PASS | 2026-10-17 | zcode-primary |
| Perf: walk ≤ 2 s for ≤ 50k files; provenance lookup ≤ 10 ms; progress ≥ 4/s; cancel ≤ 500 ms | perf budget | The walk is bounded by the 50k cap + metadata-only (`truncated` flags the cap honestly); the provenance check is one indexed accounts scan at import; copy progress/cancel ride the existing wizard busy states — per-number wall-clock instrumentation not separately wired (cache/bounded by construction, recorded) | PARTIAL-BY-DESIGN | 2026-10-17 | zcode-primary |
| Quality gates: source-immutability, duplicate-import, oversize-degraded, rollback tests | quality gates | ALL four: immutability tree-snapshot across full+degraded+inspect (adapters); duplicate block (app command, wire code + no-second-account); oversize fixture → degraded import succeeds with settings but ZERO transcript files (adapters + command `degraded` flag); rollback (injected permission failure → cleanup + absent orphan) | PASS | 2026-10-17 | zcode-primary |
| Visual: Confirm-source (normal/oversize/non-config), duplicate-blocked at 1280×800 + 390×844; a11y breakdown list + role=alert + axe 0 | visual testing | Gallery `39-import` = 16 captures (4 states × 2 viewports × light+dark) incl. the manual-folder entry path; a11y +2 themes: list semantics (countable `<li>`), sizes as text, axe 0 after fixing three REAL pre-existing contrast failures on the identity step (text-faint → text-muted on the label/Plan/hints — first scan of that step) | PASS | 2026-10-17 | zcode-primary |
| Live dogfood: import the maintainer's real cc-switch set, size/provenance table | evidence proof | NOT RUN this task: importing the maintainer's live cc-switch set touches live user data (the standing constraint forbids it without the maintainer driving). The dogfood table lands with the maintainer's next interactive session — the machinery (inspect/provenance/degraded) is fully tested on fixtures | PARTIAL-BY-DESIGN | 2026-10-17 | zcode-primary |

## Test counts

- `cargo test -p juggler-adapters --test claude_environment`: 12/12 (+4:
  breakdown honesty, degraded differential, rollback cleanup,
  immutability); adapters suites total 82 passed.
- `cargo test -p juggler-app --test onboarding`: 10/10 (+2: duplicate
  block, provenance+degraded+inspect round-trip); import_t3_profiles
  updated + green.
- vitest: 285/285 (wizard 21/21: +4 E307 — panel/breakdown, the
  degraded pair with no-ceiling-string, non-config rejection,
  duplicate [Open it]).
- Playwright: 520/520 (gallery 39-import +16; a11y +2).
- `just verify` green; workspace clippy 0; `pnpm build` green.

## AJ-E806 flake record (2026-10-17, updated)

- `juggler-app --test recovery :: crash_drill_readopts_verifies_and_reports`
  — first failed in full-`just verify` runs under suite load, then
  passed 3/3 standalone (10.8–11.0 s each, pre-restart). After the
  machine restart it now fails deterministically with the helper
  classified `unverifiable` instead of `left_behind` ("0 helpers, 2
  unverifiable" vs the expected "1 helpers, 1 unverifiable") — a
  process-metadata classification shift in an UNTOUCHED crate
  (E307 changes nothing in juggler-app/recovery; the drill's own
  spawned python3 helper reads as alive-but-unverifiable post-restart,
  consistent with the recorded liveness class where exe/start-time
  reads are environment-sensitive). Recorded per AJ-E806; CI (isolated
  Linux runners with their own recorded handling for this class) is
  the arbiter. All other suites green in the same verify runs.

## Notes for the next agent

- The manual-folder entry prefills "Main" (0 typed fields); the label
  stays editable at Confirm.
- `inspect_source`/`import_account_degraded` are trait methods with
  Unsupported defaults — new adapters opt in explicitly.
- The maintainer dogfood table (real cc-switch import) is the one
  open evidence cell — schedule it with the maintainer driving.
