# AI Juggler — Product Proposal

**Status:** draft for review · **Date:** 2026-10-12 · **Author:** Faruk Brbovic
Companion document: [Technical specification](./SPEC.md) · Evidence base: [research notes](./research/)

---

## 1. Summary

AI Juggler is a free, open-source, native desktop app (Rust + Tauri) for developers who run **multiple AI coding subscriptions and Providers** — Claude Code, Codex, ZCode, and later others — on one machine. Its thesis, in one line: **Start anywhere. Switch when you hit the wall. Never lose the thread — or leave a zombie behind.** One console for every Provider, Account, and Instance you juggle. It does three things no existing tool does together:

1. **Switch** — unlimited, bidirectional switching of the *work* (conversation, context, rules, project state) between Accounts and Providers: one click when the pair is proven, an itemised wizard when lossy; switch back resumes the original with a delta of the excursion. Quota walls *trigger* switching; they are not the product.
2. **See** — a right-hand Inspector for any selection (Provider, Account, Instance, Session, Project): usage, history, quality signals, owned processes, and the actions to manage them — honest fields only.
3. **Clean** — every process AI Juggler spawns is owned to the end: verified leftovers surface in a Cleanup Centre; automatic cleanup exists only as a per-Account opt-in on verified-owned processes, always audit-logged.

It is explicitly **not** another agent or Provider. It orchestrates the tools you already have installed — a supervisor, never a middleman.

## 2. Problem statement

Modern AI-assisted development means several subscriptions across several tools. The concrete pains (all verified in the [research notes](./research/), with issue links and reaction counts):

