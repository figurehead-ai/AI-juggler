# E23 — A runtime per tool: driven sessions, exact state, answers from AI Juggler

> Source: the maintainer, 2026-10-25 (D-40, verbatim): "Yes to D-40 option
> A: Codex through app-server by default, Claude terminal + hooks by
> default with headless as a Beta choice, no bundled SDK. Also yes to
> amending D-28 to 'never by itself'." Design: ADR-0022 (Accepted).
> Research: `docs/research/session-runtime.md` (sources accessed
> 2026-10-25), §6.1 the runtime contract and §8 the risks and fallback.
>
> Today every session AI Juggler starts runs as the tool's own terminal UI
> under a PTY, and AI Juggler watches it through files: states are partly
> guesses, questions and approvals can be answered only in the terminal,
> and limits and turn boundaries are inferred. E23 gives each launch a
> **runtime**: the terminal (as today, for every tool) or **driven** (the
> tool's own protocol, spoken from Rust to the user's own binary).
>
> **Law for every task here:**
> - **A runtime per launch, from what the adapter declares (ADR-0022 §1).**
>   `Runtime::Terminal` is always there and is the fallback for every tool.
>   `Runtime::Driven` is declared by `Capabilities::PROTOCOL_SERVER` and an
>   adapter's `DrivenRuntime`. Core, the supervisor and the UI never branch
>   on a tool id (ADR-0019); the codecs live in the adapters.
> - **No vendor SDK, no bundled tool (ADR-0022 §2).** AI Juggler speaks
>   each protocol to the user's own installed, unmodified binary. No Node,
>   no Python, no second copy of any tool; `unsafe_code = "forbid"` holds.
>   No new third-party dependency without a decision-log entry.
> - **AI Juggler owns what it starts (ADR-0010).** A driven session's
>   process is a child in the spawn ledger, piped or serving a socket in
>   AI Juggler's data folder (`0700` folder, `0600` socket); it is stopped
>   with its session and never left as an orphan. AI Juggler never attaches
>   to a vendor's own daemon.
> - **Answers are always the user's (D-28 as amended by D-40).** AI
>   Juggler carries the user's own click to the tool and never answers by
>   itself: no timer, rule, agent or default. "Don't ask again" is the
>   tool's own rule, picked from the options the tool offers, so it is
>   visible and revocable in the tool too. Every answer is audited.
> - **Per-launch files in AI Juggler's own folder (ADR-0018).** Hooks,
>   status lines and sockets live under AI Juggler's data; the user's setup
>   is only read, never written.
> - **Never read a process's environment, a credential file or the
>   Keychain (D-37).** Hooks report what the tool hands them; the key a
>   hook uses is one AI Juggler wrote into the launch.
> - **Policy (research §2).** Claude driven mode is a labelled Beta, off by
>   default, until ADR-0022 decision 4's four gates hold (the first, as
>   amended 2026-10-25: Anthropic's published terms, rechecked each
>   release, still allow it). Never `--bare`,
>   `bypassPermissions` or a spoofed entrypoint; `clientInfo` names AI
>   Juggler honestly.
> - **Unknown means the terminal, never a guess.** A tool version whose
>   protocol AI Juggler hasn't pinned starts in the terminal with the
>   reason; an unknown message is ignored and counted.

## Step 1 — the contract and the spike

