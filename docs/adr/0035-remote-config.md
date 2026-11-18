# ADR-0035: One signed remote configuration, in layers, read through typed modules

**Status:** Proposed (2026-11-18, coordinating session, Opus). Building starts once the maintainer has seen the summary.
**Date:** 2026-11-18
**Deciders:** Opus (design); the maintainer (D-128, D-129)
**Serves:** D-128 (version support without a rebuild), D-129 (a general remote configuration: flags, controls, messages, and account-level capabilities); [ADR-0014](./0014-optional-figurehead-account.md) (the optional account); [ADR-0015](./0015-usage-statistics.md) (opt-in stats, closed vocabulary); [ADR-0019](./0019-extension-architecture.md) (features as modules); [ADR-0023](./0023-local-cache.md) (the registered cache); D-52 (OS seams); D-127 (never show what can't be used)

## Context

**What the maintainer asked for (2026-11-18):**
- Version support should be updatable "from a separate remote file so we don't have to recompile and deploy whole app".
- It should then be widened into "a general config file", able to "disable or enable certain things or perform other control or maybe messages for the app".
- "for logged in users we should have additional API level capabilities".
- "architecture this carefully so it is future proof as we keep expanding our app".

**What exists today:**
- **The app works with no account and no network.** The account is optional (ADR-0014) and silent until linked; its device token lives in the Keychain.
- **Stats** are opt-in, with a closed vocabulary (ADR-0015).
- **Updates** come through the release channels (nightly, beta, stable) with signed builds.
- **Version knowledge is compiled in.** Adapters have version gates (`PROVEN_*`), and the fidelity register ships in the binary. D-128 replaces most gates with run-time shape checks, but some knowledge will always change faster than releases do.

**The risks a remote config adds,** each answered below:
1. **Tampering:** a forged or modified file.
2. **Rollback or replay:** an old, validly signed file sent again, to undo a kill switch.
3. **Privacy leaks:** fetching it must not identify the user.
4. **Remote code by stealth:** config growing into behaviour.
5. **Outages:** the app must never depend on the file being there.
6. **Schema drift:** old apps meeting new files, and new apps meeting old files.

## Decision

### 1. One envelope, many modules

The app reads one document per channel. It's a signed envelope around named **sections**, and each section belongs to exactly one feature **module** that registers a typed schema for it, the way features register with the app (ADR-0019). The config crate knows envelopes, signatures, layering and caching. It knows nothing about any feature.

```text
RemoteConfig envelope (JSON, canonical form for signing)
├─ format: 1                     the envelope's own version
├─ key_id: "rc-2026-a"           which pinned public key signed it
├─ channel: "stable"             nightly | beta | stable
├─ serial: 418                   strictly increasing per channel
├─ issued_at / expires_at        UTC; expired files are ignored
├─ min_app / max_app             app versions this file applies to (optional)
└─ sections: {                   each one is owned by a registered module
     "compat":   {...},          tool versions, shape-check versions, route state (D-128)
     "flags":    {...},          feature switches and rollouts
     "notices":  [...],          in-app messages
     "params":   {...},          tunables: intervals, limits, budgets
     "endpoints":{...},          service URLs, for the app's own services only
     "<future>": {...}           unknown sections are kept and ignored
   }
signature: ed25519 over the canonical bytes of everything above
```

- **Modules register `Section<T>`** with a name, a schema version, a `serde` type, a validator and a merge rule. A section that fails its validator is dropped on its own, and the rest of the file still applies.
- **Unknown sections and unknown fields are ignored, never errors.** That's forward compatibility: an old app meets a new file. A missing section falls back to compiled defaults, which is backward compatibility: a new app meets an old file.
- **Each section carries its own `schema` number.** A module accepts the versions it knows and ignores newer ones until the app updates. A breaking change gets a new section name (`flags2`), never a reinterpretation.

### 2. What the file may and may not do

The rule that keeps this safe, and keeps it future-proof: **the file only selects among behaviour already compiled into the app; it never adds behaviour.**

- **Flags** are declared in code (`flag!(name, default, remote: OffOnly | OnOff | Rollout)`). Most flags are `OffOnly`: remote can switch a feature off (a kill switch), not on. A flag that's safe to turn on remotely says so in code, and a reviewer sees that. Security and privacy settings are never remote-controllable.
- **Params** are declared with typed bounds in code (`param!(sync_interval_s, 60, 30..=3600)`). Values outside the bounds are clamped and logged.
- **Notices** are data, not markup:
  - severity (info, needs-you, broken, mapped to D-117's levels and the D-126 Alert);
  - plain title and body text, with no HTML;
  - at most one link, to an allow-listed host (our website or docs);
  - targeting (below);
  - an expiry;
  - whether it can be dismissed;
  - and an id, so a dismissal is remembered.

  Notices never trigger actions. A "do something" notice can only open a screen that's already in the app, chosen from an enum.
- **Endpoints** may only point at our own service hosts, from a compiled allow-list. They can't redirect the account, the update or the stats traffic anywhere else.
- **Compat** carries D-128's support list: per tool and feature, the version ranges, the shape-check versions and the route state. Like a flag, it can narrow or widen only within what the code's shape checks support.

### 3. Trust: signing, keys, rollback

- **ed25519 signatures** (`verify_strict`) over a context string and the canonical bytes (sorted keys, no insignificant whitespace, integers only). The file carries the payload as opaque base64 beside its signature, and verification happens before any parse of it.
- **Pinned keys with key ids and roles.** The app ships 2–3 public keys: the current one and the next one, which sign configuration, and an emergency one, which signs only revocation lists. A key is retired by shipping an app without it; **for a stolen configuration key, that release is the real defence.**
- **Revocation lists** (amended 2026-11-18 after the step-1 security review):
  - signed by the emergency key only; a file may carry several, one per emergency-key generation, and a list this app can't verify is skipped, never fatal;
  - they name keys by **fingerprint** (SHA-256 of the public key), and an app keeps only fingerprints of keys it pins, so a list can't pre-revoke a later release's keys;
  - they are in-band, so an attacker who holds a stolen key and controls the host can withhold them. Each release therefore also compiles in the revoked fingerprints known at build time, and the publisher keeps the list in every file until no supported app pins the revoked key.
- The private keys live only in the release environment, as GitHub environment secrets behind the same approval as Promote, never in the repo or on a laptop. `next` is kept apart from `current` (its own secret and approval), since before a rotation its only job is availability.
- **Rollback protection:** the app stores the highest `serial` it has accepted per channel and key, and rejects a lower one (the same serial only with identical content). It also rejects files past `expires_at`, about 14 days by default and at most 31 days after `issued_at`, so a stale file can't be replayed forever. The publisher must re-sign periodically. A CI job does this, and it alerts if the live file is within 3 days of expiry.
- **Time:** expiry is judged against `max(now, the newest issued_at accepted)`, so setting the clock back revives nothing, and an `issued_at` more than 24 hours in the future is rejected. The day of skew is a trade-off: a slow or wrong clock isn't shut out, at the cost that a file may apply up to a day early and its window may end up to a day past the 31-day cap; that still takes a pinned key, and the cap is still measured from the trusted time.
- **Where the trust state lives:** the floors, the revoked fingerprints and the trusted time are security state, not cache. They live in their own checksummed file in the app's durable data folder (Application Support, never Caches), written atomically (temp file, fsync, rename, fsync the folder). **Clearing the cache never touches it.** A save only moves forward (floors by maximum, revocations by union), and refreshes are serialised. If the file is missing or corrupt (read in full and failing its checksum or parse), the app fails safe: it starts from the floor and revocations compiled into the release (the serial of the newest file known at build time), never from "no floor". If it can't be read *now* (permissions, I/O, no file handles), the refresh fails closed: nothing is fetched, saved or newly served, and the next refresh reads it again; a save never overwrites a file it couldn't read. Each read-modify-write holds an advisory lock beside the file. A build that pins keys must also compile in floors (a build-time check).
- **Channel binding:** a file for `beta` is rejected by a `stable` app.

### 4. Layers and precedence

From lowest to highest; a higher layer overrides a lower one, section by section, by each module's merge rule:

1. **Compiled defaults:** always present. The app is complete with nothing else.
2. **The public channel file:** the same for everyone on a channel. It's fetched anonymously from a static host (a CDN or GitHub release asset).
3. **The account file,** only when an account is linked: per-account and per-entitlement overrides. It's fetched from the account API with the device token, and signed by the server's own pinned key (§6).
4. **The user's own settings:** a user's explicit choices win over remote *preferences*, never over remote *safety*. Kill switches (`OffOnly` flags set off) and compat narrowing beat user settings. Notices respect the user's dismissals.

`effective = merge(defaults, public, account, user)` is one pure function. The Settings › Advanced "About this configuration" view shows, per value, which layer set it, so support and debugging never guess.

### 5. Targeting, evaluated on the Mac

Targeting rules live in the file and are evaluated locally, so the anonymous fetch sends nothing about the user. Rules match on:
- app version range;
- channel;
- OS and OS version (D-52);
- installed tool versions;
- and a **rollout bucket**: a stable hash of a random per-install id, generated locally, never sent, and resettable in Settings.

A 10 % rollout means "buckets 0–9". No server ever learns who is in which bucket.

### 6. Logged-in users: the account layer and APIs

With an account linked, the app may also call the account API (ADR-0014's client) for:
- **The account file** (layer 3): per-account flags (beta programs), entitlements (a paid tier's features, such as option D's cloud summaries later), and targeted notices (billing, account security).
- **Typed capability endpoints,** added per feature behind the same transport seam, each with its own ADR when it arrives: sync, Fleet, cloud summaries, support diagnostics on request.
- **Signed responses:** the server signs the account file with its own pinned key, so a hijacked TLS session or a compromised CDN can't grant entitlements. The client also checks the account id inside the file against the linked account.

Unlinked users get layers 1, 2 and 4 only, and lose no core feature (D-18).

### 7. Fetching, caching, failure

- **When:** at launch (after first paint, per the lazy-load rule) and then at most once a day (D-128), failed attempts included. The background fetch skips Low Power Mode and metered connections where the OS says so.
- **How:**
  - one GET to a fixed URL per channel; `If-None-Match` only if a policy turns it on (off by default: a host could hand each install a unique entity tag and so recognise it);
  - no cookies;
  - a generic user agent (`AIJuggler/<version>`), with no install id, no account and no query string;
  - a timeout of 10 s;
  - a size cap of 256 KB.
- **Cache:** the last *accepted* file is kept with the trust state (§3) and re-verified on every load. A new file is used only after full verification and after the state recording it has been saved; if that save fails, nothing new is used. On any failure (network, signature, serial, expiry, size, parse), the app keeps the last good file, or falls back to compiled defaults if it's past its expiry. Clearing the registered cache (ADR-0023) never touches the trust state.
- **Old apps, new files:** a value an app can't read (a wider bound, a new option, a new field) drops only that value; a rule gated to newer app versions isn't read at all; kills and compat narrowing always apply, even from a section with a newer schema.
- **Off switch:** Settings › Privacy offers "Check for configuration updates". It's on by default, because it carries kill switches and compat; it's anonymous, and the page says what it fetches. Turning it off uses compiled defaults plus the account layer, if linked.
- **Applying:** sections apply live through a change event (no restart), in a module-defined order. A change that can't apply live is marked `on next launch`.

### 8. Publishing and governance

- **The source of truth** is a directory in the repo (`config/remote/<channel>.json`), unsigned. A change is a PR.
- **CI validates** every section against the modules' schemas, as exported by the app's build. It also dry-runs the merge against the last three released app versions, and blocks a change that would switch a flag on that's declared `OffOnly`.
- **Promote-style workflow:** a `Publish config` workflow, behind the same environment approval as releases, bumps the serial, signs, uploads to the static host and keeps the last 10 signed files for audit. Re-signing before expiry is automatic.
- **Emergency kill switch:** the same workflow with a `--kill <flag>` input. It still needs approval.
- **Observability:** with stats opted in (ADR-0015), the weekly report adds only the applied `serial` per channel, a number, so we know the file is reaching people.

### 9. Code layout

- **`crates/juggler-remote-config`** (new; pure Rust; no Tauri, no UI, so it works on every OS):
  - the envelope;
  - canonical bytes;
  - verification with pinned keys;
  - serial and expiry state;
  - the `Section` registry;
  - the layer merge;
  - targeting and bucketing;
  - the fetch through an injected transport (recordable, like the account client).

  It must not know: any feature, tool or screen.
- **Feature modules** register their sections in their own crates or features: compat in juggler-adapter, flags in each feature, notices in a small `features/notices` with its UI.
- **`juggler-account`** gains `account_config()` (layer 3) and the server-key check.
- **UI:**
  - one `useRemote(section)` hook over a Tauri command, returning typed values;
  - notices render through the D-126 Alert component, or a quiet banner on Home;
  - the "About this configuration" view.

### 10. Testing

- **Unit:**
  - canonical bytes;
  - a bad signature, an unknown key, a revoked key, a lower serial, an expired file, the wrong channel, an oversize file;
  - unknown sections and fields ignored;
  - per-section validator isolation;
  - the precedence table;
  - the bucket's stability.
- **Property tests:** merge is idempotent, and the defaults layer alone equals today's behaviour.
- **A golden file per channel** in CI, plus the dry-run against old app versions.
- **A recording transport** proves the fetch sends no identifying header or query.
- **A fault-injection run:** the host down, a truncated file, a slow host. The app starts and works identically.

## Consequences

**Good:**
- Version support, kill switches, rollouts, notices and tunables change without a release.
- New features add a section and are done.
- Old apps keep working with new files.
- Account features have a clear, signed path.

**Costs:**
- A signing key to guard.
- A re-sign job that must run.
- One more privacy statement on the Settings page.
- Reviewers must check each flag's `remote:` mode.

**Not chosen:**
- **A third-party flag service:** it adds a vendor, sends identifiers and doesn't fit the privacy promise.
- **Remote scripts or rules engines:** that's remote code.
- **Unsigned files over TLS only:** that trusts the host and the CDN.
- **Per-user server-side targeting for anonymous users:** it needs an identifier.

## Order of work

1. **This crate and the compat section (D-128),** with the publish workflow, in 0.5.
2. **Flags and notices,** with their UI, in 0.5.
3. **Params and endpoints,** as features need them.
4. **The account layer and the server key,** with the next account feature (cloud summaries or sync), ADR per capability.

## Open questions for the maintainer

1. **Where the public files are hosted:** our website's CDN, or GitHub release assets? Both are static; the website gives nicer URLs and caching control.
2. **The default for "Check for configuration updates":** on by default (proposed, because kill switches and compat must reach people), with the Privacy page explaining it.
