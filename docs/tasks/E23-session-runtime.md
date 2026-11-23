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

### AJ-E2301 The runtime lab on real tools | Status: DONE-EVIDENCED | Priority P0 | Effort S | Depends: —
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

### AJ-E2302 The runtime contract | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: —
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

### AJ-E2303 Codex through app-server, by default | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E2302
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

### AJ-E2304 Open in terminal, and Start in terminal | Status: DONE-EVIDENCED | Priority P0 | Effort S | Depends: AJ-E2303
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

### AJ-E2305 Switching and hand-over across runtimes | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E2303
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

### AJ-E2310 Driven sessions come back after AI Juggler restarts | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E2303, AJ-E2308
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

### AJ-E2306 Claude terminal sessions with hooks and the status line | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E2302
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

### AJ-E2307 Answers from Needs you and the session page | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E2303
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

### AJ-E2308 Run without a terminal: answer from AI Juggler (Beta) | Status: DONE-EVIDENCED | Priority P1 | Effort L | Depends: AJ-E2302, AJ-E2307
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

### AJ-E2309 Docs, Privacy and terms lines, evidence | Status: DONE-EVIDENCED (2026-11-06, release/0.4.1) | Priority P0 | Effort S | Depends: AJ-E2303, AJ-E2306, AJ-E2308
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

