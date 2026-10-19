# Evidence manifest — AJ-E1204

Task: `docs/tasks/E12-optional-account.md` AJ-E1204 (vault key, recovery
code, encrypted backup format). The format is ADR-0017. 2026-10-18,
reviewer claude-opus.

## How it works

**The vault key.** Turning on Cloud backup makes a 32-byte random key in
AI Juggler's own Keychain item (`ai.figurehead.juggler.vault`), separate
from the device token. It is never derived from the token or the Fleet
identity.

**The recovery code.** Settings › Account & Cloud then shows the sheet with
the code: 13 base32 groups plus a checksum group, as selectable text, with
**Copy** and **Save…** (to a text file the user picks) and the plain
warning. **Start backups** stays disabled until "I've saved my recovery
code" is ticked. **Later** leaves Cloud backup waiting, with a row to show
the code again. Once confirmed, the code is never shown again, and no
upload can start before that (enforced by AJ-E1205's upload step).

**The container.**
- Layout: `FHB1 · 0x01 · key id · nonce · ciphertext+tag`, XChaCha20-Poly1305
  with the associated data `figurehead-backup-v1`.
- Plaintext: the AJ-E1009 backup set's entries, re-packed as a zstd tar, so
  `fleet/` and every other excluded item stay out.
- Opening a container rebuilds the `.ajbackup`, which AJ-E1009 verifies
  entry by entry before anything is restored.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| First turn-on generates a 32-byte key in the Keychain (its own service) and shows a recovery code once (base32, groups of 4, Copy and Save), requiring "I've saved my recovery code" before the first upload, with the exact warning | Req.1 | `vault::tests::the_key_is_stored_as_its_own_keychain_item…` (own service, never printed). `vault_container :: the_recovery_code_is_shown_until_confirmed…`: the same code until confirmed, Save writes it with the warning, then "isn't shown again". vitest `recovery-code.test.tsx`: selectable code, Copy and Save, exact warning, Start backups disabled until ticked, Later confirms nothing | PASS | 2026-10-18 | claude-opus |
| Container: zstd tar of the AJ-E1009 set (never `fleet/`), XChaCha20-Poly1305 (RustCrypto), `FHB1` and a version byte, `aad = "figurehead-backup-v1"`; documented | Req.2 | `vault::tests::round_trip`, `known_answer` (pinned bytes: `src/vault_kat.hex`). `vault_container :: a_real_backup_seals…`: a real AJ-E1009 export is sealed and opened on a second data folder with only the recovery code; AJ-E1009 verifies every checksum and restores with matching counts. ADR-0017 documents the format | PASS | 2026-10-18 | claude-opus |
| Decryption accepts the Keychain key or the recovery code, and rejects a wrong key, a wrong magic or version, or tampered ciphertext with a named reason | Req.3 | `wrong_key_magic_version_and_tampering_are_named`: different recovery code / not a cloud backup / format 2 / damaged or altered (header, nonce, ciphertext and tag bytes, truncation). A recovery-code typo is caught as a typo | PASS | 2026-10-18 | claude-opus |
| Encrypt/decrypt ≥ 100 MB/s; a 25 MB container in < 1 s | perf | **25 MB seal + open in ≈ 90 ms (≈ 560 MB/s each way)**. The cipher crates and `juggler-account` are optimised even in the dev profile, so tests measure what users get | PASS | 2026-10-18 | claude-opus |
| Round-trip, tamper, wrong-key and known-answer tests; plaintext-canary guardrail | quality gate | All of the above, plus `no_plaintext_canary_survives…` (unit), and in the integration test a seeded project name absent from a real container | PASS | 2026-10-18 | claude-opus |
| Visual: the recovery-code sheet at 1280×800 both themes; a11y: the code is selectable text with Copy | visual / a11y | Gallery `59-vault` (sheet, confirmed, waiting row). Playwright `recovery code sheet — axe 0`: the code takes focus, then Tab reaches Copy and Save, and Start backups is disabled until confirmed | PASS | 2026-10-18 | claude-opus |

## Test counts

- `cargo test -p juggler-account vault::` 7/7; `cargo test -p juggler-app --test vault_container` 2/2 (`vault-tests.log`).
- vitest `recovery-code.test.tsx` 2/2. Playwright `59 vault` 2, `recovery code sheet — axe 0` 2.
