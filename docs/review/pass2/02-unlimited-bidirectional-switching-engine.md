# 02 — Unlimited Bidirectional Switching Engine (Pass 2)

> Second pass. Canonical input: maintainer example workflow
> (`docs/expected-worfklow.md:1-20`). Prior work cited, not restated:
> Maximum-Depth Report (`docs/review/AI-Juggler-Maximum-Depth-Report-and-Proposal.md`,
> esp. embedded deep `09-handover-and-quota-engine-design.md`, hereafter
> `Prior-09 §H*`/`§Q*`), Full Competitive & UX Review
> (`docs/review/full-competitive-ux-review.md`, hereafter `FCUR F*`),
> `docs/PROPOSAL.md`, `docs/SPEC.md`, `docs/plan/EPICS.md`.
> Every claim carries `repo/file:line`, a prior-report section, or a URL.
> Status labels: `[now]` shippable on Claude-only code, `[M4]` needs the
> handover epic, `[roadmap]` needs a new adapter. No fidelity above what
> fixtures prove may be marketed (Prior-09 §H9; FCUR F3.2).

## 0. Verdict — what pass 1 under-weighted, got wrong, or missed

The maintainer sentence that defines this product is not "hand over
once" but: switch "as many times I want and switching back and forth
as needed" (`docs/expected-worfklow.md:7`), with "all the chat
history, context, rules and guidelines" working
(`docs/expected-worfklow.md:6`), across "same tool, sometimes …
multiple subscriptions from same AI lab"
(`docs/expected-worfklow.md:5`). Prior work models one directed hop
plus a suggested return; the product needs an unbounded, cyclic,
bidirectional switching engine. Six gaps follow.

| # | Gap | Prior position | This file |
|---|---|---|---|
| G1 | One hop, not a cycle | SPEC return is one click, delta-primed (`docs/SPEC.md:288`); Prior-09 lineage is a linear `prev_handover` chain (`Prior-09 §H7`) | §2 Switch Graph: cycles, N-hop, fork/merge (§2-§6) |
| G2 | Return = copy-back | Return "resumes the original session primed with a delta" (`docs/SPEC.md:288`) — mechanism unspecified; v0 code only copies forward (`crates/juggler-app/src/commands.rs:1088-1194`) | §6: switch-back resumes the *original native session*, never a re-copy; delta is an injected turn, not a transplant |
| G3 | No divergence semantics | Prior-09 F-H-11 names "return divergence" in one table row (`Prior-09 §H4`) with no merge rule | §6.2 three-way reconciliation on message-uuid sets; both-progressed is the *normal* case, not an exception |
| G4 | Per-artifact transfer is coarse | Prior-09 §H2 inventories Claude JSONL contents; SPEC §7.2 step 4 lumps rules/memory/todos/diff into one sync flow (`docs/SPEC.md:289`) | §3 per-pair, per-artifact matrix: Copy / Translate / Regenerate / Drop, with a score per cell |
| G5 | Wizard is the only door | Prior parts specify 3-step vs 7-step wizards and never reconcile them (critic C4, `docs/review/deep/13-critic-gaps-and-crosschecks.md:148-156`) | §8 decision tree: when 1-click is *correct*, when the wizard is mandatory; resolves C4 by making step count data-driven |
| G6 | Cross-tool pairs are one tier | SPEC collapses every cross-harness move to T3 replay (`docs/SPEC.md:273`); ZCode target capped without a path up (`docs/SPEC.md:282`) | §3.3 pair matrix differentiates Claude↔Codex, Claude↔ZCode, Codex↔ZCode and roadmap pairs; §4 pack is the shared substrate that lets a pair *graduate* |

Two prior claims are wrong, not merely thin, and are corrected here:

- **"T1/T2 are effectively indistinguishable from having started on
  the target account"** (`docs/SPEC.md:276`). False for T2 as built:
  v0 copies exactly one file chosen by newest mtime
  (`crates/juggler-app/src/commands.rs:1026-1033,1120-1146`), dropping
  sibling subagent transcripts and file-history that live beside it
  (`docs/research/claude-code.md:40,44`; Prior-09 §H2). Until the
  §10 fixture matrix passes for a pair, that pair's ceiling is the
  §3 score, not "lossless" (Prior-09 §H6 rule (a); FCUR F3.4).
- **"Running sessions are never switched underneath the user"**
  (`docs/SPEC.md:286`) is used to justify switch = *new* session only.
  The maintainer workflow switches the *work*, mid-task, when quota
  dies mid-flight (`docs/expected-worfklow.md:5`;
  `docs/PROPOSAL.md:23`). The engine therefore needs a defined
  mid-task freeze protocol (§5.2); "start the next session elsewhere"
  does not satisfy the example workflow.

## 1. Terminology — maintainer language mapped to the domain

Maintainer: "we should call them providers, accounts, and instances
of the tool" (`docs/expected-worfklow.md:11`). Adopt that language in
all user-visible copy. Internal types keep their names; the mapping
below is normative for UI strings, warnings, and this file.

