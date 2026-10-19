# ADR-0017: Encrypted Cloud backup container (FHB1)

**Status:** Accepted
**Date:** 2026-10-18
**Deciders:** implementation agent (AJ-E1204); maintainer review on the PR

## Context

Cloud backup (D-18, ADR-0014) stores a user's AI Juggler backup on
Figurehead so a new Mac can get its setup back. ADR-0014 promises that
Figurehead can never read it: the key must exist only on the user's Macs
and in a recovery code the user keeps. The AJ-E1009 backup set
(ADR-0016) already decides *what* is in a backup; this ADR decides how
it is sealed.

## Decision

**The vault key** is 32 random bytes made on the Mac the first time Cloud
backup is turned on. It is stored in AI Juggler's own Keychain item
(service `ai.figurehead.juggler.vault`), separate from the device token.
It is never derived from the device token or the Fleet identity, so
unlinking a Mac or rotating its identity never touches backups.

**The recovery code** is that key written for a person:
- base32 (RFC 4648 alphabet `A–Z 2–7`), 52 characters for the key;
- then a 4-character checksum (20 bits of
  `sha256("figurehead-recovery-check" ‖ key)`), so a mistyped code is
  reported as a typo, not as a wrong key;
- shown as 14 dash-separated groups of four.

When a code is read back, case, spaces and dashes are ignored, and the
digits people confuse with letters (`0`, `1`, `8`) are read as `O`, `I`, `B`.
The code is shown until the user confirms "I've saved my recovery code",
which is required before the first upload, and never again after that.

**The container** is laid out as:

```text
offset  size  field
0       4     magic "FHB1"
4       1     version 0x01
5       8     key id: sha256("figurehead-vault-key-id" ‖ key)[0..8]
13      24    nonce (random, XChaCha20)
37      …     ciphertext ‖ Poly1305 tag (16)
```

- It is sealed with **XChaCha20-Poly1305** (RustCrypto `chacha20poly1305`),
  with the associated data `figurehead-backup-v1`.
- The plaintext is a **zstd-compressed tar of the AJ-E1009 backup set**: the
  same entries and bytes as a local `.ajbackup`, so the same allowlist,
  scrub and redaction scan apply, and the Fleet folder is never in it.
- Opening a container rebuilds a `.ajbackup` and hands it to AJ-E1009's
  verified restore, which checks every entry's checksum against its
  manifest.

**Refusals are named:**

| Condition | Reason given |
|---|---|
| Wrong magic | "this isn't an AI Juggler cloud backup" |
| Unknown version | "this backup uses format N; update AI Juggler to open it" |
| Key id mismatch | "this backup was made with a different recovery code" |
| Authentication failure | "this backup is damaged or was altered" |

## Alternatives considered

- **Encrypt the gzip `.ajbackup` as is.** One less re-pack. AJ-E1204 asks for
  zstd, which gives a smaller upload under the 25 MB limit, so the cloud
  container re-packs the entries. The local file stays gzip, so standard
  `tar -xzf` reads it.
- **Derive the key from a passphrase (Argon2).** A memorable secret invites
  a weak one, and people forget passphrases as easily as codes. A
  random key with a printed code is stronger and simpler to reason
  about.
- **No key id in the header.** AEAD alone cannot tell a wrong key from a
  changed byte. The key id is a one-way hash of the key that reveals
  nothing useful, and it lets the app say which one happened.

## Consequences

- Losing every Mac and the recovery code means the cloud backups cannot be
  opened, by anyone. The setup sheet says so plainly.
- A new format bumps the version byte and amends this ADR. Version 1 must
  stay readable, or be refused with its reason.
