# Research notes: competitive landscape, compliance & Rust desktop stack

> Compiled 2026-10-12 as input to the [proposal](../PROPOSAL.md) and [spec](../SPEC.md).
> Legend: **[V]** = verified by fetching the page · **[S]** = from search-result snippets (secondary) · **[I]** = inference/analysis · **[U]** = unknown.
> Star counts are point-in-time snapshots.

## 1. Landscape of existing tools (2025–2026)

| Tool | Platform / stack | Harnesses | Multi-account | Cross-tool handover | Usage dashboards |
|---|---|---|---|---|---|
| **claude-squad** [V] | TUI (Go), tmux + git worktrees; AGPL-3.0; ~8.6k★ | Claude Code, Codex, Gemini CLI, Aider, OpenCode, Amp | Partial (profiles/env vars only) | No — sessions stay bound to their agent | No |
| **cc-switch** [V] | Desktop GUI, **Tauri 2 + Rust + React 18 + SQLite**; MIT; v4.x; signed + notarized | 10 CLIs: Claude Code, Claude Desktop, Codex, Gemini CLI, Grok Build, OpenCode, OpenClaw, Hermes Agent, Pi, MiniMax Code | **Yes** — 90+ provider presets + "Accounts (Beta)" multi-login with explicit ToS warning in README | No — reads/resumes sessions per-tool, no handover | **Yes** — requests/tokens/cache/spend, trend charts, yearly heatmap |
| **claude-code-router** [S] | Local proxy/gateway (Node) + desktop app; ~37.6k★ | Claude Code & Codex → third-party models | N/A (routes endpoints, not accounts) | No | Partial |
| **ccusage** [V] | CLI; MIT; ~19k★ | ~20 CLIs incl. Claude Code, Codex, Gemini CLI, **ZCode**, Goose/Qwen/Amp | No | No | **Yes** — daily/session/5-h-block token + cost reports from local files |
| **vibe-kanban** [V] | Local web app: Rust backend + TS frontend; Apache-2.0; ~28.3k★; **"sunsetting"** per README | 10+ (Claude Code, Codex, Gemini CLI, Copilot, Amp, Cursor, OpenCode, Droid…) | No | No (kanban tasks, not chat handover) | No |
| **conductor** [V] | Native Mac desktop app; v0.90.0; commercial | Claude Code, Codex, Cursor, isolated workspaces | No | No | No |
| **crystal** [V] | Desktop, Electron; MIT; ~3.1k★; **deprecated Feb 2026** | Codex, Claude Code (parallel worktrees) | No | No | No |
| **claudia / opcode** [V] | Desktop GUI, Tauri 2 + React 18 + SQLite; AGPL-3.0; opcode ~22.4k★ (claudia continuation) | Claude Code only | No | No | Yes — cost/tokens by model/project |
| **aisw** [S] | CLI | Claude Code, Codex, Gemini CLI | Yes — swaps stored login state (work/personal) | No | No |
| **codex-profiles** [V] | macOS menu-bar app + CLI | Codex | Yes — named profiles → `~/.codex-<name>` | No | Partial (per-profile quota display) |
| **Gemini switchers** [S] | CLI scripts | Gemini CLI | Yes (some auto-rotate on quota — ToS-risky) | No | No |
| **ZCode managers** [U] | **None found.** Only ccusage parses ZCode usage | — | — | — | — |

Other notable: `ccmanager`, `agent-deck`, `YoanWai/agent-manager` — same shape as claude-squad; none with accounts/GUI/handover.

### Gap analysis

Criteria: (a) multi-account, (b) multi-harness, (c) desktop GUI, (d) usage dashboards, (e) cross-tool session handover.

- **cc-switch is the closest**: a+b+c+d ✅, **e ✗** — it manages *config*, not live sessions; no PTY/worktree orchestration, no stuck-session detection; its multi-account feature carries its own ToS warning.
- **claude-squad / vibe-kanban / conductor / crystal**: orchestration (b) only — no accounts, no dashboards, no handover; two are dead or sunsetting.
- **opcode**: c+d ✅, single-harness, single-account.
- **ccusage**: b+d ✅ (including ZCode), read-only CLI.

