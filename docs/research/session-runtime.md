# How AI Juggler runs the sessions it starts: terminal, headless protocol or SDK

Research date: 2026-10-25
Author: research agent (read-only on the repo and the Mac; nothing installed; no credentials, Keychain or process environments read; no `claude` or `codex` run against an account, only `--help` and `--version`, plus Codex's offline schema generator with a scratch `CODEX_HOME`)
Evidence rules: `docs/research/EVIDENCE-STANDARD.md`. Labels: **VP** = Verified-primary, **VS** = Verified-secondary, **NE** = Negative-evidence, **I** = Inferred, **U** = could not verify. Every source was accessed on **2026-10-25** unless a row says otherwise. Vendor docs pages show no dates; where a wording's age matters, the Wayback snapshot dates are given.
Decision it feeds: ADR-0022 (Proposed) and D-40.
Versions on this Mac: Claude Code **2.1.302** (npm latest 2.1.310); codex-cli **0.163.0** (stable 0.164.0, 2026-10-23); Claude desktop app 2.42304.2 embedding Agent SDK 0.3.299; TS Agent SDK latest 0.3.310 (2026-10-23); Python `claude-agent-sdk` 0.2.174 (2026-10-24); `@openai/codex-sdk` 0.164.0; Python `openai-codex` 0.163.0.

---

## 0. The answer in one page

**The question** (the maintainer): should AI Juggler run the sessions it starts through the vendors' SDKs or protocols, so that it can do much more inside its own window, rather than only through a terminal it watches? And should it decide that now, for the long term, rather than for 0.2 alone?

**What decides it is different for each tool.**

- **Codex: yes, move to the protocol, which is the app-server.**
  - Codex's own terminal UI is already an app-server client. Since 0.157 the TUI runs on an app-server, and AI Juggler today passes `--no-daemon` so that server stays inside its own process tree.
  - OpenAI says in writing that local and open-source apps may use app-server with the user's ChatGPT sign-in.
  - App-server gives every feature on the list: live state, approvals and questions as typed requests, documented 5-hour and weekly limits with reset times, interrupt, steer, model, compact, MCP and skills status, and resume and fork.
  - The terminal stays one click away, because `codex --remote unix://…` attaches the real TUI to the same server.
  - So for Codex the "SDK" route is the right one, but not through the Codex SDK packages. The TypeScript SDK wraps `codex exec` and cannot answer approvals. The Python SDK auto-accepts approvals by default.
  - AI Juggler should speak app-server's JSON-RPC from Rust.
- **Claude: no bundled SDK; keep the terminal as the default and add a headless mode as a choice.**
  - Anthropic's Agent SDK page still says: "Unless previously approved, Anthropic does not allow third party developers to offer claude.ai login or rate limits for their products, including agents built on the Claude Agent SDK."
  - Since late August 2026 its legal page also says the rule doesn't "prevent an end user from signing in to the unmodified Claude Code binary with their own Claude subscription".
  - So the safe ground is the user's own installed `claude`, signed in through Anthropic's own flow and run unmodified, whether in a terminal or with `claude -p`.
  - The Agent SDK is not safe ground: it bundles its own Claude Code binary, it is explicitly the thing the note names, and Anthropic staff limited subscription use of it to "local development and experimentation".
  - The SDK also gives a Rust app nothing that the CLI's stream-json control protocol, which the SDK itself speaks, doesn't give. It would add Node (about 100 MB per architecture) plus a second 274 MB Claude Code binary to sign and notarize.
- **What Claude gets instead:**
  1. **The terminal session, made observable through Claude Code's own hooks and status line**, added per launch through a `--settings` file in AI Juggler's folder (ADR-0018). This gives exact Working, Asks you and Idle states, limits with reset times, and turn boundaries, at no policy cost.
  2. **A headless mode the user can pick per session**: AI Juggler speaks stream-json with the user's own `claude` binary from Rust. Questions and approvals are answered in AI Juggler, and **Open in terminal** resumes the same conversation in the TUI at a turn boundary. It starts as a labelled opt-in, not the default, until three things are known: Anthropic's answer to a written request ("unless previously approved" names that route), how `--bare` becoming the `-p` default treats a subscription sign-in, and how Anthropic's third-party-app detection bills these runs.
- **What stays as it is:** the read-only readers for sessions other apps run (the Claude app, `claude` in a terminal, Codex Desktop); the bridge (`juggler-agent`, ADR-0020) and the pool rules (D-28 to D-33); per-profile isolation (`CLAUDE_CONFIG_DIR`, `CODEX_HOME`); the PTY path, kept as a supported mode and the fallback for every tool.

**Confidence:**

| Finding | Confidence |
|---|---|
| Don't bundle the Claude Agent SDK, in Node or Python | High |
| Codex on app-server, with an attachable TUI | High on policy and capability; medium on the attach and multi-client mechanics until a spike on spare accounts |
| Claude: terminal plus hooks as the default, headless stream-json as an opt-in | Medium-high |
| The runtime trait shape in §6 | Medium; the spike will refine it |

---

## 1. What exists today (repo, read-only)

| Fact | Where | Label |
|---|---|---|
| Every session AI Juggler starts runs under `portable-pty` (`PtyHandle {writer, master}`); the only production caller of `write_input` is the user's keystrokes in xterm | `crates/juggler-supervisor/src/supervisor.rs`, `crates/juggler-app/src/terminal.rs` (via `pools-messaging-tasks.md` §2.1 on release/0.2) | VP (local) |
| `LaunchMode` has `Interactive`, `Headless {prompt}`, `Resume {session_id, delta_prompt}`, `Primed {prompt, read_dirs}`; `LaunchSpec {program, args, env, cwd, needs_pty}` | `crates/juggler-adapter/src/launch.rs` | VP (local) |
| `Capabilities::PROTOCOL_SERVER` is defined but **no adapter declares it** | `crates/juggler-adapter/src/capabilities.rs:23-24` | VP (local) |
| AI Juggler speaks **no** app-server JSON-RPC. It recognises the Codex app's `codex app-server` process and avoids the daemon: every Codex terminal launch passes `--no-daemon` (ADR-0010 ownership) | `crates/juggler-adapters/src/codex.rs:20-24,76`, `codex_live.rs:56-66` | VP (local) |
| Live state is read from the tools' own files: Claude's `sessions/<pid>.json` (`status`), the Claude app's `claude-code-sessions/…/local_<id>.json` status word, a transcript tail (64 KB) for pending asks; Codex's writer locks and rollout tails | `crates/juggler-adapters/src/claude_live.rs`, `claude_asks.rs`, `codex_live.rs`, `codex_turn.rs` | VP (local) |
| Codex limits come from rollout `token_count.rate_limits`; Claude limits from the Claude app's `plan-usage-history.json`, transcript `rate_limit` errors and `.claude.json` `cachedUsageUtilization`; D-39 option A (`claude -p /usage`) is built and off | `quota_ingest.rs:940-948`; decision log D-39 | VP (local) |
| The switching lab already drives Claude headless (`claude -p --session-id`) on the maintainer's lab accounts | `docs/tasks/E15-spec-gaps.md` AJ-E1539 | VP (local) |
| SPEC already planned "Control plane (phase 2): `codex app-server` JSON-RPC (thread/turn lifecycle, approvals)" with phase 1 on PTY | `docs/SPEC.md` §4.2 (line 312), decision 6 (line 762) | VP (local) |
| D-28's hard lines (release/0.2) include "AI Juggler never answers a tool's own permission prompts" | `git show release/0.2:docs/review/decision-log.md` | VP (local) |
| The 0.2 research ruled "Driving sessions through the Agent SDK with subscription logins: **Never**" (R10) and the Codex app-server a "Phase 5 spike" (R11) | `release/0.2:docs/research/pools-messaging-tasks.md` §2.4 | VP (local) |

---

## 2. Terms and policy: may AI Juggler do this with the user's own subscription?

This is the part that can decide the question, so the quotes are exact.

### 2.1 Anthropic (Claude)

| Claim | Source | Label |
|---|---|---|
| "Unless previously approved, Anthropic does not allow third party developers to offer claude.ai login or rate limits for their products, including agents built on the Claude Agent SDK. Use the API key authentication methods described in the [Quickstart] instead." Present on every Wayback snapshot from 2026-03 to 2026-10; "Please use" became "Use" around 2026-09 | https://code.claude.com/docs/en/agent-sdk/overview (also the Quickstart) | VP |
| The SDK is "A library that runs the Claude Code binary"; "Both the TypeScript and Python SDKs bundle a native Claude Code binary" | same; https://code.claude.com/docs/en/agent-sdk/quickstart | VP |
| Use of the Agent SDK "is governed by Anthropic's Commercial Terms of Service, including when you use it to power products and services that you make available to your own customers and end users" | Agent SDK overview | VP |
| "Advertised usage limits for Pro and Max plans assume ordinary, individual usage of Claude Code and the Agent SDK." | https://code.claude.com/docs/en/legal-and-compliance | VP |
| "**OAuth authentication** is intended exclusively for purchasers of Claude Free, Pro, Max, Team, and Enterprise subscription plans and is designed to support ordinary use of Claude Code and other native Anthropic applications." | same, *Authentication and credential use* | VP |
| "**Developers** building products or services that interact with Claude's capabilities, including those using the Agent SDK, should use API key authentication through Claude Console or a supported cloud provider. Anthropic does not permit third-party developers to offer Claude.ai login into their own applications, or to route requests through Free, Pro, or Max plan credentials on behalf of their users. Moreover, developers may not collect, store, or intermediate Claude.ai credentials or session tokens — sign-in to a Claude account must complete through Anthropic's own flow." | same | VP |
| "This does not restrict how customers provision and manage their own API keys or third-party inference provider credentials […] Nor does it prevent an end user from signing in to the unmodified Claude Code binary with their own Claude subscription, including where a platform hosts Claude Code as described under *Can customers offer Claude Code in their products?* above." | same | VP |
| *Can customers offer Claude Code in their products?*: "The Claude Code binary must not be modified. […] customers may not remove, disable, or restrict any authentication method built into it"; "Customers may not pay for, resell, or intermediate Claude usage on their end users' behalf. Each end user must authenticate with their own Anthropic API key, Claude subscription plan credentials, or 3P inference provider credential" | same | VP |
| Naming: "You can accurately say, in plain text, that your product has Claude Code preinstalled or that it runs Claude Code. But you can't use the Claude Code or Anthropic names or logos as part of your own product, feature, or company name…" | same | VP |
| How the wording moved: 2026-02-18/19 strict ("Using OAuth tokens obtained through Claude Free, Pro, or Max accounts in any other product, tool, or service — including the Agent SDK — is not permitted"); softened 2026-04-01..13 (the "including the Agent SDK — is not permitted" sentence removed); 2026-08-16..30 added "Moreover…", "Nor does it prevent an end user…" and the "offer Claude Code in their products" section | Wayback snapshots of the legal page (captures in the session scratchpad; not committed) | VS (our own comparison of archived copies) |
| Headless is the Agent SDK too: "This page covers using the Agent SDK via the CLI (`claude -p`)." | https://code.claude.com/docs/en/headless | VP |
| "`--bare` is the recommended mode for scripted and SDK calls, and will become the default for `-p` in a future release." "In bare mode, Claude Code never reads OAuth credentials or the system keychain." | same | VP |
| Staff, 2026-02-18: "Nothing is changing about how you can use the Agent SDK and MAX subscriptions!" and "We want to encourage local development and experimentation with the Agent SDK and claude -p. If you're building a business on top of the Agent SDK, you should use an API key instead." (Thariq Shihipar) | https://x.com/trq212/status/2024212378402095389 and …2024212380142752025 (read through the fxtwitter API) | VS |
| Help Center, update of 2026-10-07: "Claude Max and Team plans now include monthly API credits, which cover the Claude Agent SDK, claude -p, the Claude API, and Claude Managed Agents. You can still use the Claude Agent SDK, claude -p, and third-party apps with your subscription limits." An earlier update (2026-06-15) "paused the previously-announced changes" that would have moved SDK and `-p` usage off subscription limits | https://support.claude.com/en/articles/15036540 (checked against Wayback snapshots 2026-10-08, -12, -20) | VP |
| 2026-04-04: subscriptions "will no longer cover usage on third-party tools like OpenClaw", which need extra usage or an API key (Boris Cherny); third-party harnesses that spoofed Claude Code's client were blocked on 2026-01-09 | https://x.com/bcherny/status/2040206440556826908; https://venturebeat.com/technology/anthropic-cracks-down-on-unauthorized-claude-usage-by-third-party-harnesses; https://news.ycombinator.com/item?id=47633568 | VS |
| Anthropic classifies "third-party app" usage inside the real binary from a session's content (reports of `claude -p` runs with another product named in the system prompt getting "400 Third-party apps now draw from your extra usage, not your plan limits") | secondary reports (MindStudio 2026-05-03, codenote.net 2026-09-01) | U |
| Consumer Terms: no access "through automated or non-human means, whether through a bot, script, or otherwise", "Except when you are accessing our Services via an Anthropic API Key or where we otherwise explicitly permit it" | https://www.anthropic.com/legal/consumer-terms (effective 2025-10-08) | VP |
| Usage Policy: no "services that route requests through consumer subscriptions or misrepresent the product or client being used" | https://www.anthropic.com/legal/aup (page shows a version effective 2026-11-12) | VP |
| No published definition of "third-party harness" or of "offer … rate limits for their products" was found | searches of code.claude.com, support.claude.com, anthropic.com/news, 2026-10-25 | NE |

**How each case reads (I):**

1. **(iii) The interactive TUI in a PTY: allowed. Confidence: high.**
   - The binary is the user's own, unmodified, signed in through `/login`. That is "ordinary use of Claude Code" and exactly what "Nor does it prevent an end user…" protects.
   - AI Juggler never touches the token. It doesn't offer a login: the user signs in to Claude Code, not to AI Juggler.
2. **(ii) The user's own `claude` binary headless (`-p`, stream-json): allowed, with caveats. Confidence: medium-high.**
   - The same sentence covers it ("the unmodified Claude Code binary"). The Help Center lists `claude -p` and "third-party apps" as drawing on subscription limits today.
   - Caveat 1: Anthropic calls `-p` "the Agent SDK via the CLI", so the SDK note could be read onto a product built on `-p`. That reading matters most once the product does what the note names: offering "rate limits for their products".
   - Caveat 2: billing can move. The paused June change, the October credits and the third-party detection all show Anthropic treats `-p` and SDK traffic as its own category. A headless session may one day draw from a different pool than the TUI.
   - Caveat 3: `--bare` becoming the `-p` default would stop a plain `-p` run from reading the subscription sign-in. Whether a non-bare opt-out stays is **U**.
   - Caveat 4: "ordinary, individual usage". Rules that start turns on their own (D-29), and restarts, move headless use away from "individual" use.
3. **(i) A bundled Agent SDK: avoid. Confidence: high that this is the riskiest path.**
   - It is the case the note names. It ships Anthropic's binary inside AI Juggler rather than using the one the user installed. Staff limited subscription use to "local development and experimentation".
   - The one form that is clearly allowed is the user's own API key. The SDK path would then be an API-key feature, not the default way sessions run.
4. **A written approval is a real route.** "Unless previously approved" names it. Asking Anthropic in writing whether an open-source supervisor may drive the user's own `claude -p` on their own subscription is cheap, and an answer would settle caveat 1.

**Rules the sources imply for every Claude path:**
- Run the binary unmodified.
- Never collect, store, read or relay a claude.ai token: no `setup-token` harvesting, and no `CLAUDE_CODE_OAUTH_TOKEN` injection by the app.
- Sign-in always happens in Claude's own flow.
- Never spoof the client: no entrypoint or header claiming to be Claude Code's TUI or the Claude app.
- No proxying, and no account rotation around limits.
- Never name an AI Juggler feature "Claude Code".
- Never put another product's identity into a session's system prompt.

These match ADR-0003's guardrails and should be cited there.

### 2.2 OpenAI (Codex)

| Claim | Source | Label |
|---|---|---|
| "Codex app-server is the interface Codex uses to power rich clients (for example, the Codex VS Code extension). Use it when you want a deep integration inside your own product: authentication, conversation history, approvals, and streamed agent events." | https://learn.chatgpt.com/docs/app-server (developers.openai.com/codex/app-server 308-redirects here) | VP |
| "If you've built a local or open-source application using Codex app-server authentication, you can continue using it, though we recommend migrating to Sign in with ChatGPT so users have greater control over and visibility into their usage. […] App-server authentication has never been permitted for commercial or hosted services." (paragraph added between 2026-09-27 and -29, Wayback) | same, *Auth endpoints* | VP |
| "Use `clientInfo.name` to identify your client for the OpenAI Compliance Logs Platform. If you are developing a new Codex integration intended for enterprise use, please contact OpenAI to get it added to a known clients list." | same | VP |
| "If you are automating jobs or running Codex in CI, use the Codex SDK instead." | same | VP |
| `codex app-server` is "[experimental]" in `--help`; the docs: "The app-server command and WebSocket transport are experimental and aren't supported for production workloads." | `codex --help` 0.163.0; app-server page | VP |
| Sign in with ChatGPT plan usage: "available to open-source projects, personal projects that run locally, and selected private apps"; for app-server "configure it to send inference requests … using an OAuth access token authorized for the user's ChatGPT plan"; per-app weekly caps the user sets | https://developers.openai.com/cookbook/articles/sign-in-with-chatgpt; https://developers.openai.com/siwc/token-sharing-open-source/codex-app-server; help.openai.com/en/articles/20001542 (Wayback, Oct 2026) | VP / VS |
| Terms of Use: no sharing credentials; no "circumvent any rate limits or restrictions"; "Automatically or programmatically extract data or Output" (read as aimed at scraping) | https://openai.com/policies/terms-of-use/ (effective 2026-01-01, Wayback 2026-10) | VP |

**How it reads (I):**
- AI Juggler is a local, open-source, non-hosted app, so running `codex app-server` with Codex's own ChatGPT sign-in is **explicitly allowed. Confidence: high.**
- AI Juggler should:
  - set an honest `clientInfo.name` (`ai_juggler`), never `codex_vscode` or `Codex Desktop`;
  - keep sign-in in Codex's own flow, which is `codex login` today. `account/login/start` is also Codex's own flow, because the app-server hosts the callback, but it still needs the user's click;
  - consider Sign in with ChatGPT later, when it would give users per-app caps and visibility.
- If AI Juggler ever became paid or hosted, it would need OpenAI's partner path.

---

## 3. What each surface exposes

### 3.1 Claude: the stream-json control protocol and the Agent SDK

The Agent SDK is a client of the CLI's own protocol: both SDKs spawn `claude --output-format stream-json --verbose --input-format stream-json` and speak `control_request` / `control_response` lines on stdin and stdout. Anything the SDK can do, a Rust host speaking that protocol to the user's own `claude` can do.

| Need | What exists | Source | Label |
|---|---|---|---|
| Permission prompts answered by the host | `--permission-prompt-tool stdio` (what both SDKs pass; the CLI reference documents only an MCP tool name, so `stdio` is undocumented). The CLI sends `control_request {subtype:"can_use_tool", tool_name, input, permission_suggestions, title, display_name, description, tool_use_id, …}`. The host answers `allow` (with `updatedInput`, and `updatedPermissions` for "don't ask again") or `deny` (`message`, `interrupt`). `--permission-prompts host\|none` (≥2.1.259) is documented | SDK `sdk.d.ts` 0.3.310 l.228, 2571; Python SDK `types.py` 2281-2304, 2801-2813, `_internal/query.py` 586-700; https://code.claude.com/docs/en/agent-sdk/user-input; https://code.claude.com/docs/en/headless | VP (types and docs); the `stdio` value VS |
| Unanswered asks survive a reconnect | `pending_permission_requests` in the initialize response (≥2.1.268) | TS reference | VP |
| PermissionRequest hooks alongside a host | "For a call that reaches a `--permission-prompt-tool` or the Agent SDK's `canUseTool` callback, the hooks run alongside your host, and whichever decides first applies." | https://code.claude.com/docs/en/hooks (PermissionRequest) | VP |
| AskUserQuestion | Arrives as `can_use_tool` with `tool_name:"AskUserQuestion"`, `{questions:[{question, header, options:[{label, description}], multiSelect}]}` (1–4 questions, 2–4 options). Answered by `allow` with `updatedInput.answers` | user-input page | VP |
| MCP elicitation, dialogs | `onElicitation`; `request_user_dialog` (fails closed without declared kinds) | `sdk.d.ts` ~1803-1845 | VP (types) |
| Interrupt | `control_request {subtype:"interrupt"}` (+`cancel_queued`, ≥2.1.219), with a receipt; SIGINT ends a turn cleanly, SIGTERM exits 143 without a result | TS reference; headless page | VP |
| Resume, fork | `--resume <id>`, `--fork-session`, `--session-id`, `--resume-session-at` (headless only), `--no-session-persistence` | CLI reference; `sdk.d.ts` | VP |
| One transcript for both modes | Headless sessions write the same `projects/<cwd>/<id>.jsonl`. "Claude Code leaves sessions created with `claude -p` or the Agent SDK out of the session picker and out of `claude --continue`. You can still resume one by passing its session ID to `claude --resume <session-id>`." | https://code.claude.com/docs/en/sessions | VP |
| Two writers corrupt a session | "If you resume the same session in two terminals without forking, messages from both interleave into one transcript" | sessions page; desktop page | VP |
| Model, mode, effort mid-session | `set_model`, `set_permission_mode`, `apply_flag_settings`; `/model`, `/effort`, `/fast` accepted as prompt text in `-p` | `sdk.d.ts`; headless page | VP (types and docs) |
| MCP | `--mcp-config`, `--strict-mcp-config`; `mcp_status`, `mcp_set_servers`, `mcp_reconnect`, `mcp_toggle`; init lists `mcp_servers`, `mcp_server_errors` | `sdk.d.ts`; headless page | VP |
| Limits, live | `rate_limit_event`: `status allowed\|allowed_warning\|rejected`, `resetsAt`, `utilization` documented; `rateLimitType five_hour\|seven_day\|…` only in types | TS reference §SDKRateLimitEvent; `sdk.d.ts` 5893-5935 | VP (part documented, part types) |
| Account usage (the `/usage` data) | `get_usage`, exposed as `usage_EXPERIMENTAL_MAY_CHANGE_DO_NOT_RELY_ON_THIS_API_YET()` | `sdk.d.ts` ~3127 | VP (types; explicitly unstable) |
| Cost and tokens per turn | `result` message: `usage` (per turn), `total_cost_usd` and `modelUsage` (cumulative, client-side estimates) | https://code.claude.com/docs/en/agent-sdk/cost-tracking | VP |
| What it loads | The CLI in `-p` loads the user's settings, CLAUDE.md, rules, skills, commands, agents, hooks and plugins, exactly as the TUI does (the SDK's own default system prompt is minimal unless `preset:'claude_code'`, which is irrelevant when driving `claude -p` directly). `CLAUDE_CONFIG_DIR` relocates everything, "Useful for running multiple accounts side by side" | https://code.claude.com/docs/en/agent-sdk/claude-code-features; env-vars page | VP |
| Workspace trust is skipped headless | "`-p` or SDK session: Claude Code never shows the dialog and treats the folder as trusted, so hooks committed in a repository's `.claude/settings.json` run in a folder you've never trusted" | hooks page | VP |
| Slash commands headless | Custom commands and skills work as `/<name>` prompts; `/compact` (emits `compact_boundary`), `/clear`, `/context`, `/usage`, `/model`, `/effort`, `/mcp` (text), `/config key=value` work. Lost: pickers and dialogs (`/resume`, `/permissions`, `/plugin`, `/mcp` panel, `/config` UI, `/status` panel, `/memory`, `/hooks`, `/export`), `/login`, `/theme`, and the status line | headless page; skills page; `sdk.d.ts` `supportedCommands` | VP |
| How the SDK runs | TS ≥0.2.113: a per-platform native binary dependency (`claude-agent-sdk-darwin-arm64` 274 MB unpacked); host needs Node ≥18 (or Bun/Deno); version tracks the CLI ("SDK v0.3.191 bundles Claude Code v2.1.191"). Python bundles the CLI (macOS arm64 wheel 116 MB) | npm and PyPI metadata; TS reference | VP |
| Stability | No semver policy stated; near-daily releases; breaking changes in patch-style bumps (0.1.0 rename and settings default flip, later reverted; 0.2.113 native binary and `env` semantics; 0.3.142 MCP and session API). The **output** messages are documented on the headless page; the **control** protocol is not documented as a public contract, though the TS reference speaks to clients that "parse the wire protocol yourself" and tells them to feature-detect through `system/init.capabilities` | SDK CHANGELOGs; TS reference | VP |

