# E7 — Fleet Architected and Built at Launch (Beta)

> Status: TODO · Epic owner: Fleet/Core · Source decisions: D-13 (fleet at
> launch, Phase-0 constraint, overrides schema-only), D-15 (this file is
> Stage-1 documentation only; no product code is authorised by this file
> alone). Normative sources: `docs/SPEC.md` S11 (§11, lines 362–394),
> `docs/adr/0004-fleet-transport-grpc-mtls.md`, `docs/review/pass2/08-proposal-delta-and-plan-updates.md`
> §2.2 D-2 (superseded by D-13), `docs/review/decision-log.md:126-137`.
> Section references: SPEC "§11 Fleet" (cite by section, not line).
> Architecture contract: ADR-0013 "Architecture contract (AJ-E701)".
> Current state at writing: `crates/juggler-link/src/lib.rs` was a doc-only
> stub; `ui/src/App.tsx:340` renders `<ComingSoon screen="Fleet" />`;
> `SessionRecord.machine: MachineId` already exists
> (`crates/juggler-core/src/model.rs:389-398`). Fleet SHALL ship in beta
> with honest labelling (D-04/D-11 pattern): unvalidated paths show
> `Unvalidated`/`Unavailable — reason`, never dressed-up estimates (D-06).
> Security evidence is mandatory for every task in this epic: no task
> closes on UI screenshots alone.

## Comparable-sync research baseline (applies to all tasks)

| Approach | Strength | Cost for Juggler | Verdict |
|---|---|---|---|
| Tailscale/WireGuard overlay (user VPN) | E2E encrypted, NAT traversal, identity | User must install VPN; no app relay | Adopt as remote path (SPEC.md:369) |
| Local-first LAN + mDNS, no relay | Zero cloud trust, offline-capable | LAN-only without VPN | Adopt as primary (ADR-0004) |
| Cloud relay (vendor backend) | Works anywhere | New trust target, credential risk | Rejected (ADR-0004:19) |
| CRDT/full-state sync | Automatic merge | Overkill, silent-merge risk vs D-05 | Rejected; use seq + LWW-per-field + explicit conflict UI (AJ-E708) |

