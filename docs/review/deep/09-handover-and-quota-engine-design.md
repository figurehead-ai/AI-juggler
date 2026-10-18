# 09 — Handover & Quota Engine Design (Deep Design)

> Scope: the two signature engines that make AI Juggler a *juggler* rather
> than a launcher — session handover (SPEC §7) and the usage/quota engine
> (SPEC §6). This file *designs* what the prior review only *flagged*:
> exact schemas, state machines, maths, and acceptance criteria.
> Method: repo inspection only (no live vendor calls). Every factual
> claim carries `repo/file:line` or a competitor/vendor URL.
> Verdict up front: **handover v0 must be relabelled and rebuilt before
> it is marketed; quota must ship Claude-Measured first, before any
> second adapter.** The rest of this file is the argument and the design.

## How to read this file

| Reader | Read |
|---|---|
| Solo maintainer, 10 min | §0 verdicts + §H7 + §Q9 checklists |
| Implementer, handover | §H1–H9 in order |
| Implementer, quota | §Q1–Q9 in order |
| Reviewer challenging this design | §H2/§H5 and §Q3 are the load-bearing claims |

Conventions: `H-*` = handover requirement, `Q-*` = quota requirement,
`F-H-*` / `F-Q-*` = failure. “Must” blocks v0.1-beta; “should” follows.
All timestamps are epoch-ms (`crates/juggler-core/src/model.rs:88-113`).

## 0. Verdicts (decision table — read first)

| # | Decision | Rationale (one line) | Gate |
|---|---|---|---|
| D1 | Relabel v0 to `F1 Transcript-only (experimental)` today | v0 copies 1 JSONL, no siblings (`crates/juggler-app/src/commands.rs:1120-1146`) | UI copy PR |
| D2 | Ship fidelity levels F0–F4 (§H6), keep T1–T4 as engine internals | Users need “what carries” not tier codes (SPEC `docs/SPEC.md:271-274`) | Wizard |
| D3 | Move transplant out of `commands.rs` into `juggler-handover` | Crate is doc-only (`crates/juggler-handover/src/lib.rs:1-13`) | F5.3 |
| D4 | Downgrade `TRANSPLANT` to false until §H8 fixtures pass | Capability lies today (`crates/juggler-adapters/src/claude_code.rs:111-120`) | Phase 0 |
| D5 | Quota order: Claude-Measured → ZCode-Computed → Codex-Estimated | Only Claude has a sanctioned feed (§Q1) | Roadmap |
| D6 | Never auto-rotate; recommend + one click only | ADR guardrail (`docs/adr/0003-automation-policy-compliance-guardrails.md:13-16`) | Guardrail test |
| D7 | USD ledger is *estimated* by default, never “Measured” | Token ≠ price without a dated pricing table (§Q7) | Honesty rule |

Prior review context (do not re-litigate): handover v0 is
single-file/mtime-based and T3/T4 do not exist
(`docs/review/full-competitive-ux-review.md:243-282`); quota sits at M3
after M2 breadth (`docs/review/full-competitive-ux-review.md:427-439`);
the daily loop’s #1 pain is the quota wall
(`docs/review/full-competitive-ux-review.md:743-750`). This file assumes
those findings and designs the fix — it does not restate them.

---

# PART H — HANDOVER ENGINE

## H1. Where the code actually is (reality map)

| Concern | Real location today | SPEC promise | Gap |
|---|---|---|---|
| Tier logic | None — inline in Tauri command (`crates/juggler-app/src/commands.rs:1088-1194`) | Engine crate (§7) | No engine |
| Handover crate | Doc-only, 13 lines (`crates/juggler-handover/src/lib.rs:1-13`) | “context packs, rules sync, takeover” (`crates/juggler-handover/Cargo.toml:3`) | Shell |
| Adapter contract | No export/prime/transplant methods (`crates/juggler-adapter/src/adapter.rs:26-124`) | `export`/`prime` sketched (`docs/SPEC.md:173-177`) | Trait gap |
| Capability flags | `TRANSPLANT|FORK|RESUME` advertised (`crates/juggler-adapters/src/claude_code.rs:111-120`) | Gate on verified transplant (`docs/SPEC.md:280`) | Over-claim |
| Transcript parser | Tolerant JSONL reader (`crates/juggler-adapters/src/claude_transcript.rs:63-88`) | Shared IR (`docs/SPEC.md:185`) | No IR type |
| Lineage | `HandedOver` status only (`crates/juggler-core/src/model.rs:205-207`) | A→B→A chains (`docs/SPEC.md:302`) | No lineage table |
| Storage | v1–v4, no handover/usage tables (`crates/juggler-core/src/storage/mod.rs:27-82`) | Lineage + usage store (`docs/SPEC.md:399`) | Schema gap |
| Resume | `--resume <id>` (`crates/juggler-adapters/src/claude_code.rs:294-296`) | T1 native resume (`docs/SPEC.md:271`) | Works, unsurfaced |