**Whitespace [I]:** no tool found combines user-owned multi-account switching + multi-harness live-session supervision + desktop GUI + quota dashboards + cross-tool handover. Each pillar is individually proven by an existing OSS project, but never together. That union is AI Juggler's defensible gap as of 2026-10 — and no ZCode manager exists at all.

## 2. Cross-tool session handover prior art

**Native resume flags (per-tool, mutually incompatible):**

- Claude Code: `--resume` / `-c`; sessions as JSONL under `~/.claude/projects/`.
- Codex: `codex resume --last` / `codex resume <session_id>`; rollout JSONL under `~/.codex/sessions/`.
- No common session schema exists — ccusage parses ~20 different local log formats, which is itself evidence of fragmentation [V][I].

**Rules-file conversion (the one real standard):**

- **AGENTS.md** (agents.md) — open instruction-file standard, "used by 60k+ open source projects" [S]; native to Codex and ZCode.
- **Claude Code reads AGENTS.md natively** — by default only "when you have no `CLAUDE.md` in your working directory or above it," requires **v2.1.277+**; configurable via the project-instructions setting (`claude-md-and-agents-md`); `@AGENTS.md` import syntax (max depth 4). https://code.claude.com/docs/en/memory [V]. Gemini uses GEMINI.md [S]. **Rules-file handover is essentially solved; chat-state handover is not.**

**"Context pack" / handoff patterns:**

- Matt Pocock's `/handoff` skill: "Write a handoff document summarising the current conversation so a fresh agent can continue the work" (https://www.aihero.dev/skills-handoff) [S].
- Claude Code `/export` (transcript) and `/compact` (summarize) are the primitives [S]. SUMMARY.md-style "digest for the next agent" is a common convention [I].
- **No production transcript converter between Claude Code ↔ Codex ↔ Gemini/ZCode formats was found [U]** — the dominant pattern is summarize → markdown → re-prompt, not raw transcript translation.

**Worktree + agent orchestration implementations:**

