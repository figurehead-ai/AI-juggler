# E2 — Switching Engine (Switch Graph / Pack / Fidelity)

> Epic owner: Switching Engine. Status: TODO (all tasks).
> Settled decisions (do not re-litigate): D-03 hybrid door, D-04 beta
> fidelity gate, D-05 resume+delta, D-11 labelled beta, D-14 pack
> redaction user choice. Source: `docs/review/decision-log.md:31-56,105-150`,
> `docs/review/pass2/02-unlimited-bidirectional-switching-engine.md` (all),
> `docs/review/deep/09-handover-and-quota-engine-design.md` (Part H),
> `docs/expected-worfklow.md:5-7`, `docs/SPEC.md:261-306`,
> `docs/plan/EPICS.md:99-109` (M4-1..M4-7).
> Reading order for implementers: decision-log D-03/D-04/D-05/D-11/D-14
> first, then pass2/02 §2-§10, then deep/09 §H1-H9. Code truth wins
> over prose where they disagree; flag drift in the PR, do not
> silently follow stale prose.
> Conventions: Provider = `HarnessId`, Account = `Account`,
> Instance = `SessionRecord` + `InstanceKind`, Session = conversation
> inside an Instance, Profile = launch-settings wording only
> (D-02/D-10). "Switch" = moving Work; "handover" = one hop action.
> Every task inherits `docs/plan/QUALITY.md` §2 DoD and §5 coverage
> (≥85% line on core/adapter/adapters/handover). No task ships
> marketing copy above its proven fidelity ceiling.

## Build order (dependency spine)

```
E201 Graph types+storage → E202 Pack schema → E203 Fidelity matrix
  → E204 HandoverSupport transplant → E205 Freeze/stage/verify pipeline
  → E206 Hybrid door + wizard → E207 Switch-back + divergence
  → E208 Redaction policy (D-14) → E209 Badges/ranking/graph UI
  → E210 Validation matrix + beta-exit gate
```

E208 may start after E202; E209 after E203/E206/E207. E210 fixtures
start in parallel with E204 but its gate closes last.

---

### AJ-E201 Switch Graph domain types and append-only storage | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: none
**Goal**: Replace the linear `prev_handover` chain with Work / Epoch /
SwitchEdge so A→B→A→C→B, forks, and merges are representable and
restart-identical.
**User story**: As a developer juggling three subscriptions on one
project, I want my switch history preserved as a graph so that
returning to an earlier Account never pretends earlier hops vanished.
**Requirements**:
1. The system SHALL define `WorkItem{id,project_id,title,created_at}`,
   `Epoch{id,work_id,session_ref,native_id,account_id,provider,
   resumed_from_epoch: Option<EpochId>}`, and `SwitchEdge{id,work_id,
   from_epoch,to_epoch: Option<EpochId>,from/to_account/provider,
   mechanism,pack_id,fidelity_shown,manifest_sha256,state,actor,
   parent_edge_a,parent_edge_b: Option<EdgeId>,created_at}` in
   `juggler-core`; `to_epoch` SHALL be null until launch succeeds.
2. The system SHALL append migration V5+ after existing V1-V4
   (`storage/mod.rs:82` `MIGRATIONS`) creating `work_items`, `epochs`,
   `switch_edges`, `artifact_manifests` per pass2/02 §2.3; existing
   rows SHALL migrate with zero data loss and WAL mode retained.
3. Edges SHALL be append-only (Invariant S-G1): undo, failure, and
   merge SHALL insert compensating edges/rows and SHALL NOT update
   or delete prior edges; enforced by repository API (no `UPDATE`
   on `switch_edges` except `state` transition within one edge).
4. The graph SHALL be per-project (Invariant S-G2); cross-project
   moves SHALL be recorded as T4 exports, never as edges, because
   `cwd`/native paths are baked into every format.
5. Resuming a native session after switch-back SHALL create a new
   Epoch whose `resumed_from_epoch` points at the original Epoch
   and whose `native_id` equals the original `native_id`.
6. `HandedOver` (`model.rs:188` SessionStatus) SHALL be treated as
   resting, not terminal: every Epoch node SHALL expose Resume.
**UI/UX reasoning**: No user-facing surface in this task; API shape
follows NN-g "visibility of system status" — edge `state`
(`Staged|Verified|Launched|Confirmed|Failed{stage,reason}`) must be
exhaustive so E209 can render it without inventing states. Internal
naming uses Provider/Account/Instance per decision-log D-02.
**Performance budget**: Graph read for 1,000 edges P95 ≤ 50 ms on
reference Mac; migration V5 on 10k sessions ≤ 2 s; storage overhead
≤ 1 KB/edge median; no N+1 queries (single JOIN read).
**Quality gates**: `just verify` zero warnings; property test:
random edge sequences round-trip through SQLite identically after
reopen; migration up from empty and from V4 fixture DB both pass.
**Evidence/validation proof**: `cargo test -p juggler-core switch_graph`
100% pass; fixture `a_b_a_c_b.rs` builds 4 epochs + 3 edges, reopens
DB, asserts byte-identical graph JSON; stored at
`crates/juggler-core/tests/switch_graph.rs` + gate note in PR.
Threshold: any history rewrite or lost edge = fail.
**Visual testing**: None (domain layer). Defer screenshots to
AJ-E209; this task's PR attaches the graph JSON fixture diff only.
**Files to touch**: `crates/juggler-core/src/model.rs`,
`crates/juggler-core/src/storage/mod.rs`,
`crates/juggler-core/src/storage/sessions.rs`,
`crates/juggler-core/src/lib.rs`,
`crates/juggler-core/tests/switch_graph.rs` (new).
**Research basis**: pass2/02 §2.1-2.3 (lines 86-148);
`crates/juggler-core/src/model.rs:367-416` (ProjectRecord anchor);
deep/09 §H7 (lines 237-269) chain being generalised.
**Handover note**: Do not renumber Prior-09 V5/V6 — critic C3 assigns
numbering to deep/09; sequence after them. Trap: `sessions.project`
already exists (`model.rs:401-403`) — use it as the Work anchor key,
do not add a parallel project id. Next agent: E202 consumes `pack_id`.

### AJ-E202 Portable context pack v1 — schema, deterministic generation, manifest | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E201
**Goal**: Fix one canonical pack (`format_version: 1`) that serves as
T4 export, staging artifact for every edge, and merge substrate, so
two implementations cannot diverge.
**User story**: As a developer about to switch Providers, I want a
single inspectable pack showing exactly what will travel, so that
I can trust the switch before it runs.
**Requirements**:
1. The pack SHALL contain exactly `PACK.md`, `conversation.md`,
   `state.json`, `manifest.json`, `diffs/` (patches or EMPTY marker),
   and `rules/` (canonical AGENTS.md + per-Provider renderings
   actually written) per pass2/02 §4; readers SHALL ignore unknown
   fields (forward compatibility).
2. `state.json` SHALL require `format_version, pack_id, created_at`
   (epoch-ms), `work_id, source{provider,account_label,native_session_id,
   harness_version,epoch_id}, project{root,project_id}, git{branch,
   head_sha,dirty,diff_stat}, conversation{turn_count,
   message_uuids_sha256,compacted,first/last_turn_at,
   replay_tokens_est}, artifacts{rules,mcp,skills,todos,drops[]},
   fidelity{level,score,caps_applied[]}, lineage{parent_pack_ids[],
   parent_edge_ids[]}`; merges SHALL list two parents.
3. Every Drop from the fidelity matrix SHALL appear in
   `artifacts.drops[{artifact,reason}]` and in `manifest.json`
   entries `{rel_path,sha256,bytes,kind,disposition}`; credentials
   SHALL never be written to any pack file (guardrail).