- **Parallel accounts are unsupported and hacky.** None of the three Providers natively supports running two paid accounts at once as of Oct 2026 (Claude Code: [#18435](https://github.com/anthropics/claude-code/issues/18435), 198 comments; Codex: [#20500](https://github.com/openai/codex/issues/20500), 145 reactions; ZCode: no docs at all). The community answer is hand-rolled shell aliases that export `CLAUDE_CONFIG_DIR` / `CODEX_HOME` per terminal — fragile, invisible, and easy to bill to the wrong account.
- **Quota is opaque and runs out mid-task.** Claude's 5-hour/weekly windows, Codex's 5-hour/weekly allowances, and ZCode's credit pools are the #1 complaint category on all three trackers (724 GitHub issues mention Claude's "5-hour limit"; Codex's #28879 has 560 reactions). `/usage`-style commands are local-only, per-tool, and invisible until you hit a wall mid-task — and hitting the wall can kill in-flight work (Claude #98299, Codex #21073).
- **Sessions are silos.** Each tool keeps its own transcripts in its own format (Claude/Codex: per-project JSONL; ZCode: SQLite + JSONL rollouts). There is no way to see all running sessions in one place, no stuck detection, and no sanctioned way to continue a conversation on another tool or account. Cross-tool handover today = copy-pasting context into a fresh terminal and hoping.
- **Everything above is manual.** The current "stack" for a multi-subscription developer is aliases + direnv + a pile of terminal tabs + ccusage in a cron + sticky notes.

## 3. Goals & non-goals

### Goals

- **G1 — One place to launch and manage** every Provider/Account you own, Instances running in parallel without interference.
- **G2 — Know before you go:** accurate, visual, per-Account usage across all Providers — window clocks and (user-entered) subscription clocks — with alerts before walls are hit.
- **G3 — Never lose momentum:** user-initiated, unlimited, bidirectional switching between Accounts and Providers, with a portable context pack, switch-back that resumes the original, and lineage visible in a Switch Graph.
- **G4 — See everything:** a live session board — what's running, what's idle, what's stuck, what died.
- **G5 — Extensible:** adding a new Provider is an adapter, not a rewrite. Claude Code, Codex, ZCode first; the adapter API is public.
- **G6 — Trustworthy:** local-first, credentials never handled or stored by the app, signed self-updates, resilient by construction.
- **G7 — Machine-aware parallelism:** monitor system and per-session resources, warn *before* overload, recover from crashes, and recommend configurations that actually fit the machine.
- **G8 — Fleet:** link multiple computers over the network — every session, usage number, and alert from every connected machine in one console; onboarding a new machine is a pairing code.

### Non-goals (v1)

- ❌ Replacing or wrapping the model traffic of any Provider (no proxies, no gateways, no request routing — this is a hard compliance line, see §5).
- ❌ Automating logins, captcha, OTP, or any vendor authentication flow.
- ❌ Autonomous agent orchestration (no auto-choosing which account "deserves" a task, no background account rotation).
- ❌ Team/organization features (sharing recipes or pools between people is out of scope by design).
- ❌ A new chat UI for any Provider — the Providers own their UX; AI Juggler supervises.
- ❌ Touching anything the app doesn't supervise: no killing other applications, no OS/system tuning — alerts, advice, and one-click actions on its own sessions only.

## 4. Who it's for

- **Persona A — "The Multi-Maxer"** (the founding use case): runs 2–3 Claude Max subscriptions plus Codex plus a GLM Coding Plan. Wants them all running in parallel on one project or several, wants to see remaining quota at a glance, and wants to move a task to a fresh account when one runs dry — and back when it resets.
- **Persona B — "The Work/Life Splitter":** personal account and employer-provided account of the same tool; today juggling them is error-prone (wrong-account billing is a documented complaint: Claude #87447).
- **Persona C — "The Multi-Toolist":** deliberately uses different Providers for different strengths on the same repo; wants shared project context (rules files) kept in sync and the ability to continue a session on a different tool.
- **Persona D — "The Operator":** runs many long-lived sessions (overnight refactors, CI-like agent runs); needs stuck detection, health status, and lifecycle controls.

## 5. Compliance posture (read this first)

AI Juggler's differentiator includes being **the most conservative tool in this category**. Full clause-by-clause analysis is in [research/landscape-and-stack.md §3](./research/landscape-and-stack.md); the summary:

- None of the three vendors prohibits a person from holding multiple paid accounts they own. All three prohibit **sharing accounts**, **reselling/proxying access**, and **circumventing per-account limits or protective measures**.
- Therefore the app follows seven hard guardrails:
  1. **User-owned accounts only.** No pooling, brokering, or sharing between people.
  2. **No autonomous quota-driven rotation.** The app may *surface* "account A is exhausted" and a user may *choose* to start their next session on account B — but the app never auto-rotates accounts in response to limits. Per-account limits are always respected; selection is user-initiated and identity-scoped (work/personal/project). Preparation and staging can be automated; execution is always a user click.
  3. **No login automation.** Vendors' own `login` flows are triggered and nothing more.
  4. **No traffic interception.** Usage is computed from local session files (the ccusage pattern); the app never proxies or MITMs vendor traffic.
  5. **Spawn real, unmodified CLIs** with per-recipe environment isolation. No client misrepresentation.
  6. **Local-first.** No cloud, no account, telemetry strictly opt-in.
  7. **Terms surfaced in-product** when an account is added, with a pointer to each vendor's team plans for capacity needs.
- Residual risk is disclosed honestly: multi-account use is *tolerated-but-unaddressed* by vendors today; policy can change. The app is architected so that every feature degrades gracefully if a vendor tightens terms (e.g., usage dashboards remain valuable for a single account).

## 6. Product concept

### 6.1 Providers, Accounts & parallelism (terminology per ADR-0008)

A **launch recipe** = Provider + Account + settings overlay (model, permissions, MCP servers, env). The app materializes each Account as an isolated Provider state directory (`CLAUDE_CONFIG_DIR` for Claude Code, `CODEX_HOME` for Codex, per-account `HOME` for ZCode — the mechanisms research verified as working today) and launches the real CLI into it. Parallel Accounts of the same Provider no longer fight over one `~/.claude`/`~/.codex`/`~/.zcode`. Switching which account the *next* session uses is one click; running sessions are never switched underneath you.

### 6.2 The session board

Every running Instance across all Providers in one live view: status (running / waiting-on-you / stuck / crashed / finished), age, last output, token burn, quota headroom of its account. Stuck detection = heuristics over process CPU (sysinfo), output stream silence, and transcript-file mtime (notify). One-click actions: attach (embedded terminal or open in your terminal), pause/resume where supported, stop, hand over.

### 6.3 Usage dashboards

Per-account and cross-account views: current 5-hour window and weekly/credit pools (Claude Pro/Max rate-limit feed from statusline JSON; Codex estimated from rollout token records; ZCode computed exactly from its local `model_usage` table and the published credit formula), burn-down trends (uPlot), reset timers, and alerts at configurable thresholds — including *before* starting a long task: "this session will likely exhaust account A's window; account B has 80% headroom."

### 6.4 Switching (unlimited & bidirectional) & session sync

Moving work between tools and accounts — as many times as needed, back
and forth — with chat history, context, rules and project state intact:
one-click for proven same-Provider pairs, an itemised wizard for lossy
or cross-Provider moves (labelled beta with per-pair fidelity badges),
switch-back that resumes the original conversation plus one summarised
delta turn. Post-close **process ownership** is part of this pillar:
the dev servers, watchers and MCP transports an Instance spawned are
tracked, surfaced, and (only on opt-in per Account) cleaned — verified,
and audit-logged.

The goal is **actual session continuity — sessions and chats moved with their context — not just a generated hand-off prompt**:

- **Same tool, same account:** native resume — lossless.
- **Same tool, different account:** **session transplant** — the session's native artifacts are copied into the target account's state and resumed natively there; the conversation continues exactly where it left off, with history and in-session todos intact. (Claude and Codex support this today via transcript-path/session-id resume; both accounts share the same installed binary, so compatibility is guaranteed.)
- **Different tool:** **transcript replay** — the complete conversation is reconstructed and injected as the new session's opening context, so the target model re-reads the real chat, not a summary. (Vendor precedent: Codex itself ships a Claude-chat importer.)
- **Return:** when the source account's quota window resets, one click resumes the original session primed with a delta of what happened meanwhile ("while you were away").
- **Project sync:** rules files (AGENTS.md ↔ CLAUDE.md ↔ `.zcode/AGENTS.md`), Provider memory files, and todo lists aligned across the Accounts working a repo — diff-previewed, at a click.
- **Packs** remain as the export format and as the editable preview of exactly what any handover will carry over.

Every handover is prepared and staged automatically but executed only on a user click; the only autonomous behaviors in the entire app are passive ones — notifications, alerts, and suggestions.

### 6.5 Machine health & crash recovery

Parallel Instances are heavy, and AI Juggler is the one component that can see them all — plus the machine underneath:

- **Monitor:** system RAM/swap/CPU/disk/battery plus every supervised process tree (each CLI *and* all its children), with per-recipe historical footprints learned over time.
- **Warn before it hurts:** a launch advisor answers "can this machine take another session?" *before* you start it — OK / Tight / No, with reasons and what would make it safe. Runtime alerts cover memory pressure, swap thrashing, sustained CPU/thermal load, runaway processes, and filling disks.
- **Recover:** Provider crashes are detected and classified (OOM vs quota vs plain crash), with one-click recovery — native resume keeps the conversation (transcripts persist), or relaunch/hand-over. An opt-in per-recipe auto-restart (off by default) relaunches into the same recipe. If AI Juggler itself crashes, supervised sessions survive and are re-adopted on relaunch.
- **Advise:** recommendations from real data — how many parallel Instances this machine really supports per Provider, which recipes are heaviest, which knobs (e.g. concurrency limits) shave peak load.

### 6.6 Fleet: multiple computers, one console

Instances of AI Juggler link over the network so one console shows — and manages — every session on every machine you own:

- **Onboarding a machine is a pairing code**, not a config file: enable Link on the new machine, enter the code (or scan the QR) at your console, done. Machines are discovered automatically on the LAN (mDNS/Bonjour); remote machines join through your own VPN (Tailscale/WireGuard) — AI Juggler operates no cloud relay.
- **Standard, secure transport:** gRPC over TLS 1.3 with mutual certificate authentication; every instance has its own identity key; unpaired machines cannot connect.
- **What flows:** telemetry and control only — session status, resource stats, usage, alerts stream continuously; launch/stop/handover commands travel only on your click. Full transcripts move only when you execute a cross-machine handover. Vendor credentials **never** leave their machine.
- The board, dashboards, and alerts gain a machine dimension: filter by machine, aggregate usage across the fleet, route any machine's alerts to your console.

## 7. Competitive landscape & differentiation

See [research/landscape-and-stack.md](./research/landscape-and-stack.md) for the full table. Short version:

- **cc-switch** (closest comparable): multi-account config switching + usage dashboards, but config-only — no live sessions, no handover.
- **claude-squad / vibe-kanban / conductor / crystal:** session orchestration, but single-account, and half are dead/sunsetting.
- **opcode:** beautiful single-tool GUI.
- **ccusage:** read-only analytics CLI for ~20 tools (proof that local-file usage parsing is viable and tolerated).

**No tool combines multi-account + multi-Provider + desktop GUI + dashboards + handover.** AI Juggler is that union — and the only manager that will support ZCode at all. Guardrail difference: unlike claude-code-router (proxy — inside Anthropic's prohibition zone) or auto-rotating switchers (inside OpenAI's "circumvent rate limits" zone), AI Juggler never touches traffic and never rotates autonomously.

## 8. Risks & mitigations

| Risk | Severity | Mitigation |
|---|---|---|
| Vendor ToS changes against multi-account use | Medium | Compliance guardrails from day one; every feature valuable for a single account too; positioning as supervisor, not evader |
| Harness internals churn (session formats are de-facto APIs; Codex broke config layouts in 0.134.0, Claude ships weekly) | High | Adapter trait with per-version parsers + fixture tests; capability probing at runtime; pinned re-verification in CI |
| ZCode surfaces are undocumented (no public CLI docs, no quota API, plaintext credentials) | Medium | Rely on stable, observable surfaces (`--help`, app-server, SQLite, published credit formula); degrade gracefully; re-verify each release |
| Quota estimation wrong for Codex (no public API) | Medium | Label estimates as estimates (mirroring OpenAI's own language); calibrate from observed limits; show confidence |
| Updater trust on unsigned OS builds | Medium | Signed Tauri updater feed always; OS signing ($99/yr Apple, ~$120/yr Azure) budgeted for v0.1 |
| Scope creep into "yet another orchestrator" | High | Non-goals enforced in design reviews; adapter API is the only public extension point for v1 |
| Fleet expands attack surface | Medium | mTLS-only with per-instance identities, code-based pairing, no anonymous mode, LAN/VPN scope, credentials never over the link; loopback multi-instance tests in CI |
| Fleet protocol churn across app versions | Medium | Versioned protobuf contracts + breaking-change lint; mixed-version degradation matrix tested |

## 9. Success metrics (v0.x)

- A new user goes from install → two Claude accounts running in parallel in **under 5 minutes** without reading docs.
- A second machine joins the fleet in **under 60 seconds** with a pairing code and zero configuration files.
- Handover round-trip (A → B → back on A) preserves task continuity as judged by the user in ≥ 80% of tries.
- Zero incidents of the app writing to Provider state beyond its own managed account directories.
- Stuck-session detection precision/recall ≥ 90% on the fixture corpus.

## 10. Review status & remaining questions

Decisions closed in review (spec §15): React 18 + TypeScript UI · embedded terminal in M1 · MIT license · `ai-juggler`/`juggler-*` naming · Claude Code first · macOS-first with signed+notarized builds (Apple Developer account via Vuxtra) · strict semver with `0.x` betas on a beta channel and the stable channel opening at 1.0 · fleet over gRPC/mTLS with mDNS pairing, no cloud relay.

Remaining, non-blocking:

1. Fleet wire format detail (protobuf-first vs app-server-style JSON-RPC) — decided at the M5 transport spike.
2. Whether the stable update channel opens earlier than 1.0 — revisit at v0.3.
3. Cross-machine T2 transplant constraints (same binary version, same checkout path) — validated during M6/M7.
