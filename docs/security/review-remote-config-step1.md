# Security review: `juggler-remote-config`, ADR-0035 step 1

**Reviewer:** an independent Opus session (adversarial review; it did not write the code).
**Date:** 2026-11-18.
**Scope:** ADR-0035 §1–§3, §7 and §10. The review covered:
- all of `crates/juggler-remote-config` (src and tests);
- `crates/juggler-app/src/remote_config.rs`;
- `crates/juggler-account/src/transport.rs`;
- commits 892c73813, 832eb3b9e and 2999e19d1.

**Method:**
- read every file;
- ran ten proof-of-concept tests against a scratch copy of the crate, built outside the repo with offline dependencies only;
- removed 35 defences one at a time (mutation testing) to see whether a test fails.

The repo was not changed apart from this file.

## Summary

| Severity | Count |
|---|---|
| Blocker | 0 |
| High | 2 |
| Medium | 5 |
| Low | 5 |
| Info | 8 |

**Nothing blocks merging step 1.** Nothing is live yet: `COMPILED_KEYS` and `COMPILED_ENDPOINTS` are empty, and no production `StateStore` exists. The core verification path is sound. Bytes are verified before parsing, the canonical-equality check is strict, and the trust checks behave as claimed. The tests are strong: 29 of the 35 removed defences make a test fail.

**H1 and H2 must be fixed before any key or endpoint is compiled in.** All Medium findings should be fixed before the first signed file is published.

---

## High

### H1. The rollback floor and the revocations have no durable, atomic, non-clearable home yet

**Where:**
- `src/state.rs:92-106`: the `StateStore` contract. `MemoryStore` is the only implementation.
- `src/fetch.rs:451-460`: how the state is loaded.
- ADR-0035 §7 (line 125), which puts the state "in the registered cache (ADR-0023)".

**Attack:** every §3 rollback and revocation guarantee is only as strong as `TrustState`'s storage, and the contract is underspecified:
- "Nothing saved" returns `TrustState::default()`, which is the same as a first run: floor `None`, revoked set empty.
- ADR-0023's registered cache is user-clearable by design (the local-cache ruling: "the work is off/clear"). A cache clear, a "reset" or a store that maps a parse error to default therefore does three things:
  - it resets every channel's floor, so any validly signed file still inside its window (up to 31 days) is accepted again, including one that predates a kill switch;
  - it forgets every revocation, so a stolen and revoked key verifies again until a file that carries the revocation list is fetched (see M5);
  - it resets `last_attempt`.
- A non-atomic write that tears during a crash has the same effect if the loader treats the torn file as missing.

**Impact:** rollback protection fails open on ordinary events (cache clear, corruption, crash), not only under attack.

**Fix (before wiring):**
1. Keep `TrustState` out of the user-clearable cache. Use its own app-support file (or the Keychain for the revoked set and floors). The cached envelope may stay in the cache.
2. Write it atomically: a temp file, `fsync`, `rename`, then `fsync` the directory.
3. Make `load()` return `Ok(default)` only when the file does not exist. Return `Err` when the file exists but won't parse or has the wrong version, so the store fails closed (the crate already handles `Err` correctly).
4. Never let "clear cache" or "reset install id" touch it.
5. Add a contract test for the production store: missing → default, corrupt → `Err`, and a torn write never yields a lower floor.
6. Amend ADR-0035 §7 to say so.

### H2. One value an older app can't accept drops the whole section, kill switches included, even inside a rule gated to newer apps