**Claude Code's own supervisor (new and relevant).** Agent view and background sessions are Anthropic's own "headless with an attachable terminal".

| Claim | Source | Label |
|---|---|---|
| `claude --bg "<prompt>"` starts a full interactive Claude Code session with no terminal attached, run by a supervisor process; `claude attach <id>` opens it as the full TUI; `claude logs`, `stop`, `respawn`, `rm` | https://code.claude.com/docs/en/agent-view | VP |
| "`claude agents --json` is the supported way to read session state from outside Claude Code". It lists every live session (interactive and background) with `state working\|blocked\|done\|failed\|stopped`, `status busy\|waiting\|idle`, and `waitingFor` (`permission prompt`, `input needed`, …) | same, *Read session state from a script* | VP |
| The supervisor restarts a session that exits unexpectedly, survives sleep, moves idle sessions to new versions; it is per `CLAUDE_CONFIG_DIR` | same, *The supervisor process*, *Where state is stored* | VP |
| A permission prompt in a background session can be answered only by attaching; a peek reply doesn't answer it | same, *Peek and reply* | VP |
| No documented programmatic way to answer a permission or send a reply to a background session | same (command table) | NE |
| Whether `claude agents --json` itself starts the supervisor | — | U |

### 3.2 Codex: app-server

Schemas were generated offline from 0.163.0 (`codex app-server generate-json-schema --out … [--experimental]`, scratch `CODEX_HOME`): 105 client requests, 10 server requests and 85 notifications are stable; 172 and 11 with `--experimental`.