4. Generation SHALL be deterministic from the Transcript IR
   (same IR → byte-identical `state.json`/`manifest.json`); an LLM
   summary, if ever added, SHALL write only `summary_llm.md` and
   SHALL NOT overwrite the deterministic record.
5. Even native-resume/transplant edges SHALL generate a
   manifest-only pack so lineage and fidelity evidence always exist.
6. Pack writes SHALL be atomic (temp dir → sha256 verify → rename);
   a crash SHALL leave either the old pack or none, never partial.
**UI/UX reasoning**: Follows deep/09 §H5 "pack is the preview" and
NN-g recognition-over-recall: `PACK.md` is human-readable task/
constraints/decisions/todos/next-steps (SPEC §7.3), never raw JSON
as the only surface. Copy: pack header "What travels / What stays"
— no tier codes (T1-T4 stay internal per deep/09 D2).
**Performance budget**: Pack generation for 5,000-turn session
≤ 800 ms P95, ≤ 64 MB RSS; `conversation.md` streaming write (no
full-session String in memory above 32 MB); manifest sha256 at
≥ 200 MB/s; pack read/verify ≤ 150 ms for 10 MB pack.
**Quality gates**: Golden tests: fixed IR fixture → committed
`state.json`/`manifest.json` goldens; determinism test runs
generation twice, asserts `cmp` identical; unknown-field tolerance
test passes; `cargo-deny` clean, no new `unsafe`.
**Evidence/validation proof**: `cargo test -p juggler-handover pack_v1`
all pass; determinism proof stored as golden diff in
`crates/juggler-handover/tests/goldens/`; manual check: export a
real Claude session, open `PACK.md` unaided, confirm task/decisions
readable — recorded in `docs/qa/` note. Any nondeterministic byte
or missing Drop row = fail.
**Visual testing**: Screenshot `PACK.md` rendered (light+dark,
1280×800) and pack folder listing; comparison = human review vs
§H5 carry-list (✓/✗/⚠ rows present); a11y: markdown headings in
order, no information by colour alone. Pass = reviewer can state
what dropped without opening JSON.
**Files to touch**: `crates/juggler-handover/src/lib.rs`,
`crates/juggler-handover/src/pack.rs` (new),
`crates/juggler-handover/src/manifest.rs` (new),
`crates/juggler-adapter/src/adapter.rs` (IR types if missing),
`crates/juggler-handover/tests/pack_v1.rs` (new),
`crates/juggler-handover/tests/goldens/` (new).
**Research basis**: pass2/02 §4 (lines 244-282); `docs/SPEC.md:292-306`;
deep/09 §H5 (lines 184-211); `crates/juggler-handover/src/lib.rs:1-13`
(doc-only stub today).
**Handover note**: Trap: `juggler-handover` is currently 13 lines of
docs — there is no pack code to extend. Do not put pack logic in
`commands.rs` (that is the v0 anti-pattern, deep/09 D3). E208 adds
`redactions[]` to this schema; reserve the field now as `[]`.

