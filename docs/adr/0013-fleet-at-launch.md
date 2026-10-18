# ADR-0013: Fleet at launch

**Status:** Accepted
**Date:** 2026-10-13
**Deciders:** Vuxtra engineering + maintainer (decision-log D-13)
**Implements:** D-13 (and SPEC §11)

## Context

Pass-2 recommended schema-only fleet (defer build post-1.0). The
maintainer overruled (D-13): Fleet ships with the launch, in beta, so
multi-machine integrates cleanly rather than being retrofitted.

## Decision

1. Fleet is a **Phase-0 architecture constraint**: identity
   (`MachineId`) on every ledger row, Switch Graph node, and usage
   event; ownership/verification/audit per machine; offline and
   conflict behaviour designed in.
2. Launch scope (beta): pairing (6-digit/QR, identity pinning),
   gRPC/mTLS transport (ADR-0004 preserved), remote Inspector (view),
   remote switch and cleanup under per-machine permissions
   (`View`/`Control`/`NoAccess`).
3. **Never**, at any tier: credential sync, login automation, traffic
   proxying, cloud relay. The §1/§8 invariants hold on every node; a
   console never acts on a node's spawn ledger without that node's
   permission.
4. The fleet security review (STRIDE: spoofed mDNS, MITM pre-pinning,
   replayed commands, stolen identity keys, malicious paired peer) is
   a **beta-exit gate** (AJ-E710).

## Alternatives considered

- **Schema-only now, build post-1.0** — the pass-2 recommendation;
  rejected by D-13 as retrofit risk.
- **Full fleet parity at launch (equal to local)** — rejected: remote
  control under permissions is the honest beta scope; full parity
  follows validation.

## Consequences

E7 ships identity/transport/pairing early and remote surfaces later in
beta; local designs must stay fleet-shaped in review; scope and
estimates re-planned around D-13 (recorded in EPICS phases).

## Architecture contract (AJ-E701, accepted 2026-10-18)

D-13, verbatim: "Fleet ships immediately with the launch, architected
and built from the start so multi-machine support integrates cleanly
rather than being retrofitted. This overrides the schema-only
recommendation in Part 08 §5 D6." It supersedes pass-2's D-2
(schema-only timing). ADR-0004's transport stands: gRPC over mutual
TLS, mDNS discovery, no relay. ADR-0014 adds one optional path —
account-based discovery of the user's own linked Macs — and changes
nothing below.

### Identity

- **`MachineId`** is the existing generated, persisted id
  (`local_machine_id` in app settings; every ledger row, Switch Graph
  node and usage event already carries it). It never changes after
  first run; the user-visible **label** ("Studio Mac") is a separate,
  editable, local-only display name.
- **Machine identity key**: an Ed25519 keypair generated at first run,
  private half stored in an owner-only (0600) file inside an
  owner-only (0700) `fleet/` folder in the app data dir, never logged,
  exported, sent over any link, or included in backups (AJ-E1009 and
  AJ-E1204 exclude `fleet/` by construction). It is not the device
  token of ADR-0014, not a Provider credential, and not the backup
  vault key — three separate secrets. (A Keychain-held key is a later
  hardening; it does not change the threat model below, because code
  running as the same user is out of scope — see "Accepted risks".)
- **Fingerprint**: SHA-256 of the 32-byte public key, shown as the
  first 16 bytes in 8 groups of 4 hex digits (`3F2A 9C01 …`). Pinning
  always compares the full key.
- **Lifecycle**: generate (first run) → pin (pairing or ADR-0014
  discovery) → unpair (delete the pin, revoke live sessions within
  2 s) → regenerate (only on explicit "Reset this Mac's identity";
  every peer must re-pair). Renaming changes neither id nor key.
- **Registry**: `fleet/peers.json` is signed with the identity key and
  carries a monotonic **generation** that the app also records in its
  own database; a bad signature, an unknown schema, a missing field,
  or a generation older than the recorded one (rollback) fails closed
  as `Trust broken — re-pair required`.

### Roles and permissions

- One binary, two roles: every instance is a **node** (serves its own
  state) and may act as a **console** (views and controls peers).
- Each node grants each paired peer exactly one level:
  **NoAccess** (paired, nothing served), **View** (telemetry and
  Inspector reads — the pairing default, shown as "View only"), or
  **Control** (shown as "View + control": View plus
  launch/stop/switch/clean/focus commands, each still a user click on
  the console). Control may additionally require **Ask each time**: a
  local confirmation on the node for every control command (this is
  the tier AJ-E711 calls "Act remotely: Ask each time").
- Permissions are **enforced on the node, per command and per stream
  item** — never cached from pairing, never trusted from the
  console's UI; a downgrade applies to streams already open. Raising
  a peer's level requires confirmation **on the node itself**.
