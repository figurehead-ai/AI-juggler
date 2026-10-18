# Evidence manifest — AJ-E1202

Task: `docs/tasks/E12-optional-account.md` AJ-E1202 (account client).
2026-10-18, reviewer claude-opus. Decision D-18, ADR-0014; the wire
shapes follow the Figurehead API contract.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| No account → no call to figurehead.ai | Req.1 | `juggler-app --test figurehead :: signed_out_is_silent` — launch-time resume, status, Fleet surfaces, a refresh, a poll and an unlink attempt while signed out make **zero** requests on a recording transport; the HTTP agent isn't even constructed until the user links | PASS | 2026-10-18 | claude-opus |
| Browser linking: request, open approval page, show code, poll at `interval`; approved / denied / expired / slow down / not found | Req.2 | `juggler-account --test client :: a_browser_link_starts_polls_and_delivers_the_token_once`, `every_poll_answer_maps_to_a_plain_outcome`; app: `polling_never_runs_faster_than_the_interval` (5 early polls → 0 requests). A lost poll answer is never re-polled automatically (the token is delivered once): `a_lost_poll_answer_is_never_retried`. The approval page opens only if its URL is on the service's own host | PASS | 2026-10-18 | claude-opus |
| Pairing-code linking, case and dashes ignored, plain reason for a bad code | Req.3 | `a_pairing_code_is_normalised_and_a_bad_one_says_so`; app `a_pairing_code_links_and_a_wrong_one_says_so` | PASS | 2026-10-18 | claude-opus |
| Token only in the Keychain (`ai.figurehead.juggler`, account = device id) | Req.4 | macOS: `security-framework` generic-password item; other platforms refuse to link ("needs the macOS Keychain"). `a_browser_link_stores_the_token_only_in_the_secret_store` scans **every file** under the data dir (database, WAL, settings) for `fhd_` and `fhp_` — absent. `DeviceToken`, `LinkRequest` and `HttpRequest` never print a secret in `Debug` (`no_debug_output_ever_shows_a_secret`). Tests use an in-memory store; the real Keychain is never touched by tests | PASS | 2026-10-18 | claude-opus |
| Headers, 15 s timeout, bounded backoff | Req.5 | Real transport against a local socket: `the_real_transport_sends_the_contract_headers` (bodiless `DELETE` carries `Content-Type: application/json`, `User-Agent: AIJuggler/<version>`, bearer). 15 s global timeout, no redirects; retries after 0.5/1/2 s on network errors and `5xx`, then gives up (`network_errors_and_5xx_back_off_a_bounded_number_of_times`). TLS: rustls (ring) with the OS trust store via `rustls-native-certs` (no bundled CA list — keeps the license policy unchanged) | PASS | 2026-10-18 | claude-opus |
| `401` anywhere → token deleted, toggles off, local data kept, notice; Unlink cleans up even offline | Req.6 | `a_401_unlinks_locally_and_keeps_local_data` (token gone, every toggle off, "This Mac was unlinked from Figurehead.", a local label survives); `unlink_cleans_up_even_offline` (4 network failures, still signed out locally) | PASS | 2026-10-18 | claude-opus |
| Account at launch (when linked), after linking, every 6 h; cached; entitlements drive features; `404` → Phase-1 defaults; `402` typed | Req.7 | Launch: `resume_if_linked` (no-op signed out); after linking: `adopt`; a background refresher checks every minute and refreshes after 6 h. `the_account_drives_entitlements_and_unknown_keys_are_kept_not_trusted`; `unlinked_and_upgrade_required_are_typed`; app status lists only known, available features with `access` = `included` / `upgrade` / `not_included`; `a_feature_runs_only_when_entitled_and_turned_on` (`feature_on` is the single gate). Features are never hard-coded free or paid | PASS | 2026-10-18 | claude-opus |
| Rename → `PATCH devices/self` | Req.8 | `rename_and_unlink_hit_devices_self_with_json_even_without_a_body`; e2e below | PASS | 2026-10-18 | claude-opus |
| Debug builds honour `AI_JUGGLER_FIGUREHEAD_URL`; release builds always production | Req.9 | `juggler_account::base_url()` (`cfg!(debug_assertions)` gate, http/https only) | PASS | 2026-10-18 | claude-opus |
| End to end against a local Figurehead development server | evidence | `AIJ_E2E_FIGUREHEAD_URL=http://localhost:5173 cargo test -p juggler-account --test live_server -- --ignored --nocapture`, approved in a browser on the local server's test user: linked after 12.6 s; account plan `Free`, live entitlements (`alerts, cloudBackup, configSync, fleetDiscovery, transcriptVault, usageSync`); rename applied; after unlink the old token answers `401` | PASS | 2026-10-18 | claude-opus |
| One production link approved by the maintainer | evidence | Needs the maintainer's account — not done from here | PARTIAL-BY-DESIGN | 2026-10-18 | claude-opus |

## Test counts

- `cargo test -p juggler-account`: `client` 12/12 (+1 opt-in live test).
- `cargo test -p juggler-app --test figurehead`: 7/7.

## Naming

In AI Juggler an "Account" is a Provider Account, so the app's side is
named `figurehead_*` throughout (commands, state, settings keys).