| Need | What exists | Source | Label |
|---|---|---|---|
| Transport | JSON-RPC 2.0 without the `jsonrpc` field, newline-delimited; `--listen stdio://` (default), `unix://[PATH]`, `ws://` (experimental); `initialize {clientInfo, capabilities}` then `initialized`, once per connection | app-server page; `codex app-server --help` | VP |
| Threads, turns | `thread/start` (`model`, `cwd`, `approvalPolicy`, `sandbox`, `config`, `developerInstructions`, `ephemeral`), `thread/resume`, `thread/fork`, `thread/list`, `thread/read`, `thread/compact/start`, `thread/revert`, `thread/name/set`; `turn/start`, `turn/steer` (`expectedTurnId`), `turn/interrupt`; `review/start` | generated schema; app-server page | VP |
| Live state | `thread/status/changed` (`notLoaded\|idle\|systemError\|active`), `turn/started`, `turn/completed` (`completed\|interrupted\|failed`), `item/started`/`item/completed` (authoritative), message and command-output deltas | same | VP |
| Approvals and questions | Server requests `item/commandExecution/requestApproval` (decisions `accept`, `acceptForSession`, `acceptWithExecpolicyAmendment`, `applyNetworkPolicyAmendment`, `decline`, `cancel`), `item/fileChange/requestApproval`, `item/permissions/requestApproval`, `item/tool/requestUserInput` (questions with options; marked EXPERIMENTAL in its comments), `mcpServer/elicitation/request`; then `serverRequest/resolved` | same | VP |
| Limits | `account/rateLimits/read` → `rateLimitsByLimitId` of `{primary, secondary: {usedPercent, windowDurationMins, resetsAt}, credits, planType, rateLimitReachedType, …}`; `account/rateLimits/updated` pushed "whenever a user's ChatGPT rate limits change"; `account/usage/read` | same | VP |
| Tokens | `thread/tokenUsage/updated {total, last, modelContextWindow}` | same | VP |
| Model, MCP, skills | `model/list`; `mcpServerStatus/list`, `config/mcpServer/reload`; `skills/list`, `hooks/list`; `config/read` with layers | same | VP |
| Sharing with the TUI | The TUI is itself an app-server client (resume picker calls `thread/list`); `codex --remote unix://PATH` "Connect[s] the TUI to a remote app server endpoint"; subscriptions are per connection ("If this was the last subscriber, the server unloads the thread") | openai/codex source at main (`tui/src/resume_picker.rs:2175-2196`); `codex --help`; app-server page | VP |
| Headless threads are visible in `codex resume` | app-server records source `vscode`; `codex resume` lists `cli` and `vscode` | `cli/src/main.rs:1327`, `tui/src/lib.rs:804` | VP (source) |
| What happens when two clients subscribe to one thread and an approval arrives (who is asked, who may answer) | — | U |
| `CODEX_HOME` isolation; `initialize` returns the absolute `codexHome` the server runs on | generated schema; `codex.md` §10 | VP |
| Stability | Labelled experimental, yet the VS Code extension, Codex Desktop and the TUI run on it; experimental methods need `capabilities.experimentalApi`; schemas "match that version exactly"; stables about weekly (0.163.0 on 10-15, 0.164.0 on 10-23) | app-server page; releases API | VP |
| Codex SDKs | TS `@openai/codex-sdk` spawns `codex exec --experimental-json` and has **no approval callback** ("command execution approval is not supported in exec mode"). Python `openai-codex` wraps app-server and **auto-accepts command and file approvals by default** (`client.py:833-838`) | openai/codex `sdk/typescript/src/exec.ts:95`, `codex-rs/exec/src/lib.rs` ~2112-2135, `sdk/python/src/openai_codex/client.py` | VP (source) |
| `codex mcp-server` | removed; integrators needing approvals and events are pointed to app-server | https://learn.chatgpt.com/docs/mcp-server | VP |