- **Commands reference only what already exists on the node.** Launch
  takes a Launch-settings id that exists on the node and a project
  root the node already knows; stop and clean take node-side Instance
  ids and ledger process identities; focus takes an Instance id. No
  command carries a binary path, arguments, environment, or an
  arbitrary working directory — a Control peer can never run code the
  node's user did not set up.
- **Handover pack fetch** (the only transcript-bearing message)
  requires Control **and**, by default, a local confirmation on the
  node ("Allow <machine> to take this work?"); the node — not the
  console — decides, because a console's click is not verifiable.

### Transport and wire format

- gRPC over **TLS 1.3 only** (TLS 1.2 and 0-RTT disabled) with mutual
  authentication (`tonic` + `rustls`). Certificates are self-signed
  and derived from the identity key. Each side's verifier accepts a
  peer **only if the certificate's SubjectPublicKeyInfo is exactly a
  pinned Ed25519 key**, and still verifies the TLS 1.3
  CertificateVerify signature (possession of the key, not just its
  presentation). After the handshake the peer states its `MachineId`;
  it must equal the registry entry for that key or the connection is
  dropped. There is no CA and no anonymous mode. The single exception
  is the pairing handshake below, which runs over TLS with an unpinned
  verifier and grants nothing until it succeeds.
- **Wire format: protobuf** (closes SPEC §15 open item 1 and ADR-0004's
  "decided at the M5 spike"). Contracts live in-repo with a
  `protocol_version` (major.minor); a committed descriptor snapshot is
  compared in CI so a removed or renumbered field fails the build.
  Same major and minor: full service. One minor apart: degrade to core
  telemetry (heartbeat, resources, session deltas) with a labelled
  banner (SPEC §11.5). Further apart, or a version below the node's
  own minimum: refuse with a named reason. Degraded modes never relax
  permission, replay, or size checks. Unknown command versions return
  `Unimplemented`, never a guess.
- **Control commands** travel on one ordered command stream per
  connection and carry the console's `MachineId` (which must equal the
  TLS peer), a 128-bit nonce, a per-console monotonic sequence, and an
  expiry of 30 s **on the node's clock** (the node measures each
  peer's clock offset from heartbeats and converts; a peer whose
  offset is unknown or > 5 min cannot send commands until it is
  measured). The node persists each console's high-water sequence,
  keeps nonces for the expiry window, and rejects and audit-logs
  replayed, out-of-order, or expired commands. Re-pairing resets that
  console's high-water mark (a console restored from an old backup
  re-pairs rather than replays).
- **Payload allowlist**: only message types in the link's payload
  registry can be sent; none has a field for vendor credentials,
  keychain material, or transcript text except the pack fetch above.
  Deltas are accepted only with origin `MachineId` equal to the
  authenticated peer (no relaying of a third machine's records in
  beta).
- **Pre-auth and abuse limits**: at most 8 concurrent handshakes and a
  10 s handshake timeout; at most 32 connections; HTTP/2 at most 32
  concurrent streams with rapid-reset protection; 4 MiB maximum
  decoded message (packs are chunked); 20 control commands per second
  per peer.
- Heartbeat every 2 s; two misses = offline (≤ 5 s). Reconnects never
  weaken TLS settings.
- **Never logged**: pairing codes, session or exported keys, the
  identity key, tokens, or pack contents; key-log files
  (`SSLKEYLOGFILE`) are compiled out of release builds.

### Pairing