- **claude-squad** [V]: one tmux session per task, each in its own git worktree; preview/diff tabs; attach/detach; no cross-agent handover.
- **vibe-kanban** [V]: kanban issue → workspace (branch + terminal + dev server) → headless agent run → diff review → AI-written PR → merge.
- Anthropic now documents worktree-based parallelism officially (https://code.claude.com/docs/worktrees) [S].

**State transfer reality [I]:** no coding harness supports token-level/KV state transfer; industry practice is context compaction + summary-as-state ("context engineering"). Practical design: handover = transcript extraction → structured pack (task, decisions, open TODOs, diff, environment) → rules-file sync (AGENTS.md↔CLAUDE.md↔GEMINI.md) → launch the target tool in the same worktree primed from the pack.

## 3. Compliance deep dive

### (a) Anthropic

**Consumer Terms** (effective 2025-10-08, https://www.anthropic.com/legal/consumer-terms [V]): no sharing credentials/accounts; §3 bars access "through automated or non-human means" (except API-key access) and "bypassing any of our systems or protective measures." No explicit clause forbidding one person holding multiple owned accounts [V].

**Usage Policy** (https://www.anthropic.com/legal/aup [V]): prohibits "Coordinate malicious activity across multiple accounts, or create multiple accounts, to avoid detection, circumvent product guardrails"; "Circumvent a ban through the use of a different account"; "Resell, proxy, or otherwise provide access to Claude through unauthorized means, **including services that route requests through consumer subscriptions** or misrepresent the product or client being used."

**Commercial Terms** (https://www.anthropic.com/legal/commercial-terms [V]): §D.4 bars reselling and reverse engineering; §D.5: "Customer is responsible for all activity under its account."

### (b) OpenAI

**Terms of Use** (effective 2026-01-01, https://openai.com/policies/terms-of-use/ [V]): no sharing credentials; cannot "Interfere with or disrupt our Services, including **circumvent any rate limits or restrictions** or bypass any protective measures"; no "Automatically or programmatically extract data or Output." No explicit one-person-multiple-accounts prohibition [V] — the exposure is the circumvent-rate-limits clause if multiple accounts are used to evade quota rather than separate identities.

### (c) z.ai / Zhipu

International z.ai ToS URL could not be retrieved (404/JS-rendered) **[U]**. Governing documents [S]:

- **Coding Plan FAQ** (https://docs.bigmodel.cn/coding-plan/faq): plan is subscriber-exclusive; shared accounts = improper use; quotas restricted, serious cases affect the account.
- **Subscription agreement** (https://docs.bigmodel.cn/subscription-agreement): plan bound to a single account, licensed to one natural person; sharing/renting/lending/transferring prohibited; Coding Plan quota separate from GLM API; usage restricted to designated tools; 5-hour windows; Team Edition is the sanctioned sharing path.

### (d) How OSS tools frame themselves

- **cc-switch** [V]: README warns the multi-account beta "may violate the vendor's terms of service"; framed as config management; ships signed builds.
- **claude-code-router** [S]: local gateway routing Claude Code to third-party models — its subscription-through-proxy capability sits inside Anthropic's "route requests through consumer subscriptions" prohibition.
- **aisw** [S]: framed around work/personal identity separation, not quota multiplication — the safer posture.
- **ccusage** [V]: pure local-file analytics, zero vendor-interface surface.

### (e) Enforcement signals

- Anthropic alleged ~24,000 fraudulent accounts used by competitors for distillation [S]. Consumer side: accounts disabled for credential sharing; analysis holds that owning multiple Max subscriptions is not itself the violation — sharing/reselling is [S]; bans retain data-export + appeal paths [S].
- OpenAI: scattered deactivation reports on community.openai.com (incl. claimed false positives, 2026) [S].
- **No published enforcement action against an individual user-owned multi-account switcher was found [I] — absence of evidence, not evidence of absence.**

### (f) Recommended design guardrails [I, derived from the quotes above]

1. **User-owned accounts only** — never broker/pool credentials between humans.
2. **No quota-evasion features** — never auto-rotate accounts *because limits were hit*. Account selection must be identity-scoped (work/personal/project) and user-initiated. This is the brightest line.
3. **No login automation** — the user authenticates via each vendor's own flow; the app only selects which stored profile a child process uses.
4. **No proxying/MITM of vendor traffic** — derive usage from local session files, the ccusage pattern.
5. **Spawn unmodified real CLIs** (`claude`, `codex`, `zcode`) — no client misrepresentation.
6. **Local-only data** — keychain + local SQLite; telemetry opt-in.
7. **Surface vendor terms & quotas** at account-add; refuse team-share workflows; point users to vendor team plans when they want capacity.

## 4. Rust desktop stack in 2026

### Tauri v2 — mature, recommended

- Stable since Oct 2024; latest **v2.12.x** (~Oct 2026); no Tauri 3 exists [S]. Already used by direct comparables cc-switch v4.x and opcode [V].
- **Updater plugin** (https://v2.tauri.app/plugin/updater/ [V]): public/private keypair signing (`tauri signer generate`; verification "cannot be disabled"; private key via `TAURI_SIGNING_PRIVATE_KEY` at build); per-platform artifacts + `.sig` files; **static JSON feed**; **GitHub Releases works as the feed** — `tauri-action` generates `latest.json`; no delta updates (full-bundle downloads).

### Alternatives [I — no authoritative 2026 head-to-head found]

**egui** (immediate-mode; dense diagnostics, weak rich-text ecosystem), **Dioxus** (React-like, younger), **GPUI** (Zed's renderer; fastest, least documented externally), **Slint** (embedded-strong), **Flutter** (not Rust). **Decision:** for a data-dense dashboard + embedded terminals + charts, Tauri 2 wins — HTML canvas is the only path covering both xterm.js terminals and charts; egui is the fallback if a custom PTY canvas widget is acceptable.

### Frontend + charting

- **uPlot** [S, self-reported benchmark]: 3,600 points @ 60 fps → ~10% CPU / 12 MB RAM vs Chart.js 40%/77 MB and ECharts 70%/85 MB; ~50 KB. **Decision [I]:** uPlot for time-series quota charts; ECharts only if free heatmap/calendar UX is wanted.
- Framework [I]: React 18 (ecosystem, shadcn/ui — cc-switch/opcode's choice) vs Svelte 5 (smaller/faster) vs Solid — all viable; pick team familiarity. Push pre-aggregated chart data Rust→frontend via Tauri events.

### Crates (verified unless noted)

- **keyring v4.2** (2026-08): API split into `keyring-core` + per-platform store crates; store AI Juggler's own secrets here — not plaintext.
- **portable-pty v0.9** (2025-02): wezterm-project PTY traits; ConPTY on Windows. This is how the app spawns/supervises `claude`/`codex`/`zcode`.
- **notify v9.0-rc** (2026-08): FSEvents/inotify/ReadDirectoryChangesW + polling fallback; debouncing via `notify-debouncer-full`; use for tailing session JSONL.
- **sysinfo v0.39** (2026-07): processes/CPU/memory; keep one `System` instance, use `refresh_specifics`. Feeds stuck-session heuristics.
- **tokio, serde/serde_json**: defaults [I].
- **rusqlite vs sqlx** [V/S]: SQLite is single-writer either way; rusqlite is the de-facto embedded binding — pair with a dedicated writer thread. **Decision [I]:** rusqlite (cc-switch and opcode both chose SQLite).

### Process supervision pattern [I]

One tokio task per session → portable-pty child → output fan-out to (a) xterm.js via Tauri events, (b) ring buffer for stall detection (no-output + no-CPU via sysinfo + JSONL mtime via notify), (c) incremental token accounting. Restart with backoff; **never auto-switch account on stall or quota exhaustion** (guardrail 2).

### Embedded terminal

- **xterm.js + Tauri is proven** [S]; **Tabby** (Electron + xterm.js + node-pty) is the feature-complete prior art [S] — port that architecture, swapping node-pty for portable-pty.
- **wezterm** is a full GUI, not embeddable; its durable contribution is the PTY layer (portable-pty) [V/I].

### Code signing + notarization (indie OSS reality)

- **macOS:** notarization via `notarytool` requires Apple Developer Program (**$99/yr**). cc-switch ships signed+notarized as table stakes [V].
- **Windows:** Azure Trusted Signing: **Basic $9.99/mo (5,000 signatures)**, Premium $99.99/mo, $0.005 overage; individuals pass identity verification, **organizations must be ≥3 years old** — a blocker for new OSS foundations; traditional OV CAs ~$300–600/yr [S].
- Tauri's updater signature is independent of OS signing, but OS signing determines whether updates install without scary warnings [I]. Budget ~$220/yr covers both platforms for an individual.

## 5. Prior art patterns (brief)

- **VS Code profiles** [S/I]: named profiles = isolated settings/extensions in per-profile user-data dirs with a global registry; `code --profile <name>`; profiles survive updates because they live in user data. Lesson: model harness accounts as directory-scoped profiles so switches never corrupt live state.
- **Warp** [S]: Rust app with built-in self-updater; documented M friction — lesson: always ship an update-disable switch.
- **Ghostty** [S]: historically no built-in auto-update on macOS — contrast showing a self-updater is a genuine feature, not a given.
- **Docker Desktop** [S]: unprivileged GUI ↔ supervisor owning child lifecycles over a local socket; avoid privileged helpers entirely — AI Juggler needs none.

## Distilled decisions

1. **Stack:** Tauri 2 + React (or Svelte) + uPlot + xterm.js; rusqlite (single writer), keyring v4, portable-pty, notify (debounced), sysinfo, tokio. Validated end-to-end by cc-switch and opcode.
2. **Differentiation = the union nobody ships:** cc-switch's accounts + claude-squad's worktree/PTY orchestration + ccusage's multi-harness local usage parsing (ZCode support is blue ocean) + a handover engine built on summarize→markdown-pack + AGENTS.md/CLAUDE.md conversion.
3. **Compliance as product design:** identity-scoped account selection, zero autonomous quota-rotation, zero login automation, zero traffic proxying, local-only data — the clause quotes in §3 are the review checklist. Stay a supervisor, never a middleman; avoid single-vendor roadmap dependence.