**H-1 (must):** the first handover PR adds trait methods or deletes the
flags. A capability bit with no implementation is a lie the UI will
render. Minimal honest trait (mirrors SPEC sketch, sync-first to match
the current non-async trait at `crates/juggler-adapter/src/adapter.rs:26`):

```rust
// crates/juggler-adapter/src/handover.rs (new)
pub struct SessionBundle { pub manifest: BundleManifest, pub files: Vec<BundleFile> }
pub struct BundleManifest {
    pub schema: u32, pub harness: HarnessId, pub native_id: String,
    pub cwd: PathBuf, pub harness_version: String, pub created_at: Timestamp,
    pub fidelity: Fidelity, pub redacted: bool,
}
pub trait HandoverSupport { // optional sub-trait; None = T3/T4 only
    fn bundle_session(&self, r: &TranscriptRef) -> Result<SessionBundle, AdapterError>;
    fn install_bundle(&self, b: &SessionBundle, t: &Account) -> Result<TranscriptRef, AdapterError>;
    fn prime_prompt(&self, ir: &TranscriptIr) -> Result<String, AdapterError>;
}
```

Keeping this a *separate* sub-trait (not new required methods) preserves
the semver-protected trait promise (`docs/adr/0002-harness-adapter-architecture.md:13-25`)
and lets the fake adapter (`crates/juggler-adapters/src/fake.rs:49-57`)
opt in incrementally. `juggler-handover` then owns orchestration;
`commands.rs` shrinks to parse-args → engine → emit.

## H2. What a Claude JSONL actually contains — and what it misses

Evidence base: committed fixtures (`crates/juggler-adapters/tests/fixtures/claude-code/`),
the tolerant parser (`crates/juggler-adapters/src/claude_transcript.rs:30-155`),
and the vendor directory map (`docs/research/claude-code.md:24,40-45`).
Format is internal/unofficial and version-churning — pin per version.

| Content | In main `.jsonl`? | Evidence | Handover consequence |
|---|---|---|---|
| User/assistant turns (`uuid`/`parentUuid` tree) | Yes | Fixture `session-basic.jsonl:1-5`; parser fields (`crates/juggler-adapters/src/claude_transcript.rs:99-110`) | Carries in F2+ |
| Token `usage` per assistant msg | Yes | `session-basic.jsonl:2-3,5`; `extract_usage` (`crates/juggler-adapters/src/claude_transcript.rs:140-155`) | Feeds quota (§Q1) |
| Inline sidechains (`isSidechain:true`) | Yes, inline | `session-subagent.jsonl:3-4`; flag (`crates/juggler-adapters/src/claude_transcript.rs:45-46,106-109`) | Byte-copy *does* carry these |
| `summary` leaf lines | Yes | `session-basic.jsonl:6`; kind counted (`crates/juggler-adapters/tests/claude_transcript.rs:48-53`) | Preserve verbatim; do not render as a turn |
| `system/compact_boundary` | Yes | `session-malformed.jsonl:6` (`subtype:compact_boundary`) | Marks context loss — see §H2.1 |
| Tool calls/results as *text* | Partially — parser drops them | `extract_text` keeps only `type:text` (`crates/juggler-adapters/src/claude_transcript.rs:120-137`); fixture asserts empty text (`crates/juggler-adapters/tests/claude_transcript.rs:59-63`) | F3 replay must distil, not “replay fully” |
| Separate subagent transcripts | **No — sibling files** | Vendor map: dir holds “subagent transcripts, spilled tool results, file-history” (`docs/research/claude-code.md:40`) | v0 drops them (F-H-04) |
| File-history snapshots / checkpoints | **No — sibling state** | 100 checkpoints, ~30d (`docs/research/claude-code.md:44`) | `/rewind` history does not transplant |
| In-session todos | **Unverified in JSONL** | SPEC claims lossless todos (`docs/SPEC.md:272`); no todo code in parser/handover path | Treat as *not carried* until §H8 proves it |
| MCP server state / live processes | No (by construction) | MCP config is per-dir (`docs/research/claude-code.md:62`); processes are runtime | Never promise transfer |
| Model KV / working context | No (for anyone) | Honest boundary (`docs/SPEC.md:276`) | F3 rebuilds context from text |
| Credentials / keychain | Must never copy | Metadata-only login probe (`crates/juggler-adapters/src/claude_code.rs:323-344`) | Excluded from every bundle |

### H2.1 Context compaction — the silent fidelity killer

