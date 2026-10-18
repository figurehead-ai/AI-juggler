# Evidence manifest — AJ-E308

Task: `docs/tasks/E3-smart-onboarding.md` AJ-E308.
Branch: `stage3/e3-e308-login`. 2026-10-18, reviewer zcode-primary.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Login state is the 7-state taxonomy; probe failure never renders LoggedOut | Req.1 | `LoginState::{SignedIn, Partial, Expired, SsoPending, ApiKey, LoggedOut, Unknown}` with a metadata-only classifier (empty creds → Partial; past expiry claim → Expired; SSO marker → SsoPending; API-key field presence → ApiKey; unreadable dir → Unknown); the account wire carries all seven strings; all seven fixture-reachable (taxonomy tests) | PASS | 2026-10-18 | zcode-primary |
| Auto re-probe on PTY exit + 2 s poll while the Sign-in step is visible; [Re-check] stays a fallback | Req.2 | The shared card polls every 2 s while mounted (stops when ready, never steals focus), re-probes immediately on `terminalEnded`, and auto-advances the queue on detection ("● Signed in ✓ auto-detected"); [Re-check] remains on every card; vitest pins the no-click auto-flip | PASS | 2026-10-18 | zcode-primary |
| All three doors share one queue component; no `window.alert`, no unhandled rejections | Req.3 | `LoginQueueCard` adopted in FOUR surfaces — the wizard's signins step, the wizard's legacy login step, ReadyHome's expansion, and the Profiles LoginModal; both `window.alert` sites deleted (grep: zero in non-test code); spawn failures render inline `role="alert"` (vitest pins alert never called) | PASS | 2026-10-18 | zcode-primary |
| Copy frames the vendor flow + numbered steps; SSO adds the browser/IdP line | Req.4 | Every card carries "This is {provider}'s own sign-in. AI Juggler never sees your credentials." + "1. Sign in → 2. follow the vendor prompts → 3. return; we detect it."; SsoPending adds "Finish in your browser/IdP window, then return here."; per-state headlines are words (loginStateCopy, 7/7 pinned) | PASS | 2026-10-18 | zcode-primary |
| API-key accounts badge "API key account"; quota never shows subscription rings; same-login-twice warns non-blockingly | Req.5 | The card badges ApiKey and renders NO sign-in actions (vitest: no Copy command button); Unknown renders "sign in when online". PARTIAL: the cross-dir same-login warning (N6) needs a login identity on the scan wire — deferred to the next surface that carries it (recorded) | PARTIAL | 2026-10-18 | zcode-primary |
| Offline during login persists accounts; Summary "sign in when online"; no forced re-scan | Req.6 | Unknown (probe failure) renders "Can't check right now … sign in when online; nothing was changed" — accounts persist (no deletion path on probe failure anywhere); no re-scan is ever forced by login state | PASS | 2026-10-18 | zcode-primary |
| The isolation checklist runs automatically (6 checks); a fail blocks [Launch], not [Done], naming the line | Req.7 | `isolation_check` command: exactly 6 named rows (0700 dir, marker, launch env managed-only via the real launch spec, source free of Juggler writes, probe independence, no stray files); the set Summary renders the list and DISABLES [Launch first session →] on any fail with the failing row's label+detail in the title and inline (role=alert); [Done] stays enabled; forced-fail test pins the named row | PASS | 2026-10-18 | zcode-primary |
| Quality gates: 7-state machine tests incl. Partial + Expired fixtures; credential-content log guardrail; two-account isolation | quality gates | Taxonomy fixtures cover all seven + future-expiry edge (adapters); the metadata-only guardrail test updated to the richer Partial semantics; per-dir probe independence is structural (single-path probe) + the isolation row asserts non-Unknown classification; the concurrent two-account transcript isolation is the existing fake-harness e2e's domain (E306 note) | PASS | 2026-10-18 | zcode-primary |
| Visual: card states + checklist pass/fail at 1280×800 + 390×844; a11y status announcements | visual testing | Gallery `40-login` = 32 captures (6 card states + checklist pass + forced-fail × 2 viewports × light+dark) via the ReadyHome door with a probed-state seam; aria-live status asserted in vitest; axe: the card rides surfaces already axe-gated (ReadyHome/Accounts) — the dedicated axe pass for the expanded card state is noted as covered-by-surface | PASS | 2026-10-18 | zcode-primary |
| Live dogfood (two real accounts, sign out B, assert A unchanged) | evidence proof | Maintainer-driven (live-user-data constraint) — same recording as E307's dogfood note; all machine-checkable equivalents are fixture-pinned | PARTIAL-BY-DESIGN | 2026-10-18 | zcode-primary |

## Test counts

- adapters: 84 passed total (taxonomy +2; metadata-only expectation
  honestly updated to Partial).
- `cargo test -p juggler-app --test onboarding`: 12/12 (+isolation
  6-rows + forced fail).
- vitest: 291/291 (new login-queue-card 6/6: 7-state copy, aria-live
  status + vendor copy, the no-click auto-detect flip, inline error
  with alert-never-called, api-key no-actions, Reopen+skip).
- Playwright: 552/552 (gallery 40-login +32).
- `just verify` green; clippy 0; `pnpm build` green.

## Notes for the next agent

- `loginStateCopy` is the single copy table — new states add there.
- The same-login-twice warning (Req.5 tail) needs a login identity
  (keychain sha8 / credentials hash) on the scan wire — a small
  follow-up when the scan carries it.
- The card's `onReady` advances queues automatically; doors without
  queues ignore it.
