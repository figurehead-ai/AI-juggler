# Evidence manifest — AJ-E704

Task: `docs/tasks/E7-fleet-at-launch.md` AJ-E704 (discovery and pairing
wizard). 2026-10-18, reviewer claude-opus. Contract: ADR-0013
"Architecture contract (AJ-E701)".

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| mDNS advertisement only while Link is on; off withdraws within 5 s | Req.1 | Link is **off by default** (`link_is_off_by_default_and_pairing_needs_it_on`) and only a user click turns it on (remembered; resumed at launch only if left on). `juggler_link::discovery` (`mdns-sd`) advertises `_aijuggler._tcp` with a random per-boot instance id, a synthetic host name and TXT `v` + keyed hint only. Live run on a Mac (`mdns_advertise_browse_and_withdraw`, `--ignored`): discovered and withdrawn within 1.96 s total, withdrawal 1.13 s (< 5 s). Link off stops the advertiser (goodbye packets), browser and node, and every peer reads `link_off` again (app test) | PASS | 2026-10-18 | claude-opus |
| Discovered machines listed as "Unnamed machine (<instance id>)"; labels/platform/fingerprint only inside pairing; unpaired connections dropped | Req.2 | `discovered_impl` lists only unpinned nodes as `Unnamed machine (a1b2c3d4)` (Playwright asserts the label); names and fingerprints arrive in the SPAKE2-confirmed exchange. Unpaired TLS peers are refused at Hello and audited (`hello_rejected/not_paired`, AJ-E703 `an_unpaired_console_and_a_machine_id_mismatch_are_refused`). Discovery is a convenience: if mDNS is unavailable, Link still runs and says "pair by address instead" | PASS | 2026-10-18 | claude-opus |
| User-initiated on both machines; 2-minute single-use 6-digit code as the SPAKE2 password; label + fingerprint confirmed on both screens | Req.3 | Host: "Add a machine" opens a 2-minute window and shows the code + countdown; joiner: "Join a machine" enters it. SPAKE2 (`spake2` 0.4) with an HMAC transcript bound to both keys, both ids, the protocol version and the TLS exporter. Both users see the OTHER machine's label and real fingerprint before anything is pinned — `the_right_code_and_both_confirmations_pair_and_connect_both_macs` (two app instances, two data dirs) asserts each side's displayed fingerprint equals the other's `fleet_self` | PASS | 2026-10-18 | claude-opus |
| Wrong code aborts, rotates the code, rate-limits (5 → 60 s, 20/24 h → disabled until re-enabled; persisted), logs without the code | Req.3 | `a_wrong_code_aborts_rotates_the_code_and_pins_nothing` (joiner reason `mismatch`, host code changes, no pins); `five_wrong_codes_lock_pairing_and_the_lock_survives_a_restart` (open → `locked_out`; a fresh app state over the same data dir is still locked — the guard persists in app settings); daily disable + re-enable covered by the guard unit tests and the `Turn pairing back on` UI. Attempt log: `fleet/link-audit.jsonl`, hash-chained (each line hashes the previous), `verify_audit` checks the chain; the test asserts the code's digits never appear in the file — excerpt below | PASS | 2026-10-18 | claude-opus |
| Final step sets permissions — View only default, or View + control — with one sentence on what control allows | Req.4 | Permissions step: radio group, **View only** pre-selected and focused (Playwright `host pairing completes keyboard-only, and View only is the default`); control: "can also start and stop work here, using Launch settings that already exist on this Mac — it can never read credentials." Each side chooses what it grants the other | PASS | 2026-10-18 | claude-opus |
| Pairing < 60 s; distinct copy + next action for no machines found, code mismatch, peer rejected, window closed | Req.5 | Loopback two-instance pairing completes in well under a second (asserted < 60 s). Failure states, each with its own title, body and action: `none-found` ("No machines found. Check both machines are on the same network with Link on." + pair by address), `mismatch` (Try again), `cancelled` (Start over), `expired` (Start again) — plus `locked_out`, `disabled`, `unreachable`, `fingerprint`, `not_open`, `busy` (vitest asserts no generic fallback for any of them) | PASS | 2026-10-18 | claude-opus |
| Remote joining documented as "connect with Tailscale/WireGuard first, then pair"; no relay | Req.6 | Join → "Pair by address instead (VPN)": "Not on the same network? Connect your machines with Tailscale or WireGuard first, then enter the other machine's address. AI Juggler never relays through the internet." The joiner tells the host its listening port (`PairStart.listen_port`, additive proto field 5) so both sides reconnect by the remembered address after pairing. No relay code exists | PASS | 2026-10-18 | claude-opus |
| Quality gates: mismatched codes can't complete the wizard; View only default asserted; no anonymous-mode flag | quality gates | Playwright `a mismatched code can't complete the wizard and pins nothing` (no Verify/Permissions step is ever shown; no decide call recorded); default asserted as above; there is no setting that admits unpinned peers (the directory only answers for pinned keys) | PASS | 2026-10-18 | claude-opus |
| Keyboard-only pairing; labelled code fields; axe clean | visual testing | Keyboard-only host flow (⌘⇧F → Space on the Link switch → Enter on Add a machine → Enter "Fingerprints match" → Tab/Enter Pair → Enter Done). Code fields: `role=group` "Pairing code", each box "Digit n of 6", auto-advance, Backspace/arrow keys, paste of 6 digits or a pairing link. axe-core 0 critical/serious on the Link switch, code, Verify, Permissions, join code group and a failure alert, both themes | PASS | 2026-10-18 | claude-opus |
| Screenshots: 3 steps + failure states, light/dark | visual testing | Gallery `43-pairing`: link-on, 1-host-find, 1-join-find, 1-join-address, 2-verify, 3-permissions, 4-done, host-new-code, fail-none-found / mismatch / cancelled / expired / locked / disabled at 1280×800 and 1024×680 (the window minimum — task-text note: 800×600 is below the app's minimum size) × light/dark | PASS | 2026-10-18 | claude-opus |
| QR | goal | Desktop-to-desktop pairing has no camera path, so the QR is replaced by **Copy pairing link** (`aijuggler-pair:v1:<code>:<fingerprint>`), which also lets the joiner verify the host's fingerprint automatically — a pasted link with a different fingerprint stops with "don't pair" (`JoinError::FingerprintMismatch`). Task-text note | PASS (by design) | 2026-10-18 | claude-opus |
| Perf: discovery < 5 s; wizard step < 200 ms; lockout survives restart | perf budget | Discovery < 2 s (live run above); step transitions are local state changes, polled at 500 ms for the other machine's move; lockout persisted (test above). Browse CPU < 1 % is measured on gate hardware in AJ-E710 | PARTIAL-BY-DESIGN | 2026-10-18 | claude-opus |
| Human dogfood on two physical Macs (screencast + timing) | evidence | Not possible from this environment — recorded as the AJ-E710 beta-gate item in `docs/qa/gate-E7.md` | DEFERRED-TO-AJ-E710 | 2026-10-18 | claude-opus |

## Attempt-log excerpt

Verbatim from `AIJ_PRINT_AUDIT=1 cargo test -p juggler-app --test
fleet_link a_wrong_code -- --nocapture` (2026-10-18; throwaway test
keys). A joiner typed a wrong code, then an unpaired machine tried to
connect without pairing. Each `entry` hashes into the next line's
`prev`; the peer is named by its key fingerprint; the typed code is
never recorded (asserted):

```json
{"entry":{"at_ms":1792335226910,"kind":"pairing","peer":"7FDD D6A9 3BCF 223D 615A 334F E6B4 75D5","prev":"0000000000000000000000000000000000000000000000000000000000000000","reason":"mismatch","sequence":null},"hash":"3bb1d8852d2c135d0665865068a85b8301134602fc88c7ceb728c44a12838491"}
{"entry":{"at_ms":1792335226959,"kind":"hello_rejected","peer":"unpaired","prev":"3bb1d8852d2c135d0665865068a85b8301134602fc88c7ceb728c44a12838491","reason":"not_paired","sequence":null},"hash":"4b75d8691327370edab6a287e12128ee708f6260a882f9493a0f48a2688a21c5"}
```

## Test counts

- `cargo test -p juggler-app --test fleet_link`: 4/4.
- `cargo test -p juggler-link`: unit 25/25, `pairing` 7/7 (+1 live mDNS,
  run manually above), `proto_contract` 3/3, `transport` 11/11.
- vitest `pairing-wizard.test.tsx` 8/8, `fleet-screen.test.tsx` 6/6.
- Playwright `fleet-pairing.spec.ts` 4/4; a11y `fleet — pairing wizard`
  ×2 themes; gallery `43-pairing` 24 tests.