| Maintainer term | Meaning for the user | Existing domain term (repo) | Conflict flag |
|---|---|---|---|
| **Provider** | The lab + tool family you subscribe to (Claude, Codex, ZCode; later Gemini, OpenCode) | `HarnessId` — `"claude-code" \| "codex" \| "zcode"` (`crates/juggler-core/src/model.rs:47-50`; `docs/SPEC.md:91`) | ⚠ `Profile` is *not* a provider. Vendor code also uses "provider" for model endpoints (ZCode `provider_config.json`, `docs/research/zcode.md:58`); UI must say "Provider (tool)" on first use per screen |
| **Account** | One subscription/login under a provider (e.g. two Claude Max subs) | `Account { harness, label, plan, state_dir }` (`crates/juggler-core/src/model.rs:142-157`) | ⚠ SPEC §3 sketch and code agree; PROPOSAL §6.1 calls the launchable unit a "profile" (`docs/PROPOSAL.md:75`) — user copy must never expose "profile" as a synonym for account |
| **Instance** (tool instance) | One running occurrence of a tool under one account on one project | `SessionRecord` + `InstanceKind::{Terminal,Desktop}` (`crates/juggler-core/src/model.rs:323-355,386-416`); ADR-0007 decision 1 (`docs/adr/0007-instance-centric-information-architecture.md:38-43`) | ⚠ Code names the instance record `SessionRecord`/`SessionId`; ADR-0007 already flags this drift. Keep `Session*` in Rust, render "Instance" in UI; a *session* in UI = the conversation inside an instance (ADR-0007:40-43) |
| **Profile** (internal only) | Account + settings overlay (model, env, MCP) used at launch | `Profile { account, settings_overlay, default_cwd }` (`crates/juggler-core/src/model.rs:170-184`) | ⚠ Highest-risk homonym: cc-switch and Codex both use "profile" for the *account* unit (Codex profiles are config files, `docs/research/codex.md:19-20`). Juggler UI: rename to "Launch settings" wherever user-facing; keep `Profile` in code/APIs |
| **Switch** (new verb) | Moving the *work* (conversation + project state) from one instance to another, any direction, any number of times | No type exists. v0 `session_handover` is Claude→Claude only (`crates/juggler-app/src/commands.rs:1062-1194`); `juggler-handover` is a doc-only stub (`crates/juggler-handover/src/lib.rs:1-13`) | New domain noun required (§2): `SwitchEdge`. "Handover" survives as the *one-hop user action*; "switching" is the unbounded graph behaviour the maintainer asked for |
| **Switch-back** (new verb) | Returning work to a previous instance/account, resuming *its own* native session | SPEC "Return / Resume lineage" (`docs/SPEC.md:288`); no code | Must not be implemented as a second forward copy (§6) |

Naming rule (must): every warning, ranking row, and lineage label in
§7-§9 uses Provider/Account/Instance. Any string that says "profile"
to mean an account, or "session" to mean a running tool, fails review.
This extends ADR-0007's instance-centric IA
(`docs/adr/0007-instance-centric-information-architecture.md:29-58`)
from navigation to the switching vocabulary itself.

## 2. The Switch Graph — the core model prior work lacks

SPEC lineage is a chain: "pack lineage (A→B→A chains)"
(`docs/SPEC.md:302`) and Prior-09 stores `prev_handover`
(`Prior-09 §H7`). A chain cannot represent what unlimited switching
actually produces: A→B→A→C→B, a fork where both A and B kept working,
or a return after three hops. Chains force the second return to
pretend the first never happened. Replace the chain with a graph.

### 2.1 Nodes, edges, and the work identity

- **Work item** (new): the user's continuing task on one project.
  Anchored to `ProjectRecord`, the juggling anchor keyed by canonical
  root (`crates/juggler-core/src/model.rs:367-384`;
  `docs/adr/0007-instance-centric-information-architecture.md:31-37`).
  A work item survives every switch; instances come and go under it.
- **Node** = one *instance-epoch*: an instance (`SessionRecord`,
  `crates/juggler-core/src/model.rs:389-416`) running under one
  account from launch/resume until it stops, is frozen, or hands off.
  Resuming the same native session after switch-back creates a new
  epoch node pointing at the *same* `native_id` — that pointer is
  what makes switch-back a resume, not a copy (§6).
- **Edge** (`SwitchEdge`, new): one executed switch. Fields:
  `id, work_id, from_epoch, to_epoch (null until launch succeeds),
  from_account, to_account, from_provider, to_provider, mechanism
  (native-resume|transplant|replay|pack), pack_id (§4), fidelity_shown
  (§3.4), artifact_manifest_sha256, state (§9), created_at, actor`.
  This generalises Prior-09's `handovers` table (`Prior-09 §H7`):
  keep its columns, drop the assumption that `prev_handover` is the
  only parent — an edge may have **two** parents after a merge (§6.2).
- **Invariant S-G1:** edges are append-only; a failed or undone switch
  adds an edge/state, never rewrites history (extends Prior-09 §H7
  "not history rewriting"). Undo = a compensating resume edge.
- **Invariant S-G2:** the graph is per-project. Cross-project switches
  are exports (T4 pack, `docs/SPEC.md:274`), not edges, because `cwd`
  is baked into every native format (Claude mangled project dir,
  `docs/research/claude-code.md:40`; Codex `session_meta.cwd` absolute,
  `docs/research/codex.md:41,46`).

### 2.2 What "unlimited" formally requires

1. **Idempotent re-entry:** switching A→B→A must not duplicate
   messages. Claude dedupe is first-wins by message `uuid`
   (`crates/juggler-adapters/src/claude_transcript.rs` per Prior-09
   §H7.1); every mechanism in §3 must state its dedupe key (uuid set,
   rollout `session_id`, ZCode `sess_*` id) or it is not cycle-safe.
2. **No hop-count state:** no field may assume depth ≤ 2. Token cost,
   however, *does* grow with hops for replay (§7) — the graph records
   cumulative replay tokens per work item so the UI can say so.