**A correction to the repo notes:** `docs/research/codex.md` §9 calls the `rateLimits` feed "undocumented … prohibited". `account/rateLimits/read` and `account/rateLimits/updated` are now documented app-server methods, so §9 needs an addendum. The rollout reader stays the source for sessions other apps run.

---

## 4. The options, feature by feature

Columns:
- **PTY** = today.
- **PTY+** = today plus the tool's own per-launch hooks and status line (Claude) or nothing new (Codex).
- **Driven** = headless over the protocol.
- **Hybrid** = driven, with the real TUI attachable (Codex), or switchable at a turn boundary (Claude).

### 4.1 Claude Code

| Feature | PTY (today) | PTY+ hooks | Driven (`-p` stream-json from Rust) | Notes |
|---|---|---|---|---|
| Working / Asks you / Idle / Done | Pid-file status plus a transcript tail, "probably" | **Exact**: `UserPromptSubmit`, `PermissionRequest` (immediate), `Notification` (`permission_prompt` after ~6 s, `idle_prompt`), `Stop`/`StopFailure` | **Exact**, from the stream (init, `can_use_tool`, `result`) | Hooks come from a per-launch `--settings` file (ADR-0018), and their command is `juggler-agent` |
| Answer questions and approvals in AI Juggler | No (go to the terminal) | **U**: a blocking `PermissionRequest` hook could carry the user's answer, but whether the TUI dialog shows while the hook waits is unverified; typing the choice into the PTY on the user's click is possible but fragile | **Yes**: `can_use_tool` (with the vendor's own "don't ask again" suggestions), AskUserQuestion, elicitation | The headline gain of driven mode |
| 5-hour and weekly limits with resets | Indirect (D-39 sources) | **Yes** for these sessions: a status line wrapper in the per-launch settings receives `rate_limits` (Pro/Max only), keeping the user's own status line output | **Yes**: `rate_limit_event` live (status, `resetsAt`, utilization); `get_usage` exists but is "DO_NOT_RELY" | D-39 stays the answer for sessions AI Juggler didn't start |
| Interrupt, stop | Ctrl+C keystroke; kill | Same | `interrupt` control (clean), close | |
| Resume, switching A→B→Codex→A | Works (transcript copy plus `--resume`) | Same | Same, plus exact "Finish this turn" (wait for `result`) and clean "Stop now" (`interrupt`) | Headless sessions are hidden from the TUI picker and `--continue`; AI Juggler always passes the id |
| Model choice | Launch flag or typed `/model` | Same | `set_model` control or `--model` | |
| MCP servers and skills (preflight) | File parsing only | Same | Also verifiable after start: init `mcp_servers`, `mcp_status`, `slash_commands` | The preflight still never starts a tool to detect (AJ-E1542) |
| Agent bridge (ADR-0020) | `--mcp-config` + `--allowedTools` | Same | Same flags | Runtime-independent |
| Pools, D-28 rules for repeated asks | Agent actions only | Same | Also vendor prompts, if D-28 is amended (§7) | |
| Messaging delivery (E18, D-29) | Typed into the PTY between turns (fragile turn detection) | Turn boundaries exact from hooks | `user` message on stdin between turns; turn boundaries exact | |
| Fleet (other Macs) | PTY bytes over the link | Same | Typed events and answers over juggler-link | |
| Crash restart (always on) | Exit code plus guesses | Plus `StopFailure` reasons | Typed `result` errors (`rate_limit`, `billing_error`, `oauth_org_not_allowed`) tell a quota or sign-in stop from a crash exactly | ADR-0003 point 3 needs exactly this |
| Per-account isolation | `CLAUDE_CONFIG_DIR` | Same | Same | |
| Last messages, transcript view | Transcript tail (redacted) | Same | Live structured items; AI Juggler must render a conversation view | New UI surface; nothing is stored beyond the tool's own transcript |
| Slash commands, `/compact`, plugins, hooks, status line, `/usage` | **All of it** (the real TUI) | All of it | Custom commands, skills, plugins, hooks, `/compact`, `/usage` as prompts. **Lost**: pickers and dialogs, the status line, `/login`, plugin and permission UIs | Users who live in the TUI lose most here |
| Policy | Clearly allowed | Clearly allowed (the tool's own extension points) | Allowed on the "unmodified binary" sentence; grey on the SDK note (§2.1) | |
| Cost to build | Built | Small: a settings overlay and hook commands in `juggler-agent` | Large: a protocol client, an ask UI, a conversation view, trust checks, fixtures per version | |

### 4.2 Codex

| Feature | PTY `--no-daemon` (today) | Driven app-server (owned by AI Juggler) | Hybrid: driven plus `codex --remote` TUI on the same server |
|---|---|---|---|
| Live state | Rollout tail, "probably" | **Exact** (`thread/status/changed`, turn events) | Exact |
| Approvals and questions in AI Juggler | No | **Yes** (typed server requests, `acceptForSession`, execpolicy amendments) | Yes, and in the TUI. **U** who is asked when both are connected |
| Limits | Rollout `token_count` (after a turn) | **Documented and live** (`account/rateLimits/read` and `…/updated`), before the first turn too | Same |
| Interrupt, steer | Keystrokes | `turn/interrupt`, `turn/steer` | Same |
| Resume, fork, switching | `codex resume <id>` | `thread/resume`, `thread/fork`; threads stay visible in `codex resume` | Same |
| Model, compact, review | Typed | `model/list`, per-turn model and effort, `thread/compact/start`, `review/start` | Same |
| MCP and skills | Files | `mcpServerStatus/list`, `skills/list` | Same |
| TUI features | All | Most have RPC equivalents (§3.2); `/init`, `/diff`, custom prompts are client-side | **All of them** (the real TUI) |
| Fleet | PTY bytes | Typed events; the docs also describe remote TUI over `wss://` with a token | Same |
| Policy | Allowed | **Explicitly allowed** for a local, open-source app | Same |
| Stability | TUI flags | Experimental label; pinned and schema-checked per version | Same, plus `--remote` |

---

## 5. Runtime cost for a Rust/Tauri app

| Path | What ships | Size | Signing and supply chain | Fit with the code |
|---|---|---|---|---|
| **Speak the protocol from Rust** (Claude stream-json; Codex app-server) | Nothing new. The user's own `claude` and `codex` | 0 MB | No new binary; serde types and fixture tests; `unsafe_code = "forbid"` holds (pipes through `std::process`/tokio, no FFI) | A per-adapter codec behind one runtime trait (§6); the supervisor spawns a piped child instead of a PTY, and the ledger still owns it (ADR-0010) |
| **Node plus the TS Agent SDK** as a sidecar | Node (about 100 MB per architecture, about 200 MB universal), the SDK (about 5 MB), its native Claude Code binary (274 MB arm64), npm dependencies | AI Juggler grows from 66–87 MB to about 450–700 MB | Node needs hardened-runtime JIT entitlements to notarize; a second Claude Code copy pinned to the SDK, diverging from the user's install; an npm dependency tree to audit (QUALITY and SECURITY rules) | A second process and IPC hop; the SDK's version tracks the CLI daily |
| **Python plus the Python SDK** | An interpreter plus a 116 MB wheel with the CLI inside | Larger still | The same signing problems, plus Python packaging | Worse than Node |
| **The TS SDK pointed at the user's binary** (`pathToClaudeCodeExecutable`) | Node and the SDK only | About 100–200 MB | Still Node | Saves the second binary and keeps every other cost |

The SDKs' one real advantage is a maintained protocol client. That advantage is small: the Python SDK (MIT) is a readable reference for the wire format, and the Claude protocol surface AI Juggler needs is about ten control subtypes (`initialize`, `can_use_tool`, `interrupt`, `set_model`, `set_permission_mode`, `mcp_status`, `hook_callback` if used, plus the `system`, `assistant`, `user`, `result`, `rate_limit_event` and `stream_event` messages) (I).

---

## 6. Recommended shape, per tool

### 6.1 One runtime contract in the adapter crate

Today `LaunchSpec.needs_pty` is the only switch. The proposal adds a **driven session** contract next to it, declared by `Capabilities::PROTOCOL_SERVER`, which already exists unused. Here is a sketch (I):

```rust
/// How a session AI Juggler starts is run. Chosen per launch, from what the
/// adapter declares; the user's choice is remembered per profile.
pub enum Runtime {
    /// The tool's own terminal UI under a PTY (today). Always available.
    Terminal,
    /// The tool's protocol over pipes or a socket AI Juggler owns.
    Driven,
}

/// What an adapter adds for a driven session. Codecs live in the adapter;
/// core and UI see only the neutral types below (ADR-0002, ADR-0019).
pub trait DrivenRuntime: Send + Sync {
    /// The process to spawn (the user's own binary, unmodified) and how to
    /// talk to it. Never an SDK, never a bundled copy of the tool.
    fn driven_spec(&self, account: &Account, launch: &DrivenLaunch) -> Result<DrivenSpec, AdapterError>;
    /// Wraps the spawned child's transport in a session.
    fn connect(&self, io: Transport, spec: &DrivenSpec) -> Result<Box<dyn DrivenSession>, AdapterError>;
    /// The real TUI on the same conversation: attached live (Codex
    /// `--remote`), or after the driver lets go (Claude `--resume <id>`).
    fn terminal_view(&self, session: &SessionRef) -> TerminalView;
}

pub trait DrivenSession: Send {
    fn next_event(&mut self, timeout: Duration) -> Result<Option<RuntimeEvent>, AdapterError>;
    fn send_turn(&mut self, input: TurnInput) -> Result<(), AdapterError>;   // only between turns
    fn steer(&mut self, input: TurnInput) -> Result<(), AdapterError>;       // Unsupported where absent
    fn interrupt(&mut self) -> Result<(), AdapterError>;
    fn answer(&mut self, ask: AskId, answer: AskAnswer) -> Result<(), AdapterError>; // the user's own answer
    fn set_model(&mut self, model: &str) -> Result<(), AdapterError>;
    fn compact(&mut self) -> Result<(), AdapterError>;
    fn limits(&mut self) -> Result<Option<LimitsSnapshot>, AdapterError>;  // what the tool reports
    fn release(self: Box<Self>, how: Release) -> Result<ReleaseOutcome, AdapterError>; // AfterTurn | Now
}

pub enum RuntimeEvent {
    Started { native_id: String, model: Option<String>, capabilities: Vec<String> },
    State(LiveStatus),                       // reuses live.rs's vocabulary
    TurnStarted, TurnEnded { outcome: TurnEnd, usage: Option<TurnUsage> },
    Ask(PendingAsk), AskResolved { ask: AskId, by: Resolver },
    Item(ItemSummary),                       // tool call, file change, message: for the view, never stored
    Limits(LimitsSnapshot), Compacted, ModelChanged(String),
    Error { category: ErrorCategory },       // RateLimit | Billing | Auth | Crash | Other
    Exited { code: Option<i32> },
}

pub struct PendingAsk {
    pub id: AskId,
    pub kind: AskKind,                       // Permission { tool, what } | Question { questions } | Elicitation
    pub remember_options: Vec<VendorRule>,   // the tool's own "don't ask again" suggestions, as offered
}
```

Rules that go with it:
1. **The supervisor still spawns everything.** A driven session is a piped child, or a child serving a socket in AI Juggler's data folder (`0700`/`0600`, peer uid checked as in ADR-0020). It is in the spawn ledger, owned and reaped like a PTY session.
2. **Codecs are per adapter and per version.** They are pinned in `verified_against`, feature-detected from the tool's own capability lists (Claude `system/init.capabilities`; Codex's generated schema), and tested against recorded fixtures in the conformance kit (AJ-E1521). An unknown version runs the terminal runtime with a drift note, never a guess.
3. **Answers are the user's.** `answer()` is called only from a UI command. The guardrail suite proves there is no path from a rule, an agent or a timer to `answer()` (§7).
4. **The read-only readers stay.** `live_sessions`, `app_sessions`, `pending_ask`, `tail_turn` and the quota readers remain the source for everything AI Juggler didn't start, and the cross-check for what it did.

### 6.2 Codex: driven app-server, with the TUI attachable

1. **For each session AI Juggler starts, spawn `codex app-server --listen unix://<data>/runtime/codex/<session>.sock`** with the profile's `CODEX_HOME` and `CODEX_SQLITE_HOME`.
   - The socket sits in AI Juggler's data folder, and the process is a ledger child.
   - `initialize` with `clientInfo.name = "ai_juggler"`. Check that the response's `codexHome` is the profile's, and refuse otherwise.
   - Use only stable methods: no `experimentalApi` until a method AI Juggler needs is gated.
2. **Use the protocol for everything on the list:**
   - `thread/start` and `thread/resume`, `turn/start`, `turn/steer` and `turn/interrupt`;
   - approvals and `requestUserInput` answered in Needs you;
   - `account/rateLimits/read` and `…/updated` for the limits;
   - `thread/tokenUsage/updated`, `mcpServerStatus/list`, `skills/list`, `model/list` and `thread/compact/start`.
3. **Open in terminal** runs `codex --remote unix://<that socket>` in AI Juggler's embedded terminal, or in the user's own terminal app. The user gets the real TUI on the same live thread, with every slash command.
   - **U, the spike's job:** whether the TUI resumes the loaded thread by id over `--remote`, and which client gets an approval while both are connected. If both are asked, the first answer wins (`serverRequest/resolved`). If only one is asked, AI Juggler shows "Answer in the terminal" while the TUI is attached.
4. **Fallback:** today's `codex --no-daemon` PTY, kept as a per-session choice ("Start in terminal") and used for any version whose schema doesn't match.
5. **Later, an optional server per profile** (several threads per process) once memory matters. It needs the same ownership rules and must never attach to Codex's own daemon (`app-server-control/`), which AI Juggler doesn't own.

### 6.3 Claude: terminal plus hooks by default; driven as a choice

1. **Default runtime: the TUI in a PTY, as today, plus a per-launch `--settings` overlay** (a file in AI Juggler's folder, never the user's `settings.json`):
   - **Hooks** whose command is `juggler-agent hook <event>`, which sends one line to the bridge socket:
     - `UserPromptSubmit` sets Working.
     - `PermissionRequest` sets Asks you, with the tool name only. It returns no decision.
     - `Notification` covers `permission_prompt`, `idle_prompt`, `elicitation_dialog` and `quota_auto_resume_*`.
     - `Stop` and `StopFailure` set Idle or ended, with the reason.
   - **A status line wrapper** that records `rate_limits` and runs the user's own status line command unchanged. This is the AJ-E1530 capture moved from `settings.json` edits to a launch overlay, so it needs no consent write. (I; it needs a check that `--settings` merges hooks with the user's own rather than replacing them: the settings docs say list keys merge. **U** for `hooks`.)
