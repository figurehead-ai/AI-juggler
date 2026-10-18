# Evidence manifest — AJ-E706

Task: `docs/tasks/E7-fleet-at-launch.md` AJ-E706 (cross-machine
switching implications). 2026-10-18, reviewer claude-opus. Contract:
ADR-0013 (amended here: the handover is pushed by the machine holding
the work after its own user's click, never pulled), D-03/D-04/D-05/D-14.

Naming note: in this codebase T1 = same-account native resume, T2 =
transplant (same Provider, another Account), T3 = replay, T4 = pack. The
task's "T2 … same harness binary version and same repo checkout path"
is the SPEC's rule for transplants (SPEC.md §11), so T2 here means
transplant.

## How it works

The switch wizard lists paired machines next to local Accounts. A
destination is assessed only from what that machine publishes over the
link every ~30 s: its Launch settings, its installed harness versions
(probed at most once a minute) and its projects. A project root travels
as a digest that matches only the same canonical path, never as the
path. No transcript or pack byte moves to list or assess a destination.

When the user clicks **Switch to <machine>…**, this Mac does four things:
1. Re-checks the preconditions.
2. Builds the bundle: the native session set for T2, or the distilled
   turns for T3, redacted per the destination's D-14 policy or the
   user's per-switch choice.
3. Stages the Switch Graph edge.
4. Pushes the bundle over the new `Handover` RPC: a signed command
   first (with every command check), then 1 MiB chunks, then the
   sha256.

The destination then decides:
- It re-checks that the Launch setting and project exist there, and,
  for T2, that the harness version still matches.
- It maps the transcript's working directory to its project root, but
  only under the user's confirmed mapping.
- It installs the bundle, launches, and records the **same edge** under
  the same ids.

Each machine keeps a mirror of the other's epoch (schema V23 adds
`epochs.machine_id` and `switch_edges.to_machine`), so lineage reads
whole from either one. Switch-back goes the other way: the away
machine pushes exactly one "While you were away" turn, and the
original's machine resumes its original native session.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Cross-machine targets in the switch wizard with machine label, fidelity badge (validated/unvalidated, D-04) and pack treatment | Req.1 | Inspector → "Continue on another machine": rows grouped "On <machine>" with `F3 · Unvalidated` (every machine pair stays Unvalidated until a live two-machine dogfood passes) and readiness in words. The wizard's "What's carried" lists carries / rebuilt there / **will drop** (D-03 dispositions); T3 adds the D-14 secret choice. vitest `cross-machine-switch.test.tsx`, gallery `50-cross-switch/targets` | PASS | 2026-10-18 | claude-opus |
| T2 needs the same harness version and the same checkout path; unmet preconditions listed before execution with a T3 fallback offered — never a silent downgrade | Req.2 | `fleet_switch :: a_version_mismatch_blocks_t2_offers_t3_and_never_downgrades_silently` — readiness `t2_blocked`, blocker "claude-code 2.1.0 here, 2.0.9 on Studio Mac — T2 needs the same version.", T3 offered; asking for T2 anyway is **refused** (not replayed). `the_destination_rechecks_at_execution_and_the_edge_records_the_failure` — the destination's version changed after the console looked → `version_mismatch`, the edge is recorded Failed, the source is not handed over | PASS | 2026-10-18 | claude-opus |
| The handover is fetched only after the click; browsing transfers no transcript or pack content | Req.3 | No pull exists. Only the holder pushes, from the click handler. `t2_at_the_same_path…`: before the click there is no `handover*` event on either machine's hash-chained link log; targets come from the published catalog alone. `proto_contract` now also forbids any field named `path`/`cwd`/`dir`. `transport`: view-only → `permission_denied`, nothing taken | PASS | 2026-10-18 | claude-opus |
| D-05 across machines: switch-back resumes the original native session + one delta turn; copy-back prohibited; divergence surfaced, never overwritten | Req.4 | `switching_back_resumes_the_original_with_one_delta_turn_and_never_copies_back`: `--resume native-5` plus the single "While you were away" turn (only the 2 turns added while away, never the shared ones); the original transcript's sha256 is unchanged. `a_diverged_original_blocks_switch_back_until_the_user_chooses` / `keep_mine_…`: refusal with exact counts; Keep both forks (the original is untouched), Keep mine resumes the original, Keep theirs sends nothing. A running original is refused (`busy`) | PASS | 2026-10-18 | claude-opus |
| Absolute paths remapped only via an explicit user-confirmed mapping; unmapped paths block T2 with an explanation | Req.5 | `another_checkout_path_needs_a_confirmed_mapping_and_the_session_is_remapped`: unmapped → refused; mapped but unconfirmed → refused; confirmed → every `cwd` in the installed transcript points at the destination's project (a sibling folder sharing the prefix is never moved — `remap_cwd` unit test). The wizard lists the same-name copy first as a suggestion and never pre-picks it | PASS | 2026-10-18 | claude-opus |
| Every cross-machine switch writes lineage with source/target `MachineId`, resumable from either machine | Req.6 | Both machines hold the same confirmed edge (same id); each has a mirror epoch with the other's `MachineId`; the source's edge carries `to_machine`. Switch-back is planned from the away machine's graph alone | PASS | 2026-10-18 | claude-opus |
| Target list with preconditions < 1 s for 5 machines; precondition check < 500 ms (cached probe, 60 s TTL) | perf | **1.7 ms** (in-memory: published catalogs + the version probe cached for 60 s) | PASS | 2026-10-18 | claude-opus |
| Progress for > 10 MB packs; cancel < 1 s | perf | `transport :: a_handover_arrives_whole_and_verified_with_progress` (5.5 MB in 55 ms loopback, `receiving` and `installing` reported); `a_cancelled_handover_is_discarded_within_a_second` (64 MB push cancelled < 1 s; the node discards it and logs `handover_cancelled`). The wizard shows a progress bar (bytes of total) with Cancel | PASS | 2026-10-18 | claude-opus |
| A received bundle is untrusted: complete, verified, size-capped, one at a time | security | `transport`: wrong length / extra bytes / over the 512 MB cap → `corrupt` / `malformed` / `too_large`, nothing taken; one in-flight handover per console; Command-stream handovers refused; T3 turns are handed to the destination as data (prime prompt says so) | PASS | 2026-10-18 | claude-opus |
| Fixture matrix per Provider pair extended with the machine dimension; guardrail: no pack bytes before the click; path-mapping unit tests | quality gate | Machine dimension: every cross-machine cell is Unvalidated by construction (no live cell yet). Guardrail and mapping tests above. The live per-pair cells are the AJ-E710 dogfood | PASS (live cells → AJ-E710) | 2026-10-18 | claude-opus |
| Visual: target list (T2-ready, T2-blocked, T3-only), path-mapping step, divergence on return — 1280×800, light/dark; badge has a text equivalent; keyboard-operable | visual / a11y | Gallery `50-cross-switch` (targets, wizard-t2-ready, wizard-t2-blocked, path-mapping, carried, confirm, switch-back-preview, conflict-sheet, conflict-keep-both, resolved). Playwright `fleet — cross-machine switch and conflict sheet — axe 0` (both themes): completed keyboard-only (Enter / Space / focus); blocked reasons are words | PASS | 2026-10-18 | claude-opus |
| Found while capturing: the inspector squashed its overview and terminal to zero height under a long handover list, and its footer drew over content | UX fix | The instance panel now grows with its content (the tab scrolls); the terminal keeps ≥ 280 px; the footer is sticky with a solid background. Faint text on the rail's age line, "Quota: no data" and the footer failed AA → `text-muted` | PASS | 2026-10-18 | claude-opus |
| Live dogfood A (laptop) → B (desktop) → A with continuity checklist and delta-turn screenshot; version-mismatch run; network capture summary showing zero transcript bytes while browsing | evidence | AJ-E710 gate items (two physical Macs) | DEFERRED-TO-AJ-E710 | 2026-10-18 | claude-opus |

## Test counts

- `cargo test -p juggler-app --test fleet_switch`: 8/8 (two app instances over loopback with a stub `claude`).
- `cargo test -p juggler-link --test transport`: 18/18 (5 new handover tests).
- `cargo test -p juggler-adapter remap_cwd`: 1/1; `juggler-app` lib `remote::` 8/8 (incl. origin spoof).
- vitest `cross-machine-switch.test.tsx` 6/6.
- Playwright `50 cross switch` 4, `fleet — cross-machine switch and conflict sheet — axe 0` 2.