3. **Any node can be a source again** if its native session still
   exists on disk, including `HandedOver` nodes
   (`crates/juggler-core/src/model.rs:205-207,238`). `HandedOver` is a
   *resting* state, not a terminal one — prior UI treated ended rows
   as action-less (FCUR F2.3); the graph requires Resume on every node.
4. **Fork is legal, merge is explicit:** two live epochs on one work
   item (user kept A running while B worked) is a fork node pair.
   The engine never auto-merges (§6.2); it *detects* and asks.

### 2.3 Storage delta (migration after Prior-09 V5/V6)

Critic C3 already rules Prior-09 owns migration numbering
(`docs/review/deep/13-critic-gaps-and-crosschecks.md:137-146`), so
this file does not re-specify V5/V6. Required *additions*, sequenced
after them: `work_items(id, project, title, created_at)`,
`epochs(id, work_id, session_ref, native_id, account, provider,
resumed_from_epoch)`, `switch_edges(… §2.1 …, parent_edge_a,
parent_edge_b)`, `artifact_manifests(edge, rel_path, sha256, bytes,
kind, disposition)`. `sessions.project` already exists
(`crates/juggler-core/src/model.rs:401-403`) as the work anchor key.
Acceptance: an A→B→A→C fixture chain renders as one graph with 4
epoch nodes and 3 edges, and re-opening it after restart is identical
(storage is WAL SQLite, `docs/SPEC.md:22`).

## 3. Switch matrix — per pair, per artifact

Dispositions: **Copy** = byte/move of native artifact, no semantic
change. **Translate** = format conversion with a defined mapping.
**Regenerate** = target rebuilds it (replay turn, re-derived summary,
reconnected MCP). **Drop** = does not carry; must appear on the
carry-list (Prior-09 §H5). "Translate" without a written mapping in
the adapter is marketing, not engineering (Prior-09 §H1, H-1).

### 3.1 Artifact inventory (what can even be asked to transfer)

| Artifact | Native home (evidence) | Notes |
|---|---|---|
| Chat transcript | Claude `projects/<mangled>/<uuid>.jsonl` (`docs/research/claude-code.md:40-41`); Codex `sessions/**/rollout-*.jsonl` (`docs/research/codex.md:39-42`); ZCode SQLite `session/message` (`docs/research/zcode.md:63`) | The conversation itself |
| Context summary / compaction state | Claude `compact_boundary` lines (`docs/research/claude-code.md:45`; Prior-09 §H2.1) | Post-compact live context ≠ raw file |
| Rules & guidelines | `CLAUDE.md` / `AGENTS.md` / `.zcode/AGENTS.md` / `GEMINI.md` (`docs/SPEC.md:289`; `docs/research/landscape-and-stack.md:45-48`) | AGENTS.md is the interchange form |
| MCP configuration | Claude user MCP in `~/.claude.json`, project `.mcp.json` (`docs/research/claude-code.md:62`); Codex `[mcp_servers]` (`docs/research/codex.md:18`); ZCode `mcp.servers` (`docs/research/zcode.md:26`) | Config translates; live MCP *processes* never transfer (Prior-09 §H2) |
| Skills / commands / agents | `~/.claude/skills|agents|commands` (`docs/research/claude-code.md:63`); ZCode `~/.zcode/skills` (`docs/research/zcode.md:26`); Codex plugins/agents (`docs/research/codex.md:18`) | Mostly files; formats differ |
| In-session todos / plan | Unverified inside Claude JSONL (Prior-09 §H2); ZCode has a `todo` table (`docs/research/zcode.md:63`) | Treat as Drop until a fixture proves otherwise |
| Git state & worktree | Session stores `cwd`/branch (`docs/research/claude-code.md:41`); parallel work needs worktrees (FCUR F1.1; `docs/research/landscape-and-stack.md:56-60`) | Shared via the checkout, not the transcript |
| Uncommitted diff | Working tree on disk; SPEC pack carries `diffs/` (`docs/SPEC.md:303`) | Belongs to the *checkout*, so it follows the project, not the account — unless worktrees diverge |
| Terminal env & settings overlay | `SettingsOverlay { model, env }` (`crates/juggler-core/src/model.rs:159-168`); per-account env isolation (`docs/SPEC.md:128-136`) | Model names do not translate 1:1 across providers |
| Subagent state | Inline `isSidechain` lines carry; sibling subagent transcripts do not (Prior-09 §H2; FCUR F3.4) | Live subagents/processes: always Drop |
| Credentials | Keychain/`auth.json`/`credentials.json` per account (`docs/SPEC.md:310`) | **Never** in any pack, edge, or manifest (guardrail, `docs/PROPOSAL.md:61-68`) |

### 3.2 Same provider, different account (incl. same lab, multi-sub)

The maintainer's most common case: "multiple subscriptions from
same AI lab" (`docs/expected-worfklow.md:5`). Mechanism: transplant
(SPEC T2, `docs/SPEC.md:272`) — same binary both sides, so format
compatibility is structural, not hoped-for (`docs/SPEC.md:281`).