2. **Driven runtime, per session or per profile, labelled Beta:** `claude -p --input-format stream-json --output-format stream-json --verbose --permission-prompt-tool stdio --session-id <id>` (or `--resume <id>`), with the profile's `CLAUDE_CONFIG_DIR`, the user's own binary, and never `--bare`, `bypassPermissions` or `--dangerously-skip-permissions`.
   - AI Juggler answers `can_use_tool` and AskUserQuestion only with the user's click.
   - It reads `rate_limit_event`, interrupts with the control request, and renders the conversation from the stream.
   - **Open in terminal** waits for the turn to end, closes the driver, and runs `claude --resume <id>` in a PTY. **Run in AI Juggler** does the reverse. Never both at once, since two writers interleave the transcript.
   - **Trust:** a driven session starts only in a folder Claude Code already trusts. Otherwise the first session there starts in the terminal, so Claude Code's own trust dialog runs. Headless "treats the folder as trusted" and runs repository hooks. **U**: how AI Juggler reads the trust flag without opening a credential file; `.claude.json` holds the sign-in, and D-39 already reads one key of it.
   - **Never** set or spoof `CLAUDE_CODE_ENTRYPOINT`; the transcript says what ran it.
3. **Gates before driven becomes Claude's default:**
   - (a) Anthropic's written answer to the approval request (§2.1, point 4);
   - (b) the `--bare` default lands without removing subscription sign-in for `-p`, or with a documented opt-out;
   - (c) a month of lab use shows no billing reclassification (extra usage or the "third-party apps" 400);
   - (d) the conversation view passes D-25's walk.

   Until all four hold, the terminal stays the default, and driven is offered as "Run without a terminal — answer from AI Juggler".
