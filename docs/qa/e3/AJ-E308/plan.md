# AJ-E308 WIP plan (started 2026-10-18, branch stage3/e3-e308-login)

Recon (verified this branch):
- Login today is BINARY: `LoginState` (claude_code.rs:386:
  LoggedIn/LoggedOut, metadata-only probe: non-empty
  .credentials.json OR macOS keychain `Claude Code-credentials-{sha8}`)
  → `LoginProbe` (adapter env crate, scan-side) → wire strings
  "logged_in"/"logged_out". The 7-state taxonomy (Req.1) is NEW:
  - ApiKey: `settings.json` carries an apiKey/OAuth-less key or
    env `ANTHROPIC_API_KEY` in the overlay — detect from settings
    metadata (key PRESENT, never its value).
  - Partial: `.credentials.json` EXISTS but parses to no usable
    token/empty object (abandoned OAuth — E303's "May be expired"
    hint already smells this; refine to empty-file → Partial vs
    stale mtime/expiry claim → Expired).
  - Expired: credentials file with a past expiry claim (parse
    metadata-safe: expires_at field presence only, never log content).
  - SsoPending: vendor session file present without credentials
    (claude.ai-sso marker / `.credentials.json` absent but
    `sso-session` present) — fixture-constructible.
  - Unknown: probe error (unreadable dir) — MUST NOT render LoggedOut.
- Three doors today: wizard login step + E305 signins step (shared
  patterns), ReadyHome onSignIn (App.tsx:656-674 — has a
  window.alert to replace), Profiles loginFor modal (Profiles.tsx
  ~500-520 — second window.alert). Req.3: extract ONE
  `LoginQueueCard` component (states + copy + error handling)
  used by all three; no alerts, no unhandled rejections.
- Req.2 (auto re-probe): wizard signins/login steps poll
  accountLoginState every 2 s while visible (setInterval effect,
  cleanup on unmount — never steals focus; aria-live status line
  "○ Waiting… / ● Signed in ✓ auto-detected"). PTY-exit re-probe:
  terminal.rs emits an exit event already? (terminal registry has
  `ended` flag — the App's loginTerminals track it; wire an
  onEnded → re-probe via the existing event or poll).
- Req.4 copy: one shared copy block (vendor-frame sentence +
  numbered steps; SSO adds the browser/IdP line).
- Req.5: ApiKey badge "API key account" + quota Estimated/no rings
  (AccountInfo needs login_state "api_key" propagation + ReadyHome/
  rail/launch-sheet badge); the same-login-twice warning: compare
  probe identities (keychain sha8 or credentials-file hash) across
  candidates — E307's SourceInspect could carry a login_id later;
  minimal: same keychain service name across two dirs → warn chip.
- Req.6 (offline): accountLoginState returns Unknown on probe
  failure → Summary renders "sign in when online"; no forced
  re-scan anywhere.
- Req.7 (isolation checklist, the big one): new command
  `isolation_check(account_id)` returning 6 named rows:
  1 managed dir 0700; 2 marker present; 3 launch env points ONLY
  at the managed dir (recompute launch_spec, assert env values);
  4 source tree unchanged after import (reuse E307's snapshot —
  store the pre-import snapshot? simpler: E307 immutability test
  covers import-time; the checklist re-hashes metadata of the
  source if provenance known); 5 per-dir probe independence (two
  accounts' probes don't cross — fixture-verified); 6 zero writes
  outside managed dirs (fixture canary file check). Any fail
  BLOCKS [Launch] (not [Done]) on the wizard Summary + shows the
  failing line.
- Quality gates: 7-state fixtures (all reachable: files/keychain
  stubs); log-scan guardrail (credential contents never logged —
  assert no probe output contains file CONTENTS); concurrent
  two-account transcript isolation (fake harness e2e exists —
  extend or point at it).
- Visual: gallery 40-login — Waiting/Partial/Expired/SSO/ApiKey/
  SignedIn + checklist pass + one forced fail × 1280×800 + 390×844
  × light+dark; a11y status-line aria-live + axe 0.
- Dogfood (two real accounts, sign-out-B-assert-A) = maintainer-
  driven (live-data constraint) — record PARTIAL-BY-DESIGN like
  E307.

Order: (1) LoginState 7-way + probe fixtures + wire strings;
(2) isolation_check command (6 rows) + tests; (3) LoginQueueCard
component + wizard/ReadyHome/Profiles adoption (kill both
window.alerts); (4) 2 s poll + PTY-exit re-probe + status line;
(5) ApiKey badge + same-login warning; (6) specs + gallery 40 +
a11y; (7) gates → evidence → flip → PR → CI → merge.