| Pair | Chat | Summary | Rules | MCP cfg | Skills | Todos | Git/diff | Env/model | Subagents |
|---|---|---|---|---|---|---|---|---|---|
| Claude A→Claude B `[now, unproven]` | Copy* | Copy (raw) | Copy (same files) | Copy (per-dir config, `docs/research/claude-code.md:62`) | Copy | Drop? | Shared checkout | Copy overlay; model names identical | Inline Copy; siblings Copy* |
| Codex A→Codex B `[M4]` | Copy rollout (`docs/SPEC.md:281`) | Regenerate (no compact scar format verified) | Copy (`AGENTS.md`) | Copy (`config.toml` families, `docs/research/codex.md:18`) | Copy plugins | Drop? | Shared checkout | Copy; profile files per 0.134 semantics (`docs/research/codex.md:19`) | Drop (thread children unverified) |
| ZCode A→ZCode B `[roadmap]` | Blocked: SQLite clone, schema-coupled (`docs/SPEC.md:282`; tables `docs/research/zcode.md:63`) | Blocked | Copy | Copy JSON (`docs/research/zcode.md:59`) | Copy | Copy* if clone lands (`todo` table exists) | Shared checkout | Copy | Drop |

`*` = only after Prior-09 §H8 fixtures pass: sibling-file set
(H-F1), torn-tail freeze (H-F2), collision fork (H-F3). As shipped,
v0 is single-file/newest-mtime (`crates/juggler-app/src/commands.rs:1120-1146`)
and its honest label is Prior-09 D1, `F1 Transcript-only
(experimental)` (`Prior-09 §0`), regardless of the `TRANSPLANT` bit
currently advertised (`crates/juggler-adapters/src/claude_code.rs:111-120`).
Target-dir resolution must discover-don't-compute (Prior-09 §H2.2,
H-3): naive `replace('/','-')` at `commands.rs:1121-1122` collides
on dotted/dashed paths (`docs/research/claude-code.md:40-41`).

### 3.3 Different provider (cross-tool), by pair

Mechanism: replay (SPEC T3, `docs/SPEC.md:273`) via the §4 pack, or
pack-only (T4, `docs/SPEC.md:274`) where no priming path exists.
Vendor precedent that replay is legitimate: Codex ships `/import`
of Claude Code/Cursor chats (`docs/research/codex.md:63`;
`docs/SPEC.md:276`). No production Claude↔Codex↔ZCode transcript
converter exists anywhere (landscape finding,
`docs/research/landscape-and-stack.md:54`) — Juggler's converter *is*
the product; each mapping below must land as an adapter function
with golden tests (EPICS M4-1/M4-4, `docs/plan/EPICS.md:103-106`).

| Pair (both directions unless noted) | Chat | Rules | MCP cfg | Skills | Todos | Env/model | Ceiling today |
|---|---|---|---|---|---|---|---|
| Claude ↔ Codex `[M4]` | Translate→Replay: IR turns + distilled tool outcomes (parser drops raw tool text today, Prior-09 §H2) | Translate: `CLAUDE.md`↔`AGENTS.md`; Claude reads AGENTS.md natively ≥v2.1.277 (`docs/SPEC.md:289`; `docs/research/landscape-and-stack.md:48`) | Translate: `.mcp.json`↔`[mcp_servers]` key mapping; unmapped keys listed, never silently dropped | Translate where format maps; else list as Drop | Drop→Regenerate as opening todo turn | Translate by *intent* (reasoning effort tier), never by name string | F1 (Prior-09 §H6) |
| Claude ↔ ZCode `[M4/roadmap]` | Replay from JSONL / from SQLite reads (`docs/research/zcode.md:76-77`); ZCode *target* has no verified prime path — pack-only until proven | Translate via AGENTS.md (`.zcode/AGENTS.md`, `docs/research/zcode.md:26`) | Translate JSON↔JSON (`docs/research/zcode.md:26,59`) | Copy files where dirs align (`docs/research/zcode.md:26`) | ZCode `todo` table → opening turn; reverse Drop | Translate by intent | F1 source / F0 target |
| Codex ↔ ZCode `[roadmap]` | Replay both ways via IR; neither side has a native import for the other | AGENTS.md both sides — Copy | Translate TOML↔JSON | List/Drop | Same as above | Translate by intent | F1/F0 as above |
| Any → Gemini CLI / OpenCode / future `[roadmap]` | Pack-only (F0) until an adapter passes §10; GEMINI.md joins the rules ring (`docs/research/landscape-and-stack.md:48`); these harnesses already run under peer orchestrators (`docs/research/landscape-and-stack.md:11,15`) | Translate via AGENTS.md ring | List/Drop | List/Drop | Drop | List | F0; SPEC adapter-first rule (`docs/SPEC.md:140`; `docs/PROPOSAL.md:35`) forbids promising more |

Direction matters and prior work hides it: "Claude→ZCode" and
"ZCode→Claude" have *different* ceilings (source readability vs
target primability). Every matrix cell, ranking row, and warning in
§7-§8 is therefore computed per **ordered pair**, never per provider
pair. This corrects SPEC §7.1, whose table is direction-blind
(`docs/SPEC.md:269-274`).

### 3.4 Fidelity score — per artifact, then capped composite

Prior-09 §H6 gives levels F0-F4 with user labels; keep the labels,
add the arithmetic the wizard and ranking (§7) need. Per artifact:
Copy=1.0, Translate=0.8 (0.6 if any key/field is listed-dropped),
Regenerate=0.5, Drop=0. Composite = weighted mean (chat .35,
rules .15, git/diff .15, todos .10, MCP .10, skills .05, env .05,
subagents .05), then **capped**: any Drop in chat/rules/git caps at
F1; a `compact_boundary` in the source caps at F2 (Prior-09 §H2.1);
an unproven pair (§10 cell not green) caps at F1 however it is
built. Map: ≥.95→F4/F3 label per Prior-09 §H6 ("Continues exactly"
only for same-account native resume), .75-.94→F3/F2, .45-.74→F1,
<.45→F0. The badge always ships with the carry-list (Prior-09 §H6
rule (a)); the number is internal, the label + list are the UI.
Worked example: Claude A→B done *properly* (siblings copied) scores
≈.93 → F3 "Same conversation, new account". As v0 ships (one file),
siblings Drop → ≈.86 but capped F1 until H-F1 passes — the cap, not
the mean, is the honesty mechanism (FCUR F3.4; critic K5,
`docs/review/deep/13-critic-gaps-and-crosschecks.md:196-201`).