**Where:**
- `src/schema.rs:450-482` (`parse_section` uses `?` on any control);
- `src/schema.rs:484-505` (`parse_rules` validates every rule's value whether or not its `when` could match, line 501);
- `src/schema.rs:432-438` (the whole section is dropped).

**Attack (no key needed; normal publishing triggers it):** the ADR's forward-compatibility promise is "an old app meets a new file". But many routine changes make an older app reject a value, and that rejection drops every control in the section:
- a newer app widens a `Versions` control's compiled `bounds` (the core D-128 use);
- a newer app adds a `Choice` option;
- a newer app changes a switch from `OffOnly` to `Rollout`/`OnOff`;
- a newer file adds a field inside a `VersionRange` (`deny_unknown_fields`, `version.rs:166`).

A rule gated with `when: {app: {min: "0.6.0"}}` does not help, because old apps still validate its value. The PoCs show it: a compat narrowing (a safety value) and a `"sync": false` kill switch both disappear on a 0.5.0 app because a sibling control carries a 0.6-only value. `poc_one_bad_control_drops_the_sections_kill_switches` and `poc_rule_gated_to_newer_apps_still_drops_old_apps_section` both pass. ADR §8's CI dry-run covers only the last three releases, and older installs are the ones most likely to need a kill.

**Impact:** kill switches and compat narrowing silently stop reaching older apps. That is the safety asset §3 exists to protect, lost through ordinary schema evolution.

**Fix:**
- Make validation fail-inert per rule, not per section:
  - a rule whose `when` can't match this app (or is `Unknown`) is skipped without validating its value;
  - a matching rule with an unacceptable value drops only that control, with a `SectionIssue::Invalid` that names it.
- Always apply a remote `false` for any switch even if the section is otherwise invalid ("fail toward safety").
- Keep whole-section drops for structural damage only: not an object, or no schema.
- Add tests for each of the four triggers above, including a rule gated to a newer app version.

---

## Medium

### M1. The validity cap is anchored to `issued_at`, not to now; a file can be valid for years, and a clock set back revives expired files

**Where:** `src/envelope.rs:252-262`; the self-review at `src/lib.rs:41` and `:49-51`.

**Attack:**
- Nothing checks `issued_at <= now`. A file with `issued_at = now + 10 years` and a 30-day window is accepted today and is still valid ten years from now (`poc_future_issued_at_escapes_the_validity_cap` passes).
- Any signer can produce such a file: a wrong clock in the signing environment, a units bug, or a key holder.
- That defeats the stated purpose of `MAX_VALIDITY_SECS` ("bounds a mistake or a stolen key's reach") and the self-review's "bounds how long any file can be replayed".
- The existing test `it_fetches_at_most_daily` already accepts a file whose `issued_at` is a day in the future (`now = NOW - DAY`).
- A clock set back (by the user, or by an NTP spoofer on the path) revives expired files above the floor, including on fresh installs.

**Impact:** a replay window without bound for any file signed with a future `issued_at`. The rollback protection on installs with no floor depends entirely on the wall clock.

**Fix:**
- Reject `issued_at > now + MAX_SKEW` (for example 1 day).
- Keep a monotonic `trusted_now = max(wall_now, highest issued_at accepted)` in `TrustState` and check expiry against it, so a clock set back can't revive a file older than one already seen.
- Add tests for both.

### M2. Concurrent refreshes lose updates: a revocation and the floor are silently undone

**Where:** `src/fetch.rs:444-487`. The state is loaded, network I/O runs (up to 10 s per endpoint), then the whole state is saved, with no lock and no merge.

**Attack:** two `refresh` calls overlap, for example the launch fetch and a "check now" or channel-change path.
- Both load S0, and both are "due" because neither has saved `last_attempt`.
- B accepts a newer file that revokes `current` (floor 9) and saves.
- A fails and saves its stale S0, so the floor drops back to 5 and the revocation is gone. A's caller also gets the cached file signed by the now-revoked key.

`poc_concurrent_refresh_loses_floor_and_revocation` passes.

**Impact:** a revocation and rollback state undone without any attacker capability beyond timing; a revoked key's file is served to a caller.

**Fix:**
- Hold a `Mutex` across load → fetch → save inside `Fetcher`, so a second caller waits or returns `NotDue`.
- Make `save` merge monotonically: floors take the max per key, the revoked set takes the union, `last_attempt` takes the max.
- Add a two-thread test.

### M3. A revocation part signed by a key this app doesn't pin rejects the whole file

**Where:** `src/envelope.rs:223-229`. `check_revocations(...)?` propagates `UnknownKey`, `BadSignature` and the rest.

**Attack:** the emergency key is rotated as the design describes ("retired by shipping an app without it"). Old apps still need the list signed by the old emergency key, so the publisher keeps serving it, and the wrapper holds only one `revocations` part. Every new app then rejects every file with `UnknownKey` (`poc_revocation_part_from_retired_emergency_key_rejects_whole_file` passes). The publisher has to choose which generation of apps gets no configuration at all.

**Impact:** emergency-key rotation turns off remote configuration, and so kill switches, for a whole app generation.

**Fix:**
- If a revocation part's key is unknown, or its role or signature fails, ignore that part (log it) and go on to the config part. Ignoring an unverifiable revocation can only keep trust as it was, never widen it.
- Better: let the wrapper carry `revocations: [..]` (several parts) so both generations are served.
- Add a test for each.

### M4. Revocations are by key id: future keys can be revoked in advance, and the set grows without limit

**Where:**
- `src/keys.rs:216-219` (match by id);
- `src/envelope.rs:314` (any well-formed id is accepted);
- `src/fetch.rs:552` (persisted forever).

**Attack:**
- A stolen emergency key, or a later compromised publish step, can revoke ids no app pins yet, such as `rc-2027-a` and `rc-2027-b`. The ids are predictable from the ADR's naming. When the recovery release ships with those ids, its keys are already revoked on every install that saw the list. This undercuts the self-review's recovery story ("until an update ships new keys").
- Ids from successive lists accumulate: 64 per part, per endpoint, per attempt.
- A future build that reuses an id for new key bytes inherits a revocation that was meant for different bytes.

`poc_unpinned_future_ids_are_revoked_and_remembered` passes.

**Fix:**
- Revoke by key fingerprint (SHA-256 of the 32 public-key bytes), not by id.
- Persist and apply only fingerprints of keys this build pins. Drop the rest at check time.
- Cap the stored set.

### M5. Revocation is in-band, so the attacker it targets can strip it

**Where:** the design, as implemented in `src/envelope.rs:222-229` and `src/state.rs:16-18`; the self-review's "Key compromise" row (`src/lib.rs:42`).

**Attack:** an install learns of a revocation only by fetching a file that carries the list. An attacker who holds a stolen `current` or `next` key and controls the CDN or the path simply never serves the list. These installs keep trusting the stolen key until an app update ships without it:
- every install the attacker serves;
- every fresh install;
- every install whose state was reset (H1).

The self-review says the revoked key's files "stop verifying", which holds only for installs that already saw a legitimate file carrying the list.

**Impact:** a stolen config key's reach is limited by app updates, not by revocation. The table overstates the defence.

**Fix:**
- Correct the self-review.
- Make the publish procedure carry the revocation part in every file until no supported app pins the revoked key (add a CI check).
- Also compile a `REVOKED_FINGERPRINTS` list into each release, so a revocation reaches fresh installs through the signed update channel.
- Document that the real remedy for a stolen config key is a release.

---

## Low

### L1. A failed state save still serves the new file, and the daily limit then doesn't hold

**Where:** `src/fetch.rs:483-486`. The test `a_store_that_cannot_save_still_serves_the_fetched_file` (`tests/fetch.rs:354`) asserts this fail-open behaviour.

**Attack and impact:** on a disk-full or read-only store:
- the new file is applied, but its floor and any revocation are not persisted, so after a restart the older cached file and a lower floor come back (a short replay window);
- `last_attempt` isn't stored, so every `refresh` fetches again.

**Fix:**
- When `save` fails, return the previously cached file (or none) with `Attempt::Failed(Failure::Store)`.
- Keep a per-process minimum interval in memory as well.
- Invert the test.

### L2. The transport reads up to 32 MiB before the 256 KB cap is checked

**Where:**
- `crates/juggler-account/src/transport.rs:175-180` (`limit(32 * 1024 * 1024)`);
- `crates/juggler-app/src/remote_config.rs:42-53`;
- the cap itself at `src/fetch.rs:547`.

**Attack and impact:** a hostile host or CDN makes the app allocate 32 MiB per endpoint per attempt (ADR §7 says 256 KB). The cost is memory and bandwidth, which is minor, but the cap isn't enforced where it matters.

**Fix:**
- Add a `max_body` field to `juggler_account::HttpRequest`, set it to `MAX_FILE_BYTES + 1` from `RemoteConfigTransport`, and pass it to `.limit()`.
- Test that a 257 KB body fails in the transport.

### L3. One public key can be pinned under two ids and roles

**Where:** `src/keys.rs:152-167`. Ids and roles are checked for uniqueness, but the key bytes are not.

**Attack and impact:** a build mistake that pins one key as both `Current` and `Emergency` would collapse role separation: whoever holds the config key also holds the revocation power. Revoking one id would also leave the same key trusted under the other. `poc_same_public_key_under_two_roles_is_pinned` passes.

**Fix:** reject duplicate key bytes in `KeyRing::new`, and add a test.

### L4. Kills of `OnOff` and `Rollout` switches are overridden by the account layer and the user

**Where:** `src/merge.rs:103-113` and `:120-131`. Only an `OffOnly` remote `false` beats the user, and the account layer replaces the public layer for every kind of control.

**Attack and impact:** ADR §8's `--kill <flag>` emergency workflow has no effect against a user's explicit choice or an account file when the flag is `OnOff` or `Rollout`. The same applies to compat narrowing in the public file, which the account layer can widen back (within bounds). This is by design for preferences, but a publisher reading "kill" will expect it to stick (`poc_onoff_kill_is_overridden_by_user_and_account` passes).

**Fix:**
- Make `--kill` refuse anything that isn't `OffOnly` (a CI check).
- Document in the merge table that only `OffOnly` kills are safety.
- Decide in step 4 whether a public compat narrowing should beat the account layer, as it beats the user.

### L5. Test gaps: six defences survive their removal

| Mutation | Result |
|---|---|
| `verify_strict` → `verify` | survived |
| revocation list's 64-id cap removed | survived |
| revocation `key_id` mismatch check removed | survived |
| revocation `format` check removed | survived |
| envelope `format` check removed | survived |
| 128-byte entity-tag cap removed | survived |

There is also no test for:
- a future `issued_at` (M1);
- concurrent refreshes (M2);
- deep nesting inside an unknown wrapper field (checked as harmless here);
- an empty compiled ring rejecting a well-formed file.

**Fix:** add one test per row. For `verify_strict`, add the ed25519-speccheck vectors (small-order R, non-canonical R, S ≥ L; S + L is already rejected, as `poc_s_plus_l_is_rejected` shows).

---

## Info

1. **The `next` key is a full configuration key.** Its holder can:
   - sign any compiled-safe configuration for every channel;
   - burn serial 2⁵³−1 to lock the legitimate publisher out until the emergency key revokes it.

   Before a rotation its only job is availability, so keep `next` offline or behind a separate approval rather than in the same environment secret as `current`.
2. **The emergency key's scope is as stated.** It signs only revocations and can't be revoked by a file, so there is no revocation loop. Its lists aren't channel-bound and never expire, which is harmless because revocation only grows. A thief can push the app to compiled defaults (and see M4).
3. **The unsigned wrapper isn't "four strings".** `SignedFile` and `SignedPart` accept and skip unknown fields. serde_json skips them iteratively with a heap stack bounded by the 256 KB cap, so 120,000-deep nesting is harmless (`poc_deep_nesting_in_unsigned_wrapper_is_harmless`). Consider `deny_unknown_fields` on the wrapper, which is versioned anyway, and correct `src/lib.rs:38`.
4. **ADR and code differ:**
   - fetch interval: 24 h in code (`fetch.rs:34`), 6 h in ADR §7;
   - who signs revocations: "another pinned key" in ADR §3, emergency only in code (stricter);
   - `issued_at` in the revocation payload is documented but ignored.

   Update the ADR.
5. **Rollout scope.** A `Rollout` rule with buckets `[0, 99]` switches a flag on for everyone, so it is equivalent to `OnOff`. Reviewers should know that.
6. **Torsion check.** `PinnedKey::new` rejects small-order keys but not mixed-order ones. They are irrelevant for keys we generate, but adding `is_torsion_free()` is cheap hardening.
7. **Things that can link one install's fetches:**
   - the long-lived ureq agent can reuse connections and TLS session tickets across fetches in one process, which links them like an entity tag would (the IP address already does);
   - ureq reads proxy settings from the environment by default;
   - if conditional requests are ever turned on, the entity tag from one endpoint is sent to the fallback endpoint.

   Consider a fresh agent per fetch and per-endpoint entity tags.
8. **Revoking a key resets the floor.** After a revocation the floor falls to the other keys' highest serial, which is `None` if `next` was never used on this install. Any stale `next`-signed file still inside its window is then accepted once. This is acceptable, but worth documenting next to the "burned serial" rationale.

---

## Verified sound

- **Verification before parsing:** only the size cap and the wrapper are read before `verify_strict` succeeds, and the payload stays opaque base64 until then (`envelope.rs:214-237`). Bad-signature payloads (non-JSON, 100k `[`, duplicate keys) never reach a parser.
- **The canonical form gives each signed payload one meaning:**
  - re-encoding must reproduce the exact bytes, which rules out duplicate keys, whitespace, trailing data, a byte-order mark, `\u` escapes of printable characters, surrogate escapes, `1.0`, `1e3`, `-0`, leading zeros and integers beyond 2⁵³;
  - keys sort by UTF-8 bytes;
  - Unicode normalisation forms are distinct bytes and so distinct signed content;
  - `preserve_order` (switched on elsewhere in the workspace) is neutralised by the explicit sort.
- **Signatures:**
  - context strings separate configuration signatures from revocation ones (tested);
  - `verify_strict` rejects small-order R and A, and non-canonical R;
  - S ≥ L is rejected (PoC);
  - weak pinned keys are rejected;
  - the base64 STANDARD engine is canonical.
- **Key ids can't be confused:** the wrapper's `key_id` picks the key, and the signed `key_id` must equal it. A signature under one key can't verify under another id.
- **Roles hold:** `current` and `next` sign configuration only; `emergency` signs revocations only (tested).
- **Rollback holds:**
  - the floor is the per-channel maximum over unrevoked keys;
  - a lower serial is refused;
  - the same serial needs a byte-identical payload digest (tested);
  - the floor is checked after verification and before acceptance, and recorded only on full success.
- **Channel binding:** the channel inside the signed envelope must match (tested).
- **Fail-closed paths:** empty `COMPILED_KEYS` means nothing verifies; empty `COMPILED_ENDPOINTS` means nothing is fetched; an unreadable store means no remote file is used. The cached file is re-verified on every load, against the current revocations and expiry (tested and mutation-checked).
- **§2 limits:**
  - `OffOnly` can't be turned on, even in a rule;
  - `Rollout` turns on only in bucketed rules;
  - ranges stay within compiled bounds, numbers are clamped and choices are closed;
  - unknown sections, unknown controls and newer schemas are ignored;
  - a `when` with an unknown key never matches, so it can't widen;
  - the user can only narrow ranges and can't override an `OffOnly` kill (tested by proptest).
- **Fetch privacy:**
  - the request type has no field for a query, cookie, bearer or id, and the URL is fixed per channel;
  - the transport adds only `User-Agent: AIJuggler/<version>`, `Accept` and `Content-Type`, with no bearer (tested in `juggler-app`);
  - `If-None-Match` is off by default;
  - the install id is only hashed locally and its `Debug` is redacted;
  - TLS is rustls (ring) with the OS trust store; redirects are 0, so a 3xx is a failure; the global timeout is 10 s;
  - the daily limit survives restarts through the store and treats a clock set back as due.
- **Mutation results:** of 35 defences removed one at a time, 29 make an existing test fail. The six that survived are listed in L5.

## Reproducing

The proof-of-concept tests and the mutation script were run from a scratch copy of the crate, outside the repo, with offline dependencies only. They are not committed. Each one is described above, and the names quoted are its test functions. To reproduce one, copy it into `tests/` in a scratch checkout. All ten pass today, which confirms each finding; each should fail once its fix lands.