### AJ-E2301 The runtime lab on real tools | Status: IN-PROGRESS | Priority P0 | Effort S | Depends: —
**Goal**: The five things only a real run can show are proven before driven sessions become anyone's default.
**User story**: As the maintainer (or the agent I allow to), I run the lab checklist with one or two of my real tools in a throwaway lab project, and the evidence turns from "unverified" to measured.
**Requirements**:
1. Codex: an owned `codex app-server` per session over stdio; an approval and a `requestUserInput` answered over the protocol; `account/rateLimits/read` and `…/updated`; Open in terminal handing the thread to `codex resume` at a turn boundary, and back. (Done in part by the lab of 2026-10-25: stdio speaks newline JSON, `unix://` speaks WebSocket from a symlink under `/private/tmp`, the weekly window alone came as `primary`, `ls` asked for approval and a decline resolved it. Still open: two clients on one thread, non-ephemeral resume.)
2. Claude: a per-launch `--settings` file's hooks and status line **merge** with the user's own (both run); `PermissionRequest` timing in the TUI (does the dialog show while the hook runs?).
3. Claude headless: a stream-json session with `--permission-prompt-tool stdio`, one approval, one AskUserQuestion answer and a `rate_limit_event`.
4. A mode switch at a turn boundary (headless → terminal → headless) keeps one transcript.
5. The exact commands, the before/after snapshot of the tool folders and the cleanup are `docs/qa/runtime-lab.md` (runnable by the maintainer or an agent they allow, on one or two of their real tools' accounts, 2026-10-25); `docs/internal/MAINTAINER-STEPS.md` §18 points to it. Captures go to `docs/research/evidence/` per the evidence standard.
**UI/UX reasoning**: n/a.
**Performance budget**: n/a.
**Quality gates**: the evidence standard; captures redacted with the fail-closed tool.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2301/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: `docs/research/evidence/`, the E23 manifests' "unverified" lines.
**Research basis**: `session-runtime.md` §9 step 2; ADR-0022 Consequences.
**Handover note**: The live part runs on the maintainer's accounts with their leave (2026-10-25: one or two real tools, lab project only). Everything else in E23 is built and tested on fakes first.

### AJ-E2302 The runtime contract | Status: IN-PROGRESS | Priority P0 | Effort M | Depends: —
**Goal**: One neutral contract lets any adapter offer a driven session next to the terminal.
**User story**: As an adapter author, I implement `DrivenRuntime` for my tool, declare `PROTOCOL_SERVER`, and the conformance kit tells me whether both runtimes keep the rules.
**Requirements**:
1. `juggler-adapter` SHALL define `Runtime` (`Terminal` | `Driven`), chosen per launch; the user's choice remembered per profile and changeable per session.
2. `DrivenRuntime` (on the adapter, `HarnessAdapter::driven`): the process to spawn and how to reach it (pipes or a socket), `connect`, and the terminal view of the same conversation (attached live, or after the driver lets go). Its default runtime and its maturity (Stable, Beta) are data, not a branch.
3. `DrivenSession`: `next_event`, `send_turn` (between turns only), `steer` (unsupported where absent), `interrupt`, `answer` (the user's own answer), `set_model`, `compact`, `limits`, `release` (after the turn, or now).
4. Typed `RuntimeEvent`s: started, state, turn started (with who started it) and ended (outcome, usage), text and tool deltas, `ApprovalRequested { id, kind, summary, options }`, `QuestionAsked`, resolved, `LimitsUpdated` (windows with resets), usage, compacted, model changed, errors by category (rate limit, billing, sign-in, crash, other), exited.
5. `send_turn` takes a `TurnInput` with its origin (the person, or a delivered message with its sender), so E18 can deliver into a driven session without typing into a PTY; turn boundaries come from the protocol.
6. The conformance kit tests both runtimes for every adapter that declares them: `PROTOCOL_SERVER` iff `driven()`; the driven spec runs the user's binary with the same pinned isolation and removed credentials as the terminal; sockets only in AI Juggler's runtime folder; no SDK, no `--bare`, no permission bypass; options offered are the tool's own. The fake adapter implements Driven over the fake harness.
**UI/UX reasoning**: Invisible; the runtime shows on the session page.
**Performance budget**: an event reaches the app's state within 50 ms of the tool sending it (fake).
**Quality gates**: the kit for all subjects; unit tests of the types.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2302/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: `crates/juggler-adapter/src/runtime.rs`, `capabilities.rs`, `adapter.rs`; `crates/juggler-adapters/src/fake_driven.rs`, `src/bin/fake_harness.rs`; `tests/conformance/`.
**Research basis**: `session-runtime.md` §6.1.
**Handover note**: —

## Step 2 — Codex driven through app-server

### AJ-E2303 Codex through app-server, by default | Status: IN-PROGRESS | Priority P0 | Effort L | Depends: AJ-E2302
**Goal**: Each Codex session AI Juggler starts is driven through the user's own `codex app-server`: exact state, approvals and questions in AI Juggler, and live limits with resets.
**User story**: As a Codex user, I see "Asks you: run `npm test`?" in Needs you, click Allow once, and the session carries on; the 5-hour and weekly limits show with their reset times before the first turn ends.
**Requirements**:
1. A Rust JSON-RPC client (newline-delimited, no `jsonrpc` field), one per session, on the stdin and stdout of `codex app-server --listen stdio://` with the profile's `CODEX_HOME` and `CODEX_SQLITE_HOME`. The server is a child in the spawn ledger, ends when its input closes, and is stopped with its session. *(Amended 2026-10-25 by the lab: first a unix socket in AI Juggler's folder; Codex's `unix://` speaks WebSocket from a socket under `/private/tmp`, ADR-0022 decision 3.)* Threads are never ephemeral; limit windows are placed by `windowDurationMins`, never by primary/secondary; an approval offers only the decisions Codex lists (`availableDecisions`, when it sends them).
2. `initialize` with `clientInfo { name: "ai_juggler", title: "AI Juggler", version }`; refuse a server whose `codexHome` isn't the profile's. Stable methods only (no `experimentalApi`).
3. The protocol schema is generated offline with a scratch `CODEX_HOME` (`codex app-server generate-json-schema`), the supported range pinned (from 0.163.0, the verified version), and the surface AI Juggler uses checked against the installed binary's schema at discovery. Fixtures are recorded; a fake app-server binary serves the tests.
4. Mapping: `thread/start`, `thread/resume`, `turn/start`, `turn/steer`, `turn/interrupt`, `thread/compact/start`, `model/list`; `item/commandExecution/requestApproval`, `item/fileChange/requestApproval`, `item/permissions/requestApproval`, `item/tool/requestUserInput`, `serverRequest/resolved`; `account/rateLimits/read` and `account/rateLimits/updated`; `thread/tokenUsage/updated`; `turn/started`, `turn/completed`, `thread/status/changed`, `item/*` deltas, `error`.
5. Approvals offer the tool's own options only: Allow once (`accept`), Allow for this session (`acceptForSession`), the proposed command rule (`acceptWithExecpolicyAmendment`) when Codex proposes one, Deny (`decline`), Deny and stop (`cancel`).
**UI/UX reasoning**: The session page and Needs you show the ask; nothing else changes for the user.
**Performance budget**: `initialize` answered within 10 s and the thread within 30 s, else the session starts in the terminal with the reason.
**Quality gates**: codec unit tests over the recorded fixtures; an end-to-end test against the fake app-server (start, a turn, an approval answered, a question answered, limits, interrupt, steer, compact, release); the ledger owns the server and nothing outlives the session.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2303/manifest.md`.
**Visual testing**: an ask in Needs you; the session page of a driven Codex session.
**Files to touch**: `crates/juggler-adapters/src/codex_appserver/`, `src/bin/fake_codex_app_server.rs`, `tests/fixtures/codex-app-server/`; `crates/juggler-supervisor` (a driven spawn); `crates/juggler-app/src/features/runtime/`.
**Research basis**: `session-runtime.md` §3.2, §6.2; `codex.md` §9 addendum.
**Handover note**: Who is asked when the TUI is attached too is the spike's (AJ-E2301 Req.1); until then, while a terminal is attached AI Juggler shows the ask and says it can also be answered there; the first answer wins (`serverRequest/resolved`).

### AJ-E2304 Open in terminal, and Start in terminal | Status: IN-PROGRESS | Priority P0 | Effort S | Depends: AJ-E2303
**Goal**: The real Codex TUI is one click away, and the PTY stays the fallback.
**User story**: As a user who likes Codex's TUI, I click Open in terminal and get it on the same live thread; on a profile I set to "Start in terminal", sessions run as today.
**Requirements**:
1. "Open in terminal" hands the thread over at a turn boundary: the driver lets go when the turn ends, the server ends, and `codex resume <thread> --no-daemon` runs in AI Juggler's terminal as the session's continuation (the old session is marked handed over). "Run in AI Juggler" does the reverse once the terminal has ended. Never two writers. *(Amended 2026-10-25: first a live `codex --remote` attach, which needs Codex's WebSocket socket; ADR-0022 records the trade-off.)*
2. "Start in terminal" is a per-session choice at launch and a per-profile setting: the PTY with `--no-daemon`, exactly as before E23.
3. *(Added 2026-10-25 by lab 2.)* Codex's folder trust is asked in AI Juggler, once per folder, in Codex's words ("Trust this folder?": Trust and continue, Open restricted). The answer is kept in AI Juggler's data and passed to every launch in memory (`-c projects=…`), driven, in the terminal and at a hand-over. Codex's `config.toml` is never written; unanswered means "not trusted" in memory, and a folder decided in Codex is left to it.
4. The terminal is used automatically, with the reason, when the binary is older than the pinned range, its schema lacks a method AI Juggler uses, or the server fails to start, answer, or runs on another `codexHome`.
**UI/UX reasoning**: The choice sits with the profile's launch settings; the session page names the runtime in one line.
**Performance budget**: n/a.
**Quality gates**: each fallback reason tested; the attach spec tested.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2304/manifest.md`.
**Visual testing**: the runtime line on the session page; the profile's choice.
**Files to touch**: `codex_appserver/runtime.rs`, `features/runtime/`, the launch sheet and profile settings.
**Research basis**: `session-runtime.md` §6.2 items 3–4.
**Handover note**: Two clients on one thread is the spike's.

### AJ-E2305 Switching and hand-over across runtimes | Status: IN-PROGRESS | Priority P0 | Effort M | Depends: AJ-E2303
**Goal**: A→B→Codex→A keeps history, tasks and context whichever runtime each step runs in (the 0.1 core rule).
**User story**: As a user, I switch a driven Codex session to Claude and back, and both conversations continue where they were.
**Requirements**:
1. A driven session is released at a turn boundary before a switch ("Finish this turn" waits for the turn's end; "Stop now" interrupts), and its thread is a normal rollout the readers find.
2. Resuming into a driven Codex session uses `thread/resume` with the thread id; into a terminal, `codex resume <id>` as today.
3. Tests: fake driven Codex → locate → resume (both runtimes); the switch path's release.
**UI/UX reasoning**: Unchanged switch sheet.
**Performance budget**: n/a.
**Quality gates**: the tests above; the existing switching suites unchanged.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2305/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: `codex_appserver/`, `features/runtime/`, the switch pipeline's release step.
**Research basis**: ADR-0022 Consequences (exact turn boundaries for switching).
**Handover note**: The live A→B→Codex→A run is in the switching lab (AJ-E1539) once the spike passes.

### AJ-E2310 Driven sessions come back after AI Juggler restarts | Status: IN-PROGRESS | Priority P0 | Effort M | Depends: AJ-E2303, AJ-E2308
**Goal**: A session AI Juggler drives survives AI Juggler quitting, restarting or crashing, as the 0.1 "always on" rule asks: the next start resumes it in the same runtime, or says honestly that its turn was cut off and offers Resume.
**User story**: As a user whose AI Juggler crashed or updated while a Codex session sat between turns, I reopen AI Juggler and the same session is there, driven, on the same thread. If it was in the middle of a turn, its page says the turn was cut off and Resume continues the conversation.
**Requirements**:
1. While a driven session runs, AI Juggler keeps one small mark per session in its own store (`runtime.live`): the tool's conversation id and whether a turn is running. It holds no message text. The mark goes when the session ends, is stopped, is released for a switch or is handed over.
2. At the next start, each marked session that was still live on this Mac is kept out of the "ended with AI Juggler" pass (and out of the restart-after-crash pass). Instead:
   - **Between turns:** it is resumed at once, under the same session id, in the same runtime: a fresh `codex app-server` with `thread/resume` for Codex, and `claude -p … --resume <id>` in stream-json for Claude (Beta). No turn is sent. This is coming back as it was (ADR-0003 point 5), not a restart: no account action and no new work.
   - **Mid-turn:** nothing starts by itself. The session says "Its turn was cut off when AI Juggler stopped", needs you, and offers **Resume**. Resume resumes the same conversation and sends nothing.
   - A resume that can't drive (the tool gone, too old, the Beta now off) starts its terminal on the same conversation with the reason, as any launch does. One that can't start at all ends with the reason.
3. Quitting says it: the quit sheet counts driven sessions as ones that come back when AI Juggler opens again.
4. Tests on fakes kill the app core itself (a child process holding the app's state, killed with `SIGKILL`) mid-turn and between turns, then start a new app core on the same data: between turns the session is driven again on the same thread; mid-turn it is cut off, nothing runs until Resume, and Resume continues the same thread. A session stopped before the kill never comes back.
**UI/UX reasoning**: The session keeps its row and page; a cut-off turn is one line and one primary button (D-25).
**Performance budget**: resumes run off the start-up path (a thread); the window never waits on them.
**Quality gates**: the kill tests; the unit tests of the mark; vitest for the cut-off line and the quit words; typecheck.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2310/manifest.md`.
**Visual testing**: a cut-off session's page.
**Files to touch**: `crates/juggler-app/src/features/runtime/pickup.rs`, `driver.rs`, `commands.rs`, `state.rs`, `quit.rs`; `ui/src/pages/session/DrivenPanel.tsx`, `components/QuitSheet.tsx`.
**Research basis**: the maintainer's "always on" rule (0.1); ADR-0003 point 5; ADR-0022 Consequences.
**Handover note**: The live check is in the runtime lab (`docs/qa/runtime-lab.md` step 5): quit and kill AI Juggler Dev between turns and mid-turn on a lab session.

## Step 3 — Claude: exact state in the terminal

### AJ-E2306 Claude terminal sessions with hooks and the status line | Status: IN-PROGRESS | Priority P0 | Effort M | Depends: AJ-E2302
**Goal**: Every Claude session AI Juggler starts reports its exact state and limits through Claude Code's own hooks and status line, with no guessing.
**User story**: As a Claude user, Needs you says "Asks you: Bash `rm -rf build`" the moment Claude asks, and "Working" and "Idle" are exact.
**Requirements**:
1. A per-launch `--settings` file in AI Juggler's folder (never the user's setup) adds hooks (`UserPromptSubmit`, `PreToolUse` for questions, `PermissionRequest`, `Notification`, `Stop`, `StopFailure`, `SessionEnd`) whose command is `juggler-agent hook <event>`, plus the status line wrapper that records `rate_limits` and runs the user's own status line unchanged.
2. `juggler-agent hook` reads only the fields it needs from its stdin (event, the tool's session id, tool name, a bounded summary of what it asks, notification type, stop reason), sends one line to the app's socket with the launch's report key, prints nothing (so it never decides a permission) and always exits 0 within 2 s.
3. The report key is AI Juggler's own, per launch, in the launch's environment only, separate from the agent tools' key (it can report state, nothing else).
4. Results: Asks you (with what it asks), Working, Idle, ended, exact, for every session AI Juggler starts; limits from the status line for the account.
5. Hooks from `--settings` merge with the user's own per Claude Code's docs; the design keeps both running either way, and the spike confirms it (AJ-E2301 Req.2). *(Amended 2026-10-25 by lab 2: Claude Code applies only the last `--settings`, so a launch gets exactly one, AI Juggler's, with any `--settings` the launch had folded in and their hooks combined; one that could carry a secret is never copied, and that launch keeps its own.)*
6. D-37: nothing reads an environment; the hook reads its own.
**UI/UX reasoning**: The state words lose "probably" for these sessions.
**Performance budget**: a hook round trip < 50 ms; a missing app costs the tool at most the hook's 2 s.
**Quality gates**: the overlay through the conformance kit; the hook command's parsing (fixtures per event); end to end with the fake tool calling the real `juggler-agent hook`.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2306/manifest.md`.
**Visual testing**: exact states on the rail and session page.
**Files to touch**: `crates/juggler-adapters/src/claude_hooks.rs`, `crates/juggler-agentbridge/src/hook.rs`, `features/runtime/`, `features/agent_bridge/`.
**Research basis**: `session-runtime.md` §4.1, §6.3 item 1.
**Handover note**: Answering a Claude terminal session from AI Juggler stays out until a hook route is proven (ADR-0022 Accepted).

## Step 4 — answering from AI Juggler

### AJ-E2307 Answers from Needs you and the session page | Status: IN-PROGRESS | Priority P0 | Effort M | Depends: AJ-E2303
**Goal**: A driven session's approval or question is answered in AI Juggler, with the tool's own options, by the user's click only.
**User story**: As a user, I answer "Which database?" with "Postgres" from Needs you without opening a terminal.
**Requirements**:
1. Needs you items and the session page show a driven session's ask with the tool's own options ("Allow once", "Always allow …" as the tool offers, "Deny") and a reply box (with the tool's own choices) for questions.
2. The click is carried to the tool through `answer`; "don't ask again" is the tool's own rule.
3. No path from a timer, rule, agent or default to `answer` (a guardrail test over the source).
4. Every answer is audited: session, ask kind, the option chosen (never a reply's text), when.
5. The UI reuses the session page's components until the 0.1.5 design system lands (readability scale, `AppIcon`, the conversation view).
**UI/UX reasoning**: D-25's rows and one primary per item; the options in the tool's own words.
**Performance budget**: an answer reaches the tool in < 100 ms (fake).
**Quality gates**: IPC tests; the guardrail; vitest for the ask card; typecheck.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2307/manifest.md`.
**Visual testing**: an approval, a question with choices, a free reply; both themes.
**Files to touch**: `features/runtime/commands.rs`, `ui/src/components/AskCard.tsx`, `HomeNeeds.tsx`, `pages/session/`.
**Research basis**: ADR-0022 decision 5; `session-runtime.md` §7.
**Handover note**: On the second identical approval, the ask may offer the tool's own rule first (ADR-0022 decision 5); that is a follow-up.

## Step 5 — Claude headless (Beta)

### AJ-E2308 Run without a terminal: answer from AI Juggler (Beta) | Status: IN-PROGRESS | Priority P1 | Effort L | Depends: AJ-E2302, AJ-E2307
**Goal**: A Claude session can run headless, per session, with its questions and approvals answered in AI Juggler.
**User story**: As a user who picked "Run without a terminal: answer from AI Juggler" for one session, I see its turns and answer its prompts in AI Juggler, and Open in terminal continues it in the TUI.
**Requirements**:
1. Stream-json from Rust to the user's own `claude`: `-p --input-format stream-json --output-format stream-json --verbose --permission-prompt-tool stdio` with `--session-id` or `--resume`, the profile's `CLAUDE_CONFIG_DIR`; permission prompts (`can_use_tool`, AskUserQuestion) over stdio; `interrupt`; `set_model`; `rate_limit_event`; `result`.
2. Hand back to the terminal at a turn boundary with `claude --resume <id>`, and back again; one writer per session, enforced.
3. Never `--bare`, `bypassPermissions`, `--dangerously-skip-permissions` or a spoofed entrypoint; only in a folder Claude Code already trusts.
4. Off by default, labelled Beta, per session; ADR-0022 decision 4's four gates written into the docs (guide and ADR) as what makes it a default: Anthropic's published terms, rechecked each release, still allow it; `-p` keeps subscription sign-in once `--bare` is its default; a month of lab use with no billing change; the conversation view passes a hand walk.
**UI/UX reasoning**: The choice is in the launch sheet's "More" fold, with "Beta" and one line on what changes.
**Performance budget**: n/a.
**Quality gates**: codec tests over fixtures; end to end against a fake `claude` speaking stream-json; the guardrails (no forbidden flag in any spec).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2308/manifest.md`.
**Visual testing**: the choice; a headless session's page.
**Files to touch**: `crates/juggler-adapters/src/claude_stream/`, `src/bin/fake_claude_stream.rs`, `features/runtime/`.
**Research basis**: `session-runtime.md` §3.1, §6.3 items 2–3.
**Handover note**: No written approval is sought (the maintainer, 2026-10-25): the Beta runs the user's own `claude -p`, never the Agent SDK, and never handles a sign-in. The terms check is a release step (`docs/RELEASING.md`).

## Step 6 — documents

### AJ-E2309 Docs, Privacy and terms lines, evidence | Status: DONE-EVIDENCED | Priority P0 | Effort S | Depends: AJ-E2303, AJ-E2306, AJ-E2308
**Goal**: What runs, under whose terms, and what AI Juggler holds is written down and tested.
**User story**: As a careful user, I read on the Privacy page that a driven session's messages are held in memory to show them and never stored.
**Requirements**:
1. `docs/research/codex.md` §9 addendum: app-server's rate limits are documented methods.
2. ADR-0003 cites the current Anthropic wording (research §2.1) and ADR-0022 decision 5.
3. Privacy and terms page lines naming the runtime, each tied to tests that exist.
4. `docs/guide/` sections: runtimes, answering from AI Juggler, the Beta and its gates.
5. Evidence manifests for AJ-E2302–E2308 (and AJ-E2310's own).
**UI/UX reasoning**: Plain words.
**Performance budget**: n/a.
**Quality gates**: the trust-claims test; the plain-words and terminology lints; the docs link check.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2309/manifest.md`.
**Visual testing**: the Privacy page lines.
**Files to touch**: `docs/research/codex.md`, `docs/adr/0003-*.md`, `ui/src/lib/trust-claims.json`, `docs/guide/`.
**Research basis**: `session-runtime.md` §2, §7.
**Handover note**: —