## 4. Canonical Portable Context Pack — exact fields

SPEC §7.3 sketches a directory (`docs/SPEC.md:292-306`); Prior-09
§H5 makes it the preview artifact. Neither fixes the schema, so two
implementations would diverge. Fix it here. The pack is (a) the T4
export, (b) the staging artifact for *every* edge (even transplant
generates a manifest-only pack), and (c) the merge substrate (§6.2).

```
pack/  (format_version: 1)
├── PACK.md            # human brief: task, constraints, decisions, open todos, next steps (docs/SPEC.md:298)
├── conversation.md    # T3: full replay text; T4: distilled summary + last N turns (docs/SPEC.md:299-300)
├── state.json         # schema below — the canonical record
├── manifest.json      # artifact inventory: [{rel_path, sha256, bytes, kind, disposition}] (cf. Prior-09 §H7 session_files)
├── diffs/             # uncommitted diff + per-file patches, or EMPTY marker file (docs/SPEC.md:303)
└── rules/             # AGENTS.md canonical + per-provider renderings actually written
```

`state.json` fields (all required unless marked opt): `format_version`,
`pack_id` (uuid), `created_at` (epoch-ms, `crates/juggler-core/src/model.rs:74-88`),
`work_id`, `edge_id` (opt for standalone export), `source{provider,
account_label, profile_name, native_session_id, harness_version,
epoch_id}`, `target_hint{provider, account_label}` (opt),
`project{root, project_id}`, `git{branch, head_sha, dirty:bool,
diff_stat, worktree_path(opt)}`, `conversation{turn_count,
message_uuids_sha256, compacted:bool, first_turn_at, last_turn_at,
replay_tokens_est}`, `artifacts{rules:[…], mcp:[…], skills:[…],
todos:[…], drops:[{artifact, reason}]}` — every Drop from §3 appears
here, `fidelity{level, score, caps_applied:[…]}`,
`quota_at_creation{account, used_pct, confidence}` (confidence per
`crates/juggler-core/src/model.rs:286-297`), `lineage{parent_pack_ids:[…],
parent_edge_ids:[…]}` (a list — merges have two parents, §2.1),
`redactions:[{kind, count}]` (values never stored; Prior-09 §H5.4).
Unknown fields must be ignored by readers (format churn is the norm:
Claude ships weekly, `docs/PROPOSAL.md:132`; Codex broke profiles at
0.134.0, `docs/research/codex.md:19`). Generation is deterministic
from the Transcript IR; LLM "smart distill" stays post-v1
(`docs/SPEC.md:306`) and, when it lands, writes a *separate*
`summary_llm.md` so the deterministic record is never overwritten.

## 5. Switch execution — freeze, stage, verify, launch

### 5.1 Pipeline (every edge, every pair)

`Resolve → Preflight → Freeze → Snapshot → Stage → Verify → Launch
→ Confirm → Record` — generalises Prior-09 §H7's
`Staged → Verified → Launched → Confirmed`. Resolve uses the
session's own `transcript_ref`
(`crates/juggler-core/src/model.rs:314-321,415`), never a directory
guess (Prior-09 F-H-01). Preflight computes, in order: ordered-pair
mechanism + §3.4 score, compaction scan (Prior-09 §H2.1), target
quota gate (Prior-09 §Q6 hard filters), version-drift parse check
(Prior-09 F-H-06), git/worktree state (§5.3), and the §8 door
(1-click vs wizard). Any preflight failure stops *before* any byte
moves; the failure taxonomy and copy live in §9.

### 5.2 Mid-task freeze protocol (new — the quota-wall moment)

The defining switch happens while the source is mid-turn
(`docs/expected-worfklow.md:5`; in-flight work killed by walls,
`docs/PROPOSAL.md:23`). Protocol: (1) stop accepting the fiction
that a live, appending transcript can be copied — v0's raw
`std::fs::copy` (`crates/juggler-app/src/commands.rs:1143-1146`) can
tear the last line (Prior-09 F-H-02); (2) offer exactly two freezes:
**Finish this turn, then switch** (default; supervisor waits for the
turn boundary signal) or **Stop now and switch** (stop ladder,
`docs/SPEC.md:225`; partial turn is marked truncated in the pack,
never presented as complete); (3) snapshot = copy-then-parse-verify
the tail (Prior-09 H-F2). There is no third option "copy live and
hope", and SIGSTOP-style pausing is forbidden (SPEC already bans it
for corruption risk, `docs/SPEC.md:225`). Adapter trait gap: the
contract today has no export/prime/transplant methods at all
(`crates/juggler-adapter/src/adapter.rs:26-124` vs sketch
`docs/SPEC.md:173-177`); adopt Prior-09 §H1's optional
`HandoverSupport` sub-trait verbatim — this file adds no rival trait
(critic dedupe rule, `docs/review/deep/13-critic-gaps-and-crosschecks.md:251-277`).

### 5.3 Git, worktree, and uncommitted work