`/compact` + auto-compaction replace history with a summary in-context
(`docs/research/claude-code.md:45`); the fixture’s `compact_boundary`
line is the on-disk scar. After compaction, the JSONL still *contains*
pre-compact lines, but the *model’s* working context no longer does.
Transplanting the raw file therefore over-promises: the resumed model
re-reads more than it “remembers”, yet may still miss
post-compact-only state. **H-2 (must):** fidelity computation scans for
`compact_boundary`; if present, the wizard says “session was compacted
— early detail survives as text, not as live context” and F2 drops to
the F2− label in §H6. No other tool labels this; it is cheap trust.

### H2.2 `cwd` mangling — why `replace('/','-')` is wrong

v0 encodes cwd as `source.cwd.replace('/', "-")`
(`crates/juggler-app/src/commands.rs:1121-1122`). The codebase already
knows decode is ambiguous: `transcript_cwd` reads the true `"cwd"` from
the first 25 lines because dir-name decode fails on dots/long paths
(`crates/juggler-app/src/commands.rs:1046-1060`). Encode has the mirror
problem: `/a/b.c` and `/a/b-c` collide under naive replacement, and
vendor mangling is undocumented/versioned
(`docs/research/claude-code.md:40-41`).

**H-3 (must):** never *compute* the target dir from cwd. Resolve the
source file from `SessionRecord.transcript_ref`
(`crates/juggler-core/src/model.rs:314-321,415`) — the record already
knows its file — and derive the target dir by *discovering* the target
account’s existing project dir for that cwd (scan its `projects/*`,
match by embedded `"cwd"` via `transcript_cwd`), falling back to the
vendor-documented mangling only when no dir exists, with fallback
flagged in the manifest. Golden test: cwd with dots + dashes + spaces
and a >200-char path must round-trip (fixture H-F6, §H8).

## H3. T1–T4 reality check (per harness)

SPEC tiers (`docs/SPEC.md:269-274`): T1 native resume, T2 transplant,
T3 replay, T4 pack. Reality against code + research:

| Tier | Claude | Codex | ZCode |
|---|---|---|---|
| T1 | Real: `--resume <id\|path>`, `--fork-session` (`docs/research/claude-code.md:42`; `crates/juggler-adapters/src/claude_code.rs:294-296`) | Real: `codex resume` / `exec resume <id>` (`docs/research/codex.md:44`) | Real: `--resume sess_*` / `-c` (`docs/research/zcode.md:65`) |
| T2 | Plausible, unproven: copy JSONL + resume-by-path (`docs/SPEC.md:280`); binary shared so format matches | Plausible: copy `rollout-*.jsonl` (`docs/SPEC.md:281`); `session_meta.cwd` is absolute (`docs/research/codex.md:46`) | **Blocked**: SQLite clone, schema-coupled (`docs/SPEC.md:282`); tables at `docs/research/zcode.md:63` |
| T3 | Needs IR + prime; parser drops tool text today (§H2) | Vendor precedent: Codex `/import` of Claude chats ([changelog via `docs/research/codex.md:63`](https://learn.chatgpt.com/docs/changelog)) | Source feeds T3 via SQLite reads (`docs/research/zcode.md:76-77`) |
| T4 | Pack format specced (`docs/SPEC.md:296-304`), zero code | Same pack | Same pack; redact rollout headers (`docs/research/zcode.md:64`) |

**H-4 (must):** T2-Claude ships first and alone. T2-Codex waits for the
Codex adapter; T2-ZCode stays T3 until a schema-versioned clone is
proven against two ZCode versions — SPEC already concedes this
(`docs/SPEC.md:282`), so the wizard must show ZCode targets as
“Replay” and never grey out into a fake “Transplant”.

## H4. Failure taxonomy (each needs a test + a user sentence)

| ID | Failure | Trigger in v0 | Engine behaviour (required) | User sees |
|---|---|---|---|---|
| F-H-01 | Wrong session copied | Newest-by-mtime (`crates/juggler-app/src/commands.rs:1026-1033`) with 2 live sessions | Resolve via `transcript_ref`; refuse if ambiguous | “Couldn’t tell which file is this session — pick it” |
| F-H-02 | Torn last line | Source appending during `std::fs::copy` (`crates/juggler-app/src/commands.rs:1143-1146`) | Freeze: stop source or snapshot-copy + parse-verify tail | “Source is still writing — stop it first?” |
| F-H-03 | ID collision on target | Same `sessionId` already present; no fork (`docs/SPEC.md:280` unimplemented) | Fork: new UUID + rewrite `sessionId` chain, keep `parentUuid` tree | “Already on B — opened as a fork” |
| F-H-04 | Sibling state lost | Only 1 file copied (§H2) | Copy `<session>*` set + manifest inventory | Carry-list shows ✓/✗ per item (§H6) |
| F-H-05 | Mangled-dir collision | §H2.2 encode guess | Discover-don’t-compute (H-3) | No collision possible; fallback flagged |
| F-H-06 | Version drift | Parser pinned to 2.1.296 (`crates/juggler-adapters/src/claude_code.rs:29-32`) | Pre-flight: parse target file, count malformed; abort if >0 unknown-critical | “B runs Claude X, file is Y — proceed as Replay?” |
| F-H-07 | Compacted history | `compact_boundary` present (§H2.1) | Downgrade label, keep raw copy | F2− wording |
| F-H-08 | Secrets in pack | Verbatim export (`docs/SPEC.md:296-304`) | Redact pass + preview (§H5) | Diff of redactions, approve to export |
| F-H-09 | Target quota-full | No headroom check in v0 | Quota gate (§Q6 score; block/warn) | “B at 94% — A or C recommended” |
| F-H-10 | Half-copied target | Crash mid-copy | Stage to temp dir, verify, atomic rename | Either old or new — never partial |
| F-H-11 | Return divergence | A resumed while B continued | Delta priming (§H7.3), never silent merge | “While you were away” summary shown |
| F-H-12 | Worktree/branch clash | Two accounts, same checkout (`docs/SPEC.md:113` cwd-only) | Warn same-branch live sessions; offer worktree | Branch chip + warning (§UX prior F1.1) |

Invariants for all rows: source is *copied, never moved*
(`docs/SPEC.md:287`); every failure leaves source resumable; no
state change without the confirm click (ADR-0003:13-16).

## H5. Preview, diff & redaction (the trust surface)

The pack *is* the preview artifact (SPEC §7.3,
`docs/SPEC.md:292-306`): `PACK.md` + `conversation.md` + `state.json`
+ `diffs/`. Design the wizard around rendering it *before* execution:

1. **Carry-list (always).** Generated from the bundle manifest: turns
   count, sidechains (inline ✓ / sibling files n, bytes), todos
   (carried / not found / not carried), file-history (not carried),
   MCP (reconnect on target / not carried), compaction flag. Each row
   is ✓/✗/⚠ — never prose alone. This replaces tier codes in the UI.
2. **Conversation preview.** First/last 3 turns + full-text search over
   `conversation.md`; T3 shows the *exact* priming text (tool outcomes
   distilled — labelled “distilled”, since the parser drops raw tool
   text today, §H2). Byte count + estimated target tokens shown.
3. **Diff preview (project sync + return).** Rules files
   (AGENTS.md ↔ CLAUDE.md ↔ `.zcode/AGENTS.md`, `docs/SPEC.md:289`),
   uncommitted diff stat, and the “while you were away” delta render
   as unified diffs; nothing applies without the apply click.
4. **Redaction (export-only by default).** Pattern pass over pack +
   conversation: `sk-*`/`Bearer …`/`.env` values/private keys/ZCode
   rollout headers (`docs/research/zcode.md:64`). Local same-machine
   handover defaults *off* (fidelity), export defaults *on*; the
   preview lists each redaction as a diff hunk the user can un-redact
   per item. Redacted values never touch disk outside the pack.

**H-5 (must):** no handover executes from a preview the user has not
seen. The confirm click is on the preview screen, not before it.

## H6. Proposed fidelity levels with user-facing labels

Keep T1–T4 inside the engine; show users *fidelity* — what survives.
Levels are computed per (source, target) pair at wizard-open, never
stored as a static harness property.

| Level | User label (wizard badge) | Engine tier | Carries | Explicitly does not carry |
|---|---|---|---|---|
| F4 | **Continues exactly** | T1 same-account resume | Everything native | — (same account; no copy) |
| F3 | **Same conversation, new account** | T2 transplant | Turns, inline + sibling sidechains, usage history | File-history/rewind, MCP runtime, running tools |
| F2 | **Same conversation, new account (compacted)** | T2 + `compact_boundary` | As F3, raw text complete | Live pre-compact context (model re-reads) |
| F1 | **Full story, fresh start** | T3 replay | Every turn as text, distilled tool outcomes | Native todos/file-history; target rebuilds context |
| F0 | **Briefing pack** | T4 pack | Task/decisions/TODOs/diffs summary (`docs/SPEC.md:296-304`) | Verbatim history beyond last N turns |

Rules: (a) the badge always pairs with the §H5 carry-list — a badge
alone is marketing; (b) F-level can only *drop* after pre-flight
(parse check, compaction scan, quota gate), never rise; (c) ZCode
targets cap at F1 until H-4 lifts; (d) cross-machine T2 additionally
requires same binary version + same checkout path
(`docs/SPEC.md:388`) else F1. Competitor anchor: no orchestrator
labels fidelity at all — Claude Squad/Conductor keep sessions
agent-bound (`docs/research/landscape-and-stack.md:11,60-62`); this
label is the differentiator, so it must be conservative.

## H7. Rollback & lineage data model

Today lineage is a status flip (`crates/juggler-app/src/commands.rs:1164-1188`).
Required: an append-only lineage graph that survives A→B→A and forks.

```sql
-- migration V5 (append to crates/juggler-core/src/storage/mod.rs MIGRATIONS)
CREATE TABLE handovers (
  id TEXT PRIMARY KEY, created_at INTEGER NOT NULL, actor TEXT NOT NULL DEFAULT 'user',
  source_session TEXT NOT NULL REFERENCES sessions(id),
  target_session TEXT REFERENCES sessions(id),          -- NULL until launch succeeds
  source_account TEXT NOT NULL, target_account TEXT NOT NULL,
  tier TEXT NOT NULL, fidelity TEXT NOT NULL,           -- T2/F3 etc. as shown to user
  bundle_sha256 TEXT, bundle_bytes INTEGER, manifest_json TEXT NOT NULL,
  state TEXT NOT NULL,                                  -- §H7 state machine
  failure TEXT, prev_handover TEXT REFERENCES handovers(id)  -- chain link
);
CREATE TABLE session_files (  -- bundle inventory, one row per copied file
  handover TEXT NOT NULL REFERENCES handovers(id), rel_path TEXT NOT NULL,
  sha256 TEXT NOT NULL, bytes INTEGER NOT NULL, kind TEXT NOT NULL, -- main|sidechain|spill|pack
  PRIMARY KEY (handover, rel_path));
CREATE INDEX idx_handovers_source ON handovers(source_session);
```

State machine (handover): `Staged → Verified → Launched → Confirmed`,
any → `Failed{stage,reason}`; `Confirmed → Returned` when the return
flow (§H7.3 analogue of SPEC `docs/SPEC.md:288`) completes. Transitions
are storage transactions; file install is stage-verify-rename (F-H-10)
*inside* `Staged→Verified`, so a crash never yields a half-installed
target. Rollback = delete staged/target copy + mark `Failed`; source
needs no rollback because it is never mutated. “Undo handover” in UI
means: stop target, resume source (T1) — offered for 24h, implemented
as a new lineage edge, not history rewriting.

### H7.1 Dedupe & fork identity

Resumed files re-emit history; `dedupe_messages` already implements
first-wins by `uuid`, uuid-less always kept
(`crates/juggler-adapters/src/claude_transcript.rs:167-186`, proven by
`crates/juggler-adapters/tests/claude_transcript.rs:91-101`). Bundle
install must call it pre-verify: if target already holds ≥90% of
source uuids under a different `sessionId`, treat as F-H-03 fork
(new sessionId, rewritten chain) rather than a duplicate copy.

## H8. Verification test plan (fixtures + live dogfood)

Fixture corpus lives beside existing goldens
(`crates/juggler-adapters/tests/fixtures/claude-code/`). Each new
fixture is anonymised, version-tagged, and asserted on *counts*
(lines/kinds/messages/usage) like the current suite
(`crates/juggler-adapters/tests/claude_transcript.rs:42-89`).

| ID | Fixture / test | Asserts | Blocks |
|---|---|---|---|
| H-F1 | Sibling set: main + 2 subagent files + spill | Bundle copies all; inventory complete | F-H-04 |
| H-F2 | Concurrent-append: writer thread during bundle | Freeze/snapshot; tail parses; no torn line | F-H-02 |
| H-F3 | Collision: target holds same sessionId | Fork rewrite; both resumable (fake) | F-H-03 |
| H-F4 | Compacted session (`compact_boundary`) | Label F2; raw bytes identical | F-H-07 |
| H-F5 | Malformed tail (reuse `session-malformed.jsonl`) | Degrade-count, never panic (`crates/juggler-adapters/tests/claude_transcript.rs:103-121`) | F-H-06 |
| H-F6 | Pathological cwd (dots/dashes/spaces/long) | Discover-don’t-compute round-trip (H-3) | F-H-05 |
| H-F7 | Golden path, PTY-independent (piped fake) | Handover e2e runs in CI (today PTY-bound: `docs/review/full-competitive-ux-review.md:401-412`) | Ship gate |
| H-F8 | A→B→A chain (fake harness) | Lineage graph, delta prime, no dup uuids | Return flow |

Live dogfood matrix (manual, `#[ignore]` like existing dogfood):
Claude {last verified 2.1.296 (`crates/juggler-adapters/src/claude_code.rs:32`), current latest}
× project shapes {tiny single-file, monorepo, worktree, compacted
long session, subagent-heavy} × {F3, F2, F1-to-Codex when adapter
exists}. Pass = target resumes, next turn answers a question only
answerable from pre-handover context, carry-list matched reality.
Any cell failing drops that pair’s advertised fidelity one level
until fixed — the matrix *is* the capability flag’s source of truth.

### H9. Handover acceptance criteria (ship gate)

- [ ] `TRANSPLANT` advertised only for pairs green in §H8 matrix.
- [ ] H-F1–H-F8 pass in CI (H-F7 without PTY); source bytes unchanged.
- [ ] Every wizard execution passed through §H5 preview; lineage row
  written for success *and* failure; A→B→A dogfood cell green.
- [ ] Copy-paste check: UI never says “lossless” except F4.

---

# PART Q — QUOTA ENGINE (design from scratch)

No quota code exists: `UsageSnapshot` is modelled
(`crates/juggler-core/src/model.rs:299-309`) but handover inserts it
empty (`crates/juggler-app/src/commands.rs:1182`) and SPEC §6
(`docs/SPEC.md:254-259`) is prose. This part designs the engine to
build. Goals G2/G3 (`docs/PROPOSAL.md:32-33`) depend on it.

## Q1. Data sources per harness (sanctioned only)

Rule: local files + vendor-fed local JSON only. No dashboard scraping,
no undocumented quota endpoints — OpenAI research explicitly places
scraping near the prohibited line (`docs/research/codex.md:70-72`)
and ZCode has no public quota API (`docs/research/zcode.md:77`).

| Harness | Primary source | Fallback | Confidence ceiling |
|---|---|---|---|
| Claude | Statusline stdin JSON `rate_limits.five_hour/seven_day.{used_percentage,resets_at}` — Pro/Max only ([statusline docs](https://code.claude.com/docs/en/statusline); `docs/research/claude-code.md:54`) | Transcript `usage` sums (`crates/juggler-adapters/src/claude_transcript.rs:140-155`) | **Measured** |
| Claude exhaustion | `system/api_retry` rate_limit (headless) + `StopFailure` hook (`docs/research/claude-code.md:54`) | Transcript “hit your … limit” pattern (SPEC `docs/SPEC.md:194`) | Measured event |
| Codex | Rollout `token_usage_record` (`docs/research/codex.md:43`) + `exec --json` `turn.completed.usage` (`docs/research/codex.md:59`) | `/statusline` widgets `five-hour-limit`/`weekly-limit` (`docs/research/codex.md:51`) | **Estimated** (no public API, `docs/research/codex.md:52`) |
| ZCode | SQLite `model_usage` tokens × published credit formula (`docs/research/zcode.md:76`) | Plan caps as configured ceilings (`docs/SPEC.md:210`) | **Computed** (pool total still configured) |

Claude statusline capture: install a managed statusline script per
profile dir (SPEC `docs/SPEC.md:192`) that appends the stdin JSON to
an append-only spool the engine tails — the script must be a
pass-through (chain any user statusline) or adoption will break
users’ existing setups. `/usage`/`stats-cache.json` are local-machine
only (`docs/research/claude-code.md:50`) — use as reconciliation, never
as truth for multi-device accounts. ccusage proves local-JSONL
aggregation at scale ([ccusage](https://github.com/ccusage/ccusage);
`docs/research/claude-code.md:55`); cc-switch shows tray-level
5h/weekly display is the expected shape
([cc-switch](https://github.com/farion1231/cc-switch);
`docs/review/full-competitive-ux-review.md:665-669`).

## Q2. Confidence levels (honesty contract)

Reuse `Confidence::{Measured, Computed, Estimated}` — default
`Estimated`, “never overstate trust”
(`crates/juggler-core/src/model.rs:286-297`; SPEC honesty rule
`docs/SPEC.md:259`). Engine rules:

- **Q-1:** confidence is per-(account, window, instant), stored with
  every sample — a Claude account is Measured only while statusline
  events are fresh (<10 min); stale Measured decays to Estimated in
  UI (“last measured 2h ago”), never silently stays green.
- **Q-2:** forecasts inherit the *lowest* confidence of their inputs.
- **Q-3:** ZCode credits are Computed; its 5h/weekly *pool* % stays
  Estimated until the user enters plan caps (tiers at
  `docs/research/zcode.md:14`: Lite 2k/10k, Pro 12k/60k, Max 28k/140k).
- **Q-4:** USD is never Measured (§Q7). UI badge text is exactly
  “Measured / Computed / Estimate” — no stars, no hidden footnotes.

## Q3. Window maths (5h / weekly)

Vendor facts: Claude = rolling 5h + weekly, shared across models
(`docs/research/claude-code.md:51-53`); Codex = 5h rolling + weekly,
plan-dependent, Pro has no 5h limit (`docs/research/codex.md:50`);
ZCode = 5h rolling + 7d-from-purchase weekly with peak/off-peak
credit rates (`docs/research/zcode.md:14-16`).

Definitions (all UTC epoch-ms; windows are *sets of events*, not
buckets — bucketing breaks rolling windows):

- Event: `UsageEvent{account, session?, ts, model, tokens(TokenCounts
  at crates/juggler-core/src/model.rs:261-275), credits?, source, confidence}`.
- 5h usage at `t`: sum events in `(t−5h, t]`. Weekly: sum in
  `(t−7d, t]` (ZCode: `(anchor+k·7d]` phases from purchase anchor).
- % used (Measured): trust vendor `used_percentage` verbatim; only
  *interpolate* between samples by adding locally-seen token deltas
  scaled by the last observed %-per-token slope, clamped [0,100].
- % used (Computed/Estimated): `100 × window_sum / cap`, where `cap`
  is user-confirmed plan cap; if cap unknown, show absolute burn +
  “set your plan to see %” — never invent a denominator.
- Reset: Measured → vendor `resets_at`. Estimated → earliest event
  expiry `min(ts)+5h` (rolling) — display as “≈” with confidence.

Property tests (SPEC M3-2, `docs/plan/EPICS.md:91`): window sum is
monotone under event insertion; events exactly on the boundary follow
the `(t−W, t]` rule on both windows identically; clock-skew (±5 min)
never yields negative headroom; dedupe by message `uuid` before
summing (resumed files double-count otherwise — the parser’s dedupe
contract, `crates/juggler-adapters/src/claude_transcript.rs:167-169`).

## Q4. Forecast, burn-rate & reset-queue

- **Burn-rate:** EWMA over 5/15/60-min token (or credit) rates per
  account; headline uses 15-min, cone shows 5↔60 spread. Cold start
  (<10 min data): “learning — no forecast yet”, never a fake number.
- **Time-to-exhaust:** `TTE = headroom / rate15` when rate > 0;
  display only when TTE < window remaining (else “won’t exhaust this
  window at current pace”). Forecast confidence = Q-2 inheritance;
  Estimated forecasts render as ranges (“40–70 min”), Measured as
  point + range on hover.
- **Launch/handover gate:** projected cost of the *intended* session
  (median session burn for this project, from history) vs headroom →
  OK / Tight / No, mirroring the admission advisor’s verdict language
  (`docs/SPEC.md:237`). “No” warns, never blocks launch (SPEC:237).
- **Reset-queue:** on `QuotaBlocked`, offer “Queue resume at ≈reset”.
  Implementation is a passive timer + notification; the resume itself
  fires only from the notification click (ADR-0003) — Claude already
  self-waits on v2.1.234+ (`docs/research/claude-code.md:53`), so the
  queue’s value is *cross-account* return (F-H-11 delta), not babysitting.

## Q5. Alert policy (anti-fatigue by construction)

Pain evidence: quota walls kill in-flight work (Claude #98299 via
`docs/PROPOSAL.md:23`) yet notification fatigue is itself a ranked
pain (`docs/review/full-competitive-ux-review.md:751`). Policy:

| Trigger | Channel | Cooldown / dedupe |
|---|---|---|
| TTE < 45 min (any window) | Tray + ring amber, no sound | Once per window per account |
| TTE < 15 min or ≥90% | Notification (actionable: Hand over / Queue) | Once; snooze 30 min |
| Exhausted / `QuotaBlocked` | Notification + inbox row | Once per exhaustion |
| Reset reached (queued item) | Notification (click = resume) | Once |
| Forecast was wrong by >2× | Silent log + calibration note | Never interrupts |

Digest by default: non-actionable crossings batch into an hourly
digest; per-project mute; quiet hours honoured. Every alert carries
its confidence badge — an Estimated 90% alert reads differently from
Measured, and the copy says so. Alert state machine per (account,
window): `Armed → Warned → Critical → Exhausted → (reset) → Armed`;
no backward transitions without a reset or manual re-arm, which is
what guarantees at-most-once semantics above.

## Q6. Smart target-account recommendation (suggest, never switch)

Used at NewSession, handover-target, and reset-return. It *ranks*; the
user clicks (ADR-0003; prior WOW-d at
`docs/review/full-competitive-ux-review.md:778-781`). Hard filters
first (eliminate, don’t score): wrong harness for a T2-only request,
logged-out (`crates/juggler-adapters/src/claude_code.rs:332-344`),
quota-exhausted with no reset before need-by. Then score survivors:

`score = 0.45·headroom + 0.20·affinity + 0.15·fidelity + 0.10·costFit + 0.10·calm`

- *headroom*: min(5h, weekly) headroom % (confidence-discounted:
  Estimated × 0.85 so Measured truth wins ties).
- *affinity*: this project’s historical account share (from
  `ProjectHistory.accounts`, `crates/juggler-core/src/model.rs:357-365`)
  — also the wrong-billing guard (Claude #87447,
  `docs/research/claude-code.md:36`).
- *fidelity*: F-level numeric (F3 > F1) for handover; launch-neutral.
- *costFit*: inverse projected USD (§Q7) normalised across candidates.
- *calm*: 1 − (active sessions on account / soft cap) — spreads load.

Explanation UI (required, not optional): ranked rows each with the
top-2 contributing factors in words (“80% headroom · usual account
for this repo”) + confidence badge + TTE. One “Why this order?”
expander shows all five inputs. No score without an explanation ships.

## Q7. Cost ledger in USD

Tokens already parse per message (§H2); pricing is the missing table.
Design: versioned `pricing(model, effective_from, in, out, cache_r,
cache_w, source_url)` rows, bundled + user-overridable, modelled on
ccusage’s per-model tables ([ccusage](https://github.com/ccusage/ccusage))
and cc-switch’s models.dev sync
(`docs/review/full-competitive-ux-review.md:665-669`). Ledger entry =
`UsageEvent` priced at event-time rates; rollups per session/account/
project/day (prior F1.2). Labels: token counts keep their §Q2
confidence; USD is always “Estimate” (Q-4) except statusline
`cost.total_cost_usd` passthrough, shown as “vendor-reported”.
Subscription accounts show *both* views — “≈$X at API prices (you pay
flat Max)” — so Persona A’s Max-vs-API question
(`docs/review/full-competitive-ux-review.md:782-784`) is answerable
without pretending the flat fee is metered. CSV export reuses the
rollup query; ZCode credits show alongside USD, never converted at an
invented rate (formula at `docs/research/zcode.md:14`).

## Q8. Exact schemas & engine state machines

```sql
-- migration V6 (quotas; V5 is §H7 — order by build sequence)
CREATE TABLE usage_events (
  id INTEGER PRIMARY KEY, account TEXT NOT NULL, session TEXT,
  ts INTEGER NOT NULL, model TEXT, input INTEGER NOT NULL DEFAULT 0,
  output INTEGER NOT NULL DEFAULT 0, cache_r INTEGER NOT NULL DEFAULT 0,
  cache_w INTEGER NOT NULL DEFAULT 0, reasoning INTEGER NOT NULL DEFAULT 0,
  credits REAL, usd_est REAL, source TEXT NOT NULL,      -- statusline|transcript|rollout|sqlite|api_retry
  confidence TEXT NOT NULL, msg_uuid TEXT, UNIQUE(account, msg_uuid));
CREATE TABLE quota_samples (
  account TEXT NOT NULL, window TEXT NOT NULL, ts INTEGER NOT NULL,
  used_pct REAL NOT NULL, resets_at INTEGER, confidence TEXT NOT NULL,
  PRIMARY KEY (account, window, ts));
CREATE TABLE reset_queue (
  id TEXT PRIMARY KEY, session TEXT NOT NULL, account TEXT NOT NULL,
  resume_at INTEGER NOT NULL, state TEXT NOT NULL DEFAULT 'queued', -- queued|notified|resumed|dismissed
  created_at INTEGER NOT NULL);
CREATE INDEX idx_usage_acct_ts ON usage_events(account, ts);
```

Engine pipeline (per account, single writer like storage today):
`Ingest(source event) → Dedupe(uuid) → Window fold → Forecast →
Alert FSM (§Q5) → Recommend (§Q6 on demand)`. Account quota state
machine: `Unknown → Learning → Tracking ⇄ Stale(>10m, Measured only)
→ Exhausted → (reset) → Tracking`. `Unknown/Learning` render empty
states, never 0% rings (prior F2.4). All maths is pure functions over
`usage_events` so §Q3 property tests need no harness.

## Q9. Quota acceptance criteria & build order

- [ ] Claude-Measured within ±5% of `/usage` over a full window on 2
  live accounts (SPEC M3 gate, `docs/plan/EPICS.md:95`); statusline
  chaining preserves a pre-existing user statusline.
- [ ] §Q3 properties + §Q4 cold-start + §Q5 at-most-once alerts pass
  on fixture timelines (no live account needed).
- [ ] Every rendered number carries confidence; Estimated forecasts
  show ranges; unknown caps show no % (Q-1–Q-4 audited in UI pass).
- [ ] Recommendation explains itself (§Q6) and never executes.
- [ ] Build order: spool+ingest → windows/forecast → rings/alerts →
  recommendation → USD ledger → Codex/ZCode estimators. Each step
  ships value alone; none waits for a second harness.

*End of design. Unresolved evidence is listed in the return schema,
not buried here: live-format truth (§H8 matrix) and ZCode/Codex
quota behaviour remain unproven until dogfood runs.*