### AJ-E701 Fleet architecture ADR and threat model | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: none
**Goal**: Freeze fleet identity, transport, sync, and offline contracts in an ADR so local switching, inspector, spawn-ledger, and cleanup are fleet-shaped from day one.
**User story**: As a maintainer (P4 Operator), I need one signed-off fleet contract so three agents do not invent three incompatible sync models.
**Requirements**:
1. The ADR SHALL supersede D-2 schema-only timing and cite D-13 verbatim, while preserving ADR-0004 transport (gRPC/mTLS/mDNS, no relay).
2. The ADR SHALL define `MachineId`, node/console roles (either role per binary, SPEC.md:368), identity lifecycle, and per-machine permission levels (`View`/`Control`/`NoAccess`).
3. The ADR SHALL include a STRIDE threat model covering spoofed mDNS, MITM before pinning, replayed commands, stolen identity key, and malicious paired peer.
4. The ADR SHALL state invariants: vendor credentials/keychain/transcripts never traverse the link except explicit user-clicked handover pack fetch (SPEC.md:377,383).
5. The ADR SHALL compare Tailscale-overlay, LAN-only, cloud-relay, and CRDT options with a decision table and rejected-option rationale.
**UI/UX reasoning**: No user UI. Doc review follows NN/g visibility-of-system-status: every later fleet surface must expose connection, trust, and staleness state defined here. Copy terms fixed: `Machine`, `Pair`, `Unpair`, never `device sync`.
**Performance budget**: ADR review ≤2 rounds; no runtime budget. Decision latency target: accepted before AJ-E702 starts.
**Quality gates**: `cargo test -p juggler-core` green (no code change expected); markdown links resolve; ADR follows `docs/adr/0000-template.md`.
**Evidence/validation proof**: Strict: ADR file committed; threat-model table reviewed in PR with checklist signed by a second reviewer; evidence stored in the PR description and `docs/adr/0013-fleet-at-launch.md`. Pass = no unresolved `TODO(threat)` rows.
**Visual testing**: None (document). Verify rendered Markdown table in PR preview at 100-col width; no broken links.
**Files to touch**: [0013-fleet-at-launch.md](../adr/0013-fleet-at-launch.md), [SPEC.md S11](../SPEC.md#L362), [decision-log.md](../review/decision-log.md#L126)
**Research basis**: decision-log.md:126-137 (D-13); SPEC.md:362-394; ADR-0004:13-25; pass2/08 §2.2 D-2 superseded.
**Handover note**: Do not re-litigate D-13 or reintroduce a cloud relay. Trap: SPEC.md:476 still lists protobuf-vs-JSON-RPC as open — this ADR must close it (recommend protobuf per ADR-0004 versioning).

### AJ-E702 Machine identity and pairing trust store | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E701
**Goal**: Each install generates a durable Ed25519 machine identity; paired peers are pinned in a tamper-evident registry under `fleet/`.
**User story**: As a developer with a laptop and a Mac mini, I pair once and trust that no other machine can impersonate either.
**Requirements**:
1. First run SHALL generate an Ed25519 keypair; private key SHALL be stored in a 0600 file inside a 0700 `fleet/` folder and SHALL never be exported over the link, logged, or included in backups. *(Task-text fix 2026-10-18, AJ-E701 contract: a Keychain-held key is a later hardening; same-user code is an accepted risk either way.)*
2. The self-signed TLS certificate SHALL be derived from the identity; fingerprint SHALL be shown as grouped hex during pairing.
3. The paired-peer registry (`fleet/peers.json` via atomic write) SHALL record `MachineId`, label, fingerprint, permissions, paired-at, last-seen, schema version, and a monotonic generation mirrored in the app database (rollback detection).
4. Registry tamper (bad signature/checksum/missing fields) or rollback (generation older than the recorded one) SHALL fail closed: peer shown `Trust broken — re-pair required`, no connection attempted.
5. Unpair SHALL delete the pin, revoke live sessions within 2 s, and require re-pairing for any future connection.
6. Machine rename SHALL be local-label-only and SHALL not change `MachineId` or fingerprint.
**UI/UX reasoning**: Apple HIG (identity/verification): show fingerprint in monospace groups of 4 with a Copy button; destructive Unpair uses a confirmation sheet naming the machine. Nielsen #1: pairing state (`Unpaired/Pairing/P aired/Trust broken`) always visible in Fleet header.
**Performance budget**: Keygen <200 ms; registry load <50 ms for 50 peers; unpair revocation <2 s; storage <5 KB per peer.
**Quality gates**: Unit coverage ≥90% for `juggler-link` identity module; `cargo clippy -- -D warnings`; no private-key type implements `Serialize` for network DTOs.
**Evidence/validation proof**: Strict: `cargo test -p juggler-link identity::` passes including tamper, unpair-revocation, and key-permission (0600) tests; `grep -rn "private_key" crates/juggler-link/src --include="*.rs"` shows no log/network sink; evidence pasted in PR and `docs/qa/gate-E7.md`. Security evidence required: negative test proves a modified fingerprint is rejected.
**Visual testing**: Screenshots: fingerprint sheet at 1280×800 and 1024×680 (the window minimum; 800×600 is below it — task-text fix 2026-10-18), light/dark; a11y: fingerprint reachable by keyboard, `aria-label="Machine fingerprint"`; pass = no truncation, copy button focusable.
**Files to touch**: [lib.rs](../../crates/juggler-link/src/lib.rs), [model.rs](../../crates/juggler-core/src/model.rs#L64), [storage/mod.rs](../../crates/juggler-core/src/storage/mod.rs), [SPEC.md storage](../SPEC.md#L395)
**Research basis**: SPEC.md:374-375,402; EPICS M6-1 (`docs/plan/EPICS.md:130`); ADR-0004:13; deep/08 §1 (link stub).
**Handover note**: Trap: do not reuse harness credentials or the app config-encryption key as the fleet identity. `MachineId` already exists — extend it, do not create a parallel `DeviceId`.


### AJ-E703 Transport and security model (gRPC/mTLS, versioned protobuf) | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E701, AJ-E702
**Goal**: Authenticated, encrypted, versioned streaming between nodes with graceful mixed-version degradation and replay rejection.
**User story**: As a console user, I see live remote state and know commands cannot be forged, replayed, or silently downgraded.
**Requirements**:
1. Transport SHALL be gRPC over TLS 1.3 with mutual certificate auth (`tonic`+`rustls`); plaintext listeners SHALL not exist outside `#[cfg(test)]` loopback fixtures.
2. Protobuf contracts SHALL live in-repo, carry `protocol_version`, and pass breaking-change lint in CI; unknown command versions SHALL be rejected with `Unimplemented`, never guessed.
3. Mixed-version peers within one minor window SHALL degrade to core telemetry (heartbeat/resources/session deltas) per SPEC.md:393; the UI SHALL label degraded mode with reason.
4. Every control command SHALL carry nonce + monotonic sequence + expiry (≤30 s); replayed or expired commands SHALL be rejected and audit-logged.
5. The link SHALL never carry vendor credentials, keychain material, or raw transcripts except an explicit user-clicked handover pack fetch (AJ-E706); a type-level allowlist SHALL enforce payload classes.
6. Connection loss SHALL surface within 5 s (heartbeat 2 s, 2 misses) and SHALL not retry with weakened TLS settings.
**UI/UX reasoning**: NN/g #1 visibility: connection chip shows `Connected · mTLS · v1.2` / `Degraded — peer v1.0, telemetry only` / `Offline`. Errors use plain language plus a Details disclosure with the TLS/version reason; no raw stack traces (WCAG 3.3.1 error identification).
**Performance budget**: LAN heartbeat RTT p95 <50 ms; telemetry delta <1 s end-to-end at 20 Instances/machine; TLS handshake <300 ms LAN; link CPU <1% idle, <3% at 5 machines × 20 Instances; memory <30 MB per peer connection.
**Quality gates**: breaking-change check in CI (committed descriptor snapshot, `tests/proto_contract.rs` — replaces `buf lint`/`buf breaking`, task-text fix 2026-10-18: no third-party CLI under the org action policy); clippy pedantic clean; fuzz target for frame decoding exists (`fuzz/`, nightly) and a 20 000-frame randomized decode test runs in CI without panic.
**Evidence/validation proof**: Strict: loopback two-node test (`cargo test -p juggler-link --test transport`) proves mTLS success, wrong-cert rejection, replay rejection, and downgrade matrix (current↔previous minor); packet-fixture assertion proves no credential-shaped fields in telemetry protos; logs stored as CI artifacts linked from `docs/qa/gate-E7.md`. Security evidence required: negative tests, not just happy-path.
**Visual testing**: Screenshots: degraded-mode banner and offline banner in Fleet at 1280×800; pass = banner names the cause and offers `Retry`/`Unpair`, keyboard-focusable; colour never sole channel (icon + text).
**Files to touch**: [lib.rs](../../crates/juggler-link/src/lib.rs), [Cargo.toml](../../Cargo.toml), [SPEC.md S11.2](../SPEC.md#L371), [ADR-0004](../adr/0004-fleet-transport-grpc-mtls.md)
**Research basis**: SPEC.md:373-377,391-394; ADR-0004:13-25; SPEC.md:440 (loopback fleet tests); Tailscale tradeoff table in epic header.
**Handover note**: Trap: do not add a JSON-RPC fallback to "move faster" — SPEC.md:476 open item is closed by AJ-E701. Certificates pin identity from AJ-E702; regenerating identity must force re-pair.

### AJ-E704 Discovery and pairing wizard (mDNS + code/QR) | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E702, AJ-E703
**Goal**: A new machine joins in under 60 seconds via mDNS discovery plus a 6-digit code or QR, with per-machine permissions chosen at pair time.
**User story**: As a developer adding a second machine, I open Fleet, tap Add a machine, compare a code, and I am paired — no config files.
**Requirements**:
1. Nodes SHALL advertise via mDNS/Bonjour (`mdns-sd`) only when Link is enabled; disabling Link SHALL withdraw advertisement within 5 s.
2. The wizard SHALL list discovered unpaired machines as "Unnamed machine (<instance id>)" — labels, platform and fingerprint are revealed only inside the pairing exchange, never from mDNS (AJ-E701 minimal TXT record) — and SHALL drop unpaired/anonymous connections.
3. Pairing SHALL be user-initiated on both machines inside a 2-minute window: the console shows a single-use 6-digit code (or QR) that the new machine enters as the SPAKE2 password, followed by a label + fingerprint Confirm on both screens; a wrong code SHALL abort, rotate the code, rate-limit (5 attempts → 60 s lockout; 20 in 24 h → pairing disabled until re-enabled; persisted), and log the attempt without the entered code. *(Task-text fix 2026-10-18, AJ-E701 contract.)*
4. The final step SHALL set permissions (`View only` default, or `View + control`) and SHALL explain in one sentence what control allows.
5. Pairing SHALL complete in <60 s on a LAN fixture; failure states (no machines found, code mismatch, peer rejected, pairing window closed after 2 min) SHALL each have distinct copy and a next action.
6. Remote (non-LAN) joining SHALL be documented as "connect your machines with Tailscale/WireGuard first, then pair" — the app SHALL not offer a relay or public-internet pairing path.
**UI/UX reasoning**: Nielsen #5 error prevention + Apple HIG progressive disclosure: 3 steps max (`Find → Verify → Permissions`), Back always available, code input auto-advances per digit and supports paste. Shortcut: `⌘⇧F` opens Fleet from anywhere. Empty state copy: "No machines found. Check both machines are on the same network with Link on."
**Performance budget**: Discovery list populated <5 s on LAN; browse CPU <1%; wizard step transition <200 ms; lockout timer survives app restart (persisted).
**Quality gates**: Wizard cannot be completed with mismatched codes in Playwright flow; permissions default is `View only` (asserted in test); no anonymous-mode flag exists in settings.
**Evidence/validation proof**: Strict: two-instance loopback acceptance script times pairing (<60 s) and proves unpaired connection dropped, mismatch aborted, lockout enforced; human dogfood on 2 physical Macs recorded (screencast + timing) in `docs/qa/gate-E7.md`. Security evidence required: attempt-log excerpt showing rejected anonymous and mismatched attempts.
**Visual testing**: Screenshots for all 3 steps + 4 failure states at 1280×800 and 800×600, light/dark; QR fallback shown when camera unavailable; a11y: full keyboard pairing without camera/mouse, VoiceOver labels on code fields; pass = task completable keyboard-only.
**Files to touch**: [App.tsx](../../ui/src/App.tsx#L340), [lib.rs](../../crates/juggler-link/src/lib.rs), [bridge.ts](../../ui/src/lib/bridge.ts), [SPEC.md S11.4](../SPEC.md#L385)
**Research basis**: SPEC.md:369,375,387; EPICS M6-2/M6-3; pass2/08 §4 Gate V6 criteria moved to launch by D-13.
**Handover note**: Trap: mDNS is unauthenticated discovery only — never treat discovery as trust. The SPAKE2 code exchange is the trust step; do not auto-pair "same Apple ID/network" machines. ADR-0014 account discovery likewise never pins silently — the fingerprint is confirmed on both machines (AJ-E1206).
**Task-text notes (2026-10-18, AJ-E704)**: (1) The QR is replaced by a copyable pairing link (`aijuggler-pair:v1:<code>:<fingerprint>`) — desktop-to-desktop pairing has no camera path, and the link also lets the joiner verify the host's fingerprint automatically. (2) Screenshots use 1280×800 and 1024×680 (the app's minimum window) instead of 800×600, which is below it. (3) Each side chooses what it grants the other in its own Permissions step. Evidence: [AJ-E704 manifest](../qa/evidence/AJ-E704/manifest.md).

### AJ-E705 Remote inspector (fleet-shaped right inspector) | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E703
**Goal**: Selecting a remote Machine, Account, or Instance fills the same six-tab right inspector as local selections, with honest staleness and permission states.
**User story**: As a console user (P1 Multi-Maxer), I click an Instance on my mini and see Overview, Usage, History, Quality/Performance, Processes, and Actions without learning a second UI.
**Requirements**:
1. The inspector SHALL render all six tabs (D-06) for remote selections; fields the peer does not emit SHALL show `Unavailable — <reason>`, never estimates dressed as measurements (D-07: measurable signals only, source-labelled).
2. Every remote value SHALL carry source machine, measured-at timestamp, and freshness state (`Live <5 s`, `Stale`, `Offline — last seen <time>`).
3. Remote Actions SHALL be permission-gated: `View only` peers show actions disabled with tooltip "View-only pairing — change in Fleet settings"; control actions still require an explicit click (automation invariant, SPEC.md:26).
4. Inspector render SHALL be <200 ms from cached deltas; a remote selection SHALL not block the local board when the peer is slow or offline.
5. Quality/Performance SHALL show only D-07 signals (error/retry, stuck/waiting, context use, tokens/cost per completed task where known, process health) — no composite score.
6. Switching the selected machine SHALL preserve inspector tab and scroll position per selection.
**UI/UX reasoning**: NN/g #3/#4 (consistency, user control): identical tab order local/remote; machine breadcrumb `Fleet / mac-mini / Instance` with `⌘[` back. WCAG 2.1: stale data announced via `aria-live="polite"` summary, not per-field chatter. Shortcut: `]` cycles tabs.
**Performance budget**: Render <200 ms; delta application <50 ms; UI stays 60 fps with 5 machines × 20 Instances; cached inspector payload <200 KB per machine.
**Quality gates**: Playwright: select remote → all tabs render → offline peer shows stale state without spinner deadlock; no fabricated-field snapshot test (empty source renders Unavailable).
**Evidence/validation proof**: Strict: fixture peer emitting partial data proves Unavailable rendering; two-machine dogfood: select remote Instance, read Usage/Processes, screenshot + timing log in `docs/qa/gate-E7.md`; pass = every field source-labelled. Security evidence: view-only peer action buttons provably disabled and commands rejected node-side (AJ-E709 test reference).
**Visual testing**: Screenshots: remote Instance Overview/Processes (live, stale, offline, view-only) at 1440×900 and 1024×768, light/dark; a11y contrast ≥4.5:1, stale icon+text redundant; comparison = side-by-side local vs remote layout diff, pixel-identical tab geometry.
**Files to touch**: [WorkPanels.tsx](../../ui/src/screens/WorkPanels.tsx), [WorkflowRail.tsx](../../ui/src/components/WorkflowRail.tsx), [bridge.ts](../../ui/src/lib/bridge.ts), [SPEC.md S9](../SPEC.md#L316)
**Research basis**: D-06/D-07 in decision-log.md:60-77; pass2/04 §§5-6; SPEC.md:350,385-389; expected-worfklow.md:14-15.
**Handover note**: Depends on the local inspector existing — if E-inspector tasks slip, ship remote inspector read-model against the same DTO, do not fork a fleet-only panel. Trap: never hide staleness to look "real-time".

### AJ-E706 Cross-machine switching implications | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E703, AJ-E705
**Goal**: The standard switch wizard works across machines, computing fidelity honestly and pre-validating path/binary constraints before the user commits.
**User story**: As a developer who hit a quota wall on my laptop, I switch the same work to my desktop Account in the same wizard I use locally.
**Requirements**:
1. Cross-machine targets SHALL appear in the switch wizard with machine label, fidelity badge (validated/unvalidated per D-04), and pack treatment (copy/translate/regenerate/drop per D-14 policy for the destination Provider).
2. T2 cross-machine SHALL require same harness binary version and same repo checkout path on the target; unmet preconditions SHALL be listed before execution with a T3 (replay) fallback offered, never a silent downgrade.
3. The handover pack SHALL be fetched on demand only after the user clicks Switch; browsing targets SHALL transfer no transcript or pack content.
4. D-05 SHALL hold across machines: switch-back resumes the original native session plus one summarised delta turn; copy-back over the original session is prohibited; divergence is surfaced, never overwritten.
5. Absolute paths in sessions SHALL be remapped only via an explicit user-confirmed mapping shown in the wizard; unmapped paths SHALL block T2 with an explanation.
6. Every cross-machine switch SHALL write lineage (Switch Graph edge with source/target `MachineId`) resumable from either machine after sync (AJ-E708).
**UI/UX reasoning**: D-03 hybrid: same-Provider proven-fidelity = one click + undo window; cross-machine/low-fidelity/diverged = brief wizard. Nielsen #5: irreversible or lossy steps list "will drop" items in plain language before the Switch button enables. Copy: button `Switch to mac-mini…`, never "lossless".
**Performance budget**: Target list with preconditions <1 s for 5 machines; pack fetch progress for >10 MB packs, cancel <1 s; precondition check <500 ms per target (cached binary-version probe, 60 s TTL).
**Quality gates**: Fixture matrix per Provider pair extended with machine dimension; guardrail test: no pack bytes flow before click (network fixture assertion); path-mapping unit tests.
**Evidence/validation proof**: Strict: live dogfood A(laptop)→B(desktop)→A with continuity checklist and delta-turn screenshot; fixture test proves T2 blocked + T3 offered on version mismatch; evidence (timings, lineage export) in `docs/qa/gate-E7.md`. Security evidence required: pre-click network capture summary proving zero transcript bytes during target browsing.
**Visual testing**: Screenshots: wizard target list (T2-ready, T2-blocked, T3-only), path-mapping step, divergence-on-return at 1280×800; a11y: fidelity badge has text equivalent, wizard fully keyboard-operable; pass = blocked reason readable without colour.
**Files to touch**: [lib.rs](../../crates/juggler-link/src/lib.rs), [WorkPanels.tsx](../../ui/src/screens/WorkPanels.tsx), [model.rs](../../crates/juggler-core/src/model.rs#L389), [SPEC.md S11.4](../SPEC.md#L388)
**Research basis**: D-03/D-04/D-05/D-11/D-14; SPEC.md:388; pass2/02 §§2-6 (Switch Graph); pass2/08 F-N1/F-N5.
**Handover note**: Trap: sessions store absolute paths (SPEC.md:388) — the #1 cross-machine failure. Do not promise T2 until the live-binary dogfood for that Provider pair passes; label `Unvalidated` until then.


### AJ-E707 Remote process ownership and cleanup | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E703, AJ-E705
**Goal**:
Remote Instances' spawned processes stay owned, visible, and
verifiably cleanable from the console, with the same safety proof
as local cleanup — never a blind remote kill.
**User story**:
As a P4 Operator whose laptop Instance left a dev server holding
port 3000 on my desktop, I see it in the console Cleanup view,
verify it belongs to that Instance, and clean it in one click.
**Requirements**:
1. Every remote spawn SHALL be recorded in the owning node's
   spawn ledger (Instance id, `MachineId`, pid, start-time,
   process-group id, cwd, ports) at spawn time, not reconstructed
   from `ps` after the fact.
2. Process identity SHALL be verified before any kill by
   pid + start-time + executable + owning Instance; a pid-reuse
   mismatch SHALL abort with `Identity changed — not touched`.
3. Cleanup tiers SHALL follow D-08 per Account on the owning
   machine: notify-only by default; Clean-safe opt-in kills only
   verified-owned processes; every kill SHALL be audit-logged
   on both node and console.
4. Remote Clean SHALL require `View + control` permission and
   an explicit console click; the node SHALL re-verify ownership
   at execution time even if the console verified earlier.
5. Orphan detection on the node (owner Instance exited, child
   alive) SHALL surface to all paired consoles within 10 s
   with age, CPU/RAM, listening ports, and owning Instance.
6. Unowned or unverifiable processes SHALL be listed as
   `Not owned — never touched` and SHALL have no Clean action.
7. Killing a remote process group SHALL use the local
   SIGTERM→grace→SIGKILL ladder on the node; the console SHALL
   show per-process outcome (`Exited`, `Needed SIGKILL`,
   `Already gone`, `Refused — identity changed`).
**UI/UX reasoning**:
Nielsen #1/#5: Cleanup Centre groups by Machine → Instance,
each row shows verification state before Clean enables.
Copy: `Clean 3 verified processes on mac-mini` (destructive,
red only after verification). WCAG: outcome announced via
`aria-live`; shortcut `⌘⌫` only inside Cleanup with focus.
**Performance budget**:
Ledger write <5 ms per spawn; orphan detection ≤10 s;
remote Clean round-trip <2 s LAN for ≤50 processes;
Processes tab refresh 2 s poll, <1% CPU per machine.
**Quality gates**:
Fixture harness spawns child + grandchild + port listener;
after parent kill all three are detected, attributed, and
cleaned with zero collateral (same-binary unrelated process
untouched) — locally and remotely. PID-reuse fixture passes.
**Evidence/validation proof**:
Strict: `cargo test -p juggler-supervisor -p juggler-link`
cleanup fixtures green; two-machine dogfood leaves a real
`vite` orphan, cleans it remotely, and stores before/after
`lsof`/process screenshots + audit-log excerpt in
`docs/qa/gate-E7.md`. Security evidence required: audit log
proves who/what/when per kill; unverifiable-process negative
test proves no kill path exists.
**Visual testing**:
Screenshots: Cleanup grouped view (verified, unverified,
cleaning, done, identity-changed abort) at 1440×900 and
1024×768, light/dark. A11y: Clean reachable by keyboard,
focus returns to group header after action. Pass: zero
Clean buttons enabled on unverified rows.
**Files to touch**: [supervisor.rs](../../crates/juggler-supervisor/src/supervisor.rs), [journal.rs](../../crates/juggler-supervisor/src/journal.rs), [lib.rs](../../crates/juggler-link/src/lib.rs), [WorkPanels.tsx](../../ui/src/screens/WorkPanels.tsx)
**Research basis**:
D-08 (decision-log.md:79-87); pass2/05 §§2,9-10; SPEC.md:224-225,
234; expected-worfklow.md:17-18; deep/08 §1 (journal PID-only risk).
**Handover note**:
Trap: today's journal is PID-only (deep/08) and unsafe under
PID reuse — AJ-E707 must land start-time identity before any
remote kill ships. Remote kill without ledger ownership is a
launch blocker, not a beta caveat.

### AJ-E708 Offline and conflict behaviour | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E703
**Goal**:
Machines work fully offline, reconnect without silent data
loss, and surface conflicts explicitly instead of merging
them away.
**User story**:
As a developer on a train with my laptop offline, I keep
working locally; back home, my console catches up and shows
me exactly what diverged — nothing was overwritten quietly.
**Requirements**:
1. Offline SHALL degrade, never block: local launch, switch,
   inspector, and cleanup work identically with zero peers;
   fleet surfaces show `Offline — last seen <time>`.
2. Each node SHALL stamp syncable records with origin
   `MachineId`, monotonic per-node sequence, and wall-clock
   timestamp; clock skew >5 min SHALL be flagged, not trusted
   for ordering (sequence wins, clock is display-only).
3. Reconnect SHALL sync by sequence-vector exchange
   (what-I-have per peer) and transfer only missing deltas;
   full-state transfer is allowed only on first pair.
4. Conflicts (same record changed on two machines while
   apart) SHALL resolve per field class: telemetry =
   newest-sequence wins; labels/settings = last-writer wins
   with loser preserved in History; lineage/session records
   = never auto-merged, shown as `Diverged` for user choice.
5. A diverged record SHALL block destructive actions on it
   (switch-back, clean) until the user picks `Keep mine`,
   `Keep theirs`, or `Keep both (fork)`, each previewed.
6. Sync SHALL be idempotent: replaying the same delta batch
   (retry, duplicate delivery) SHALL change nothing.
7. Data at rest for queued outbound deltas SHALL survive
   app crash and OS reboot (durable outbox in SQLite WAL).
**UI/UX reasoning**:
NN/g #1: persistent sync chip (`Synced`, `Offline — 12 changes
queued`, `Diverged — 1 needs you`). Conflict sheet follows
Apple HIG: side-by-side, plain-language field names, default
focus on `Review later` (non-destructive). No auto-dismiss.
**Performance budget**:
Outbox durability write <10 ms; reconnect catch-up for
1,000 deltas <3 s LAN; idle sync CPU <0.5%; queued storage
<10 MB per peer per 24 h offline at typical telemetry rates;
conflict detection <100 ms per reconnect batch.
**Quality gates**:
Property test: shuffled/duplicate delta delivery converges;
kill-during-sync fixture loses zero records; clock-skew
fixture (peer ±10 min) orders by sequence correctly.
**Evidence/validation proof**:
Strict: automated partition test (two nodes, 200 offline
edits each incl. deliberate conflicts, reconnect) asserts
zero silent overwrites and exact conflict list; dogfood:
laptop offline 1 day, reconnect, conflict sheet screenshot
+ outbox log in `docs/qa/gate-E7.md`. Security evidence:
queued deltas are mTLS-only on reconnect; a rogue peer with
valid discovery but no pin receives zero deltas (negative test).
**Visual testing**:
Screenshots: offline board, queued-changes chip, conflict
sheet (3 choices), resolved state at 1280×800, light/dark.
A11y: conflict choices are radio group with descriptions;
pass = resolution completable keyboard-only, no colour-only
diff (added/removed also labelled in text).
**Files to touch**: [lib.rs](../../crates/juggler-link/src/lib.rs), [storage/mod.rs](../../crates/juggler-core/src/storage/mod.rs), [sessions.rs](../../crates/juggler-core/src/storage/sessions.rs), [App.tsx](../../ui/src/App.tsx)
**Research basis**:
D-05 divergence rule (decision-log.md:50-58); SPEC.md:381-383;
epic-header sync table (CRDT rejected, seq+LWW+explicit);
pass2/02 §6 (divergence), EPICS M7-4 clock-skew criterion.
**Handover note**:
Trap: do not adopt CRDT auto-merge for lineage — D-05 forbids
silent overwrite. Wall-clock last-writer-wins across machines
without the skew flag is the classic fleet data-loss bug.


### AJ-E709 Fleet indicators, permissions, and alert routing | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E704, AJ-E705
**Goal**:
Every board, tray, and inspector surface answers "which
Provider/Account/model is running where, and does it need me"
at a glance — with permissions enforced on the node, not
just hidden in the UI.
**User story**:
As a P1 Multi-Maxer with Instances on three machines, I scan
one board, spot the quota wall and the memory hog by machine,
and never wonder which laptop a row belongs to.
**Requirements**:
1. Every Instance row SHALL carry: machine chip (label +
   connection state), Provider icon, Account label, model,
   status dot + text, quota micro-bar with reset time where
   known (`Unknown` where not, per D-12 — never inferred).
2. Machines SHALL act as filters/columns across board,
   Overview, and Usage (SPEC.md:387,350); `All machines` is
   the default and SHALL persist per user across restarts.
3. Fleet alerts (memory pressure, orphan found, quota wall,
   peer offline >10 min) SHALL route to every paired console
   with source machine named; alert SHALL deep-link to the
   owning inspector tab.
4. Permissions SHALL be enforced node-side: a `View only`
   console's launch/stop/clean/switch commands SHALL be
   rejected with `PermissionDenied` and audit-logged, even
   from a modified client.
5. Changing a peer's permission SHALL require confirmation
   on the node machine itself (a local prompt), never
   silently from a console. *(Task-text fix 2026-10-18:
   the AJ-E701 contract removed the "pre-authorised console"
   path — a console can never raise its own access.)*
6. Tray SHALL aggregate fleet state (worst quota, orphan
   count, offline machines) using the same indicator grammar;
   colour SHALL never be the sole channel (icon + text).
**UI/UX reasoning**:
Pass2/04 indicator grammar (I-01..I-11) extended with machine
as first chip; NN/g #4 consistency across tray/board/inspector.
Density toggle (Comfortable/Compact) retained. Legend one
hover away; shortcut `⌘1..⌘5` filters by machine index.
**Performance budget**:
Board render 60 fps at 5 machines × 20 Instances; alert
routing <2 s from node event to console; filter switch
<100 ms; tray update <500 ms, no polling under 5 s.
**Quality gates**:
Playwright: filter by machine, click alert → correct remote
inspector tab; node-side permission negative tests pass;
colourblind simulation + text-equivalent audit green.
**Evidence/validation proof**:
Strict: screenshot-identification test — reviewer names
Provider+Account+model+machine+quota-state for 10 mixed
rows in <30 s (pass2/04 F-N4 method) — plus recorded
3-machine dogfood alert walk in `docs/qa/gate-E7.md`.
Security evidence required: forged-client command against
a view-only node is rejected and logged (test output filed).
**Visual testing**:
Screenshots: board in All-machines and single-machine filter,
tray aggregate, alert deep-link landing, at 1440×900 and
1024×768, light/dark, colourblind simulation. Pass: machine
identifiable on every row without colour.
**Files to touch**: [WorkflowRail.tsx](../../ui/src/components/WorkflowRail.tsx), [App.tsx](../../ui/src/App.tsx), [lib.rs](../../crates/juggler-link/src/lib.rs), [SPEC.md S11.4](../SPEC.md#L385)
**Research basis**:
Pass2/04 §§3-5 (indicators I-01..I-11, inspector); SPEC.md:350,
381,389; expected-worfklow.md:13; D-12 (expiry unknown, never guessed).
**Handover note**:
Trap: UI-hidden buttons are not permission enforcement —
reviewers must check the node-side rejection test, not the
disabled state. Quota `Unknown` for remote peers without a
feed is correct behaviour, not a bug to "fix" with estimates.
**Task-text notes (2026-10-18, AJ-E709)**: (1) There is no menu-bar tray in this build, so Req.6 waits for one. (2) `⌘1..⌘5` is not bound for machines — `⌘1–⌘6` already switch inspector tabs (D-06); the board's Machine filter is the control. (3) Overview and Usage stay this-Mac-only until usage data travels the link (AJ-E708). Evidence: [AJ-E709 manifest](../qa/evidence/AJ-E709/manifest.md).

### AJ-E710 Fleet beta gate, dogfood, and security evidence pack | Status: TODO | Priority P0 | Effort M | Depends: AJ-E702, AJ-E703, AJ-E704, AJ-E705, AJ-E706, AJ-E707, AJ-E708, AJ-E709
**Goal**:
Fleet exits internal alpha into launch beta only with a
filed, reproducible evidence pack: pairing, switching,
cleanup, offline, and security negatives all proven on real
machines — or honestly labelled unvalidated.
**User story**:
As a maintainer cutting the launch beta, I open one gate
file and see exactly what fleet proved, on which machines,
and what still carries an `Unvalidated` badge.
**Requirements**:
1. Gate SHALL require: pair <60 s on 2 physical Macs;
   one board across 3 machines (or filed waiver naming why
   2 sufficed); cross-machine A→B→A switch with continuity
   checklist; remote orphan cleanup with zero collateral.
2. Security checklist SHALL be filed per task: identity
   tamper and registry rollback rejected, anonymous dropped,
   pairing brute-force / parallel-session / restart lockout
   held, TLS 1.2 and wrong-MachineId rejected, replay
   rejected, view-only and free-form-launch forged commands
   rejected, pre-auth flood bounded, credential-shaped
   payload absent from telemetry, no secrets in logs,
   pre-click pack bytes = 0 — one negative test per STRIDE
   row of the ADR-0013 architecture contract.
3. Any Provider pair or path not live-dogfooded SHALL ship
   labelled `Unvalidated` in the wizard (D-04); the gate
   file SHALL list every unvalidated combination by name.
4. Performance evidence SHALL report measured numbers vs
   budgets in AJ-E703/E705/E708 (heartbeat RTT, inspector
   render, catch-up time) on the gate hardware, not estimates.
5. Mixed-version evidence SHALL cover current↔previous
   minor: telemetry works, new commands degrade with the
   labelled banner, nothing crashes (SPEC.md:393).
6. Rollback SHALL be documented: disabling Link returns the
   app to fully local behaviour with zero fleet residue
   blocking local switch/clean (tested, not asserted).
7. Gate SHALL NOT pass with any P0 security negative test
   red; beta labelling does not waive security negatives.
**UI/UX reasoning**:
NN/g #1 applied to release honesty: beta badge on Fleet
nav (`Fleet · Beta`), unvalidated badges at point of action
(wizard), never only in release notes. D-09 glossary rule:
first use of "zombie" in fleet cleanup copy gets the
plain-language explanation.
**Performance budget**:
Gate script (automated portion) <15 min in CI; human
dogfood script <90 min for 2 testers; evidence pack
<20 MB stored in-repo (logs/screenshots, no pcaps with
payloads — headers/summaries only).
**Quality gates**:
`just verify` green; `cargo test -p juggler-link
-p juggler-supervisor -p juggler-core` green; guardrails
suite extended to link (no credential egress) green.
**Evidence/validation proof**:
Strict: `docs/qa/gate-E7.md` contains per-requirement
pass/fail with linked CI run, dogfood recordings, timing
table, unvalidated-combination list, and two tester
sign-offs. Pass threshold: items 1, 2, 6, 7 fully green;
item 3 list complete (gaps labelled, not hidden).
**Visual testing**:
Final screenshot set re-captured on the gate build:
Fleet board (3 machines), pairing, remote inspector,
cross-machine wizard, cleanup, offline/conflict — the
AJ-E704..E709 viewports repeated; a11y sweep (keyboard-only
pair + switch + clean) recorded. Pass: no state reachable
only by mouse, no colour-only meaning.
**Files to touch**: [gate-E7.md](../qa/gate-E7.md), [EPICS.md](../plan/EPICS.md#L126), [App.tsx](../../ui/src/App.tsx#L340), [SPEC.md S13](../SPEC.md#L434)
**Research basis**:
D-04/D-13; SPEC.md:440-442 (fleet + guardrail tests);
pass2/08 §4 gates (switch-budget + cleanup-collateral
evidence); EPICS Gate V6/V7 definitions folded into launch.
**Handover note**:
Trap: D-15 — this gate authorises beta labelling, not a
stable claim. If a security negative cannot be run, the
gate fails closed and fleet ships disabled-by-default with
Fleet nav showing `Not available in this build — reason`,
never a silent Coming Soon.

### AJ-E711 Remote Focus Window — go to an instance's window on another machine | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E705, AJ-E414
**Goal**: Extend AJ-E414 so "Focus window" and the "Go to instance…" hotkey/palette work for desktop instances on paired fleet machines, executed on that machine with its own permissions.
**User story**: As a user at my laptop, I want to jump to a desktop agent's window on my studio machine and have that machine bring the exact window forward, with the outcome reported back to me.
**Requirements**:
1. A remote focus request SHALL be sent over the E7 transport (AJ-E703) to the owning machine, which SHALL execute the identical AJ-E414 tiered focus locally (activate, un-minimise, AXRaise) using that machine's Accessibility permission state; the requesting machine SHALL never inject input into the remote desktop directly.
2. The request SHALL require the AJ-E709 permission tier `Act remotely: Ask each time` (= Control with Ask each time, AJ-E701 contract) or stronger on the target machine; `View only` machines SHALL reject focus with a named reason returned to the requester.
3. Outcomes SHALL round-trip as the same honest classifications as local focus (`Focused window`, `App opened — window not found`, `App not running`, `Permission needed on <machine>`) and render as text in the remote Inspector (AJ-E705) and the palette result.
4. Each remote focus SHALL be audit-logged on both machines (requester, target instance, outcome, timestamp); no focus SHALL execute while the target machine's user has an active "do not disturb / locked" state, which SHALL be reported instead.
5. Where AJ-E415 preview exists for the remote instance, the palette SHALL offer `Preview` and `Focus on <machine>` as distinct actions; preview SHALL NOT be used to fake focus success.
**UI/UX reasoning**: NN-g #1/#3 across machines: same vocabulary, same outcome text, explicit consent tier per machine (Apple HIG permission-at-moment-of-need on the machine that owns the window). Remote action copy names the machine before anything executes.
**Performance budget**: Focus round-trip <1.5 s on LAN including remote AX execution (<500 ms local budget from AJ-E414); palette lists remote instances <200 ms from cached fleet state with staleness shown.
**Quality gates**: Two-machine fixture (or loopback two-node harness): correct window focused on target, view-only node rejects, locked target reports instead of executing; audit entries exist on both sides; `just verify` green.
**Evidence/validation proof**: Store two-node run log + target-machine screen recording + audit excerpts in `docs/qa/evidence/AJ-E711/`; pass = correct window frontmost on the target in all permitted cases, zero execution in rejected/locked cases.
**Visual testing**: Capture remote Inspector focus outcomes (all four classifications), palette with mixed local/remote instances, and the view-only rejection at 1280x800 both themes; pass = machine and outcome unambiguous as text (AJ-E803/E804).
**Files to touch**: `crates/juggler-app/src/focus.rs` (AJ-E414 service, remote dispatch), E7 transport/proto (AJ-E703 definitions), `ui/src/screens/WorkPanels.tsx`, `ui/src/App.tsx` (palette/hotkey), AJ-E709 permission model.
**Research basis**: AJ-E414 tiered AX focus; AJ-E705 remote inspector; AJ-E707 remote cleanup pattern (execute locally, log both sides); D-13 fleet at launch; wireframe screen 24.
**Handover note**: Depends on AJ-E414 landing first — do not build a second, fleet-only focus path. If the target machine lacks Accessibility permission, the answer is the permission outcome text, never a fallback that activates the whole desktop or raises an arbitrary window.

## Epic exit criteria (all must hold)

- AJ-E701 ADR accepted; AJ-E702–AJ-E710 evidence filed in
  `docs/qa/gate-E7.md` with no P0 security negative red.
- Local behaviour is byte-for-byte unaffected when Link is
  off (regression: local switch/inspector/clean suites green).
- Every fleet surface shows beta + validation state honestly;
  no fleet claim uses the word "lossless" (D-11).

## Sequencing and dependency map

```text
AJ-E701 (ADR/threat model)
  └─ AJ-E702 (identity/pairing store)
       └─ AJ-E703 (transport/mTLS/versioning)
            ├─ AJ-E704 (discovery + pairing wizard)
            ├─ AJ-E705 (remote inspector)
            │    ├─ AJ-E706 (cross-machine switching)
            │    └─ AJ-E707 (remote ownership/cleanup)
            ├─ AJ-E708 (offline/conflict)
            └─ AJ-E709 (indicators/permissions/alerts)
                 └─ AJ-E710 (beta gate + evidence pack)
```

| Task | Priority | Effort | Security evidence gate |
|---|---|---|---|
| AJ-E701 ADR + threat model | P0 | M | Threat table signed off |
| AJ-E702 Identity + trust store | P0 | M | Tamper/unpair negatives |
| AJ-E703 Transport mTLS/protobuf | P0 | L | Wrong-cert + replay negatives |
| AJ-E704 Discovery + pairing | P0 | M | Anonymous/mismatch negatives |
| AJ-E705 Remote inspector | P0 | L | View-only node rejection ref |
| AJ-E706 Cross-machine switch | P0 | L | Zero pre-click pack bytes |
| AJ-E707 Remote cleanup | P0 | L | No-kill on unverifiable proc |
| AJ-E708 Offline/conflict | P0 | L | Rogue-peer zero-delta test |
| AJ-E709 Indicators/permissions | P1 | M | Forged-client rejection |
| AJ-E710 Beta gate/evidence | P0 | M | All P0 negatives green |

## Out of scope for this epic (do not build here)

- Any cloud relay or internet pairing service (rejected in
  ADR-0004; D-13 does not reopen it). The optional Figurehead
  account (D-18, ADR-0014) may act only as a discovery
  directory for the user's own Macs — built in AJ-E1206, never
  a relay and never a silent pin.
- Windows/Linux fleet packaging (CI core builds only; SPEC
  platform sequencing is unchanged).
- Auto-switching or auto-cleanup without the D-08 per-Account
  opt-in and an explicit user click (automation invariant).
- Merging diverged lineage automatically (D-05; AJ-E708).