4. **Considered, not adopted yet: Claude Code's own background sessions** (`claude --bg`, `claude agents --json`, `claude attach`).
   - They are Anthropic's own supervisor, with a supported state interface, crash restart and attach, all as ordinary Claude Code use. They are the strongest policy position for "always on without a window".
   - But AI Juggler would not own the processes (ADR-0010), permissions can be answered only by attaching, and there is no programmatic reply.
   - Two things are worth doing now: (i) use `claude agents --json` as a **read-only** state source for background sessions other tools started, if it doesn't start the supervisor (**U**); (ii) revisit as a third runtime ("Hosted by Claude Code") when Anthropic adds a reply or answer interface.

### 6.4 What 0.2's E17, E18 and E19 should build on

- **E17 (bridge):** unchanged. MCP through per-launch flags works in every runtime. Add the hook commands to `juggler-agent`, sharing ADR-0020's socket and per-epoch key, so Claude terminal sessions report exact state.
- **E18 (delivery, D-29):** deliver through `DrivenSession::send_turn` when the session is driven, where turn boundaries are exact from the protocol. In terminal sessions, deliver with turn boundaries from the hooks (Claude) and the same guard. Codex uses `turn/start` on the owned app-server; no PTY typing is needed for Codex once it is driven. Claude-to-Claude keeps Claude Code's own `SendMessage` where it applies.
- **E19 (tasks):** unchanged, since tasks are MCP tools. Assignment still never reads quota (D-31).
- **D-28 rules for repeated asks:** they stay about AI Juggler's own tools. For the tools' own prompts, see §7.

