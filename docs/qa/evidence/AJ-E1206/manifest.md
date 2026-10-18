# Evidence manifest — AJ-E1206

Task: `docs/tasks/E12-optional-account.md` AJ-E1206 (Fleet discovery via
account). Built against the Figurehead API contract's `GET /devices`
directory. 2026-10-18, reviewer claude-opus.

## How it works

With Fleet discovery on, Fleet shows **Your Macs on Figurehead**: your
other linked Macs, listed by name with their state in words.

| State | Shown as |
|---|---|
| `not_paired` | Not paired |
| `waiting` | Confirmed here — waiting for it to confirm this Mac |
| `paired` | Paired via your Figurehead account |
| `no_key` | Can't pair this way — use a pairing code |

The directory is read at most once a minute. A key from the directory is
only a candidate, because a compromised account could offer an attacker's
key. Trust comes from the people at both Macs:

1. **Compare and pair…** shows this Mac's fingerprint and the other Mac's
   (from the directory key) side by side, and asks to check them on both
   Macs. **Pair (View only)** stays disabled until "Both fingerprints
   match on both Macs" is ticked.
2. Pairing re-reads the directory and pins the key only if its
   fingerprint is still the one the user confirmed. A key swapped after
   the comparison is refused ("compare it again").
3. The pin is a provisional id (`acct-<device>`), at View, marked
   `paired_via: account`, which the peer list shows as "via your Figurehead
   account".
4. Mutual TLS only connects two Macs that have each pinned the other, so
   nothing connects until the other Mac's user confirms too.
5. The first Hello binds the peer's real `MachineId`. Only the holder of
   the pinned key can send it, the bind happens once, and never for a
   code-paired peer.

Traffic stays machine to machine over mutual TLS (ADR-0004); Figurehead
relays nothing. Signed out, or with discovery off, Fleet pairs with the
6-digit code or QR exactly as before (E7), and no `/devices` call is made.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| With discovery on, list peers from `GET /api/v1/devices`; off, no `/devices` call | Req.1 | `fleet_discovery :: off_means_no_devices_call` (**0** calls with discovery off, even when listing or pinning is asked for). `listed_macs…`: lists the other Macs and not this one, and one read covers repeat calls within a minute. vitest: the card is hidden and makes no call while off | PASS | 2026-10-18 | claude-opus |
| Pinned only after the user confirms the fingerprint on both machines (each shows the other's, from the directory's `publicKey`), never silently; starts at View, labelled "via your Figurehead account"; TLS still proves possession | Req.2 | Listing pins nothing; a wrong fingerprint and a Mac with no key are refused; a confirmed fingerprint pins at View, `paired_via: account`, state `waiting`. `account_pairing :: the_first_hello_binds…` (two-node loopback): the first TLS-authenticated Hello binds the real id, once; the registry re-signs with it. vitest: Pair stays disabled until the match is ticked | PASS | 2026-10-18 | claude-opus |
| Machine to machine (LAN or VPN) over mutual TLS; no relay | Req.3 | Only `GET /devices` reaches Figurehead. Connections use the existing link (`client::connect` against the pinned key); the directory carries no address | PASS | 2026-10-18 | claude-opus |
| Unlinked, Fleet pairs with the 6-digit code or QR exactly as E7 | Req.4 | The card is absent when signed out or with discovery off. Playwright `42 fleet` (unchanged) and the full E7 suites pass | PASS | 2026-10-18 | claude-opus |
| Directory refresh ≤ 1 s; no polling faster than once a minute | perf | One request per read (fake directory: milliseconds). The backend caches for 60 s and the card refreshes every 60 s | PASS | 2026-10-18 | claude-opus |
| Mock directory test; **substitution test** (an attacker key in the directory is never pinned without the two-sided confirmation, and a confirmed pin whose TLS key differs is rejected); discovery-off zero-call test | quality gate | `a_key_swapped_after_the_comparison_is_never_pinned` (the directory swaps in an attacker key after the comparison: refused, nothing pinned). `account_pairing :: nothing_else_can_use_the_bind`: a stranger's key → "isn't paired", nothing bound; a code-paired peer is never renamed; a provisional id can't be claimed; a node whose TLS key differs from the confirmed pin is refused at the handshake. Zero-call as Req.1 | PASS | 2026-10-18 | claude-opus |
| Evidence: the two-node loopback run | evidence | `loopback.log` (`account_pairing` 2/2, `fleet_discovery` 3/3) | PASS | 2026-10-18 | claude-opus |
| Visual: the Fleet list with account-discovered peers, 1280×800 both themes | visual / a11y | Gallery `61-account-macs` (list with all three states, fingerprint comparison). Playwright `account macs and compare — axe 0` (both themes): Pair disabled until confirmed; Escape closes without pairing | PASS | 2026-10-18 | claude-opus |
| Two real Macs on one account pair this way on a LAN | evidence | `docs/qa/dogfood-backlog.md` | DEFERRED-TO-DOGFOOD | 2026-10-18 | claude-opus |
