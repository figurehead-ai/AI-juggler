# Evidence manifest — AJ-E708

Task: `docs/tasks/E7-fleet-at-launch.md` AJ-E708 (offline and conflict
behaviour). 2026-10-18, reviewer claude-opus. Contract: ADR-0013.

Part 1 (#23) landed the sync substrate: the durable outbox on every
node, the durable replica on every console, sequence-vector catch-up,
epochs, idempotent apply, clock-skew handling and the sync chip.

Conflicts (Req.4–5) need a record that two Macs can change while apart.
Every record synced so far has exactly one writer: Instances,
resources, alerts and processes belong to the Mac that owns them, and
labels and permissions stay local to each Mac. The first record two
Macs can change is cross-machine lineage. Part 2 therefore ships with
AJ-E706: the `Diverged` detection, the conflict sheet and the
destructive-action block.

## Design in one paragraph

Each node appends every durable change to an Instance row (status,
model, project, timestamps — never live numbers) to `fleet_outbox`, a
SQLite table (WAL, one transaction per change, AUTOINCREMENT so a
sequence is never reused). The table keeps only the latest change per
record, so it stays bounded. Live numbers (CPU, memory, processes),
resources and alerts travel unsequenced (`sequence = 0`) and are never
stored. Each console keeps a per-origin replica (`fleet_replica`) and a
high-water mark under that origin's **sync epoch**, a random id the
node announces in its Hello. On reconnect the console asks for
`since = high-water`, and the node sends only the outbox entries after
it. A new epoch means the machine was reset: the console drops that
origin's replica and starts again from 0. Applying is "newest sequence
wins" per record, so retries, duplicates and reordering change nothing.
The node subscribes to live updates *before* it reads the backlog, so
no change can fall between the two. A console that falls behind ends
its stream instead of silently skipping changes, then resumes from its
mark.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Offline degrades, never blocks; fleet surfaces show `Offline — last seen <time>` | Req.1 | Nothing local depends on a peer (all local paths are unchanged; the link is off by default). Fleet shows the offline banner with last seen; an away machine's rows stay as **last known**, labelled `Last known · as of …` and `was running` on Fleet and on the board (`○ offline · last known`). Live numbers are never shown as if current (asserted: `cpu_percent` is `None` while offline) | PASS | 2026-10-18 | claude-opus |
| Records stamped with origin `MachineId`, per-node monotonic sequence, wall clock; skew > 5 min flagged, never trusted for order | Req.2 | Every outbox row has origin (the node), an AUTOINCREMENT `seq` (monotonic, never reused, asserted after compaction) and `wall_ms`. The console measures skew from the node's Hello (`clock_skew_ms`) and flags it above 5 min ("… order always follows its own change sequence, never its clock"). Property test `a_skewed_clock_never_reorders`: ±10 min wall-clock jitter, delivered in wall-clock order → the replica still ends at each record's highest sequence (64 cases) | PASS | 2026-10-18 | claude-opus |
| Reconnect syncs by sequence-vector exchange, only missing deltas; full transfer only on first pair | Req.3 | `fleet_sync :: an_offline_console_catches_up_with_only_what_it_missed_and_keeps_last_known_rows`: first pair asks from 0 (the one full transfer); after **200+ edits** while the console was offline, it asks from its stored mark and receives exactly the 5 changed records (the node's outbox log records `since` and `count`, asserted) — never a full transfer | PASS | 2026-10-18 | claude-opus |
| Conflicts resolve per field class: telemetry newest-sequence wins | Req.4 (telemetry) | Single writer per record (the owning Mac), so a telemetry "conflict" is two versions of the same writer: newest sequence wins (property tests) | PASS | 2026-10-18 | claude-opus |
| Labels/settings last-writer wins with loser in History | Req.4 (labels) | No label or setting is synced in v1 (each Mac names itself; permissions are set only on the node — ADR-0013 amended) — nothing to merge | N/A-BY-DESIGN | 2026-10-18 | claude-opus |
| Lineage/session records never auto-merged — shown as `Diverged` for the user's choice | Req.4 (lineage) | A cross-machine edge records the original transcript's length + sha256 when the work left. Switch-back is refused `diverged` when the original was rewritten or gained turns, with exact counts and the last activity — never content. `fleet_switch :: a_diverged_original_blocks_switch_back_until_the_user_chooses` (1 laptop turn → `turns_since = 1`, nothing ran there). Zero silent overwrites: the original's sha256 is asserted unchanged through every path | PASS | 2026-10-18 | claude-opus |
| A diverged record blocks destructive actions (switch-back, clean) until Keep mine / Keep theirs / Keep both, each previewed | Req.5 | The divergence is recorded until the user picks. While it is pending, switch-back is refused by the original's machine, and **Clean** of the Instance's processes is refused with zero signals (`KILL_DIVERGED_COPY`). The conflict sheet describes each choice: Keep mine resumes the original after its newer turns; Keep theirs sends nothing (audited `divergence_kept_theirs`); Keep both forks the original and leaves its own continuation untouched. `fleet_switch` asserts all three plus the record lifecycle | PASS | 2026-10-18 | claude-opus |
| Idempotent: replaying the same batch changes nothing | Req.6 | Property test `shuffled_duplicated_delivery_converges` (random order + up to 200 duplicates, 64 cases; replaying the whole batch returns "unchanged" for every delta); the partition test replays the node's whole outbox into the console → zero changes | PASS | 2026-10-18 | claude-opus |
| Queued outbound deltas survive a crash and reboot (durable outbox in SQLite WAL) | Req.7 | The outbox and replica are SQLite tables (WAL); the console's replica and mark survive a restart (`a_console_restarted_mid_sync_resumes_from_its_durable_high_water_mark`: the dropped and reopened console shows its last-known rows before reconnecting, then resumes from its stored mark and receives exactly the one change it missed). Property test `a_kill_after_any_prefix_resumes_and_loses_nothing` (file-backed DB, cut at every possible point): the catch-up is exactly what the kill cut off, zero records lost | PASS | 2026-10-18 | claude-opus |
| A reset machine is re-synced afresh, never shadowed by old sequences | Req.3 / 7 | `a_reset_node_gets_a_new_sync_epoch_and_the_console_starts_it_afresh`: new epoch → the console drops the stale replica (a ghost record disappears) and asks from 0 | PASS | 2026-10-18 | claude-opus |
| Persistent sync chip `Synced` / `Offline — N changes queued` / `Diverged — 1 needs you` | UI | Fleet → each paired machine: `✓ Synced` while connected; while away, how many changes on THIS Mac wait for it (`⇡ 12 changes queued`, from the node's per-console delivery mark, saved every 2 s); `⚠ Diverged — N needs you` outranks both while a divergence waits. Words + a shape. vitest `fleet-sync.test.tsx`; partition test asserts `queued_changes` = 5 while away and 0 after catch-up | PASS | 2026-10-18 | claude-opus |
| Outbox durability write < 10 ms | perf | Asserted per write for 1,000 writes in `a_thousand_queued_changes_catch_up_inside_three_seconds` (debug build) | PASS | 2026-10-18 | claude-opus |
| Reconnect catch-up for 1,000 deltas < 3 s LAN | perf | **90 ms** loopback, including the reconnect and TLS handshake (debug build, M-series) — budget asserted in the test | PASS | 2026-10-18 | claude-opus |
| Idle sync CPU < 0.5% | perf | A sync pass compares each row with an in-memory copy of the last synced bytes; unchanged rows never touch SQLite. The 2 s tick reads the same snapshot the board already used. Measured on gate hardware at AJ-E710 | PARTIAL-BY-DESIGN | 2026-10-18 | claude-opus |
| Queued storage < 10 MB per peer per 24 h offline | perf | The outbox keeps one row per record (latest state), so its size is bounded by the number of Instances (≈ 200 B each), not by time offline or by edit count — 200+ edits left 5 rows in the partition test | PASS | 2026-10-18 | claude-opus |
| Conflict detection < 100 ms per reconnect batch | perf | One sha256 over the original's bytes at handover plus a uuid scan of the appended tail. The whole refusal round trip (push, detect, report) measured **5–18 ms** over loopback | PASS | 2026-10-18 | claude-opus |
| Security: deltas are mTLS-only on reconnect; a rogue peer with valid discovery but no pin receives zero deltas | security | Catch-up rides the same pinned mTLS stream (AJ-E703). `juggler-link --test transport :: a_rogue_peer_skipping_hello_receives_zero_deltas`: an unpinned machine opening TLS and calling Telemetry directly, without a Hello, from sequence 0 → `PermissionDenied`, zero deltas | PASS | 2026-10-18 | claude-opus |
| A port change can't strand peers paired by address | robustness | Found while testing: a node that restarted on another port was unreachable for peers paired by address over a VPN, with no mDNS. The node now tries its last-used port first, then the preferred one, then any free port (`fleet_link_last_port`) | PASS | 2026-10-18 | claude-opus |
| Visual: offline board, queued-changes chip, conflict sheet (3 choices), resolved state — 1280×800, light/dark | visual | Gallery `49-fleet-sync` (synced, offline-queued, clock-skew, offline-board) and `50-cross-switch` (conflict-sheet, conflict-keep-both, resolved) | PASS | 2026-10-18 | claude-opus |
| A11y: state in text, not colour; conflict choices a radio group with descriptions, resolvable keyboard-only; axe | a11y | `fleet — sync chip, skew, last known — axe 0` and `fleet — cross-machine switch and conflict sheet — axe 0` (both themes; "Review later" focused by default, a choice made with Space) | PASS | 2026-10-18 | claude-opus |
| Spoofed delta origin rejected | security | A delta is applied only when its origin is the authenticated machine on that connection (v1 never relays) — `remote::tests::a_delta_naming_another_machine_is_never_applied` (unit) | PASS | 2026-10-18 | claude-opus |
| Dogfood: laptop offline 1 day, reconnect; outbox log + conflict sheet screenshot | evidence | AJ-E710 gate item (outbox log = the node's `sync_catch_up` entries in `fleet/link-audit.jsonl`) | DEFERRED-TO-AJ-E710 | 2026-10-18 | claude-opus |

## Test counts

- `cargo test -p juggler-app --test fleet_sync`: 4/4 (partition, crash/restart resume, epoch reset, 1,000-delta catch-up); `--test fleet_link` 9/9.
- `cargo test -p juggler-core --test fleet_sync_props`: 3 properties × 64 cases; `storage::fleet_sync` unit tests 3/3.
- `cargo test -p juggler-link --test transport`: 13/13 (incl. the rogue-peer negative); `proto_contract` 3/3 (snapshot blessed: additive `HelloResponse.sync_epoch = 7`).
- vitest `fleet-sync.test.tsx` 4/4 (+ the Diverged state via `cross-machine-switch.test.tsx`).
- Part 2 (with AJ-E706): `juggler-app --test fleet_switch` 8/8 (divergence, Keep mine / theirs / both, busy original, record lifecycle).
- Playwright `49 fleet sync` 4, `fleet — sync chip, skew, last known — axe 0` 2.