---

## 7. Policy consequences inside AI Juggler

- **ADR-0003 point 2 ("every state-changing action executes on an explicit user click")** already covers answering a tool's prompt from AI Juggler. The user's click is the answer, and AI Juggler only carries it to the tool.
- **D-28's hard line "AI Juggler never answers a tool's own permission prompts"** needs one sentence of amendment so the two don't conflict:
  - AI Juggler carries the user's own answer and never answers by itself: no timer, rule, agent or default.
  - When the user wants a repeated ask to stop, the remembered choice is **the tool's own rule**, picked from the options the tool itself offers:
    - Claude's `updatedPermissions` suggestions, written where the tool writes them;
    - Codex's `acceptForSession` or execpolicy amendment.
  - So the rule is visible and revocable in the tool too, and AI Juggler keeps no rule of its own that silently answers a vendor's prompt.
  - On the second identical approval, AI Juggler may offer that tool rule, mirroring D-28's pattern.
- **Privacy page:** driven sessions mean AI Juggler holds a session's live messages in memory to show them. It doesn't store them; the tool's own transcript is the record. Codex's `account/rateLimits/read` is the tool asking OpenAI with its own sign-in, the same reasoning as D-39 option A. AI Juggler still makes no request to an AI provider itself.
- **Terms page (AJ-E1518):** name the runtime used and its terms note. For Claude driven sessions: "runs your own Claude Code with your own sign-in; Anthropic may count it as Agent SDK use".

---

## 8. Risks, and how the PTY path stays the fallback

| Risk | Likelihood (I) | Mitigation |
|---|---|---|
| Anthropic reads a third-party supervisor driving `claude -p` on a subscription as "offering rate limits for their product" | Medium | Driven is opt-in and labelled; ask Anthropic in writing; the terminal is the default; a remote kill switch is not acceptable (no calls home, D-18), so the user-visible setting plus a release is the off switch |
| `--bare` becomes the `-p` default and drops subscription sign-in | Medium | Capability probe per version; on a version where `-p` can't use the sign-in, driven is unavailable with the reason, and sessions run in the terminal |
| Headless usage billed as "third-party app" or moved to the API-credit pool | Medium | Never add product identity text to prompts; surface `billing_error` and `rate_limit_event` exactly; usage pages say which pool the tool reported |
| Codex app-server breaking changes (experimental label) | Medium | Pin `verified_against`; generate the schema from the installed binary in CI and at discovery, and refuse driven on a mismatch; stable methods only |
| Two clients on one Codex thread misbehave | Unknown | The spike decides; if it fails, "Open in terminal" stops the driver after the turn and resumes in the TUI, as for Claude |
| Two writers on one Claude transcript | Known | One writer at a time, enforced by the runtime switch; the held-session reader (`held_sessions`) refuses to drive a session another app holds |
| Headless skips Claude's workspace trust | Known | Drive only trusted folders (§6.3) |
| A conversation view is a large UI to build well | High | Ship driven with Needs you, the last messages and approvals first; the full transcript view stays "Open in terminal" until the view earns D-25's walk |
| Protocol drift causes wrong states | Medium | Conformance fixtures per version; an unknown message is ignored and counted, never guessed (as the readers do today) |

**The PTY path stays a first-class runtime, not a corpse:**
- it remains `Runtime::Terminal` for every adapter;
- it is the default for Claude, the fallback for Codex, and the only runtime for tools without a protocol (E16);
- the conformance kit runs both runtimes for Claude and Codex.

---

## 9. What changes in the repo, in order (I)