Git state follows the *checkout*, so same-checkout switches need no
transfer — but two live instances on one checkout on one branch is
the P0 hazard FCUR F1.1 names. Rules: same branch + both live →
warning + offer worktree for the target (worktree-per-instance is
the proven peer pattern, `docs/research/landscape-and-stack.md:56-60`);
uncommitted diff is snapshotted into `diffs/` at freeze (so a later
conflict has a base, §6.2) but *not* re-applied elsewhere — the
target opens the same tree; diverged worktrees → the switch carries
a named patch, applied only from the diff preview (Prior-09 §H5.3;
SPEC diff-preview rule, `docs/SPEC.md:289`). Zombie children of a
frozen instance (expected-workflow pain,
`docs/expected-worfklow.md:17-18`) are listed at freeze; cleanup is a
separate confirmed action, never a silent side-effect of switching.

## 6. Switch-back and divergence — return is a resume, not a copy

### 6.1 The return path (quota reset is the trigger)

When the original account's window resets, the app suggests — never
executes — the return (automation invariant: prepares/user executes,
`docs/SPEC.md:26,263`; ADR-0003 via Prior-09 D6). Execution:
(1) **resume the original native session** (`claude --resume <id>`,
`codex exec resume`, `zcode --resume`, `docs/SPEC.md:271`) — the
original transcript was never moved (copy-never-move,
`docs/SPEC.md:287`), so nothing is copied back; (2) inject *one*
delta turn: "While you were away" — turns, decisions, diff stat,
and todo changes from the away epoch(s), rendered from their packs;
(3) record a `switch-back` edge whose `resumed_from_epoch` points at
the original epoch (§2.1). Why this matters: a copy-back (the naive
reading of SPEC `docs/SPEC.md:288`) would fork the original
session's identity, break its `sessionId` chain, and double-count
usage on uuid re-ingest (Prior-09 §Q3 dedupe contract). Resume
keeps one native identity per epoch-line — that is what makes the
*N*-th switch-back identical in cost to the first. Claude v2.1.234+
can self-wait for reset (`docs/research/claude-code.md:53`); the
engine's value-add is therefore not babysitting one session but the
cross-account/cross-provider return with delta (Prior-09 §Q4).

### 6.2 Both sides progressed — three-way reconciliation

After switch-back is *offered*, the common case is divergence: the
user (or a still-running away instance) added turns on B while A
also advanced, or the worktree changed under both. Prior work stops
at naming this (Prior-09 F-H-11). Rule set:

- **Common ancestor** = the pack/manifest at the fork edge; its
  `message_uuids_sha256` + uuid set is the base (`state.json`, §4).
- **Classify:** away-only turns (uuids ∉ base, ∈ B) → delta turn,
  auto-includable; original-only turns → already native, nothing to
  do; *same-file edits on both sides* (diff-vs-diff on one path) →
  conflict list, never auto-merged; todo/state conflicts → listed.
- **User choices per conflict:** Keep original / Take away version /
  Keep both (away version saved as `…away-<date>` sibling + a turn
  pointing at it). Conversation conflicts are never text-merged:
  transcripts are trees (`uuid`/`parentUuid`,
  `docs/research/claude-code.md:41`), so "merge" = delta injection,
  not interleaving. The merge edge records both parents (§2.1) and
  the choice per conflict in `manifest.json`.
