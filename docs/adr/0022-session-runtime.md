# ADR-0022: How AI Juggler runs the sessions it starts — a runtime per tool

- Status: Accepted (D-40, 2026-10-25; decision 4's first gate amended by the maintainer the same day: no written approval is sought, the published terms are rechecked each release)
- Date: 2026-10-25
- Deciders: the maintainer (D-40); research agent (draft)
- Relates to: ADR-0002 (adapters), ADR-0003 (automation policy), ADR-0010 (spawn ledger), ADR-0018 (per-launch files in AI Juggler's folder), ADR-0019 (tools as data), ADR-0020 (agent bridge, on release/0.2), D-28 to D-33, D-37, D-39
- Research: `docs/research/session-runtime.md` (sources accessed 2026-10-25)

## Context

Every session AI Juggler starts runs today as the tool's own terminal UI under a PTY. AI Juggler watches it through files: pid files, status words, transcript tails and rollouts. That gives the real TUI with everything users rely on: slash commands, `/compact`, plugins, hooks, the status line and `/usage`. It costs three things:
- states are partly guesses ("probably");
- questions and approvals can be answered only in the terminal;
- limits, turn boundaries and stop reasons are inferred.

The maintainer wants users to "handle as much as they can within our tool with always optionally to go to the tool directly", to "quickly monitor things" and be "only involved if it requires their attention with input". He wants the architecture chosen for the long term, not for one version.

**The facts that decide it** (research §2 and §3):

**Claude**
- **Terms.** Anthropic's legal page says its rules don't "prevent an end user from signing in to the unmodified Claude Code binary with their own Claude subscription". Its Agent SDK page says "Unless previously approved, Anthropic does not allow third party developers to offer claude.ai login or rate limits for their products, including agents built on the Claude Agent SDK". Headless `claude -p` is "the Agent SDK via the CLI", and staff limited subscription use of the SDK to "local development and experimentation". The user's own `claude`, in a terminal or headless, is on solid ground. A bundled SDK is not.
- **What the protocol exposes.** The SDK is a client of the CLI's stream-json control protocol, so a Rust host speaking that protocol to the user's own binary gets everything the SDK gets:
  - permission and question requests (`can_use_tool`, AskUserQuestion);
  - interrupt and model switching;
  - live limit events;
  - typed stop reasons.
- **What headless loses.** The TUI's pickers and dialogs and the status line. The protocol is not a documented public contract. `--bare`, which never reads the subscription sign-in, "will become the default for `-p`".
- **Two more useful facts.**
  - Claude Code's own hooks (PermissionRequest, Notification, Stop) and status line give exact state and limits for terminal sessions too.
  - Claude Code ships its own background-session supervisor with "`claude agents --json` … the supported way to read session state".

**Codex**
- **The TUI is already an app-server client.** AI Juggler passes `--no-daemon` today so that server stays in its tree.
- **Terms.** OpenAI permits app-server with the user's ChatGPT sign-in for "a local or open-source application".
- **What app-server exposes.** Threads and turns, approvals and questions as typed server requests, documented rate limits with reset times (`account/rateLimits/read` and `…/updated`), steer, interrupt, compact, and MCP and skills status. `codex --remote unix://…` attaches the real TUI to a server.
- **Caveat.** It is labelled experimental, yet the VS Code extension, Codex Desktop and the TUI run on it.
- **The Codex SDKs don't fit.** The TypeScript SDK wraps `codex exec`, which cannot answer approvals. The Python SDK auto-accepts approvals by default.

**Cost of bundling an SDK.** Node (about 100 MB per architecture) plus the SDK's own 274 MB Claude Code binary, both signed and notarized, take AI Juggler from 66–87 MB to about 450–700 MB. They also add an npm tree to audit and a second Claude Code copy that diverges from the user's.

## Decision

1. **A session's runtime is chosen per launch from what its adapter declares.**
   - `Runtime::Terminal` is the tool's TUI under a PTY, as today. It is always available and is the fallback for every tool.
   - `Runtime::Driven` is the tool's own protocol, spoken by AI Juggler over pipes or a socket it owns.
   - The user's choice is remembered per profile and can be changed per session. The adapter declares `Capabilities::PROTOCOL_SERVER`, which already exists unused, and implements a `DrivenRuntime` that yields neutral `RuntimeEvent`s (research §6.1).
   - Core, the supervisor and the UI never branch on a tool id (ADR-0019).
2. **No vendor SDK is bundled.** AI Juggler speaks each protocol from Rust to the user's own installed, unmodified binary. It adds no Node, Python or second copy of any tool. `unsafe_code = "forbid"` holds.
3. **Codex: driven through app-server by default, with the TUI attachable.**
   - Each session AI Juggler starts gets its own `codex app-server --listen unix://<AI Juggler data>/runtime/codex/<session>.sock`, run with the profile's `CODEX_HOME`. It is a child in the spawn ledger.
   - It uses `clientInfo.name = "ai_juggler"` and stable methods only. AI Juggler checks that the server's `codexHome` is the profile's.
   - **Open in terminal** attaches the real TUI with `codex --remote unix://…`.
   - The PTY (`--no-daemon`) stays as "Start in terminal", and is used automatically for a Codex version whose generated schema doesn't match the pinned one.
4. **Claude: the terminal stays the default, made observable through Claude Code's own extension points. Driven is a labelled per-session choice.**
   - Terminal sessions get a per-launch `--settings` overlay in AI Juggler's folder, never the user's files. It holds hooks that report state through `juggler-agent` and a status line wrapper that records `rate_limits` and still runs the user's own status line.
   - Driven sessions run `claude -p --input-format stream-json --output-format stream-json --permission-prompt-tool stdio` with the profile's `CLAUDE_CONFIG_DIR`. They never use `--bare`, `bypassPermissions` or a spoofed entrypoint, and they run only in folders Claude Code already trusts.
   - **Open in terminal** and **Run in AI Juggler** hand the conversation between modes at a turn boundary with `--resume <id>`. There is never more than one writer.
   - Driven becomes Claude's default only by a later decision, after four gates: Anthropic's published terms, rechecked each release, still allow it (the Beta runs the user's own installed `claude -p`, which Anthropic's Help Center of 2026-10-07 lists as drawing on subscription limits, never the Agent SDK, and AI Juggler never handles a sign-in); `-p` keeping subscription sign-in once `--bare` is its default; a month of lab use with no billing reclassification; and the conversation view passing the D-25 walk.
5. **Answers are always the user's.**
   - AI Juggler carries the user's own answer to a tool's question or permission prompt, from Needs you, the session page or a notification action. It never answers one by itself: no timer, rule, agent or default does.
   - "Don't ask again" is the **tool's own rule**, chosen from the options the tool offers (Claude's `updatedPermissions` suggestions; Codex's `acceptForSession` or execpolicy amendment), so it is visible and revocable in the tool too.
   - On the second identical approval, AI Juggler may offer that tool rule.
   - This amends D-28's hard line "AI Juggler never answers a tool's own permission prompts" to read "never by itself".
6. **What stays as it is:**
   - the read-only readers for sessions other apps run (the Claude app, `claude` in a terminal, Codex Desktop, D-37), which also cross-check driven sessions;
   - the bridge and the pool rules (ADR-0020, D-28 to D-33);
   - profile isolation (`CLAUDE_CONFIG_DIR`, `CODEX_HOME`);
   - D-39's limit sources for sessions AI Juggler didn't start.
7. **Claude Code's own background sessions** (`claude --bg`, `claude agents --json`, `claude attach`) are not a runtime yet, because AI Juggler would not own the processes and permissions can be answered only by attaching. `claude agents --json` may be used as a read-only state source if it doesn't start the supervisor; that needs a check.

## Alternatives considered

- **Keep PTY only (today).**
  - The user keeps the real TUI and nothing changes.
  - The user gives up answering in AI Juggler, exact states, live limits with resets, and exact turn boundaries for E18 delivery and switching.
  - Rejected as the long-term shape, especially for Codex, whose TUI is already a client of the protocol AI Juggler would use.
- **Bundle the Claude Agent SDK (Node or Python sidecar).**
  - It is the path Anthropic's SDK note names.
  - It ships a second Claude Code binary and makes AI Juggler about 5–8 times larger, with new signing and supply-chain work.
  - It adds no capability over the protocol.
  - Rejected. The only acceptable form would be an API-key feature the user turns on, which is not how sessions run by default.
- **Driven Claude as the default now.**
  - Most of the product gain, but the policy is grey (§2.1 caveats), `--bare` is coming, billing classification may move, and the TUI experience is lost by default.
  - Deferred behind the four gates in decision 4.
- **Codex SDK (TypeScript) or `codex exec --json`.**
  - Cannot answer approvals.
  - Rejected for interactive sessions. `exec` stays for one-shot runs.
- **Attach to the vendors' own daemons** (Codex's `app-server-control/` socket; Claude Code's supervisor).
  - AI Juggler would not own the processes (ADR-0010), and two supervisors would compete over restarts.
  - Rejected for now. It is revisited if a vendor adds an attach and answer interface for third parties.

## Consequences

**Easier**
- **Needs you becomes exact,** with the question or the tool and command in view, and the user answers it in AI Juggler.
- **Live limits** come with the vendor's own reset times for driven sessions: Codex always, Claude when driven.
- **Exact turn boundaries** make switching's "Finish this turn", E18's delivery between turns, and ADR-0003's never-restart-after-a-quota-stop rule exact.
- **Fleet** can carry typed events and answers instead of terminal bytes.

**Harder**
- **Protocol codecs to keep current.** One per tool, pinned per version, with recorded fixtures in the conformance kit and a capability check at discovery. An unknown version falls back to the terminal, never a guess.
- **A conversation view to build.** A driven session has no terminal to look at; until the view is good, "Open in terminal" is the full view.
- **A spike before code, on spare accounts (the maintainer's step, like AJ-E1701):**
  - Codex `--remote` with two clients on one thread;
  - Claude hooks and the status line merging through `--settings`;
  - PermissionRequest timing in the TUI;
  - a stream-json session with an approval, a question and a limit event;
  - a mode switch at a turn boundary.

**Accepted**
- Claude terminal sessions still can't be answered from AI Juggler, until driven mode is chosen or a hook route is proven.
- A Claude session started headless doesn't appear in `claude --resume`'s picker or `--continue`. AI Juggler always passes the id.

**Documents**
- `codex.md` §9 gets an addendum: rate limits are documented in app-server.
- `claude-code.md` §7 and ADR-0003 cite the current Anthropic wording.
- The Privacy page says driven sessions' messages are held in memory to show them and never stored.
- The terms page names the runtime and its terms note.