1. **Now, documents only:**
   - this file, ADR-0022 (Proposed) and D-40;
   - an addendum to `codex.md` §9 (rate limits are documented);
   - an addendum to `claude-code.md` §7 (the legal page's August wording, the SDK note, the Help Center);
   - ADR-0003 citing §2's rules.
2. **0.2, spike (the maintainer's, on spare accounts, like AJ-E1701):**
   - Codex: an owned app-server per session, approvals, rate limits, and `--remote` attach with two clients on one thread.
   - Claude: per-launch `--settings` hooks and status line merging with the user's own; PermissionRequest timing in the TUI; a stream-json session with `--permission-prompt-tool stdio`, an AskUserQuestion answer and `rate_limit_event`; switching a session headless → TUI → headless.
3. **0.2:**
   - the runtime trait;
   - Codex driven as the default for new Codex sessions, with the PTY a choice;
   - Claude hooks and status line in terminal sessions;
   - E18 delivery through the runtime.
4. **0.3:** Claude driven as a Beta choice, behind §6.3's gates; the conversation view; Fleet answers over the link.
5. **When the gates pass:** reconsider Claude's default (a new decision).

---

## 10. Search log (negative evidence)

- **"third-party harness" definition:** searched code.claude.com, support.claude.com, anthropic.com/news and engineering for "third-party harness", "third party apps", "offer claude.ai login" and "rate limits for their products". No definition found.
- **Programmatic reply or answer for Claude background sessions:** read https://code.claude.com/docs/en/agent-view in full (command table, peek and reply, supervisor). Only `claude attach` and the peek panel found.
- **The Feb 2026 OpenAI post "Unlocking the Codex harness":** https://openai.com/index/unlocking-the-codex-harness returned 403. Its backward-compatibility claim is known only from an unofficial summary, so it is **U**.
- **Codex approval routing with two subscribers:** app-server page and generated schema read. Nothing states it. **U**.
- **Captures:** the raw page captures from this research are in the session scratchpad only, not committed. A spike that relies on a quote should capture it under `docs/research/evidence/` per the evidence standard.

---

## Lab, 2026-10-25

This lab ran live on the maintainer's lab accounts, with the maintainer's permission of 2026-10-25: one Claude Code profile and the default Codex home, in a throwaway git repo. One real tool process ran at a time, with tiny prompts. There were no sign-ins and no "Allow". Credentials, the Keychain and process environments were not read. Each account folder was snapshotted before and after each run (paths, sizes and times only).

Versions: Claude Code 2.1.302 and codex-cli 0.163.0. Everything below was seen live and is **VP (lab)** unless it is marked otherwise. Ids, emails and paths are left out.

The bridge results (MCP versions, `--mcp-config`, Codex `env_vars` and per-tool approval) are in `docs/qa/evidence/AJ-E1704/lab-2026-10-25.md`.

### Claude Code, headless on a profile whose sessions normally run in the Claude app

| Finding | Detail |
|---|---|
| `claude -p` works on such a profile with only `CLAUDE_CONFIG_DIR` set | `-p "Reply with the single word: ok" --output-format json --no-session-persistence` in a clean environment gave `result: "ok"`, model `claude-opus-5-5`, 1 turn in about 4.7 s, and `total_cost_usd` 0.068 as the list-price equivalent. In stream-json the init event says `apiKeySource: "none"`: the run used the subscription sign-in. Nothing asked for a sign-in |
| `stream-json` with `-p` needs `--verbose` | Without it: `Error: When using --print, --output-format=stream-json requires --verbose` (exit 1, no model call). AI Juggler's `LaunchMode::Headless` for Claude omits it today. A driven runtime (§6.3) must pass it |
| `rate_limit_event` appears in a headless stream | `rate_limit_info {status, rateLimitType: "five_hour", resetsAt, unifiedWindows: {five_hour: {utilization, resetsAt}}, isUsingOverage, overageStatus, overageDisabledReason}`, once per run here. It covers only the window in force, not both |
| What a `-p` start writes in the profile, with persistence off too | Claude Code itself rewrites `.claude.json` (and rotates a backup). It also refreshes `remote-settings.json` and `policy-limits.json`, syncs `skills/synced/…` and `plugins/synced/…` from the server, and touches `cache/model-catalog/…` and `.last-cleanup`. It creates `projects/<cwd slug>/memory/` (empty). With persistence on, the transcript lands in `projects/<cwd slug>/<id>.jsonl` |
| D-39 option A, `claude -p /usage --no-session-persistence --strict-mcp-config --settings '{"disableAllHooks":true}'` from an empty folder | Works: exit 0, about 5 s, no model call and no cost. It prints text, not JSON: "Current session: N% used · resets …", "Current week (<model family>): N% used · resets …", then a "What's contributing" breakdown. `cachedUsageUtilization.fetchedAtMs` in `.claude.json` moved from a reading about 7 days old to the run's own time, with `utilization.five_hour` and `utilization.seven_day` both present. Correction to D-39's "nothing is left behind": Claude Code creates an empty `projects/<folder slug>/memory/` for the probe's folder (one, as long as the folder stays the same) |

### Codex app-server (D-40), first look

| Finding | Detail |
|---|---|
| Transports | `--listen stdio://` speaks newline-delimited JSON-RPC, as documented. **`--listen unix://PATH` does not speak newline JSON:** a JSON line got no reply in 10 s. Over the same socket an HTTP `Upgrade: websocket` got `101 Switching Protocols`, and JSON-RPC in WebSocket text frames then worked. So AI Juggler's unix-socket client must speak WebSocket over the socket (§6.2) |
| Where the unix socket is | `PATH` is created as a **symlink** to `/private/tmp/codex-daemon-<uid>/<64-hex>`, a `0600` socket with a `.lock` beside it. It also creates `~/.codex/app-server-control/app-server-startup.lock` |
| Clean-up | After SIGTERM with no client connected, the symlink and socket were removed. After SIGTERM while a connection that hadn't upgraded was still open, the socket, its lock and the symlink were **left behind**. AI Juggler should close its connection first, and sweep its own stale symlink |
| `initialize` (`clientInfo.name = "AI Juggler lab"`) | Result keys: `userAgent`, `codexHome` (an absolute path), `platformFamily`, `platformOs`. Notifications that follow without being asked for: `remoteControl/status/changed`, `account/updated` |
| `account/read {refreshToken: false}` | `{account: {type: "chatgpt", email, planType}, requiresOpenaiAuth, workspaceRouting: {chatgptAccountId, backendOrigin, accountRoutingOverride}}`. `workspaceRouting` isn't in the generated stable schema. It holds an email and an account id: read it only for the sign-in check, and keep neither (D-33's rule) |
| `account/rateLimits/read` | `{ordinaryUsageAllowed, rateLimits, rateLimitsByLimitId: {codex: {limitId, limitName, normalModelSlug, primary, secondary, credits {hasCredits, unlimited, balance}, individualLimit, spendControlReached, planType, rateLimitReachedType}}, rateLimitResetCredits {availableCount, credits[{id, resetType, status, grantedAt, expiresAt, title, description}]}, accountId, rateLimitUpsell}`. On this account `primary` was `{usedPercent, windowDurationMins: 10080, resetsAt}`, the **weekly** window, and `secondary` was `null`. **No 5-hour window was reported.** AI Juggler must place a window by `windowDurationMins`, never by primary/secondary, and must show "no 5-hour limit reported" honestly |
| Live limits during a turn | `account/rateLimits/updated` came twice in one short turn |
| `thread/start {cwd, approvalPolicy: "untrusted", sandbox: "read-only", ephemeral: true}` | The result has keys `thread`, `approvalPolicy`, `approvalsReviewer`, `activePermissionProfile`, `cwd`, `sandbox`, `model`, `modelProvider`, `reasoningEffort`, `serviceTier`, `multiAgentMode`, `runtimeWorkspaceRoots`, `instructionSources`, `disabledPluginIds`. `thread` carries `id`, `sessionId`, `status`, `ephemeral`, `path`, `source`, `originator`, `turns` and more. An ephemeral thread wrote **no rollout** in `~/.codex/sessions` |
| Events of one turn ("Run `ls` in this folder, then reply with the single word: ok") | `thread/started`, `mcpServer/startupStatus/updated` ×6 (the configured MCP servers starting, **I**), `thread/status/changed` ×4, `turn/started`, `item/started`/`item/completed` ×5 (userMessage, reasoning, commandExecution, agentMessage), `item/agentMessage/delta` ×11, `thread/tokenUsage/updated` ×2, `account/rateLimits/updated` ×2, `turn/completed` (`completed`) |
| The approval flow | `ls` asked for approval even under `untrusted` with a read-only sandbox. Codex sent the server request `item/commandExecution/requestApproval {threadId, turnId, itemId, kind: "command", command, cwd, commandActions: [{type: "listFiles", …}], availableDecisions (three, one of them `acceptWithExecpolicyAmendment`), proposedExecpolicyAmendment, environmentId, startedAtMs}`. The lab answered `{decision: "decline"}`. Then `serverRequest/resolved` arrived, the item completed with `status: "declined"`, the server logged `exec_command failed: … rejected by user`, and the agent still replied "ok". Each approval maps to one typed request with the vendor's own choices, which is the shape §6.2 assumed |
| Shutdown | Closing stdin, then SIGTERM: exit 0, the process gone (checked with `ps -p`) |
| What a start writes in `~/.codex` | The same as any Codex start: `skills/.system/…` re-extracted, the remote plugin markers, the apps and model caches, the shared SQLite files and `tmp/arg0-macos`. `config.toml` got new modification times, each time with the same size, both during a lab `codex exec` run and while only the maintainer's own Codex sessions ran. Codex rewrites it on its own, so AI Juggler must never treat a change in its modification time as a user edit |

**What this changes for the D-40 build:**
- The unix-socket transport is WebSocket, not newline JSON.
- The socket path is a symlink into `/private/tmp/codex-daemon-<uid>/`.
- Windows are told apart by `windowDurationMins`, and an account may report only the weekly window.
- An ephemeral thread leaves no rollout. A driven Codex session must therefore not be `ephemeral` if switching and resume are to work.
- Approvals arrive as the typed requests §3.2 lists.
- Not yet tested: `codex --remote` attach, two clients on one thread, and non-ephemeral resume.