- If the away instance is **still running**, switch-back first
  offers freeze-or-fork (§5.2); resuming A while B runs is legal
  (parallel accounts are the founding use case,
  `docs/PROPOSAL.md:51`) but the UI must say both are now live on
  one work item — silence here is how wrong-account billing happens
  (Claude #87447, cited `docs/PROPOSAL.md:52`).

## 7. Destination ranking — cost/quota-aware, explainable, never silent

Adopt Prior-09 §Q6 wholesale as the base (hard filters, then
`0.45·headroom + 0.20·affinity + 0.15·fidelity + 0.10·costFit +
0.10·calm`, confidence-discounted, explained in words). Pass 1
under-weights three switching-specific costs; extend, don't replace:

- **Switch cost (new term, −):** replay tokens for this work item on
  this target (pack `replay_tokens_est`, §4) priced at the target's
  rates — a 200k-token replay onto a nearly-full 5h window can be
  worse than a lower-fidelity pack onto an empty one. Transplant and
  native resume cost ≈ 0 here; that asymmetry must be visible.
- **Return likelihood (new term, +):** if the source resets in
  < 60 min (reset timers, `docs/SPEC.md:257`), prefer targets that
  maximise *returnability* (same provider > replay-only), because a
  cheap hop now plus a lossless return beats a marginally better
  one-way hop. This is the bidirectional correction to §Q6, which
  scores only the outbound leg.
- **Cumulative hop cost:** the work item's spent replay tokens (§2.2)
  are shown on the ranking screen ("3 switches so far ≈ N tokens");
  hiding compounding cost across unlimited hops would make
  "unlimited" a billing trap (quota opacity is the #1 pain,
  `docs/PROPOSAL.md:23`).

Hard filters stay eliminative (logged-out, exhausted with no reset
before need-by, wrong mechanism for the request — Prior-09 §Q6).
Ranking *suggests*; execution is always the user's click
(`docs/SPEC.md:26`; `docs/PROPOSAL.md:63`). Auto-rotation on quota
is prohibited product-wide (`docs/PROPOSAL.md:63`;
`docs/research/landscape-and-stack.md:101-102`) — ranking copy must
never say "we switched you", only "you switched".

### 7.1 User warning copy (verbatim — implement these strings)

Terminology per §1. `{…}` = interpolated. Each string names what
does *not* carry; generic "some context may be lost" is banned.

| Situation | Copy |
|---|---|
| Transplant, proven pair | "Continues on {Account B} with the full conversation. Running tools and subagents stop; {Account A} stays resumable." |
| Transplant, unproven (v0 today) | "Experimental: only the main transcript moves. Subagent files and rewind history stay on {Account A}. {Account A} stays resumable." (Prior-09 D1) |
| Replay cross-provider | "Fresh start on {Provider B}: it re-reads the full conversation as text ({N} turns, ≈{T} tokens). Native todos and tool state don't carry — they're rebuilt from the story." |
| Compacted source | "This conversation was compacted. Early detail carries as text, not as live memory — {Provider B} may ask about early decisions." (Prior-09 §H2.1) |
| Target quota-tight | "{Account B} has ≈{X}% left (≈{TTE} at its recent pace). {Account C} has more room. Switching never happens automatically — your call." |
| Both sides progressed | "Both sides moved since the switch. {n} new turns and {m} file conflicts need your choice — nothing is merged silently." (§6.2) |
| Switch-back ready | "{Account A} has reset. Resume the original conversation there? Work from {Account B} arrives as one 'while you were away' update." (§6.1) |
| Same-branch parallel | "{Instance A} and {Instance B} are both live on branch {b}. Edits can collide — give {Instance B} its own worktree?" (§5.3) |
| Failure, source safe | "Switch didn't complete — {reason}. Nothing changed on {Account A}; you can resume it exactly where you left it." (§9) |

## 8. One click vs wizard — decision tree (resolves critic C4)

Critic C4: prior parts specify 3-step and 7-step wizards for the
same flow (`docs/review/deep/13-critic-gaps-and-crosschecks.md:148-156`).
Resolution: there is no fixed step count. The door is chosen by
*risk computed at preflight* (§5.1); the wizard renders only the
steps whose data is non-trivial.

```
Is target the usual account for this project AND mechanism =
  transplant/native-resume AND fidelity = F3/F4 AND no compaction,
  no conflicts, no listed Drops, target headroom ≥ Tight?
├─ YES → 1-CLICK: button "Switch to {Account} ▸" → confirm popover
│        (carry-list one-liner + cost) → execute. Undo toast 10s
│        ("Switched — Undo" = resume source, §9).
└─ NO  → WIZARD, steps included only if needed:
         1 Target (ranked rows, §7 — always)
         2 What's carried (carry-list + pack preview — always;
           Prior-09 §H5: no execution from an unseen preview)
         3 Conflicts (only if §6.2 finds any)
         4 Rules/diff (only if rules or worktree diverged, §5.3)
         5 Confirm (cost, fidelity label, target headroom — always)
```

1-click is therefore not a lesser wizard; it is the wizard with all
conditional steps proven empty. Telemetry distinguishes
`switch_oneclick` vs `switch_wizard{steps_shown}` so the team can
prove the fast path is earned, not assumed. Keyboard: `⌘⇧H` switch,
`⌘R` resume/switch-back (extends the shortcut debt noted FCUR F2.x;
exact bindings owned by the screen spec, not this file — dedupe
rule, critic `docs/review/deep/13-critic-gaps-and-crosschecks.md:259`).

### 8.1 Switch history / lineage graph UI

Surface: Project panel (the juggling anchor, ADR-0007
`docs/adr/0007-instance-centric-information-architecture.md:31-37`;
SPEC Projects view `docs/SPEC.md:325-330`). Rendering rules:
horizontal timeline per work item; epoch node = provider icon +
account label + instance kind badge + state chip (indicators the
maintainer demands, `docs/expected-worfklow.md:13-14`); edge =
labelled arrow (`transplant F3 · 12:04` / `replay F1 · ≈41k tok` /
`switch-back ↺`); fork = branching lines; merge = converging lines
with conflict-count badge; failed edge = dashed red, clickable to
the §9 reason. Clicking any node offers Resume (if its native
session exists), Switch from here…, and Export pack…. The graph —
not a session list — is where "how did this work travel?" is
answered; lists stay for Instances/Sessions (SPEC `docs/SPEC.md:331-348`).
Scale guard: collapse epochs older than the last fork/merge into
"+n switches" with expand; never render unbounded raw lists (the
unbounded-history flaw FCUR found in project session rows).

## 9. Failure and rollback — source is never the casualty

Extends Prior-09 §H4 (F-H-01…F-H-12); every row there stays in force.
Switching adds cycle-specific failures; each leaves the source
resumable and writes a failed edge (§2.1) — silent partial state is
the only unforgivable outcome.

| ID | Failure (stage) | Engine behaviour | Rollback |
|---|---|---|---|
| S-F1 | Wrong transcript resolved (Resolve) | Refuse if `transcript_ref` missing/ambiguous; never newest-mtime (v0 flaw, `commands.rs:1026-1033`) | Nothing moved; user picks the file |
| S-F2 | Torn tail / still writing (Freeze) | §5.2 two-choice freeze; parse-verify tail (Prior-09 F-H-02) | Resume source; no edge beyond `Failed{freeze}` |
| S-F3 | ID collision on target (Stage) | Fork: new native id + rewritten chain (Prior-09 F-H-03; SPEC `docs/SPEC.md:280`) | Fork is the rollback-safe path; original target untouched |
| S-F4 | Crash mid-copy (Stage) | Temp-dir stage → verify (sha256 vs manifest) → atomic rename (Prior-09 F-H-10) | Delete staged copy; source untouched by construction |
| S-F5 | Target launch fails (Launch) | Keep staged artifacts 24h for retry; mark edge `Failed{launch, reason}` | Resume source (T1); retry reuses the verified stage |
| S-F6 | Target quota dies during replay prime | Abort prime; charge is disclosed, not hidden (§7 switch cost) | Resume source; edge records tokens burned |
| S-F7 | Switch-back target session deleted/expired | Offer nearest surviving epoch or pack-primed fresh instance, labelled F0/F1 | Never fabricate a resume; graph shows the dead node greyed |
| S-F8 | Merge conflict unresolved (Return) | Work item stays forked; both epochs resumable | No forced merge; fork persists until the user chooses (§6.2) |
| S-F9 | N-hop loop (A→B→A→B…) with growing replay cost | Allowed — user-initiated — but cumulative-cost line (§7) turns amber; suggest transplant-capable target | Cost honesty, not a block (advisor informs, never blocks, `docs/SPEC.md:237`) |

Undo semantics (all doors): "Undo switch" within the toast window
= stop target epoch + resume source epoch + compensating edge
(Prior-09 §H7). After the window, the same actions remain available
from the graph (§8.1) — undo is a navigation, not a time bomb.

## 10. Acceptance criteria and test matrix per pair

Ship gate (extends Prior-09 §H9; blocks marketing, not dogfooding):
a pair may advertise its §3.4 label only when its row below is
green in CI fixtures *and* one live dogfood cell passes. Fixtures
H-F1…H-F8 (Prior-09 §H8) are prerequisites for every row.

| Pair (ordered) | Fixture tests (CI, fake harness + goldens) | Live dogfood cell | Advertised ceiling when green |
|---|---|---|---|
| Claude→Claude (diff account) | Sibling set, torn tail, collision fork, compacted→F2, pathological cwd (Prior-09 H-F1-F6); A→B→A→B 4-hop uuid-dedupe | 2 Max accounts, subagent-heavy repo; return on reset resumes original id | F3 (F2 if compacted) |
| Codex→Codex | Rollout copy + `exec resume` pickup; absolute-`cwd` path check (`docs/research/codex.md:46`) | 2 Codex accounts, same checkout | F3 (pending binary proof; today F1) |
| ZCode→ZCode | SQLite clone against 2 schema versions, or stays blocked (`docs/SPEC.md:282`) | 2 GLM plans, CLI-only (`docs/research/zcode.md:85`) | F1 until clone proven |
| Claude→Codex / Codex→Claude | IR golden round-trip; tool-outcome distill assertions; rules-ring conversion goldens (EPICS M4-4/M4-6, `docs/plan/EPICS.md:106-108`) | Continue a Claude task in Codex; next answer uses pre-switch context only | F1 |
| Claude→ZCode / ZCode→Claude | Replay-from-SQLite goldens; prime-path probe for ZCode target | Source-side F1 demo; target pack-open | F1 source / F0 target |
| Codex↔ZCode | IR goldens both ways | One cross continuation each way | F1/F0 as §3.3 |
| Any→Gemini/OpenCode (roadmap) | Pack-open smoke only; no replay claim | Pack imported manually | F0 |
| Switch-back (all proven pairs) | A→B→A: original `native_id` resumed, delta turn present, zero duplicate uuids; both-progressed merge fixture (§6.2) | Quota-reset return judged continuous by the user (PROPOSAL round-trip ≥80%, `docs/PROPOSAL.md:144`) | Same as outbound pair |
| Ranking/doors | §Q6 explanation snapshot tests + §8 door selection per risk fixture | — | n/a (gate for UI ship) |

Global acceptance: (1) source bytes hash-identical after every
test; (2) no edge executes without a user click — guardrail suite
(`docs/SPEC.md:26,441`; `docs/plan/EPICS.md:109` M4 gate); (3) the
UI never says "lossless/exactly" below F4 (Prior-09 §H9); (4) an
A→B→A→C→B graph survives restart (§2.3). Failure of any cell
*lowers that pair's advertised ceiling one level until fixed* —
the matrix is the capability flag's source of truth (Prior-09 §H8),
which also finally justifies the `TRANSPLANT` bit
(`crates/juggler-adapters/src/claude_code.rs:111-120`): it may stay
only for pairs whose row is green (Prior-09 D4).

## 11. Build order, risks, unresolved

Order (respects EPICS M4 scope, `docs/plan/EPICS.md:99-109` — this
file adds no milestone, it deepens M4): (1) honest relabel + sibling
set for Claude→Claude (fixes the shipped over-claim first, FCUR
F3.2); (2) packs + manifest for every edge (§4); (3) Switch Graph
storage + Project-panel timeline (§2.3, §8.1); (4) switch-back as
resume+delta (§6.1); (5) divergence handling (§6.2); (6) ranking
terms + doors (§7-§8); (7) cross-provider replay pairs in §10
order. Risks: native formats churn weekly (mitigation: per-version
parsers + fixture canary, `docs/SPEC.md:184,437`; `docs/PROPOSAL.md:132`);
ZCode surfaces undocumented (degrade to F0, never reverse-engineer,
`docs/SPEC.md:133`); replay cost surprises (mitigation: §7 cost
terms + S-F9). Unresolved, honestly: ZCode prime path and SQLite
clone safety; Codex cross-account resume against the real binary;
Gemini/OpenCode adapter shapes; whether Claude sibling-file copy
survives the next format change — each is a §10 cell to prove, not
a paragraph to write. What is *not* unresolved: the shape of the
answer — graph, pack, resume-don't-copy, ranked-never-silent — which
is what "as many times I want, back and forth"
(`docs/expected-worfklow.md:7`) demands of the design.





