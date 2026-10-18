# ADR-0014: Optional Figurehead account and cloud services

**Status:** Accepted (partly supersedes ADR-0004's "cloud relay / vendor backend — rejected outright")
**Date:** 2026-10-17
**Deciders:** maintainer (D-18)

## Context

AI Juggler is local-first: it supervises vendor CLIs on the user's own
machines and, until now, promised "no cloud, no account". Users who own
several Macs want their setup backed up and their machines to find
each other without typing pairing codes; Figurehead (the project's
publisher) runs figurehead.ai, which can host an account for that. The
desktop app must stay complete for people who never want an account,
and nothing private — vendor credentials, transcripts, plaintext
backups — may reach a server.

## Decision

AI Juggler offers **two paths** (D-18):

- **Desktop (no account)** — everything the app does today. With no
  account linked, the app makes no calls to figurehead.ai at all.
- **Cloud (optional account)** — the user links this Mac to a
  Figurehead account and turns on individual cloud features, each with
  its own toggle and a plain statement of what it sends.

Phase 1 cloud features:

| Feature | What leaves the Mac |
| --- | --- |
| Encrypted cloud backup and restore | A backup file encrypted on the Mac (XChaCha20-Poly1305, key in the macOS Keychain, recoverable only with the user's recovery code). Figurehead stores ciphertext it cannot read. |
| Fleet discovery | Device metadata (name, platform, app version, public identity keys) so the user's own linked Macs can find and pin each other. Traffic stays machine-to-machine (ADR-0004); there is no relay. |
| Usage stats sync (with E11) | The opt-in weekly usage report, attributed to the account instead of sent anonymously. |

Architecture:

- **Linking** uses a browser approval (the app opens figurehead.ai,
  the signed-in user approves this Mac, the app polls for the result)
  with a pairing-code fallback. The resulting device token lives only
  in AI Juggler's own Keychain item — never in SQLite, settings, logs,
  packs, backups or crash reports.
- **Entitlements** (plan and limits) are read from the account API and
  drive the UI. The app never hard-codes which features are free or
  paid; at launch every feature is free.
- A `401` from any account call means the Mac was unlinked on the
  website: the app drops the token, turns every cloud toggle off, and
  keeps all local data. A plan change never deletes local data.
- **Allowed network calls** become: the update check; user-initiated
  actions in managed Providers; opt-in usage statistics; and, only
  when an account is linked, the documented account endpoints — each
  called only for a feature that is turned on (except the account and
  this-Mac endpoints).
- The automation policy is unchanged: cloud features never perform a
  state-changing action without a click.

## Alternatives considered

- **Keep "no cloud, no account" forever** — leaves multi-Mac users
  with manual exports and pairing codes; rejected by D-18, which keeps
  that path fully intact as the default.
- **Required account** — violates local-first and the compliance
  posture; rejected.
- **Server-side (non-E2E) backups** — would make Figurehead a holder
  of users' project metadata and history; rejected.
- **A Fleet relay through figurehead.ai** — out of scope until a
  separate decision; discovery is a directory, not a relay.

## Consequences

- A small HTTP client, Keychain items and an encryption format become
  part of the app; each is covered by guardrail tests (signed-out is
  silent, toggles mean it, no plaintext leaves the Mac, token only in
  the Keychain, `401` cleanup, entitlements drive the UI).
- README, PROPOSAL, SECURITY and SPEC now describe the optional,
  end-to-end encrypted cloud path instead of "no cloud".
- Fleet (E7) gains account-based discovery and key pinning as an
  alternative to the pairing code, never a replacement for it.