### AJ-E2311 0.4.6's deferred session findings: hand-over rows, New session's order, the Stop toast, the switch dialog at 1280×800, the wizard rail, recovery limits, Codex's added messages, usage readings | Status: DONE-EVIDENCED (2026-11-21: every item proven in 0.4.7 and 0.4.8, except R6's one-time upgrade edge 6(b), fixed in batch 3 for 0.5.0; R7, the live Codex check, moved to 0.5's live sign-in sitting by D-141) | Priority P1 | Effort L | Depends: AJ-E2310
**Deferred from 0.4.6's exit RC (rc3h), 2026-11-19 (D-114: the 0.4 line takes blockers only after beta).** The source list, with the walk rows that stay open and their reasons: [docs/research/0.4.6-deferred-to-0.5.md](../research/0.4.6-deferred-to-0.5.md). Work it as one batch (D-137): find, triage, fix together, one build.
**Requirements**:
1. **(rc3h item 8)** **During a terminal hand-over the sidebar shows two rows for the session.** Home's last-12-hours list shows both "h-b Finished" and "h-b Stopped by you".
2. **(rc3h item 9)** **New session suggests the maintainer's other projects first, and Recent reshuffles.** Its layout shifts when the memory line appears.
3. **(rc3h item 10)** **The Stop toast names the folder** ("Stopping web · …"), not the session.
4. **(rc3h item 11)** **The switch dialog** (Continue in another app…) at 1280×800 didn't show its "what comes along" part or a Cancel, and didn't scroll there. Check its layout at the minimum window.
5. **(rc3h item 12)** The pool wizard's steps can't be pressed from its left rail.
6. **(rc3h item 13)** **Recovery limits** (from F-rc3f-3's agent): a driven session whose conversation file was deleted still ends in a failed `-t`. A session with no turn yet, quit under a build from before d4c840a46, fails once more after the upgrade. A cross-Mac switch back takes the original's name, not one given on the other Mac. A rare race: if a live look is already running when a process ends, it can write the entry back until the next look.
7. **(rc3h item 14)** Codex: whether a message the person adds during a running Codex turn comes back as a `userMessage` item isn't verified live. If it doesn't, those messages won't show as "You".
8. **(rc3h item 15)** **Usage readings:** the Claude codec's `percent()` reads any value ≤ 1 as a fraction, so a whole "1" would read as 100%. A week on one model from the stream shows only on the session panel, still labelled "Weekly".
9. **(rc3h triage, pre-existing since 78bf9c22a; FIXED in 0.4.6's rc3h blocker batch by the coordinating session's ruling, so here only its test is tracked, 45ac2facb on release/0.4.6, tests in ui/src/components/own-conversation-page.test.tsx, reaching 0.5 with the next forward merge)** A session AI Juggler runs is shown as "Started outside AI Juggler… never stops or changes it", with no Send box: `DrivenPanel.tsx` offers "Earlier turns are in its conversation. Show them" for a session with no turn yet, and `ConversationPage.tsx` always passes `outside` to `SessionOverview`. Pass `outside` only when the conversation has no AI Juggler session (`conversation.session_id === null`), and offer "Show them" only once the conversation is written. A test for each.
**Handover note**: Item 15 is a correctness bug (a whole 1 % read as 100 %): fix it first, with a test over the codec's boundary values; it touches the honest-usage promise. Item 14 needs a live Codex check in a lab profile (real accounts read-only).
**Quality gates**: verify-static; a failing-before test per fixed item; the screens touched walked once by Sonnet and checked by Opus.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2311/manifest.md`, one row per item.

### AJ-E2312 0.4.8's walk findings: limits after a reset read the same everywhere, the Parked card, short time words, sessions after Bring in, New session's sheet and memory, the pool wizard's pick, This Mac's room, the bell's scroll, the lab tools | Status: IN-PROGRESS (batch 3 on release/0.4.9 from main, merged into release/0.5: it ships in 0.5.0 with the 0.4.8 soak's findings, no separate 0.4.9, 2026-11-21; D-137) | Priority P1 | Effort L | Depends: AJ-E2311
**Found walking 0.4.7's nightly and 0.4.8's build (2026-11-20/21), and added to as the 0.4.8 soak finds more.** The source list: `docs/qa/0.4-walks/soak-0.4.6/findings.md` (W2, V1, V2, the persona runs, LC1), then the 0.4.8 soak's. Worked as one batch (D-137): every item fixed together, one build, one re-walk.
**Requirements**:
1. **After a window resets, nothing still says the session is at its limit.** In PM5 the header kept "Limit reached · 4 min" and Home's row "Needs you — quota reached" after the reset, while Home's heading said "All clear" above "Needs you (1)"; the rail said "Asks you" while the header said "Limit reached" (SK-20). One state, read the same in the header, the rail, Home and the bell; a session at its limit that wasn't parked is told its window reset and offered a way on, as a parked one is; Home's heading never contradicts its own Needs you.
2. **The Parked card reads as a sentence** beside "Resume the same conversation" after the reset, never one word per line.
3. **Short time words are the app's own everywhere:** the session header says "1 min ago" as the rail and Home do, not "1m ago".
4. **Bring in doesn't show a running session as ended** (SK-21: after Find my apps and Bring in, the second setup's running session read as ended for about 30 s).
5. **New session:** the sheet holds still when its warning leaves (N5 about 50 px, P7 about 20 px), and "Run without a terminal" is remembered for the App it was chosen on.
6. **The pool wizard's project pick follows the template:** a pick made under one template is kept only where the new template uses a project the same way, else cleared, and Review names what is picked.
7. **This Mac:** "Plenty of room" is never green with swap nearly full (8.2 of 9.0 GB): the words and the rule agree; the Overview shows its last readings at once, never an empty Memory card, "Not measured yet" and "No AI tools are running" while tools run.
8. **The bell's panel scrolled by accessibility** drew blank and left the window taking no presses for about 4 minutes (LC1): reproduce on a Dev build and judge (the covered-window pause or a real hang); fix a real one.
9. **The lab and test tools:** a lab's PATH finds no real OpenCode, Kimi Code or Grok Build; floor_drive reaches what is drawn over main by a system-wide hit-test, as Voice Control does; lab builds don't share the Dev bundle's web storage; promote.yml's `blocker_fix` says "soaked base" (D-138).
10. **SK-12's count rechecked** with a role set (walked).
11. **Move to account waits for the profile's whole process tree and still files, never leaves its dialog stuck, says why in plain words, and deletes an undone move's backup.** The maintainer's 406 MB move on 0.4.8 was refused ("the backup of … doesn't match it", a home path in the words) and its dialog wouldn't close by Esc, × or Cancel: the Claude app's Code tab wrote the history's last lines after the profile's main process ended (the live trace: helpers gone at 0.1 s, Claude Code wrote and ended at 0.3 s, its `disclaimer` wrapper at 6.3 s). A move looks at the whole tree before the quit and waits for all of it (bounded, naming what still runs); every touched file holds still before the backup; a backup that caught a write settles and tries once more (the journal says so); the refusal is calm when nothing changed, with the path only under Copy details; the dialog shows each step and the backup's progress; an undone move's backups go, and at start those of moves already undone.
**Quality gates**: `just keep-going verify-static` and `verify-tests`; a failing-before test per fixed item; the visual and accessibility gates; the screens touched walked by Sonnet and checked by Opus on the 0.4.9 build.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2312/manifest.md`, one row per item.

### AJ-E2313 A hooked session whose turn hangs is said "may be stuck", without false alarms on long thinking | Status: TODO (after 0.5.0: a design call, 2026-11-21, below) | Priority P1 | Effort M | Depends: AJ-E2312
**Found** finishing 0.4.8 soak item SK48-7 for 0.5.0: the stuck rule (AJ-E405) leaves alone every session whose state its tool reports (its hooks, or a driven one: `runtime.is_exact`), so a Claude Code session AI Juggler launched with hooks reads Working for ever when its turn hangs (the lab's `--lab-open-turn` stand-in shows it). A session picked up after a restart is covered since 0.5.0 (its transcript's last write is its last sign of work).
**Why not in 0.5.0** (decide by the goal, calm attention): neither sign is sound yet. The terminal's output can't be it (Claude Code redraws its "Thinking…" timer all the time), and the transcript's last write can't alone (long extended thinking writes nothing for minutes), so a rule built on either would raise L2 alarms on healthy sessions.
**Requirements**:
1. Measure on real hooked sessions (the aij-test profile, Sonnet runs, Opus reads): how long thinking, a long tool call (a build, a download) and a real hang go without a transcript write, a hook event or CPU, across a day's work.
2. From those numbers, a rule for a hooked session reported Working: e.g. no hook event and no transcript write for N minutes, its tree's CPU under a floor, no running tool call (PreToolUse without its PostToolUse means a tool runs). Said as "Probably stuck" with a hollow ring (PAT-ATT-5), never louder.
3. Tests from the measured traces: a hang flagged after N; thinking, a build and a download never flagged.
**Quality gates**: `just keep-going verify-static`; a failing-before test; a live check on aij-test.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2313/manifest.md` (the measured traces, the rule's numbers).

### AJ-E2314 0.5.0's walks: the later findings, in the 0.5.1 rebuild before beta | Status: TODO (0.5.1) | Priority P2 | Effort M | Depends: AJ-E2312
**Found** in 0.5.0's final walk and its two re-walks (`docs/qa/evidence/0.5.0-final-walk.md`, `0.5.0-rewalk.md`, `0.5.0-rewalk-2.md`); triaged "later" by Opus: none blocks 0.5.0.
**UI**: yes
**Usability reasoning** (Opus, before building; re-checked on the built screen):
(1) Daily users: a session page at small window sizes, a pool's page, a new watch, Home's feed and New session. Each item fixes where a screen said less, or other, than its name promised.
(2) L4: "Always" says what it means. L3: one name for a twin everywhere. L5: no doubled finishes and no "2 min–2 min" range.
(3) Each fix is in place on the screen where the problem was; nothing new is added.
(4) n/a: no new surface.
(5) None.
(6) Watch for: New session opening on another Account than the one the user clicked (L6), and a dimmed sheet left behind a dialog (L7).
**Requirements**:
1. L1, walk kit: `dev_bridge_url.py`'s `#token=` URL connects in the browser pane (it worked only with `&token=`).
2. L2: a session page at 1024x680 with an account banner gives the right column a bigger window than 204 px of 723 (fold or shorten the banner, or give the column priority).
3. L3: a pool's page names a twin one way everywhere (the diagram's "web (2)" and Who's in's "web · started … (2)").
4. L4: the new-watch preview says what Always means (it said "Ends at midnight tonight.").
5. L5: after a digest, Home's feed doesn't show the same finishes twice, and a range never reads "2 min–2 min".
6. L6: Home's "Start a new session … Personal" opens the sheet on Personal (seen opening on Work, twice, driven by accessibility): reproduce, then fix.
7. L7: the New session sheet isn't left dimmed behind the "already working in api" worktree dialog.
8. S3, walk kit: `lab_drive resize` really resizes Dev's window (it reports success but leaves 1701x1348).
**Quality gates**: `just keep-going verify-static`; a failing-before test per app fix; the visual gate.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2314/manifest.md`, one row per item.

### AJ-E2315 Move to account counts only the moving profile's own processes: a wrapper is its by the program it runs, never by a flag's path; one shared ownership module every site uses | Status: IN-PROGRESS (0.5.1) | Priority P0 | Effort M | Depends: AJ-E2312
**Found** by the coordinating session, 2026-11-22, root-caused live: Move to account on 0.4.9 (Claude app, one profile to another) was refused every time, "didn't finish closing within 30 seconds: claude and disclaimer still run". Those two processes were another profile's: a Code-tab session of a different running Claude app profile, whose `disclaimer` wrapper had `--add-dir=<a folder inside the moving profile's folder>`. `places/tree.rs` `of()`'s third mark ("every program of the app's own bundle that names a path inside the profile's folder") matches any argument value inside the folder, so the other profile's wrapper and its `claude` child were counted as the moving profile's, never ended, and `rest_ended` timed out on every retry. The same code is on release/0.5.
**UI**: no
**Requirements**:
1. A bundle wrapper is this profile's only through the program it runs (for `disclaimer`, the wrapped executable after `--`). Flag values (`--add-dir`, `--settings`, `--plugin-dir`, `--mcp-config`, and any other) never mark a process. No special case per flag: the rule reads what runs, not what is passed (architecture standard).
2. Stronger evidence wins: a process whose executable is inside another known profile's folder, whose `--user-data-dir` is another profile's, or whose ancestry reaches another known profile's running main process is never this profile's.
3. A leftover that belongs to another running app or profile never gets a "try again" refusal (retrying can't fix it); after 1 and 2 it shouldn't occur.
4. Tests: this exact shape (another profile's disclaimer → claude, with an `--add-dir` into the moving profile) is not a member; the existing members (its own disclaimer → its own claude-code, helpers by `--user-data-dir`, the pre-quit tree) still are; the move suite passes.
**Quality gates**: `just keep-going verify-static`; failing-before tests; the move suite; a live re-walk of Move on lab PL1 with a second lab profile's Code session holding an `--add-dir` into the moving one.
5. **Widened by the switch reliability review (`docs/research/move-reliability-review.md`, B1, E1–E4, E11–E13, E17):** one shared ownership module (`features::ownership`, OS parts behind `juggler_platform`), built from `agent_bridge/attest.rs`' primitives: a strict instance parse (one `flag=value`, never after `--`), a canonical `InstanceKey`, a profile proven running by the holder of its instance lock, a `ProcId` with start time. Every site goes through it and keeps no matching of its own: `places/tree.rs`, `running_apps` and `running_now`, `conversations/places.rs`' `host_of`, `lab_scope.rs` (the same `--add-dir` class: an argument value never makes a process the lab's), `places/link.rs`' folder-name linking (evidence: the profile's sessions' pid files), and the move lock's keys. The conflict rule (main > strict helper > writer > descendant) and the self rule (refuse up front when AI Juggler's own ancestry, or the other AI Juggler's, would be quit).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2315/manifest.md`.

### AJ-E2316 The rail's gallery captures wait for the rail to be still | Status: DONE (0.5.1) | Priority P2 | Effort S | Depends: AJ-E2314
**Found** on CI for bde5fa4d8 (visual 4/4): `pools-gallery.spec.ts` "the rail's Pool lens — dark" differed by 4668 px; the actual image showed the rail's "N new above" pill and rows still moving: a notice arrived while the test's pointer held the list, and the held reorder applied after it left. No commit of that push touched the rail; a flake, fixed for good rather than by a higher threshold.
**UI**: no
**Requirements**:
1. A shared `railStill(page)` (ui/tests/visual/fixtures.ts) waits for the rail at rest: no `rail-new-above` pill, every `[data-line-key]` line's place unchanged across several checks.
2. The Pool lens capture waits on it before its hover and before its screenshot.
**Evidence/validation proof**: the capture passes 3 runs of each theme on the host (`--repeat-each=3`); CI's visual gate.

### AJ-E2317 Move picks the source by the conversation's holder and the target by the App the user picked, never by account | Status: TODO (0.5.1) | Priority P0 | Effort S | Depends: AJ-E2315
**Found** Review A2.2, A2.3, E5, E6: `places/handoff.rs` `profile_for(account)` takes the running or last-seen profile of an account; with two profiles on one account the move quits, writes into or opens the wrong one. (`docs/research/move-reliability-review.md`).
**UI**: no
**Requirements**:
1. The source is the profile whose records list the conversation (the holder); two claiming it is a Bug refusal.
2. The target is the App the user picked in the switch list: its place id carried end to end through `switch_move_execute` (actions target by id).
3. Two profiles on one account: fixtures and a PL1 walk.
**Quality gates**: `just keep-going verify-static`; failing-before tests; the move suite.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2317/manifest.md`.

### AJ-E2318 Move waits only on real writers and the profiles' main processes, decides before it disrupts, and quits in parallel | Status: TODO (0.5.1) | Priority P0 | Effort M | Depends: AJ-E2315
**Found** Review A1.1–A1.5, A1.10, A3.6, A9.2, B0.2–B0.3, B2: the whole pre-quit tree had to end (a background dev server, crashpad, another profile's wrapper blocked every retry), and ownership conflicts showed only after both apps were quit. (`docs/research/move-reliability-review.md`).
**UI**: yes
**Usability reasoning** (Opus, before building; re-checked on the built screen):
(1) Someone moving a conversation to another Account. They expect Move to quit only what holds the conversation, and to say before quitting anything if it can't go ahead.
(2) The offer names any blocker up front, in plain words ("a dev server your session started keeps running"), before anything quits.
(3) Fewer stuck moves, and no apps quit for nothing. The words stay in the Move sheet, nowhere else.
(4) When it can't move, nothing quits; the sheet says why and what to do.
(5) None new: quitting another app is an Apple event already in the registry (Automation), primed by AJ-E1547.
(6) Watch for: both apps quit and then the move refused; a background process the user cares about being killed.
**Requirements**:
1. An adapter `writers(tool_dir, conversations, table)` hook in `InPlaceMove`: Claude's `sessions/<pid>.json` (pid and procStart) and `--resume=<id>`; open-file write checks only as a backup. Run for both accounts' folders, whatever the place.
2. The wait set is the main process of each instance plus the writers; the rest of the old tree is reported ("a dev server your session started keeps running"), never waited on.
3. The offer runs ownership and writers before anything quits (≤ 500 ms) and refuses up front what it can know.
4. Both profiles quit in parallel, with per-class deadlines.
**Quality gates**: `just keep-going verify-static`; failing-before tests; the move suite.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2318/manifest.md`.

### AJ-E2319 Move's refusals are typed (transient, needs you, setup, bug), name what holds the files and carry the move id; Retry only for transient | Status: TODO (0.5.1) | Priority P0 | Effort S | Depends: AJ-E2318
**Found** Review A9.1, A3.1, A5.5, B6, E9: Retry showed for every "nothing changed" failure (each retry quit and restarted both apps again), and the UI's Undo picked the newest interrupted move, not this one. (`docs/research/move-reliability-review.md`).
**UI**: yes
**Usability reasoning** (Opus, before building; re-checked on the built screen):
(1) Someone whose move didn't go through. They expect to know why, whether trying again can help, and what to do.
(2) Every refusal names what holds the files and the next step. Retry shows only when retrying can help; otherwise "Check again" or the door to fix it.
(3) An honest reason in the Move sheet itself, replacing a generic "nothing changed, try again".
(4) n/a: it is the unavailable state.
(5) None.
(6) Watch for: Retry quitting and reopening both apps again for a cause that won't change; Undo or Finish acting on a different move than the one on screen.
**Requirements**:
1. `MoveStop { class, holders, move_id, unchanged, words, detail }` replaces `AppError::Move`.
2. Transient: Retry (auto-retried once). Needs you: no Retry; "Check again" re-reads the offer without quitting; the holder named with a door. Setup: Move hidden, Copy offered. Bug: Copy details and Report.
3. The UI targets Undo and Finish by the error's move id; never `find(interrupted)`.
4. A table test from each failure to its class and Retry's visibility, Rust and UI.
5. Left by AJ-E2318: Undo keeps the move's "still running" notes (`MoveUndoneInfo` carries them), as the move itself does.
**Quality gates**: `just keep-going verify-static`; failing-before tests; the move suite.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2319/manifest.md`.

### AJ-E2320 One shared, persistent hold marker every AI Juggler honours, and a move never quits AI Juggler's own parents | Status: TODO (0.5.1) | Priority P0 | Effort S | Depends: AJ-E2315
**Found** Review A3.4, A7.1–A7.3, A1.5, B5.1, E10, E11: `MoveHold` lived in one process's memory; the other AI Juggler's keep alive relaunched a profile mid-move (every retry), and the Dev AI Juggler started from a profile's Code tab could be quit by its own move. (`docs/research/move-reliability-review.md`).
**UI**: no
**Requirements**:
1. A shared move area for every AI Juggler on the Mac (a platform path): canonical-root lock files, the journals, and a `held/<place-key>` marker per profile written before the first quit, removed only on Committed or RolledBack, naming the move id and its owner (pid, start).
2. Keep alive, Open, Restart and automation in every AI Juggler (Dev and installed) check the marker; a hold whose owner is gone is a crashed move, not a free profile.
3. The self rule: refuse up front when the processes to be quit include AI Juggler's own ancestry or the other AI Juggler's.
4. Left by AJ-E2318: keep alive's Restart kill path (`places::restart` → `Starter::quit(pid)`) signals through `ownership::signal::quit_checked` (pid and start time checked right before), like Move's.
**Quality gates**: `just keep-going verify-static`; failing-before tests; the move suite.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2320/manifest.md`.

### AJ-E2321 A move survives a crash: startup recovery before keep alive, and a Moves door to undo or finish each move by id | Status: TODO (0.5.1) | Priority P0 | Effort M | Depends: AJ-E2320
**Found** Review A5.1–A5.4, A6.1, A6.4, B5.2–B5.5: no recovery at startup (keep alive restarted apps onto a half-moved folder), no door back to an interrupted move, Undo blocked by a full disk, committed backups kept for ever. (`docs/research/move-reliability-review.md`).
**UI**: yes
**Usability reasoning** (Opus, before building; re-checked on the built screen):
(1) Someone whose move was interrupted (a crash, a restart, a full disk), or who wants to undo one later. "Moves" promises the list of moves with Undo and Finish.
(2) Each move shows from → to, the conversation's title, the date and its state in plain words ("Interrupted: finish or undo"). Undo says what it restores.
(3) One place to see and fix moves: on the conversation's page and in Settings › Desktop apps, with nothing duplicated elsewhere.
(4) With no moves, the door is hidden, or says one line. An interrupted move is never hidden.
(5) None: undo and finish relaunch apps through the gate's existing Automation entry.
(6) Watch for: keep-alive restarting an app onto a half-moved folder before recovery; Undo of the wrong move; a crash leaving no trace on screen.
**Requirements**:
1. Startup recovery, before keep alive's first tick: Planned/Holding/Quiet/BackedUp roll back; Applying rolls back by the journal; Verified/Reopened finish forward; unclear keeps the hold and says "Finish or Undo" (bell and the conversation's page).
2. A Moves door (the conversation's page and Settings › Desktop apps): each move with Undo (committed) or Finish/Undo (interrupted), by id; the timeline in Copy details.
3. Preflight counts the room Undo needs; committed moves' backups are dropped after the next move of the family commits, or after N days.
4. Rolling forward an interrupted Applying move comes later; the release notes say so.
**Quality gates**: `just keep-going verify-static`; failing-before tests; the move suite.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2321/manifest.md`.

### AJ-E2322 Move says "opened" only when the conversation is open, and keeps a timeline of every phase | Status: TODO (0.5.1) | Priority P1 | Effort S | Depends: AJ-E2318
**Found** Review A3.7, A3.8, B7, B9 12 and 15: "opened" was reported as verified on the link's delivery alone, and a fixed 6 s settle gave false results on a slow start. (`docs/research/move-reliability-review.md`).
**UI**: yes
**Usability reasoning** (Opus, before building; re-checked on the built screen):
(1) Someone who just moved a conversation. "Opened" must mean they can see it in the target app now.
(2) "Opened in <app>" only when it is; otherwise "Opening…" or "Open it from <app>" with a button.
(3) An honest last step of the Move sheet; the timeline sits in Copy details, not on screen.
(4) When it can't be confirmed open, it says so plainly and offers Open.
(5) None.
(6) Watch for: "opened" shown while the app is still starting or shows another conversation.
**Requirements**:
1. Verified only when the adapter's after-open check sees the conversation; otherwise pending, said plainly.
2. Wait for the records' rewrite by event, capped at 15 s, instead of a fixed 6 s.
3. A `timeline` per phase in the journal (ms), each wait's set (program, reason, evidence) and what remained; Copy details is the redacted summary.
**Quality gates**: `just keep-going verify-static`; failing-before tests; the move suite.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2322/manifest.md`.

### AJ-E2323 The switch's verification matrix: every ownership shape as a fixture in CI, and the real cases walked in lab PL1 | Status: TODO (0.5.1) | Priority P0 | Effort M | Depends: AJ-E2315
**Found** Review D1–D4: the fixtures and walks that prove E2315 and E2317–E2322. (`docs/research/move-reliability-review.md`).
**UI**: no
**Requirements**:
1. D1's 13 ownership fixtures (`procs::fixture`, no OS), each asserting what blocks, what is reported and the refusal class.
2. D2's engine fault injection (crash after each phase, ENOSPC during apply and undo, two AI Juggler data folders sharing the move area) and D3's app-level integration (a second AppState's keep alive, a relaunching agent, an app asking at quit, parallel quit, startup with a held profile).
3. A PL1 lab walk (Sonnet, verified by Opus) of: two profiles on one account; the E2315 shape; another AI Juggler with keep alive; a crash mid-move then restart.
4. The walk also starts from an EMPTY Dev-identity data folder with real running lab profiles and tries every add path (Welcome's Find my apps, the rail's Add, Settings, ⌘K), then checks each App's sessions show (AJ-E2327).
5. Left by AJ-E2318: a timing test that the offer's checks (one process table, the pid files, one tree per profile) stay within 500 ms on a large fixture; AJ-E2318's evidence manifest.
**Quality gates**: `just keep-going verify-static`; failing-before tests; the move suite.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2323/manifest.md`.

### AJ-E2324 Move's speed: event-driven waits, an open-file write guard and half the reads | Status: TODO (0.5.1) | Priority P2 | Effort M | Depends: AJ-E2322
**Found** Review A4.2, A6.2, A6.3, A1.9, B3, B4, C: polling waits (about 1–2 s of CPU per move), a 1.5 s stillness floor, every byte read and hashed four times. (`docs/research/move-reliability-review.md`).
**UI**: no
**Requirements**:
1. Process-exit and file-change waits by kqueue (Linux pidfd/inotify, Windows wait handles), behind `juggler_platform`.
2. An open-file write guard before backup and apply (unknown writers: sync tools, editors).
3. Clone first, hash the clone once, cold check by identity, size and mtime: two full reads, not four.
4. Targets: a typical move ≤ 10 s p50, ≤ 20 s p95; CPU during the waits ≤ 50 ms.
**Quality gates**: `just keep-going verify-static`; failing-before tests; the move suite.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2324/manifest.md`.

### AJ-E2325 Move backups clean themselves up: refused attempts at once, committed moves after a chosen time, sizes that say what deleting frees | Status: TODO (0.5.1) | Priority P0 | Effort M | Depends: AJ-E2321
**Found** by the maintainer (2026-11-22): every successful move's backup was kept for ever. On the real Mac `moves/` held 21 GB (19 GB from one morning's move, plus four tiny folders from refused attempts).
**UI**: yes
**Usability reasoning** (Opus, before building; re-checked on the built screen):
(1) Anyone who has moved conversations. They expect backups for Undo to be kept for a while and then cleaned up, without filling the disk silently.
(2) Each backup shows its size (what deleting really frees), "Kept until <date>", and Delete backup says "You can no longer undo this move". The setting names its choices in days.
(3) Disk space back without a manual clean-up. It sits in the Moves door and in the header's "N to clean up", one surface each.
(4) With no backups, the list says nothing is kept. Interrupted moves are never deleted automatically.
(5) None.
(6) Watch for: a backup deleted while its Undo still shows; a size that frees less than it said (APFS clones); a clean-up offer during a move.
**Requirements**:
1. A refused or "nothing changed" attempt deletes its move folder at once (nothing was changed, nothing to undo).
2. A committed move's backup is kept for Undo, then deleted automatically. The setting "Keep move backups for: 1 day / 7 days (default) / 30 days / Until I delete them". The sweep runs at startup and once a day, after AJ-E2321's recovery, never while a move runs (it honours AJ-E2320's hold and the move lock). Interrupted or uncommitted moves are never deleted automatically: they go to the Moves door.
3. The Moves door (AJ-E2321) shows each move's date, from → to, the conversation's title, its backup's size and "Kept until <date>", with Undo, Delete backup ("You can no longer undo this move") and Keep; and the total. Rich mode of the design system: sticky header and footer.
4. Disk pressure: when free space falls below the disk guard's warn line, or a new move's backup won't fit (AJ-E2321's room check), offer to clear backups older than a day, naming what it frees. Never cleared silently.
5. Backups count toward the header's "N to clean up", with their size, so the existing clean-up surface finds them.
6. Sizes are what deleting frees: APFS clones share blocks, so the reclaimable amount is measured (behind `juggler_platform`), not a `du` total.
7. The first 0.5.1 sweep applies the policy to backups made by 0.4.9 and 0.5.0: under the 7-day default, the 21 and 22 Nov backups stay until 28 and 29 Nov; refused-attempt folders go at once.
**Quality gates**: `just keep-going verify-static`; tests for retention by age, never during a move, never an interrupted move, the refused-attempt clean-up, the reclaimable size and the disk-pressure offer; in AJ-E2323's matrix and its PL1 walk.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2325/manifest.md`.

### AJ-E2326 A refused move leaves the conversation where it was, never in a terminal here: every start path honours a move's hold, the source reopens it, and two live places are seen and said | Status: TODO (0.5.1) | Priority P0 | Effort M | Depends: AJ-E2315, AJ-E2317, AJ-E2320
**Found** by the maintainer (2026-11-22, checked read-only on the installed 0.4.9). Every attempt was Desktop → Desktop Move to account; no CLI was ever asked for. **The root cause is ours.** A move quits the profile's Claude app; a refused move restarted the profile but did not reopen the conversation, so it stayed not live. A conversation that isn't live puts "Continue in AI Juggler" in the very panel the user retries the move from (the ContinueOn panel's `continue-carry-in`, in place of "close it first"), and as the conversation page's primary action (`conversation-continue`), with the same icon as the "Move to account" choice (`MoveChoice.tsx:237`). The retries became a terminal session on the source Account (`conversations_continue`: an instance of kind terminal, no App, the profile's recipe; ruled out by its records: Copy, a switch, `epoch_resume`, a pickup after a restart, `continue_as`). That `claude --resume <id>` (its parent AI Juggler) ran on; the next morning the profile's Claude app's own Code child resumed the same conversation too. Two live writers on one conversation, and AI Juggler said nothing: the session showed as the Account's "Claude Code CLI" and the desktop App's row said "nothing open" while its Code child held it.
**UI**: yes
**Usability reasoning** (Opus, before building; re-checked on the built screen):
(1) Someone retrying a move that was refused. They expect the conversation back where it was, never a new terminal session they didn't ask for.
(2) After a refused or undone move, the conversation is reopened in its app. A session continued here says where it came from ("Continued here from Claude app · <profile>"). Two live places on one conversation show "Open in two places", with one click to stop ours.
(3) No accidental terminal sessions on the wrong Account; every live place shown in one view.
(4) n/a.
(5) Reopening the source app is the same Automation entry as the move's (primed by AJ-E1547).
(6) Watch for: "Continue in AI Juggler" sharing the Move icon; it showing in the move panel after a refusal; "nothing open" while the app's Code child holds the conversation.
**Requirements**:
1. **Every start path refuses a held conversation.** In the backend, not only the UI, `continue_impl`, `epoch_resume`, resume, pickup and any launch with `--resume` refuse a conversation a move holds, or one within a short settle window after a refused or undone move, by AJ-E2320's shared hold marker.
2. **A refused or undone move restores the conversation where it was.** The source profile restarts and the conversation is reopened in it by the same link the move uses for the target, so it is live there again; the reopen is checked (AJ-E2322's signal).
3. **The move panel never offers the terminal.** The Move choice gets its own icon. "Continue in AI Juggler" never shows in the switch and move panel while a move is refused or retrying. On the conversation page, when its last place was a desktop app, the primary action is "Open in <app>"; "Continue in AI Juggler" is secondary and says it starts in a terminal here.
4. **The guard holds for the whole run.** When the tool's own app opens a conversation one of our sessions holds, both rows show "Open in two places", with one click to stop ours. The other app's process is never touched (D-28).
5. **Every live place is shown.** A conversation open in a desktop app's Code child shows under that desktop App too, and "nothing open" is never said while a child of it holds a conversation (open now means running). Attribution goes through the shared ownership module (AJ-E2315).
6. **A continued session says where it came from** ("Continued here from Claude app · <profile>"), never reading as a new CLI App. After a refused move, the sheet's next steps point to Retry or the move, never to "Continue in AI Juggler".
**Quality gates**: `just keep-going verify-static`; tests: a refused move → the conversation reopened in the source app and no carry-in offered; `continue_impl` refused while held (and in the settle window); the Move and Continue icons differ; a continued session, then the desktop app resumes the same id → the notice on both rows; a desktop Code child's conversation attributed to its App; the continued-from label. In AJ-E2323's matrix and its PL1 walk.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2326/manifest.md`.

### AJ-E2327 Add on a running desktop profile brings in its Claude Code folder by evidence, so its sessions show at once; the rail keeps its place and never contradicts itself | Status: TODO (0.5.1) | Priority P0 | Effort M | Depends: AJ-E2315
**Found** on a fresh 0.5.0 Nightly install (the maintainer, 2026-11-23; checked read-only by the coordinating session): no Accounts and no launch settings; six desktop Claude App rows, all with no folder, three added by the rail's "Running, not added" Add; all six profiles running with live Code sessions, each Code child's `CLAUDE_CONFIG_DIR` pointing at its Claude Code folder. Yet each added row showed a greyed icon and "nothing open", no session appeared anywhere, the rail read "Open now 3" and "Nothing open yet — Start one with New session…" at once, and each Add scrolled the rail so its top was cut off. A blocker: a new user's first Add shows nothing. The walks missed it because they ran on setups with Accounts already imported, never fresh install → the rail's Add with real running profiles.
**UI**: yes
**Usability reasoning** (Opus, before building; re-checked on the built screen):
(1) A new user who sees their running Claude profiles under "Running, not added" and clicks Add. They expect the App to appear with its sessions, as it does in the app itself.
(2) The Add confirmation names what comes with it ("Also uses its Claude Code folder <folder> so its sessions show"). A row that can't show its work says why in plain words, never just a grey icon.
(3) The App becomes useful at once: its sessions, live state and subsessions show where every App's do; nothing new on screen.
(4) No folder found: the row says so and offers "Choose its folder…"; the grey icon is explained wherever it is used (tooltip and row words).
(5) Reading a running process's environment and the tools' folders: app-data access to other apps' folders is primed by AJ-E1547's gate.
(6) Watch for: "Open now 3" beside "Nothing open yet"; the rail jumping on Add; a folder linked by a name guess to the wrong Account; something added that the user didn't click (D-84).
**Requirements**:
1. Add (the rail's, Welcome's "Find my apps", Settings, ⌘K) on a running desktop profile finds its Claude Code folder by evidence through the shared ownership module (AJ-E2315), never a name guess and never a process's environment (D-37: the Claude app's sessions carry a sign-in token there; `tests/trust.rs` guards it): first the running Code child's own pid file (`<its folder>/sessions/<pid>.json`, the folder its `CLAUDE_CONFIG_DIR` names, written by the child itself), its pid proven the profile's by the ownership module; then the profile's own records (its Code sessions' ids matched to `<folder>/projects/*/<id>.jsonl`, which works while it is closed); then the folder AI Juggler itself starts it with. A recorded name match is kept only as a fallback that this evidence confirms or corrects (folded in from AJ-E2315 part 2's linking gap and AJ-E2317), so a profile added while not running is still linked and its sessions listed.
2. That folder is brought in where it is (nothing copied) and linked to the App; the Add confirmation says so. The user's Add is the consent (not automatic adding, D-84).
3. Its sessions, live state and subsessions appear at once, and the icon isn't greyed.
4. No folder found: the row says why and offers "Choose its folder…"; never a silent grey row.
5. The rail keeps its scroll position on Add and scrolls the added row into view; "Nothing open yet" shows only when Open now is really empty, worded for its lens; the greyed icon is explained wherever it is used.
**Quality gates**: `just keep-going verify-static`; tests: a fresh data folder with running desktop profiles whose Code children carry `CLAUDE_CONFIG_DIR` → Add → Account linked, sessions listed; a profile added while not running → still linked, sessions listed; restarting it uses its folder; no evidence → the row's words and "Choose its folder…"; scroll kept; the empty state. In AJ-E2323's matrix, and the 0.5.1 persona walk starts from an EMPTY Dev-identity data folder with real running lab profiles and tries every add path (Welcome's Find my apps, the rail's Add, Settings, ⌘K).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2327/manifest.md`.

### AJ-E2328 A move carries the worktree's ownership: the Claude app's worktree lease, its cleanup sightings, the side-session notes and the permission paths move with the conversation; a health check finds and repairs sessions already split | Status: TODO (0.5.1) | Priority P0 | Effort L | Depends: AJ-E2315, AJ-E2317
**Found** by the coordinating session on 2026-11-23 (forensics from the Claude app's own logs and each profile's `git-worktrees.json`, read-only): the 0.4.9 Move of 22 Nov carried a family of 28 session records from one desktop profile to another, but not the app's per-profile ownership of their worktrees. The source profile kept the lease (`worktrees.<name>.leasedBy`), the lease's `branch` no longer matched the worktree's HEAD, so the target app "recycled" the worktree and made a fresh one; the source profile's cleanup sweep, which tracks folders it sees but doesn't lease (`untrackedDirGc`), then reaped that fresh one 24 h later; and the app's worktree-write guard blocked the moved session's writes (`sibling_worktree`). Same pattern on 18/19 Nov. Bug class: the move carries the session's identity, not its ownership of outside resources (same family as AJ-E2315/E2317).
**UI**: yes
**Usability reasoning** (Opus, before building; re-checked on the built screen):
(1) Someone who moves a Claude conversation that works in its own worktree (the app's "isolated copy" of the repo) to another account. They expect it to keep working where it was: same folder, same branch, same files, nothing lost. A user hit by an older move sees a session that "lost its folder" or can't write, and expects AI Juggler to say why and fix it.
(2) The move preview names the worktree in plain words ("Its worktree <folder name> (branch <branch>) moves with it: the other account will own it"); no "lease", "sightings" or "untrackedDirGc" on screen. The repair names the profiles it will quit and restart, says the files in the worktree are never touched, and asks before quitting.
(3) Value: no silent data-loss risk after a move, and one-click repair of the damage older moves did. One place each: the move's own preview/plan for new moves; the health check's row (where other health findings live) for existing damage; no new screen.
(4) Nothing to carry (no worktree): the preview says nothing about it. The Claude app's format not recognised (newer/older version): the move is refused with that reason and what to do (update AI Juggler), never a half-carried move. Nothing to repair: the health check row doesn't appear.
(5) OS prompts: none new. Quitting and restarting the involved profiles uses the existing quit/start path (already primed, AJ-E1547); reading the app's files is app-data access already primed.
(6) Watch for: a repair that quits profiles the user didn't expect (name each first); a repair that touches worktree files (never); a worktree on a different branch than the record says (say it, rewrite the record's branch to the real one, never move HEAD); a worktree root another profile also sweeps (say so in the plan notes); Undo leaving one lease in each profile.
**Requirements**:
1. **Research, behind the adapter.** The formats (the app's per-profile worktree file: lease entries and the cleanup sweep's sightings; the side-session notes folder; the record's permission and shell-folder paths; the worktree-write guard's rule) are confirmed from on-disk files and the app's own logs only, written up in the adapter's docs (no app internals copied into the repo). A version gate (the file's schema version and the installed app version range we checked) refuses the move, with the reason, when a format is unknown.
2. **Carry the lease.** `ClaudeMove::plan` moves each moved session's worktree lease entry from the source profile's file to the target's as a new reversible move step with a backup; Undo restores both files byte-for-byte.
3. **Check the branch.** Beside `folder_missing`, read the worktree's HEAD; when it differs from the lease/record branch, the carried lease and record name the real branch (never a checkout), and the plan notes say so; an unreadable HEAD refuses with a reason the user can clear.
4. **Clear the sightings.** The moved worktrees are dropped from the source profile's cleanup sightings; a worktree root another profile also sweeps is named in the plan notes.
5. **Carry the notes and paths.** The parent's side-session notes folder for each moved record is carried as a host-side area (like the session areas); source-profile paths in the record's permission grants and shell folders are rewritten to the target's.
6. **The preview and Undo.** The move preview lists the worktree(s) and notes carried in plain words; Undo restores both profiles' files and the notes.
7. **Health check and Repair.** A health check finds sessions whose worktree lease sits in another profile than the one holding the session, or whose recorded worktree/branch differs from where they work (the damage older moves left). It offers "Repair ownership", which names and quits only the involved profiles (with the user's consent), moves the lease, clears the stale sightings, restarts them, and verifies; it never touches a worktree's files. Refused cleanly when a profile can't be quit or the format is unknown.
8. **Matrix.** `claude_move_matrix.rs` cases: a session with a worktree moved, then the old profile's sweep (nothing reaped); the new profile resuming in the same tree; a diverged HEAD; the notes folder; Undo restoring both leases; the format gate refusing; the repair on a fixture of a 0.4.x-split session. Fixtures and the lab only, never the maintainer's real profiles.
9. **Constant checking as Claude app versions ship** (the maintainer, through the coordinating session, 2026-11-23). (a) A known-format manifest in the Claude adapter: the keys and shapes of the session record, the per-profile worktree file (leases), the cleanup sightings, the side-session notes layout, and the log lines the forensics relied on; each entry names the Claude app versions it was verified on. (b) `scripts/claude_compat_check.py`: read-only, no network, no environment reads; parses the installed Claude app's version and every profile's on-disk formats by structure only (never prints contents), compares them with the manifest, and prints a short JSON verdict (`ok`, `drift` with the exact keys, or `unknown_version`) with distinct exit codes; tested on fixture profiles. (c) The app's version gate (req 1) reads the same manifest; on an unknown Claude app version the move and the repair refuse in plain words; Settings › Apps shows "Claude app <version> · moves checked" or "· not yet checked". (d) Documented in `docs/design/` beside the adapter docs.
**Quality gates**: `just keep-going verify-static`; `cargo test -p juggler-adapters --test claude_move_matrix`; the repair's IPC and UI tests; in AJ-E2323's PL1 walk: a refused then successful move of a session in its own worktree, checking the lease follows it.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2328/manifest.md`.