### AJ-E203 Per-pair fidelity matrix and capped scoring engine | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E202
**Goal**: Compute, per ordered Provider pair and per artifact,
Copy/Translate/Regenerate/Drop plus a capped F0-F4 label, so no
pair ever advertises fidelity it has not proven.
**User story**: As a developer choosing between two Accounts, I want
an honest badge ("Same conversation, new account" vs "Full story,
fresh start") backed by a carry-list, so I never discover loss
after switching.
**Requirements**:
1. The engine SHALL implement the artifact inventory of pass2/02
   §3.1 (chat, summary/compaction, rules, MCP config, skills,
   todos, git/diff, env/model, subagents) with dispositions
   Copy=1.0, Translate=0.8 (0.6 if any key dropped), Regenerate=0.5,
   Drop=0 and weights chat .35, rules .15, git .15, todos .10,
   MCP .10, skills .05, env .05, subagents .05.
2. Scores SHALL be computed per ordered pair (Claude→Codex ≠
   Codex→Claude); direction-blind aggregation SHALL NOT exist.
3. Composite SHALL be capped: any Drop in chat/rules/git caps at
   F1; a `compact_boundary` in source caps at F2; an unvalidated
   pair (E210 cell not green) caps at F1 regardless of mean.
4. Label mapping SHALL be ≥.95→F4/F3 rules, .75-.94→F3/F2,
   .45-.74→F1, <.45→F0 per pass2/02 §3.4 and deep/09 §H6 labels
   ("Continues exactly" only for same-Account native resume);
   the badge SHALL always ship with the carry-list, never alone.
5. "Translate" without a written adapter mapping SHALL score as
   Drop; the engine SHALL list every unmapped MCP/rules key by
   name, never silently omit it.
6. In-session todos SHALL default to Drop until a fixture proves
   carriage (deep/09 §H2); ZCode targets SHALL cap at F1 source /
   F0 target until E210 proves a prime path (SPEC:282).
**UI/UX reasoning**: Nielsen/NN-g "match between system and real
world" + deep/09 D2: users see fidelity labels + carry-list
(✓ carried / ⚠ rebuilt / ✗ stays), never T1-T4 codes. Exact states:
badge + expandable carry-list; empty state "Not yet validated —
treat as Briefing pack (F0)". No green badge below validated F3.
**Performance budget**: Matrix compute for 6 ordered pairs × 9
artifacts ≤ 20 ms P95 (pure function, no I/O); score recompute on
preflight ≤ 5 ms; memory ≤ 1 MB; no network calls.
**Quality gates**: Table-driven unit tests cover every cell in
pass2/02 §3.2-3.3; cap tests assert each cap independently lowers
the label; golden snapshot of the full matrix committed.
**Evidence/validation proof**: `cargo test -p juggler-handover fidelity`
100% pass incl. worked example (proper Claude A→B ≈.93→F3; v0
single-file ≈.86 capped F1); snapshot at
`crates/juggler-handover/tests/fidelity_matrix.rs`. Any cell
scoring Translate without a mapping test = fail.
**Visual testing**: Screenshot badge+carry-list for F4/F3/F2/F1/F0
at 1280×800 and 800×600, light+dark; comparison = pixel-diff vs
approved baseline (≤1% diff); a11y: badge text not colour-only,
WCAG 2.2 AA contrast ≥4.5:1, carry-list navigable by keyboard.
Pass = all five labels distinguishable with colour removed.
**Files to touch**: `crates/juggler-handover/src/fidelity.rs` (new),
`crates/juggler-handover/src/lib.rs`,
`crates/juggler-adapters/src/claude_transcript.rs`,
`crates/juggler-handover/tests/fidelity_matrix.rs` (new).
**Research basis**: pass2/02 §3.1-3.4 (lines 150-242);
deep/09 §H6 (lines 213-235), §H2 (lines 89-110);
`docs/SPEC.md:269-282`.
**Handover note**: The cap, not the mean, is the honesty mechanism
— do not "fix" a capped F1 by raising weights. Trap: SPEC §7.1 is
direction-blind (SPEC:269-274); this engine supersedes it. E210
owns which cells flip from capped to validated.

### AJ-E204 HandoverSupport trait and honest Claude transplant rebuild | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E203
**Goal**: Move transplant out of `commands.rs` into `juggler-handover`
behind an optional `HandoverSupport` sub-trait, copy the full sibling
set, and stop advertising `TRANSPLANT` until fixtures prove it.
**User story**: As a developer with subagent-heavy work, I want the
whole session — main transcript plus sibling files — to move, so
switching Accounts does not silently amputate my history.
**Requirements**:
1. The system SHALL add `crates/juggler-adapter/src/handover.rs`
   with `SessionBundle`, `BundleManifest`, and optional trait
   `HandoverSupport{bundle_session, install_bundle, prime_prompt}`
   verbatim in shape from deep/09 §H1; it SHALL be a sub-trait so
   the semver-protected `HarnessAdapter` (ADR-0002) is unchanged
   and `fake.rs` can opt in incrementally.
2. Claude transplant SHALL resolve the source via
   `SessionRecord.transcript_ref` (`model.rs:315-321,415`), never
   newest-mtime (`commands.rs:1026-1033`) and never computed
   mangling `replace('/','-')` (`commands.rs:1121-1122`); target dir
   SHALL be discovered by scanning target `projects/*` and matching
   embedded `"cwd"` (discover-don't-compute, H-3), fallback flagged.
3. Transplant SHALL copy the full `<session>*` sibling set (main +
   subagent transcripts + spill + file-history inventory) with a
   manifest row per file; single-file copy SHALL NOT be reachable.
4. ID collision on target SHALL fork (new native id + rewritten
   `sessionId` chain, `parentUuid` tree preserved, dedupe first-wins
   by `uuid`); source bytes SHALL remain hash-identical (copy-never-move).
5. `Capabilities::TRANSPLANT` (`claude_code.rs:111-120`) SHALL be
   removed/false until AJ-E210 H-F1..H-F6 pass for that pair;
   interim UI label SHALL be "F1 Transcript-only (experimental)".
6. `session_handover_impl` (`commands.rs:1088-1194`) SHALL shrink
   to parse-args → engine → emit; all tier logic SHALL live in
   `juggler-handover`.
**UI/UX reasoning**: Apple HIG "preserve user work" + deep/09 D1:
while unproven, the button reads "Switch (experimental)…" and the
confirm shows Prior-09 D1 copy verbatim: "Experimental: only the
main transcript moves…" replaced by the true sibling carry-list
once proven. Error state S-F1 copy: "Couldn't tell which file is
this session — pick it." No silent fallback to mtime.
**Performance budget**: Bundle+install for 50 MB session set
≤ 3 s P95 local SSD; sha256 verify included; parse-verify of tail
≤ 200 ms; memory ≤ 128 MB (streamed copy, no whole-file Vec for
files > 8 MB); supervisor poll unchanged (no new polling loop).
**Quality gates**: H-F1 sibling set, H-F2 torn tail, H-F3 collision
fork, H-F6 pathological cwd (dots/dashes/spaces/>200 chars) all
pass; source-hash invariant asserted in every test; clippy pedantic
clean; coverage ≥85% on new handover code.
**Evidence/validation proof**: `cargo test -p juggler-handover -p
juggler-adapters transplant` 100% pass; live proof: two real Claude
Accounts, subagent-heavy repo, resume on B answers a pre-switch-only
question — log + transcript sha256 before/after stored under
`docs/qa/` and linked in PR. `TRANSPLANT` bit grep must show false
until E210 sign-off. Any source mutation = fail.
**Visual testing**: Screenshots: wizard carry-list with siblings
(n=3 files, bytes) at 1280×800; collision-fork notice; experimental
badge state. Comparison = baseline pixel-diff ≤1% + manual copy
check. A11y: file list is a real list (screen-reader order matches
visual), focus lands on confirm, WCAG AA contrast.
**Files to touch**: `crates/juggler-adapter/src/handover.rs` (new),
`crates/juggler-adapter/src/adapter.rs`,
`crates/juggler-adapter/src/lib.rs`,
`crates/juggler-handover/src/transplant.rs` (new),
`crates/juggler-adapters/src/claude_code.rs`,
`crates/juggler-adapters/src/claude_transcript.rs`,
`crates/juggler-adapters/src/fake.rs`,
`crates/juggler-app/src/commands.rs`.
**Research basis**: deep/09 §H1-H4 (lines 50-182), D1/D3/D4
(lines 30-33); pass2/02 §3.2 (lines 175-196);
`crates/juggler-app/src/commands.rs:1026-1146` (v0 flaws).
**Handover note**: Biggest trap in the epic: the naive
`replace('/','-')` collides (`/a/b.c` vs `/a/b-c`). If discovery
finds no target dir, flag the fallback in the manifest — never
hide it. Do not re-add `TRANSPLANT` early "for testing" on main.

### AJ-E205 Switch execution pipeline — freeze, stage, verify, launch, rollback | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E204
**Goal**: One pipeline for every edge — Resolve→Preflight→Freeze→
Snapshot→Stage→Verify→Launch→Confirm→Record — where any failure
leaves the source exactly resumable and writes a failed edge.
**User story**: As a developer hitting the quota wall mid-task, I
want to freeze safely and switch without tearing my transcript,
so a failed switch never costs me the work itself.
**Requirements**:
1. Preflight SHALL compute in order: ordered-pair mechanism +
   E203 score, compaction scan, target quota gate, version-drift
   parse check, git/worktree state, and E206 door (1-click vs
   wizard); any failure SHALL stop before any byte moves.
2. Mid-task freeze SHALL offer exactly two choices — "Finish this
   turn, then switch" (default; wait for turn boundary) or "Stop
   now and switch" (SPEC:225 stop ladder; partial turn marked
   truncated in pack) — and SHALL NOT copy a live appending file
   or use SIGSTOP pausing (forbidden, SPEC:225 corruption risk).
3. Stage SHALL write to a temp dir, verify sha256 against the
   manifest, then atomic-rename; crash mid-copy SHALL leave no
   partial target (S-F4/F-H-10) and source untouched.
4. Launch failure SHALL keep staged artifacts 24 h for retry and
   mark the edge `Failed{launch,reason}`; replay-prime quota death
   (S-F6) SHALL abort, record tokens burned, and resume source.
5. Every failure S-F1..S-F9 (pass2/02 §9) SHALL produce its specified
   engine behaviour, user copy from §7.1 verbatim, and a failed
   edge; silent partial state SHALL be impossible by construction.
6. Zombie children of a frozen Instance SHALL be listed at freeze;
   cleanup SHALL be a separate confirmed action, never a silent
   side-effect of switching (`expected-worfklow.md:17-18`).
7. Same-branch + both-live SHALL warn and offer a worktree for the
   target; uncommitted diff SHALL be snapshotted into `diffs/` at
   freeze but SHALL NOT be auto-applied to another worktree.
**UI/UX reasoning**: Nielsen error-prevention + WCAG status
messages: freeze dialog is modal with two labelled radio choices,
default pre-selected, `Esc` = cancel (source keeps running),
`⏎` = confirm freeze choice. Verbatim failure copy (pass2/02
§7.1): "Switch didn't complete — {reason}. Nothing changed on
{Account A}; you can resume it exactly where you left it."
Progress is determinate per stage (Freeze→…→Launch), never a
bare spinner > 400 ms (NN-g visibility of system status).
**Performance budget**: Preflight ≤ 300 ms P95 (excl. user wait);
freeze-to-stage for 50 MB ≤ 3 s; verify ≤ 500 ms; pipeline adds
≤ 1% CPU idle (event-driven, no polling below 1 s); rollback
(delete staged + resume source) ≤ 2 s.
**Quality gates**: State-machine transition tests for all
`Staged→Verified→Launched→Confirmed` + every `Failed{stage}`;
fault-injection (kill mid-copy, kill mid-launch) asserts source
hash unchanged; guardrail suite: no edge executes without a user
click (SPEC:26,441).
**Evidence/validation proof**: `cargo test -p juggler-handover pipeline`
incl. fault-injection 100% pass; H-F2 concurrent-append fixture
(writer thread during bundle → no torn line, tail parses); live
dogfood: kill app mid-switch on a real session, relaunch, resume
source — screen recording + edge log in `docs/qa/`. Any torn line
or mutated source byte = fail.
**Visual testing**: Screenshots: freeze dialog (both choices),
per-stage progress, each S-F failure banner at 1280×800/800×600,
light+dark; comparison = baseline diff ≤1%; a11y: dialog traps
focus, `aria-live=polite` stage announcements, Esc/⏎ verified
by keyboard-only walkthrough. Pass = failure banners state the
source is safe in words, not icon alone.
**Files to touch**: `crates/juggler-handover/src/pipeline.rs` (new),
`crates/juggler-handover/src/lib.rs`,
`crates/juggler-app/src/commands.rs`,
`crates/juggler-app/src/state.rs`,
`ui/src/screens/WorkPanels.tsx`.
**Research basis**: pass2/02 §5.1-5.3, §9 (lines 284-335, 488-510);
deep/09 §H4 F-H-01..F-H-12 (lines 163-182); `docs/SPEC.md:225,287`.
**Handover note**: There is no third freeze option "copy live and
hope" — if a reviewer asks for it, cite §5.2. Trap: v0 does raw
`std::fs::copy` (`commands.rs:1143-1146`); that path must become
unreachable, not merely avoided by convention.

### AJ-E206 Hybrid door — 1-click for proven switches, wizard for lossy ones (D-03) | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E205
**Goal**: Implement settled D-03: same-Provider proven-fidelity
switches run in one click with an undo window; cross-Provider,
low-fidelity, or diverged switches open a brief data-driven wizard.
**User story**: As a developer whose Claude Account just hit its
limit, I want the usual switch to be one click, but I want a clear
wizard whenever anything would translate or drop, so speed never
hides loss.
**Requirements**:
1. The door SHALL be chosen by preflight risk only: 1-click iff
   target is the usual Account for this project AND mechanism is
   transplant/native-resume AND fidelity is F3/F4 AND no compaction,
   no conflicts, no listed Drops, target headroom ≥ Tight; otherwise
   the wizard SHALL open (pass2/02 §8 tree, verbatim).
2. 1-click SHALL be: button "Switch to {Account} ▸" → confirm
   popover (carry-list one-liner + replay/switch cost) → execute →
   undo toast 10 s ("Switched — Undo" = stop target + resume
   source + compensating edge); after 10 s the same actions SHALL
   remain available from the Switch Graph (E209).
3. The wizard SHALL have no fixed step count; it SHALL render only
   non-trivial steps among: 1 Target (ranked, always), 2 What's
   carried (carry-list + pack preview, always — no execution from
   an unseen preview, deep/09 H-5), 3 Conflicts (only if any),
   4 Rules/diff (only if diverged), 5 Confirm (cost, fidelity
   label, headroom, always).
4. No handover SHALL execute from a preview the user has not seen;
   the confirm click SHALL live on the preview/Confirm step.
5. Telemetry SHALL distinguish `switch_oneclick` vs
   `switch_wizard{steps_shown}` so the fast path is proven earned;
   ranking SHALL suggest only — execution is always the user's
   click; copy SHALL never say "we switched you" (SPEC:26).
6. All warning strings SHALL be the verbatim §7.1 table (transplant
   proven/unproven, replay, compacted, quota-tight, both-progressed,
   switch-back ready, same-branch, failure-safe); generic "some
   context may be lost" SHALL fail copy review.
**UI/UX reasoning**: Nielsen/NN-g heuristics #5 error prevention +
#6 recognition: 1-click is the wizard with all conditional steps
proven empty, not a lesser flow. Shortcuts: `⌘⇧H` Switch,
`⌘R` Resume/switch-back (final bindings owned by screen spec).
States: default / hover / confirm popover / executing (determinate
progress from E205) / undo toast / failed (source-safe copy).
Wizard step indicator shows only rendered steps (e.g. "Step 2 of
3"), never phantom steps. Apple HIG: destructive/lossy actions
get explicit confirmation; reversible ones get undo.
**Performance budget**: Door decision ≤ 5 ms after preflight;
popover/wizard first paint ≤ 100 ms; wizard step transition
≤ 50 ms; undo resume ≤ 2 s; UI holds 60 fps with 20 live
Instances (SPEC §1); keyboard path ≤ 3 keystrokes to confirm.
**Quality gates**: Decision-tree tests: one fixture per tree leaf
(1-click + each wizard-step combination); copy snapshot test
asserts all nine §7.1 strings byte-exact; guardrail test: zero
executions without click; Playwright flow green.
**Evidence/validation proof**: `cargo test -p juggler-handover door`
+ Playwright `switch-door.spec` 100% pass; dogfood: daily
quota-wall switch timed — median ≤ 2 clicks / ≤ 5 s from click to
target launch on proven pair, logged in `docs/qa/`. Threshold:
any lossy switch reachable in 1-click = P0 fail.
**Visual testing**: Screenshots: 1-click popover, undo toast,
wizard at 2-step / 3-step / 5-step renders, each §7.1 warning,
1280×800 + 800×600, light+dark; comparison = baseline diff ≤1% +
copy-diff; a11y: full keyboard traversal, focus visible, wizard
steps announced via `aria-live`, WCAG 2.2 AA. Pass = lossy state
cannot be screenshotted without its Drop list visible.
**Files to touch**: `crates/juggler-handover/src/door.rs` (new),
`crates/juggler-app/src/commands.rs`,
`ui/src/screens/WorkPanels.tsx`,
`ui/src/components/primitives.tsx`,
`ui/src/screens/NewSession.tsx`.
**Research basis**: decision-log D-03 (`decision-log.md:31-37`);
pass2/02 §7.1-§8 (lines 388-467); deep/09 §H5 H-5 (lines 210-211);
critic C4 resolved pass2/02 §8 (3-step vs 7-step wizards).
**Handover note**: Do not implement "always wizard for safety" —
that re-litigates D-03 and kills the daily flow. Equally, do not
widen 1-click to unproven pairs "temporarily": the E203 cap is
the gate. If `⌘⇧H` conflicts with the shell, flag it, don't
silently pick another key.

### AJ-E207 Switch-back resume+delta and both-progressed divergence (D-05) | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E206
**Goal**: Returning to the original Account resumes its original
native session and delivers interim work as one summarised delta
turn — never a copy-back — with explicit three-way reconciliation
when both sides progressed.
**User story**: As a developer whose first Account has reset, I
want to continue in the original conversation with a single
"while you were away" update, so the round trip feels like I
never left.
**Requirements**:
1. Switch-back SHALL resume the original native session
   (`claude --resume <id>` / `codex exec resume` /
   `zcode --resume`, SPEC:271); the original transcript SHALL NOT
   be copied, moved, or re-ingested; copy-back SHALL be
   prohibited in code (no code path writes B's transcript over A).
2. The interim work SHALL arrive as exactly one delta turn —
   "While you were away": turns, decisions, diff stat, todo
   changes rendered from away-epoch packs — plus the pack(s)
   attached; the engine SHALL record a `switch-back` edge whose
   Epoch resumes the original `native_id` (E201 Req 5).
3. The return SHALL be suggested, never executed, on quota reset
   (automation invariant SPEC:26,263); verbatim copy: "{Account A}
   has reset. Resume the original conversation there? Work from
   {Account B} arrives as one 'while you were away' update."
4. Divergence SHALL use three-way reconciliation: base = fork-edge
   pack (`message_uuids_sha256` + uuid set); away-only uuids →
   auto-includable delta; original-only → no-op; same-file edits
   on both sides → conflict list, never auto-merged; todo/state
   conflicts → listed. Conversation conflicts SHALL NOT be
   text-merged (transcripts are `uuid`/`parentUuid` trees).
5. Per conflict the user SHALL choose Keep original / Take away /
   Keep both (away saved as `…away-<date>` sibling + a turn
   pointing at it); the merge edge SHALL record both parents and
   each choice in `manifest.json` (E201 two-parent edges).
6. If the away Instance is still running, switch-back SHALL first
   offer freeze-or-fork (E205); resuming A while B runs SHALL be
   legal but the UI SHALL state both are live on one Work item.
7. If the original session was deleted/expired (S-F7), the engine
   SHALL offer the nearest surviving Epoch or a pack-primed fresh
   Instance labelled F0/F1, and SHALL never fabricate a resume.
**UI/UX reasoning**: NN-g #1 visibility + Apple HIG continuity:
switch-back is one suggested card (not a wizard) when there are
zero conflicts; conflicts upgrade it to wizard step 3 (E206).
Copy for divergence (§7.1): "Both sides moved since the switch.
{n} new turns and {m} file conflicts need your choice — nothing
is merged silently." Shortcut `⌘R` resume/switch-back. States:
suggested / resuming / delta-injected / conflict-review / forked.
**Performance budget**: Delta render for 500 away turns ≤ 400 ms;
uuid-set reconciliation for 20k messages ≤ 200 ms (hash sets, no
O(n²)); resume launch ≤ 2 s; delta turn ≤ 4k tokens default
(summarised, full pack attached — not replayed verbatim).
**Quality gates**: A→B→A fixture: original `native_id` resumed,
delta present once, zero duplicate uuids (dedupe first-wins);
both-progressed fixture covers each reconciliation class;
unresolved conflicts leave Work forked with both Epochs resumable
(S-F8); source-hash invariant holds throughout.
**Evidence/validation proof**: `cargo test -p juggler-handover switch_back`
100% pass incl. H-F8 A→B→A chain; live dogfood: quota-reset return
on 2 real Accounts judged continuous by user; PROPOSAL round-trip
criterion A→B→A continuity ≥80% (`PROPOSAL.md:144`) measured over
≥10 returns, log in `docs/qa/`. Any copy-back or duplicated uuid
in live transcripts = fail.
**Visual testing**: Screenshots: switch-back suggestion card,
delta turn rendered in target, conflict list (3 choices/file),
both-live warning, dead-session (S-F7) state; 1280×800/800×600,
light+dark; comparison = baseline diff ≤1%; a11y: conflict choices
are radio groups with file-path labels, keyboard completable,
WCAG AA. Pass = reviewer can resolve a 3-file conflict keyboard-only.
**Files to touch**: `crates/juggler-handover/src/switch_back.rs` (new),
`crates/juggler-handover/src/reconcile.rs` (new),
`crates/juggler-app/src/commands.rs`,
`ui/src/screens/WorkPanels.tsx`.
**Research basis**: decision-log D-05 (`decision-log.md:50-58`);
pass2/02 §6.1-6.2 (lines 337-386), S-F7/S-F8; deep/09 §H7.3/F-H-11;
`docs/SPEC.md:288` (mechanism this task pins down).
**Handover note**: SPEC:288 "primed with a delta" is ambiguous —
the settled reading is resume+inject, pinned by D-05. Trap: a
copy-back forks `sessionId` identity and double-counts usage on
uuid re-ingest; if tempted, re-read pass2/02 §6.1. Nth switch-back
must cost the same as the first.

### AJ-E208 Pack redaction policy — global default plus per-Provider override (D-14) | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E202
**Goal**: Secret handling in the pack is always the user's remembered choice — global policy with per-Provider override — never a hard-coded product default, with the active policy visible before any switch.
**User story**: As a security-conscious developer, I want to choose once how secrets are treated and override it per Provider, so packs never leak keys yet I stay in control.
**Requirements**:
1. Settings SHALL store `redaction_global` plus `redaction_override[provider]`; resolution SHALL be override-if-set else global; there SHALL be no code path applying an unchosen hard-coded policy. First run SHALL ask the user to choose the global policy explicitly before any pack is generated (D-14).
2. The wizard/1-click confirm SHALL display the active policy for the destination Provider verbatim (e.g. "Redaction for Codex: Strip secrets (your global choice)" / "…(Codex override)") before execution; changing it SHALL be one click to Provider settings and back without losing wizard state.
3. Detection SHALL cover `sk-*`, `Bearer …`, `.env` values, private-key blocks, and ZCode rollout headers; the manifest SHALL record `redactions:[{kind,count}]` (kinds+counts only — values SHALL never be stored, logged, or written outside the pack).
4. The preview SHALL list each redaction as a diff hunk the user can un-redact per item for this switch only; un-redact SHALL NOT change the remembered policy. Credentials (`auth.json`/keychain/`credentials.json`) SHALL never enter any pack regardless of policy.
5. Per-Provider overrides SHALL live in Provider settings; global choice SHALL live in Settings → Privacy; both SHALL persist across restarts and be covered by settings migration tests.
**UI/UX reasoning**: NN-g user control + WCAG: policy picker uses plain words with examples ("Strip — replace keys with •••", "Keep — pack stays on this Mac", "Ask each time"); first-run choice is a required radio group with no pre-selected default (no dark pattern). States: unset (blocks first pack with chooser), global-only, overridden (badge "Override"). Shortcut: none (settings surface only).
**Performance budget**: Redaction scan ≥ 20 MB/s (regex set compiled once); added pack time ≤ 100 ms per 10 MB; settings resolve ≤ 1 ms; zero network calls.
**Quality gates**: Policy-resolution unit tests (global / override / unset-blocks); secret-canary test: seeded fake keys in transcript+diffs appear in zero pack bytes under Strip; guardrail: credentials excluded under every policy incl. Keep.
**Evidence/validation proof**: `cargo test -p juggler-handover redaction -p juggler-core settings` 100% pass; live dogfood: export pack with canary `.env`, grep pack for canary = 0 hits, screenshot of manifest counts in `docs/qa/`. Threshold: one leaked canary value = P0 fail.
**Visual testing**: Screenshots: first-run chooser (no default selected), wizard policy line (global + override variants), redaction diff preview; 1280×800, light+dark; baseline diff ≤1%; a11y: radio groups labelled, counts announced, WCAG AA. Pass = destination policy readable without opening Settings.
**Files to touch**: `crates/juggler-handover/src/redaction.rs` (new), `crates/juggler-core/src/model.rs`, `crates/juggler-core/src/storage/mod.rs`, `crates/juggler-app/src/commands.rs`, `ui/src/screens/Profiles.tsx`, `ui/src/screens/WorkPanels.tsx`.
**Research basis**: decision-log D-14 (`decision-log.md:139-150`); pass2/02 §4 `redactions` (line 276); deep/09 §H5.4 (lines 203-208), F-H-08; `docs/SPEC.md:310`.
**Handover note**: Trap: deep/09 defaults (local off / export on) are superseded by D-14 — do not implement them as imposed defaults; they may only appear as the options the user picks from. "Ask each time" must still record counts in the manifest.

### AJ-E209 Beta fidelity badges, explainable ranking, and Switch Graph UI | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E203, AJ-E206, AJ-E207
**Goal**: Every switch surface shows its honest beta fidelity badge, an explainable destination ranking, and the per-Work Switch Graph timeline, so "available in beta" is never mistaken for "lossless" (D-04/D-11).
**User story**: As a developer picking where to continue, I want ranked destinations that explain themselves and a timeline of how my work travelled, so I can choose and retrace without guessing.
**Requirements**:
1. Every switch entry point SHALL show the ordered-pair badge (F-label + "Beta" + validated/unvalidated state) from E203; cross-Provider switches SHALL always show the badge plus wizard Drop list; the UI SHALL never render "lossless/exactly" below F4 and SHALL never claim cross-Provider switching without the beta qualifier (D-11).
2. Ranking SHALL adopt deep/09 §Q6 wholesale (hard filters eliminative + logged, then `0.45·headroom + 0.20·affinity + 0.15·fidelity + 0.10·costFit + 0.10·calm`, confidence-discounted) extended with pass2/02 §7 switch cost (−replay tokens priced at target rates), return likelihood (+, prefer returnable targets when source resets <60 min), and cumulative hop cost displayed ("3 switches so far ≈ N tokens", amber per S-F9 — inform, never block, SPEC:237).
3. Each ranked row SHALL show its top-2 factors in words + confidence badge + TTE (e.g. "80% headroom · usual account for this repo"); a "Why this order?" expander SHALL show all inputs incl. switch cost; ranking SHALL suggest only, never execute.
4. The Project panel SHALL render the Switch Graph: horizontal timeline per Work item; Epoch node = Provider icon + Account label + Instance-kind badge + state chip; edge = labelled arrow (`transplant F3 · 12:04` / `replay F1 · ≈41k tok` / `switch-back ↺`); fork = branching, merge = converging with conflict-count badge, failed edge = dashed red linking to its §9 reason; epochs older than the last fork/merge SHALL collapse to "+n switches" (never unbounded lists).
5. Clicking any node SHALL offer Resume (if native session exists), "Switch from here…", and "Export pack…"; clicking a failed edge SHALL show the S-F reason and the source-safe recovery action.
**UI/UX reasoning**: NN-g #1/#6 + pass2/02 §8.1: the graph — not a session list — answers "how did this work travel?". Badge states: validated (solid) vs unvalidated-beta (hatched + "Not yet validated"); both carry text labels (never colour-only, WCAG 2.2). Shortcuts: `⌘⇧H` Switch, `⌘R` Resume/switch-back from a selected node. Empty state: "No switches yet — your work starts here." Density must hold with 20 Instances at 60 fps (SPEC §1).
**Performance budget**: Ranking compute ≤ 30 ms for 12 candidate Accounts; graph render ≤ 100 ms for 200 Epochs (virtualised/collapsed beyond); badge lookup ≤ 5 ms (cached matrix); UI event-to-paint ≤ 300 ms (EPICS M1-6 bar); no polling below 1 s.
**Quality gates**: §Q6 explanation snapshot tests + E206 door-selection fixtures green; copy lint fails on "lossless" below F4 or beta-less cross-Provider claims; ranking determinism test (same inputs → same order + same explanation).
**Evidence/validation proof**: `cargo test -p juggler-handover ranking` + Playwright `switch-graph.spec` 100% pass; dogfood: A→B→A→C→B on a real project, restart app, graph identical (screenshot pair in `docs/qa/`). Threshold: any unbadged cross-Provider switch or unexplained rank = fail.
**Visual testing**: Screenshots: ranked target list with explanations, badge set (validated/unvalidated × F0-F4), graph (linear, fork, merge, failed edge, collapsed "+n"), 1280×800 + 800×600, light+dark; baseline diff ≤1%; a11y: graph operable keyboard-only (nodes focusable, arrows labelled), contrast AA, badges legible in greyscale. Pass = full journey completable without a mouse.
**Files to touch**: `crates/juggler-handover/src/ranking.rs` (new), `crates/juggler-app/src/commands.rs`, `ui/src/screens/WorkPanels.tsx`, `ui/src/components/WorkflowRail.tsx`, `ui/src/components/primitives.tsx`, `ui/src/types/` (generated bridge types).
**Research basis**: decision-log D-04/D-11 (`decision-log.md:39-48,105-111`); pass2/02 §7-8.1 (lines 388-486); deep/09 §Q6 (lines 445-469), §H6; `docs/PROPOSAL.md:142-144` success metrics.
**Handover note**: Do not invent a composite quality score (D-07 forbids it) and do not reuse quota rings as fidelity badges. Trap: affinity uses `ProjectHistory.accounts` (`model.rs:360-365`) — it is also the wrong-billing guard; changing its meaning breaks both. Fleet shape (D-13): node/edge ids must stay machine-qualified-ready even though fleet UI is another epic.

### AJ-E210 Fixture + live validation matrix per Provider pair and beta-exit gate (D-04) | Status: DONE-EVIDENCED | Priority P0 | Effort XL | Depends: AJ-E204, AJ-E205, AJ-E207, AJ-E209
**Goal**: Prove every ordered pair with CI fixtures plus live-binary dogfood, and make the matrix the single source of truth that graduates pairs out of beta labelling (D-04) — shipping in beta now, validation as the mandatory parallel workstream.
**User story**: As a developer trusting a beta switch, I want each Provider pair's badge backed by a passing test matrix I can inspect, so honesty is enforced by evidence, not promises.
**Requirements**:
1. CI SHALL implement fixtures H-F1..H-F8 (deep/09 §H8): sibling set, concurrent-append freeze, collision fork, compacted→F2, malformed tail (reuse `session-malformed.jsonl`, degrade never panic), pathological cwd, PTY-independent golden path (fake harness), A→B→A chain with delta + zero duplicate uuids; every test SHALL assert source bytes hash-identical afterwards.
2. The matrix SHALL cover every ordered pair of pass2/02 §10 (Claude→Claude, Codex→Codex, ZCode→ZCode, Claude↔Codex, Claude↔ZCode, Codex↔ZCode, Any→Gemini/OpenCode pack-only, switch-back on all proven pairs) with per-pair fixture tests (fake harness + goldens), one live dogfood cell, and an advertised ceiling; cross-Provider cells SHALL assert IR golden round-trip + distilled tool-outcome assertions + rules-ring conversion goldens.
3. ZCode→ZCode transplant SHALL remain blocked/F1 until a SQLite clone passes against two schema versions; ZCode target prime SHALL remain F0 until a verified prime path exists; the harness SHALL record unproven cells as unvalidated, never skip them silently.
4. A pair SHALL advertise its E203 label only when its row is green in CI AND one live dogfood cell passes; any failing cell SHALL automatically lower that pair's advertised ceiling one level until fixed, and `TRANSPLANT` (E204) SHALL follow the same rule. Global gates: no edge without a user click (guardrail suite SPEC:26,441), UI never says lossless below F4.
5. Live dogfood SHALL run per pair on real binaries (Claude last-verified + current; Codex/ZCode as adapters land) across project shapes {tiny, monorepo, worktree, compacted-long, subagent-heavy}; pass = target resumes and its next turn answers a question answerable only from pre-switch context, with the carry-list matched against reality; results SHALL be recorded per cell, not as one aggregate.
6. Beta-exit per pair SHALL require: matrix row green, dogfood cell green, failure-taxonomy (S-F1..S-F9) evidence, and the D-04 validation-debt entry closed; the gate report SHALL list remaining unvalidated pairs as open beta debt, never as passed.
**UI/UX reasoning**: NN-g honesty/status: Settings → Beta shows the same matrix users' badges derive from (pair, ceiling, validated ✓/✗, last dogfood date); unvalidated pairs read "Beta — not yet validated" everywhere, including marketing-adjacent surfaces. No new interaction; this task's UI is disclosure only. Copy rule: claim is always "available in beta" (D-11), never "lossless".
**Performance budget**: Full fixture matrix in CI ≤ 15 min (fake harness, parallel); single-pair re-run ≤ 3 min; live dogfood cell ≤ 30 min human time; fixture corpus growth ≤ 5 MB/pair; canary re-verification monthly (SPEC §13) without blocking PR CI.
**Quality gates**: `just verify` + guardrail suite + coverage ≥85% (handover/adapters) green; H-F7 golden path runs without PTY on all three OSes; any matrix cell red blocks that pair's ceiling bump (not the whole beta, per D-04 ship-now rule).
**Evidence/validation proof**: Commands: `cargo test -p juggler-handover -p juggler-adapters --test validation_matrix` 100% pass for claimed-green cells; live evidence: per-cell dogfood logs + before/after sha256 + continuity verdict stored in `docs/qa/gate-E2-validation.md`; A→B→A continuity ≥80% over ≥10 live returns (PROPOSAL:144). Threshold: one green-claimed cell without live evidence = gate fail.
**Visual testing**: Screenshots: Settings → Beta matrix (all pairs, mixed validated/unvalidated), one unvalidated badge in situ on the wizard, gate-report summary view; 1280×800, light+dark; comparison = manual evidence review (not pixel-only, since dates/counts vary); a11y: matrix is a real table with row headers, status in text, WCAG AA. Pass = user can tell exactly which pairs are proven.
**Files to touch**: `crates/juggler-adapters/tests/claude_transcript.rs`, `crates/juggler-adapters/tests/fixtures/claude-code/`, `crates/juggler-handover/tests/validation_matrix.rs` (new), `crates/juggler-adapters/src/fake.rs`, `docs/qa/gate-E2-validation.md` (new), `docs/plan/EPICS.md` (M4 gate link only).
**Research basis**: decision-log D-04 (`decision-log.md:39-48`); pass2/02 §10 (lines 512-540); deep/09 §H8-H9 (lines 281-315), D4; `docs/SPEC.md:434-442` testing strategy.
**Handover note**: D-04 means do not block the beta on this gate — block only the *label graduation*. Trap: pass 2 ran no live dogfood and proved no Codex/ZCode binary behaviour (pass2/00 §6); every such claim here starts unproven. If a vendor format changes weekly, add a version-tagged fixture — never loosen an assertion to make CI green.

---
---
---

**Status report (2026-10-16, AJ-E210)**: DONE-EVIDENCED — evidence
`docs/qa/evidence/AJ-E210/manifest.md`. The validation matrix
(`juggler_handover::validation`) is the single source of truth that
graduates a pair out of "Beta": three never-conflated layers —
mechanism green (the pair's E203 shape proven over fixtures in CI;
never promotes a badge), ci green (both sides' REAL adapters have
fixture rows — claude-code→claude-code only today), and a recorded
live dogfood cell (human-run, before/after sha256 + continuity
verdict, persisted + restart-pinned, unknown pairs refused).
Validated = ci green AND a passing cell; a FAILED cell lowers the
advertised ceiling one level until fixed (`FidelityLabel::lowered`,
floor F0) — into the badges AND the ranking fidelity slot (one
source). H-F1..H-F8 are complete (H-F4 F2-label + bytes-identical,
H-F7 PTY-independent golden path, H-F8 A→B→A chain with zero
duplicate uuids; every test asserts source hashes). Settings → Beta
(Profiles) renders the SAME table (real `<table>`, scoped headers,
status in text); `just e2-gate` regenerates
`docs/qa/gate-E2-validation.md` from the matrix — 0/11 validated,
11 open beta debt, never listed as passed. Coverage recorded
honestly: handover libs 92.7%, adapters libs 77.9% (claude_code.rs
launch paths need real binaries — E808/E707), combined 85.9%. E2 is
COMPLETE subject to live dogfood cells, which belong to humans with
authenticated binaries per the security constraints.

---

**Status report (2026-10-16, AJ-E209)**: DONE-EVIDENCED — evidence
`docs/qa/evidence/AJ-E209/manifest.md`. The badge is a per-ordered-pair
WIRE string (`pair_badge` → "F1 · Beta"), never a global claim:
cross-Provider rows always carry badge + Drop list (D-11), and the copy
lint fails on "lossless" anywhere in a switch surface. Ranking is §Q6
(E611) + the switch extensions: real pair fidelity in the 0.15 slot,
priced replay cost into costFit (cross-Provider only), +0.05 return
bonus when a returnable original's source resets <60 min (post-formula,
never folded into weights), and the S-F9 hop note ("3 switches so far
≈N tokens", amber, inform-only). Rows explain themselves (top factors +
confidence + TTE + Why expander incl. switch cost); filtered rows stay
visible with their reason; suggest-only — the click opens the E205 flow.
The Project panel renders the Switch Graph per Work (provider-marked
epoch nodes with the honest resumable gate, labelled arrows, ↺
switch-backs, fork/merge text chips, failed edges trailing their source
with the §9 reason + source-safe recovery, "+n switches" collapse, empty
state); node menu = Resume (⌘R) / Switch from here / Export pack (E208
GLOBAL policy — unset blocks; Strip leaves zero canary bytes, pinned).
Found + fixed on the way: no-diff packs never wrote (`diffs/EMPTY`
ENOENT, regression test added); a text-faint-on-surface contrast miss
(3.09:1) in the graph card caught by the new a11y scan and fixed.
vitest 245/245, playwright 386/386 (gallery 32). Live A→B→A→C→B dogfood
PENDING E210's per-pair cells (stub round trip green end-to-end).

---

**Status report (2026-10-16, AJ-E207)**: DONE-EVIDENCED — evidence
`docs/qa/evidence/AJ-E207/manifest.md`. The return is a RESUME, never a
copy-back: `LaunchMode::Resume` gained `delta_prompt` (claude:
`--resume <id> "<delta>"`), the switch-back path has NO transcript-write
function (file choices run over the fork manifest's non-transcript rows
only), and the A→B→A fixture pins the original transcript hash
unchanged + the delta present exactly once in the resume argv.
Reconciliation (`reconcile.rs`): conversation base = uuids(A) ∩ uuids(B)
(exact for transplants — B began as A's bytes; replay-born sides degrade
to all-away), files three-way over the fork manifest (both-changed-and-
disagree => conflict; deletions count; identical changes don't);
Keep-original/Take-away/Keep-both choices recorded as `conflict-choice`
manifest rows on the parent-linked return edge. The delta turn is ONE
bounded "While you were away" render (<=40 one-liners + pack pointer;
dedupe first-wins proven with a re-emitted uuid). S-F7 never fabricates
(nearest surviving epoch verified to exist, else honestly-labelled
FreshPackF0); S-F8 refuses to guess (unresolved => the work stays
forked, nothing touched). Suggestions: open excursions whose ORIGINAL
account has headroom again -> verbatim reset card (suggest-only; returns
and compensations don't re-suggest). UI: card in the inspector +
SwitchBackFlow review (delta preview, conflict radio trios,
freeze-or-fork + both-live copy, labelled stages, focus trap).
handover 46/46; switch_back_path 2/2 (10/10 stable — one TEST-harness
stub race fixed by waiting for the setup write); vitest 230/230 (new
guardrails 6/6); visual 342/342 (gallery 30-switchback 20 + a11y 2,
axe 0 both themes). Honest notes recorded: production file conflicts
are empty until non-transcript manifest rows ship (E209/E210) — the
fixture seeds a todos row via the public append API to prove the full
path; live quota-reset dogfood PENDING maintainer login (gate-E5).

**Status report (2026-10-16, AJ-E208)**: DONE-EVIDENCED — evidence
`docs/qa/evidence/AJ-E208/manifest.md`. Secret handling in packs is
the user's REMEMBERED choice (D-14): `redaction_global` +
`redaction_overrides[provider]` KV settings, resolution
override-if-set-else-global, and unset BLOCKS the first pack (the
first-run chooser has NO pre-selected default and disables Switch
until saved — deep/09's local-off/export-on defaults appear only as
options, never applied). Detection (no regex dep): sk-keys, Bearer
tokens, .env values (diff-prefixed lines included), private-key
blocks, and ZCode rollout headers — values cross the engine ONLY as
masked previews (first char + ••• + last 4); the pack manifest
records kinds+counts set at generation (hashes final bytes).
Per-item un-redact addresses GLOBAL detection indices (preflight and
pack-apply share the segment order) and rides the executed edge as
"this-switch-only" rows — never a policy change. The ALWAYS-ON
credential guard still refuses credential material under every
policy incl. Keep, now tightened to real key shapes so the manifest's
own "sk-key" kind name can't false-positive. UI: the verbatim policy
line on popover + Confirm ("(your global choice)" / "(override)"),
per-switch Ask radios, per-item un-redact list on the preview step,
and the per-Provider override row in Provider settings. handover
59/59; redaction_policy 3/3 (persist-across-restart, masked-only
bridge, edge recording); vitest 234/234 (new guardrails 4/4); visual
360/360 (gallery 31-redaction 16 + a11y 2; the preflight seam now
MERGES fixtures over the default — 40 stale-fixture failures found
in the full run and fixed at the seam). Honest notes: the literal
diff-hunk render belongs with E209's pack previews; live canary
export dogfood PENDING maintainer login (gate-E5).

## Epic exit criteria (all must hold)

- [ ] A→B→A→C→B graph survives restart; switch-back resumes the original `native_id` with one delta turn and zero duplicate uuids.
- [ ] No ordered pair advertises above its E210-validated ceiling; `TRANSPLANT` matches the matrix.
- [ ] Every lossy switch passed through a seen preview with carry-list, fidelity badge, redaction policy (D-14), and §7.1 copy.
- [ ] Source transcripts hash-identical after every automated and live switch test.


**Status report (2026-10-16, AJ-E204)**: DONE-EVIDENCED — evidence
`docs/qa/evidence/AJ-E204/manifest.md`. The sub-trait landed
(HarnessAdapter untouched, registry slot optional); the Claude rebuild
resolves via transcript_ref (S-F1 copy on missing), bundles the FULL
sibling set with a torn-tail refusal, installs by discovered cwd
(mangle = flagged fallback), forks collisions with the uuid tree
preserved, and copies never moves (hash-invariant tests). TRANSPLANT
bit off everywhere + interim "F1 Transcript-only (experimental)" UI
label; newest-mtime and `replace('/','-')` code paths DELETED from
commands.rs (engine in juggler-handover::transplant). The real
compact_boundary line shape (`system` type + subtype, per
session-malformed.jsonl:6) is now captured by the parser. 6 fixture
tests + updated golden_path; visual 280/280. Live two-account proof
PENDING maintainer login (recorded); wizard visuals ride E206.

**Status report (2026-10-16, AJ-E205)**: DONE-EVIDENCED — evidence
`docs/qa/evidence/AJ-E205/manifest.md`. The pipeline (preflight →
two-choice freeze → snapshot → atomic stage/verify → caller-hook
launch → record) lives in juggler-handover with the §7.1 copy table
verbatim and the S-F1..S-F9 taxonomy; commands record every attempt
as an E201 edge (Staged before execution, Confirmed + manifest after,
Failed{stage,reason} in place — history never rewritten). Fault
injection 8/8 (torn tail, mid-copy crash, launch failure with
retained stage, source-mutation detection); golden_path pins the app
recording + copy-never-move. UI: SwitchFlow (freeze dialog, labelled
stages, source-safe banner) + gallery 28-switch (16) + axe 0; visual
298/298. Live mid-switch-kill dogfood PENDING maintainer login
(recorded).

**Status report (2026-10-13, AJ-E206)**: DONE-EVIDENCED — evidence
`docs/qa/evidence/AJ-E206/manifest.md`. The §8 door engine
(`juggler-handover::door`: DoorInputs + choose_door + telemetry_kind,
one test fixture per tree leaf) is wired into preflight: the usual
account is the project's UNIQUE-maximum account by session count
(+ imported-history labels; ties are not habits), headroom ≥ Tight
maps to the E606/E610 quota gate, drops come from the E203 base
dispositions — so today every real pair opens the wizard (todos
default Drop until E210's fixtures, by design: the P0 test proves a
DOMINANT-account transplant still gets the wizard). switch_execute
now returns the edge id (SwitchExecutedInfo) and switch_undo
compensates: stop target (abort-before-resume on real stop failure —
never two live instances), resume the source's own native id under
its own recipe, parent-linked NativeResume/F4 edge (S-G1 append
rules kept). UI: 1-click popover (carry one-liner + cost + §5.2
freeze) → undo toast with visible 10 s countdown; wizard renders
ONLY non-trivial steps ("Step 2 of 3", no phantom steps) with the
confirm click on the last step; local-only KV door counters
(`switch_oneclick` vs `switch_wizard{steps}`). Guardrails 7/7 vitest
(zero executions without click; undo only from click); switch_door
integration 3/3; handover 34/34; vitest 224/224; visual 320/320
(gallery 29-door 20 new + gallery 28 re-captured door-aware; axe 0
both themes). Spec's "2-step wizard" render cannot exist (min 3 by
the tree — recorded, not skipped); ⌘⇧H/⌘R left to the screen spec
that owns final bindings; live quota-wall dogfood PENDING
maintainer login + E210 fixture flip.

---

**Status report (2026-10-16, foundation batch E201–E203)**: all three
DONE-EVIDENCED — manifests `docs/qa/evidence/AJ-E201..E203/manifest.md`
(one PR, one gate run per the maintainer's batching directive).

- **AJ-E201**: graph types + V20 migration + append-only repo; 6/6
  tests incl. the a→b→a→c→b restart-identity fixture and a 64-case
  round-trip property. Migration numbering continues after V19
  (the doc's V5/V6 references are Prior-09 prose; current tree is
  at V19 pre-batch — V20 is the graph).
- **AJ-E202**: pack v1 in the new `juggler-handover` code (manifest.rs
  + pack.rs); 9/9 tests + 4 committed goldens; atomic writes with the
  always-on credential guard. PACK.md visual render DEFERRED to
  E206's wizard preview (same artifact, real host) — recorded in the
  manifest, not skipped. Live export dogfood PENDING maintainer login.
- **AJ-E203**: fidelity engine (12/12 tests) — weights/caps/labels
  exactly per §3.4+§H6, ordered-pair only, todos default Drop,
  zcode-target F0; committed full-matrix golden (12 pairs, all ≤F1
  today per D-04). Badge UI DEFERRED to E209 (its spec owns the
  F0–F4 renders). `has_compact_boundary` added to the Claude parser.

**Implementation-mapping**: model.rs (graph types + id-macro serde),
storage V20 + switch_graph.rs, juggler-handover (manifest/pack/
fidelity + goldens), adapters `has_compact_boundary`,
tests: switch_graph 6, pack_v1 9, fidelity_matrix 12.

Next: AJ-E204 (HandoverSupport trait + honest Claude transplant
rebuild) on this branch's successor.