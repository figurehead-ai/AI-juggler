# Evidence manifest — AJ-E1001

Task: `docs/tasks/E10-derived-workflows-and-beta.md` AJ-E1001
(per-Instance identity assertion, the wrong-account billing guard).
2026-10-18, reviewer claude-opus.

## How it works

Every adapter gains `probe_identity(account)`, which answers:
- `SignedIn { subject, label }` — a stable vendor account id (compared,
  never shown) plus a masked label (`o…@domain · Org`);
- `SignedOut`;
- `Unverifiable { reason }` — the default.

Claude Code's probe reads only `oauthAccount` in the config dir's
`.claude.json`. The Keychain item and `.credentials.json` are never
opened.

The first identity an Account is seen with is remembered, and shown as
"first seen" so a wrong first sign-in can be noticed. Before every
launch, switch, local switch-back, cross-machine handover and
cross-machine switch-back, the Account is probed again. A different
identity refuses the launch with the typed `identity_mismatch` error
("«Work Max» is signed in as … here, not … — nothing was launched").
There is no "continue anyway" anywhere: the bridge's `launchSession`
raises one app-level dialog (switch to the Launch setting that IS that
identity, check again, or cancel), and the backend refuses a mismatch
regardless of the UI. Every assertion is appended to
`identity_assertions` (schema v24): who ran where, when, at which
moment.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Identity asserted at launch; Provider + Account + model shown on the Instance before the first prompt | Req.1 | The rail row carries a focusable identity marker named in words (`Signed in as o…@work.example · Acme — verified` / `first seen` / `Not signed in yet` / `Identity unverified`) beside the Provider, Account disc and model; the inspector's Overview chips spell it out. `identity_guard :: a_matched_account_launches…` asserts `first_seen`, then `verified` with the masked label on the Instance row | PASS | 2026-10-18 | claude-opus |
| A mismatch pauses with expected vs detected and switch / re-probe / cancel; silent continuation never offered | Req.2 | Backend: `launch_profile` returns `identity_mismatch`, nothing spawns (the stub never runs), no Instance row is created. The check lists the Launch settings whose Account IS the detected identity ("switch selection"). UI: vitest `identity-guard.test.tsx` covers switch, check again, cancel, and a mismatch with no dialog mounted, which rejects and never continues; no "continue anyway" text exists | PASS | 2026-10-18 | claude-opus |
| Re-run on resume and switch-back; recorded in the session history | Req.3 | The same assertion guards local switches (`a_switch_to_a_mismatched_target_is_refused_before_anything_moves`: refused, no transcript lands), local switch-back, and the cross-machine handover and switch-back on the receiving Mac (refusal code `identity_mismatch`). History: `identity_assertions` rows carry the moment (`launch` / `switch` / `switch_back` / `handover`) | PASS | 2026-10-18 | claude-opus |
| Metadata only: no credential content read, copied or logged; the probe per Provider documented in the adapter contract | Req.4 | `claude_identity :: credential_content_never_reaches_the_probe_result` (a token in `.credentials.json` and a token-shaped field inside `.claude.json` never surface; the full email is never shown). SPEC §4 documents the probe contract and what Claude Code's probe reads; Codex shows `Identity unverified` until a verified source exists | PASS | 2026-10-18 | claude-opus |
| Probe < 1 s at launch; zero polling; < 100 ms added to resume | perf | A launch including the probe: **10–19 ms** (stub harness). The probe is one small JSON read, run only at launch / resume / switch — there is no polling | PASS | 2026-10-18 | claude-opus |
| Mismatched fixture blocks 100%; matched fixture launches clean | quality gate | `identity_guard` 3/3 (mismatch blocked in every path tested; matched and signed-out launch); `identity::tests` verdict table | PASS | 2026-10-18 | claude-opus |
| Visual: identity chip on the Instance row and the mismatch warning — 1280×800 + 1024×680, light/dark; actions focusable in order | visual / a11y | Gallery `51-identity` (rail-and-chip, wrong-account). Playwright `identity — rail marker and wrong-account pause — axe 0` (both themes): focus starts on "Use «Personal»", Tab reaches "Check again", Esc cancels | PASS | 2026-10-18 | claude-opus |
| Live dogfood on one real multi-account machine | evidence | Listed in `docs/qa/dogfood-backlog.md` (never run against a live account here — the programme's rule) | DEFERRED-TO-DOGFOOD | 2026-10-18 | claude-opus |

## Test counts

- `cargo test -p juggler-app --test identity_guard`: 3/3; `juggler-app` lib `identity::tests` 1/1.
- `cargo test -p juggler-adapters --test claude_identity`: 3/3; `juggler-adapter` `identity::tests` 1/1.
- `cargo test -p juggler-core` `identity` repo test 1/1; schema upgrade tests at v24.
- vitest `identity-guard.test.tsx` 5/5. Playwright `51 identity` 4, `identity … axe 0` 2.