- Discovery (mDNS `_aijuggler._tcp`, or ADR-0014's account directory)
  is **unauthenticated** and never implies trust. The mDNS TXT record
  is minimal — protocol version and a random per-boot instance id; no
  label, platform, or fingerprint. Labels and fingerprints are
  exchanged only inside TLS. The listener exists only while Link is on.
- Pairing is **user-initiated on both machines and time-boxed**: the
  console opens a 2-minute pairing window and shows a random, single-use
  **6-digit code** (and a QR carrying the code and the console's
  fingerprint); the user enters (or scans) it on the new machine.
  Only one pairing session runs at a time per node.
- The code is the password of a **SPAKE2** exchange inside the pairing
  TLS session. Key confirmation is a MAC, under the SPAKE2 key, over a
  transcript of both identity public keys, both `MachineId`s, the
  `protocol_version`, and the TLS exporter value — so the confirmed
  key, the TLS-observed key, and the key that gets pinned are one and
  the same, and a network attacker gets one online guess per attempt
  and learns nothing offline. A scanned QR additionally requires the
  console's TLS key to match the QR fingerprint.
- Each failure ends the session and rotates the code. Five failures
  lock pairing on that node for 60 s; twenty failures in 24 h disable
  pairing until the user re-enables it. Both counters persist across
  restarts. The attempt log records time, peer address, and outcome —
  never the code entered.
- Success shows the other machine's label and fingerprint on **both**
  screens with a Confirm on each, then pins both keys and records the
  chosen permission (View by default).
- ADR-0014 discovery lists the user's own linked Macs, but **never
  pins silently**: the user confirms the peer's fingerprint, shown on
  both machines, before the pin; such peers start at View and are
  labelled "via your Figurehead account". The TLS handshake still
  proves possession. A compromised account can therefore offer a key,
  but not get one pinned without the user's confirmation on both ends.

### Sync, offline and conflicts

- Each node stamps syncable records with origin `MachineId`, a
  per-node monotonic sequence (for delta exchange) and a **hybrid
  logical clock** timestamp (for ordering across machines); wall-clock
  time is display-only, and skew > 5 min is flagged.
- Reconnect exchanges sequence vectors and sends only missing deltas
  (full state only on first pair). Delivery is idempotent. Outbound
  deltas sit in a durable SQLite outbox.
- Conflict classes: telemetry — newest hybrid-logical-clock value
  wins; labels/settings — last writer by hybrid logical clock wins,
  loser kept in History; lineage and session records — **never
  merged**, shown as `Diverged` and blocking destructive actions until
  the user picks Keep mine / Keep theirs / Keep both (D-05).
- Freshness on every remote value: `Live` (< 5 s), `Stale`,
  `Offline — last seen <time>`.
- Process rows are keyed by `session_id + machine_id + boot_id`
  (PID/PGID are unique only per machine and boot). Remote cleanup
  runs the **owning node's** verify-before-kill ladder, never the
  console's.
- Every remote control action is audit-logged on node and console;
  fleet audit entries are **hash-chained** so a deleted or edited
  entry is detectable.
- A received handover pack is **untrusted input**: shown in the
  switch wizard's preview before use, redacted per the destination's
  policy (D-14), and handed to the destination CLI as data.

### What never crosses the link

Vendor credentials, keychain contents, the identity private key, the
ADR-0014 device token and vault key, and transcript text — except the
explicit, user-clicked handover pack fetch.

### Options considered

| Option | Strength | Cost | Verdict |
|---|---|---|---|
| LAN + mDNS, mutual TLS, no relay | Zero cloud trust, works offline | LAN-only without a VPN | **Adopted** (primary) |
| User's own VPN (Tailscale/WireGuard) | Encrypted, NAT traversal, user-owned | User installs it | **Adopted** for remote machines |
| Account directory (ADR-0014) | No pairing code between your own Macs | Needs an optional account | **Adopted** as an alternative to the code, never a replacement |
| Cloud relay | Works anywhere | New trust target, bandwidth, credential risk | **Rejected** (needs its own decision) |
| CRDT / full-state auto-merge | Automatic convergence | Silent merges violate D-05 | **Rejected** — sequence vectors + per-class rules + explicit conflicts |
| JSON-RPC wire format | Simple, app-server style | No schema'd versioning or breaking-change check | **Rejected** — protobuf |

### STRIDE threat model

| Threat | STRIDE | Mitigation | Owning task / negative test |
|---|---|---|---|
| Spoofed mDNS advertisement | Spoofing | Discovery is never trust; minimal TXT; connections need a pinned key; pairing needs the SPAKE2 code | AJ-E704 anonymous-drop + mismatch tests |
| Online guessing of the pairing code | Spoofing | Single-use 6-digit code, rotated per failure; one session at a time; 2-min user-opened window on both ends; 5-fail/60 s and 20-fail/24 h lockouts persisted across restarts | AJ-E704 brute-force, parallel-session and restart tests |
| MITM during pairing | Spoofing / Tampering | SPAKE2 + key confirmation binding both keys, both MachineIds, protocol version and the TLS exporter | AJ-E704 MITM-relay test (confirmation fails) |
| MITM after pairing | Tampering / Info disclosure | TLS 1.3 only; SPKI must equal a pinned key; CertificateVerify checked; claimed MachineId must match the pin | AJ-E703 wrong-cert, TLS 1.2, key-with-other-MachineId tests |
| Protocol downgrade | Tampering | Node minimum version; degraded modes keep every security check | AJ-E703 downgrade-matrix test |
| Replayed or delayed command | Tampering / Repudiation | Ordered command stream; persisted per-console high-water sequence; nonce cache; 30 s expiry on the node's clock with measured offset; console id = TLS peer | AJ-E703 replay, reorder, expired and wrong-console tests |
| Control peer runs arbitrary code | Elevation | Commands reference only node-existing ids (Launch settings, project roots, Instances, ledger processes); no binary/args/env/cwd fields | AJ-E709 forged free-form-launch test |
| Malicious peer pulls transcripts | Info disclosure | Pack fetch needs Control and, by default, a confirmation on the node; payload allowlist | AJ-E706 zero-pre-click-bytes + node-refusal tests |
| Spoofed delta origin | Tampering | Deltas accepted only with origin = authenticated peer | AJ-E708 origin-spoof test |
| Account-directory key substitution | Spoofing | Account peers never pin silently: fingerprint confirmed on both machines; start at View | AJ-E1206 substitution test |
| Tampered or rolled-back registry | Tampering | Signed with the identity key; generation mirrored in the app DB; fail closed | AJ-E702 tamper + rollback tests |
| Stolen identity key (other user or a copied backup) | Spoofing | 0600 file in a 0700 folder; never in backups or logs; "Reset this Mac's identity" + Unpair on peers | AJ-E702 permission + regenerate tests; AJ-E1009 exclusion test |
| Resource exhaustion before or after auth | Denial of service | Handshake/connection/stream caps, rapid-reset protection, 4 MiB decode limit, per-peer command rate | AJ-E703 flood test |
| Secrets in logs | Info disclosure | Codes, keys, tokens, pack contents never logged; key-log off in release | AJ-E710 log-scan test |
| Malicious pack contents (prompt injection) | Tampering | Packs are untrusted data: previewed, redacted, passed as data | AJ-E706 |
| Console kills the wrong process | Tampering | Owning node re-verifies identity (pid + start time + exe + owning Instance); unverifiable = never touched | AJ-E707 PID-reuse + unverifiable tests |
| Clock manipulation reorders history | Tampering | Hybrid logical clocks; wall clock display-only; skew flagged | AJ-E708 skew test |
| Repudiation of remote actions | Repudiation | Hash-chained audit on node and console | AJ-E707 / AJ-E709 audit-chain tests |

No row is open: each threat has a mitigation and a named negative test;
the AJ-E710 beta gate fails closed if any of those tests is red.

### Accepted risks

- **Code running as the same user** (malware with the user's
  privileges) can read the identity file, re-sign the registry, and
  drive the app. That is outside what an unprivileged desktop app can
  defend; the mitigation is detection and recovery (Reset identity,
  Unpair from peers), not prevention.
- **LAN metadata**: an observer on the same network can see that an AI
  Juggler node is present (the minimal mDNS record and the open port).
- **Pairing lockout abuse**: while a pairing window is open, a LAN
  attacker can spend the failure budget and lock pairing (60 s, or
  until re-enabled after 20 failures). This is a denial of service on
  pairing only — it never pins anything — and is the price of making
  online guessing infeasible.

## Open questions (beta-exit criteria, AJ-E511) — resolved by the contract above

The five questions below were recorded by AJ-E511; the AJ-E701
contract answers each: (1) labels are local-only, collisions show
both with their fingerprint; (2) row-level deltas keyed by
`session_id + machine_id + boot_id`; (3) permission re-checked per
command, remote cleanup on the owning node's ladder; (4) `Live` < 5 s,
then `Stale`, then `Offline — last seen`, foreign rows display-only
while stale; (5) the STRIDE table above, gated by AJ-E710.

Recorded per D-13/D-15 without implementing transport — shaping the
data is this task's whole scope (`juggler-link` is SPEC §11):

1. **Identity**: machine labels are user-editable display names over
   the generated id (`This Mac` default) — when does a re-labelled
   machine propagate, and what wins on label collision?
2. **Sync/transport**: gRPC/mTLS per ADR-0004 — do ledger deltas sync
   row-level (composite `session_id+machine_id+boot_id` key) or as
   per-machine snapshots? PID/PGID are unique only per machine+boot;
   any merge without all three re-creates the false-kill bug at fleet
   scale (E511's disjoint-rows test is the standing guard).
3. **Remote ownership**: a console must never signal without the
   owning node's permission (`View`/`Control`/`NoAccess`) — is
   permission re-checked per command or cached per pairing? Remote
   cleanup rides the OWNING node's E507 ladder, never the console's.
4. **Offline**: stale foreign rows must not present as live — the
   inventory marks foreign rows display-only until a freshness
   heartbeat exists; what is the staleness threshold, and do orphaned
   foreign rows ever expire locally?
5. **Security**: the STRIDE review (spoofed mDNS, MITM pre-pinning,
   replayed commands, stolen identity keys, malicious paired peer) is
   the AJ-E710 beta-exit gate; identity pinning UX (6-digit/QR) is
   unvalidated.
