> **Decided (D-66, 2026-11-03).** The plugin mod and the terminal permission relay are dropped: a session a desktop app holds is answered in that app, or continued here as a CLI session through the switch. The richer conversation, find, the composer for sessions AI Juggler runs, and the reply area are E34 in 0.5 ([`docs/tasks/E34-reply-and-read.md`](../tasks/E34-reply-and-read.md)). The rest of this file is the research as written.

# Quick replies and a richer chat: what 0.5 can do

Research for release 0.5, 2026-11-03. Asked for by the maintainer (2026-11-03): *"send chat prompts to Native Desktop apps. Sometimes you just want quick reply or decision without going to the app. Also expanding the chat capabilities so our chat screen shows images, and other capabilities that claude, codex and other chats offer. Right now is great but bit too primitive."*

**How this was done.** I read the repo (`release/0.4` worktree) and searched the web. Nothing was run against a real app or account: no sign-in, no Allow, no transcript, credential, Keychain or app-storage contents read, and no process environments.

**Labels** (as in `docs/research/EVIDENCE-STANDARD.md`):
- **VP**: the vendor's own docs or code say it.
- **VS**: a reliable second-hand source.
- **NE**: searched and not found.
- **I**: my inference.
- **U**: unverified.

Facts I re-read today carry "(read 2026-11-03)". Facts taken from the repo's own research of 2026-10-25/27 cite that file and its original URL, and were not re-fetched unless the line says so.

---

## Executive summary

**Quick replies: what is possible.** A reply can only reach a session through one of five official doors. Which ones apply depends on who runs the session.

1. **Sessions AI Juggler drives.** Codex runs through app-server by default; Claude runs through stream-json as a Beta. A reply is already possible here (`DrivenPanel.tsx`).
   - What's missing is a real composer: images, slash commands and skills, `@files`, and replying from Needs you or a notification.
   - Both protocols accept images: Claude's stream-json takes image content blocks (VP), and Codex's `turn/start` takes `localImage` and `image` items (VP).
   - **Recommended, first.**
2. **Terminal sessions AI Juggler started in its own PTY.** AI Juggler already has a keyboard path into its own terminal (`terminal_write_impl`).
   - A quick-reply box that types the user's words into that terminal is just the user's own keyboard. It is clearly allowed and needs no new permission.
   - It is safe only when the session is exactly idle or working, never while a permission or question dialog is open: typed keys could answer the dialog. The hooks overlay (AJ-E2306) gives that exact state.
   - **Recommended, with that gate.**
3. **Sessions nobody has open** (idle or ended, with no process holding them). "Reply" can mean **continue it here**: resume it under AI Juggler's driven runtime and send the turn.
   - The resume uses `thread/resume` for Codex and `claude -p --resume <id>` (Beta) for Claude.
   - The one-writer rule must hold.
   - **Recommended for Codex; Claude follows its Beta.**
4. **Sessions open in the Claude app's Code tab, or in another terminal.** The only official ways in are Claude Code's own extension points, installed into the user's profile with consent:
   - a plugin **mod** whose `$.prompt.submit({ text, asUser: true })` "waits until the session is idle and then starts a new turn" (VP; mods run in the Desktop Code tab);
   - the per-session **inbox socket** (VP; but its messages arrive as "from another session, not from you", so they're the wrong voice for the user's own reply).

   Both ride on AI Juggler's planned Claude Code plugin (AJ-E1905, waiting on D-43 point 6). **Possible with caveats; Beta behind a lab.**
5. **The Claude app's Chat, Cowork, and ChatGPT's Codex app.** There is no sanctioned way in.
   - Deep links only prefill a **new** chat.
   - UI scripting is against the terms ("automated or non-human means") and fragile.
   - The apps' private IPC (`codex_app` pipe, the Claude app's stores) is off-limits.
   - The vendors' own answer to "reply without going to the app" is **Remote Control**: Claude's from claude.ai/code and the phone, including images and permission prompts (VP, read 2026-11-03); OpenAI's from the ChatGPT phone app (VP).
   - AI Juggler's honest version is **"Reply in the app"**: it copies the typed reply and opens that exact session (Go to it), and the user presses ⌘V ⏎.
   - **Not advisable** to go further.

**Answering a "needs you" (a permission or a question) from outside.**
- **Driven sessions:** exact and done (AJ-E2307).
- **Claude terminal and Code-tab sessions:** still unsolved. There are two candidates, both unverified and both lab-first:
  - the documented "whichever decides first applies" PermissionRequest hook;
  - the channel permission relay, which is research preview, interactive only, and needs a development flag in every session or Anthropic's allowlist.
- **Until a lab proves one:** "Answer in the terminal" or "Answer in the app".

**A richer chat: what is possible.** The transcripts already carry far more than AI Juggler draws:
- images in your messages and in tool results (screenshots);
- edits with their old and new text;
- command output;
- todo lists and plans;
- thinking;
- per-message token usage;
- subagent links.

ADR-0024's feed keeps only text, narration and one-line steps (`conversation_feed.rs`). The work is mostly backend typing plus a few renderers, all inside the feed's existing bounds. The heavy parts (image bytes, long outputs, full diffs) are fetched lazily by byte offset, so feeds stay small. **Recommended.**

**The recommendation for 0.5** is a new area, "Reply and read in AI Juggler", filed as E34. Proposed order:

1. **Typed conversation, one pipeline.** Images, diffs, command output, todos and plans, thinking, per-turn tokens and cost, and subagent cards. Driven sessions read the same feed, so there is one renderer.
2. **Find in conversation** (⌘F), backed by the existing contentless search index.
3. **One composer** with text, pasted and dropped images, a slash and skill picker, and an `@file` picker. It serves driven sessions, AI Juggler's own terminal sessions (typed, state-gated) and resume-and-reply for idle sessions. It is reachable from the session page, from a Needs you row, and from a notification ("Reply…" opens a quick-reply sheet).
4. **"Reply in the app"** (copy and open) for sessions an app holds.
5. **Behind a lab, as a Beta:** the plugin mod for the Code tab, and permission answering for terminal sessions.

Each adapter declares what it supports, so core and UI never branch on a tool (ADR-0019).

---

## 1. Where AI Juggler is today (repo, read-only)

| Area | What exists | Files |
|---|---|---|
| Runtimes | `Runtime::Terminal` (PTY, every tool) and `Runtime::Driven` (Codex app-server by default; Claude stream-json, Beta, off by default). "Answers are always the user's" (D-28 as amended) | `docs/adr/0022-session-runtime.md`; `docs/tasks/E23-session-runtime.md`; `crates/juggler-adapters/src/codex_appserver/`, `claude_stream/` |
| Sending to a driven session | `runtime_send` and steer from a single-line `<input>`; asks shown with the tool's own options (`AskCard`); Interrupt; Open in terminal | `ui/src/pages/session/DrivenPanel.tsx`; `crates/juggler-app/src/features/runtime/` |
| Turn input | `TurnInput { text, origin }`: **text only, no attachments** | `crates/juggler-adapter/src/runtime/events.rs` |
| Typing into AI Juggler's own terminal | `terminal_write_impl` → `supervisor.write_input` (the embedded terminal's keyboard) | `crates/juggler-app/src/terminal.rs`; `crates/juggler-supervisor/src/supervisor.rs:1046` |
| Exact state for terminal sessions | A per-launch `--settings` overlay with hooks and a status-line wrapper (AJ-E2306) | `crates/juggler-adapters/src/claude_hooks.rs` |
| Sessions other apps run | Read-only: pid files, the Claude app's `local_<id>.json`, transcripts; "Go to it"; hand-off *into* the Claude app with `claude://resume?session=` sent to the profile's own process (0.3.5) | `crates/juggler-adapters/src/claude_live.rs:250`; `crates/juggler-app/src/features/places/handoff.rs` |
| Conversation feed | Append-only by byte offset, followed while shown, ≤300 turns / 4 MiB per feed, ≤8 MiB per line (longer lines are passed over), redacted as read (D-14). Blocks: `Text`, `Narration`, `Steps` (a one-line title per call, a few inputs, `failed` and file `kind`), thinking strings | `docs/adr/0024-conversation-feed.md`; `crates/juggler-adapters/src/conversation_feed.rs:60-330` |
| Rendering | Hand-written markdown (headings, lists, quotes, tables, fences, inline marks; links shown, never followed; no HTML). Claude-style stretches and Codex-style "Worked for" folds by `layout`. No syntax highlighting, copy button, images, diffs or outputs | `ui/src/pages/session/Markdown.tsx`, `ConversationTurn.tsx`, `TurnStretches.tsx`, `TurnWork.tsx` |
| Images | Counted, not shown. Claude: `image`/`document` content parts; Codex: `input_image` parts | `crates/juggler-adapters/src/claude_transcript.rs:258-276`; `codex_transcript.rs:266-302` |
| Search | Contentless outside-conversation index (terms and line positions; words re-read from the tool's file); AI Juggler's own sessions in AJ-E1007's index | `crates/juggler-app/src/features/outside_search/mod.rs` |
| Webview | CSP `img-src 'self' data:`; UI dependencies are only `react` and `react-dom` | `crates/juggler-app/tauri.conf.json:26`; `ui/package.json` |
| Notifications | `tauri-plugin-notification` 2.6 (a click reaches `on_action`) | `crates/juggler-app/Cargo.toml:71-73` |
| Prior research this builds on | Every Claude, Codex and other-tool control surface, as of 2026-10-27 | `docs/research/capabilities/{claude-code,codex,opencode,cursor-antigravity-kiro,zcode-gemini,kimi-grok,hermes-muse}.md`, `FINDINGS.md`; `docs/research/session-runtime.md`; `docs/research/native-conversation-display.md` |

**House rules that shape the answer.**
- Enhance, never replace: the user's own tools, through their official APIs and CLIs.
- ADR-0003: state changes only on the user's click, no UI automation, no proxy.
- D-28 as amended: never answer by itself.
- D-37: no process environments and no chat caches.
- D-14: redact.
- ADR-0019: no per-tool branches outside adapters.
- D-58: local caches are registered, owner-only and clearable.
- OS seams for Windows (0.6) and Linux (0.7).

---

## 2. Sending a prompt or a quick answer, per tool

### 2.1 Claude Code CLI

**a) Sessions AI Juggler drives (stream-json, Beta).**
- **Mechanism.** `claude -p --input-format stream-json --output-format stream-json --permission-prompt-tool stdio`, already built (ADR-0022, AJ-E2308).
  - A user message can carry content blocks, images included: "Image uploads: attach images directly to messages", with `{type:"image", source:{type:"base64", media_type:"image/png", data}}`. Single-message mode does **not** support images. VP, https://code.claude.com/docs/en/agent-sdk/streaming-vs-single-mode (read 2026-11-03).
  - Permission prompts and AskUserQuestion arrive as `can_use_tool` and are answered over the protocol. VP, `session-runtime.md` §3.1.
- **Live vs idle.** Live by design. A reply sent mid-turn is queued, and interrupt exists.
- **Reply path.** The stream, and the same `projects/<cwd>/<id>.jsonl` transcript the feed follows.
- **Latency.** Immediate.
- **Reliability.** The control protocol is "not documented as a public contract". It is pinned per version, and an unknown version falls back to the terminal (ADR-0022).
- **TCC.** None.
- **Terms.** As ADR-0022 says: the user's own unmodified binary is allowed. `-p` is "the Agent SDK via the CLI", so it stays a Beta behind four gates.
- **Windows and Linux.** Pipes, portable.
- **Verdict: recommended.** Add images, slash commands and `@files` to the composer.

**b) Terminal sessions AI Juggler started (its own PTY).**
- **Mechanism.** On the user's click, AI Juggler types the reply into the PTY with `terminal_write_impl`. That is the same thing as the user typing in AI Juggler's terminal view.
  - Claude Code queues "messages typed while Claude works".
  - A paste over 800 characters or 3 lines collapses to `[Pasted text #N]`, and "Claude is told pasted text may hold instructions the user didn't write". VP, https://code.claude.com/docs/en/interactive-mode (from `capabilities/claude-code.md` A4).
  - So a short reply is typed, and a long one is sent as a bracketed paste, knowing that marker.
- **The safety gate.** Never type while a permission or AskUserQuestion dialog is open: a digit, `y` or Enter could answer it. The route is offered only when the exact state is `working` or `idle`, from:
  - the PermissionRequest, Notification and Stop hooks (AJ-E2306), or
  - OSC 7501, which Claude Code emits to a terminal that answers its probe (VP, https://code.claude.com/docs/en/terminal-config, ≥2.1.295, from the repo's research).

  The state is re-checked just before writing. Text is stripped of control and escape characters, so a reply can't inject terminal sequences (I).
- **Live vs idle.** Both. Typed mid-turn, the reply is queued by Claude Code itself.
- **Reply path.** The feed (transcript).
- **Latency.** Immediate, plus the feed's 150–500 ms batch.
- **Reliability.** High for text. Images can't be pasted through PTY bytes; whether typing an image file's path attaches it is **U**.
- **TCC and terms.** None. It is the user's own keyboard in the user's own TUI.
- **Windows and Linux.** ConPTY and openpty, through the same supervisor seam.
- **Verdict: recommended for text replies, gated on exact state.** Answering a dialog by keystroke (e.g. "1"/"2") is **not advisable**: dialog layouts change between versions and a wrong key approves.

**c) Answering a permission or question in a terminal session (Needs you).**
- **PermissionRequest hook carrying the user's answer.** Hooks can return allow or deny. With a `--permission-prompt-tool` or SDK host, "whichever decides first applies" (VP, hooks page).
  - Whether a late hook decision closes the **TUI's** dialog is **U**: Lab 2 saw the dialog drawn 53 ms before the hook started (`session-runtime.md` Lab 2).
  - If it holds, it is the cleanest route: no flags, and it also reaches the Code tab through a plugin hook.
  - **Possible with caveats; lab first.**
- **Channel permission relay.** An MCP server declares `claude/channel` and `claude/channel/permission`. It receives `notifications/claude/channel/permission_request {request_id, tool_name, description, input_preview}` and answers `notifications/claude/channel/permission {request_id, behavior}`. "Both stay live … Claude Code applies whichever answer arrives first and closes the other." VP, https://code.claude.com/docs/en/channels-reference (read 2026-11-03).
  - **Catches** (VP, same page):
    - research preview;
    - custom channels need `--dangerously-load-development-channels`, with a full-screen warning at each launch, or Anthropic's allowlist (official marketplace) or a Team/Enterprise `allowedChannelPlugins`;
    - the flag is ignored in `-p` and the SDK;
    - "Relay covers tool-use approvals like Bash, Write, and Edit"; whether AskUserQuestion relays is **U**;
    - `description`/`input_preview` are sanitised, masked and capped at 3,500 code points.
  - **Possible with caveats; lab only** until AI Juggler is allowlisted.
- **AskUserQuestion answers.** Only driven mode answers these today (`updatedInput.answers`). PreToolUse `defer` lets a driven session hold one with no process alive (VP, hooks page; `-p` only).

**d) Claude Code sessions running in another terminal (iTerm, Ghostty, tmux), not started by AI Juggler.**
- **Inbox socket.** VP, https://code.claude.com/docs/en/cross-session-messaging (read 2026-11-03).
  - Each session (interactive, and `-p` unless `--bare`) binds a user-only Unix socket (a named pipe on Windows).
  - The path and a per-session token are exported to hooks and Bash as `CLAUDE_CODE_MESSAGING_SOCKET` and `CLAUDE_CODE_MESSAGING_TOKEN`. The docs name the use: "when you want a script or hook to post into a session".
  - Delivery: "between tool calls during an active turn … When the receiving session is idle, Claude Code starts a new turn with the message".
  - **But:** "Claude Code tells B's Claude that the message came from another session, not from you". It "can't approve anything", commands in it don't run, and inbound rules (`crossSessionInbound accept|hold|refuse`) apply.
  - On macOS the auth line is optional. On Windows it is required.
  - **Discovery without reading environments (D-37):** the path is known only when a hook AI Juggler installed reports it. Scanning `/tmp/cc-socks-<uid>` is undocumented (**U**, not advisable).
  - **Verdict: possible with caveats.** It suits AI Juggler's *own* notes (pool messages, E18) better than the user's own reply, because it is labelled as a peer.
- **`claude agents --json`.** "The supported way to read session state from outside Claude Code", with `waitingFor` (VP, agent-view page, from the repo's research). It reads state only. There is "no documented programmatic way to answer a permission or send a reply to a background session" (NE, ibid.).
- **Deep link `claude-cli://open?q=`.** Opens a *new* terminal window with the prompt filled in and never sent (VP, deep-links page). It can't reach an existing session.
- **Verdict for sessions in other terminals:** read-only plus "Go to it" in 0.5. The plugin route (§2.2a) reaches them too, once installed.

**e) Idle sessions nobody holds.**
- **Mechanism.** `claude --resume <id>` driven by AI Juggler (`-p` stream-json, Beta) or in AI Juggler's terminal. Headless sessions write the same transcript (VP, sessions page).
- **One-writer rule.** "If you resume the same session in two terminals without forking, messages from both interleave" (VP). AI Juggler offers this only when the conversation has no `places` and no live process.
- **Verdict: recommended,** as "Continue here", through the runtime that already exists.

### 2.2 The Claude desktop app (Code tab, Chat, Cowork)

**a) Code-tab sessions.** The Code tab runs the embedded Claude Code (VP, desktop page). It has no per-launch flags: "Scripting and automation … Not available" (VP, from the repo's research). The doors:

- **A plugin mod.** Mods run in "CLI, the Desktop Code tab (not WSL), VS Code and `-p`" (VP, mods overview, from the repo's research).
  - `$.prompt.submit({ text })` starts a turn. With `asUser: true` the text is sent "as the user's own words, without that sentence"; the call "waits until the session is idle and then starts a new turn". VP, https://code.claude.com/docs/en/plugins/mods/api (read 2026-11-03).
  - A mod can reach AI Juggler with `$.process.spawn` (a long-lived `juggler-agent watch <session>` that prints queued replies) or `$.http.fetch` to a loopback listener. Limits: 10 s of the hook's own execution per event, excluding mods API calls; `$.http.fetch` has 30 s (VP).
  - After the user presses Stop, `prompt.submit` is refused "until the user's next input" (VP). That is a good property: AI Juggler can't restart a stopped session.
  - **Org controls** can stop mods (`allowManagedModsOnly`, `disableAllHooks`). The built-in guard `sec-default` may skip some hooks of user-installed mods under Team/Enterprise sign-in. VP, https://code.claude.com/docs/en/plugins/mods/events.
  - **Consent:** installing the plugin into a profile is a settings write the user approves (I).
  - **Must never** answer `tool.check`/`tool.call` by itself (D-28). A mod *can* answer a tool call ("No permission prompt appears"), which is exactly why AI Juggler's mod mustn't register those hooks. AskUserQuestion and permission answering stay on the hook-relay track (§2.1c).
  - **Verdict: possible with caveats.** It is the only sanctioned route that sends the *user's own* words into a live Code-tab session. Beta, lab-gated, part of the AJ-E1905 plugin (D-43 point 6).
- **The inbox socket.** Code-tab sessions bind one ("A session in the VS Code extension or the Desktop app can't show the dialog", VP, cross-session-messaging). Its discovery needs the same plugin (a SessionStart hook reporting the path and token). It is the peer voice. **Possible; second choice.**
- **Remote Control.** The vendor's own "reply without the app":
  - "Connect new sessions to Remote Control" in Settings > Code, or `/remote-control` in a session;
  - then the session is driven from claude.ai/code or the phone: messages, "Send images and files", permission prompts and AskUserQuestion kept open until answered, and push "when actions required";
  - Pro, Max, Ultra, Team and Enterprise; API keys are not supported.

  VP, https://code.claude.com/docs/en/remote-control (read 2026-11-03).
  - AI Juggler can't be a Remote Control client: it goes through Anthropic, and embedding claude.ai would mean hosting a claude.ai sign-in, which the legal page forbids.
  - AI Juggler *can* say so in Settings and Needs you ("Want to answer from your phone? Turn on Remote Control in the Claude app").
  - **Verdict: point to it.**
- **Deep links.** Documented links only open or prefill: `claude://claude.ai/new?q=`, `claude://claude.ai/chat/{id}`, `claude://code/new?q=&folder=` ("Only the first folder value is applied today"; `file` "Accepted but not currently supported"); `q` is cut at about 14,000 chars. VP, https://support.claude.com/en/articles/14729294-open-claude-desktop-with-a-link (read 2026-11-03).
  - No link prefills an *existing* Code session.
  - `claude://resume?session=` is undocumented (VP binary strings, from the repo's research); AI Juggler already uses it for hand-off.
  - **Verdict:** use it for "Reply in the app": copy the reply, open the session, the user pastes.
- **Accessibility and AppleScript.** The app has no scripting dictionary (VP, Info.plist, from the repo's research). Driving the composer through AX is "automated or non-human means" under the Consumer Terms, and needs the Accessibility TCC grant. **Not advisable** (D-37 already rejected it).
- **Idle Code-tab sessions.** "Continue here" (§2.1e) works only when the app no longer holds the session. If it is still in the app's sidebar, two writers are possible and how the app reconciles a turn written elsewhere is **U**. **Not advisable while the app holds it.**

**b) Chat and Cowork.**
- Chats live on Anthropic's servers. "There is no sanctioned way to insert a message beyond a prefilled `claude://claude.ai/new?q=`" (NE, from the repo's research).
- Cowork: only `claude://cowork/new?q=`. Dispatch (phone to Desktop) is Anthropic's own.
- **Verdict: not advisable** beyond "open with prefill" or "copy and open".

### 2.3 Codex CLI and ChatGPT's Codex app

- **Sessions AI Juggler drives (app-server, the default).**
  - `turn/start` takes input items `text`, `image` (`url`), `localImage` (`path`) and `skill` (`name`, `path`). `turn/steer` adds to a running turn. Approvals and `item/tool/requestUserInput` are server requests. VP, https://learn.chatgpt.com/docs/app-server (read 2026-11-03).
  - Terms: "If you've built a local or open-source application … you can continue using it … App-server authentication has never been permitted for commercial or hosted services" (VP).
  - **Verdict: recommended.** Add images (pasted images saved to AI Juggler's `0700` runtime folder and passed as `localImage`; never a remote `image` URL), skills from `skills/list`, and steer from Needs you.
- **Codex terminal sessions AI Juggler started (`--no-daemon` PTY).** The same typed quick reply as Claude (§2.1b), gated on exact state.
  - Codex terminal state is a rollout-tail guess today. A Codex `Stop` hook is trust-gated per hash (VP, D/hooks, from the repo's research).
  - Since Codex defaults to driven, this matters less.
  - **Possible; low priority.**
- **Codex TUI sessions the user started (daemon mode, the default since 0.157).** `codex queue --thread <id|name> --message TEXT` is an official CLI command (VP, D/developer-commands, from the repo's research).
  - AI Juggler doesn't own the daemon (ADR-0010).
  - What the TUI shows for a queued item is **U**.
  - **Possible with caveats; lab.**
- **Stored, idle threads.** An AI Juggler-owned app-server can `thread/resume` and `turn/start`. `thread/read` and `thread/turns/list` (with `itemsView`, `cursor`) read without resuming (VP).
  - One writer: only when no app or TUI has the thread loaded. The desktop app's server can't be asked, so this is offered only for threads with no live process (I).
  - **Recommended** as "Continue here".
- **ChatGPT's Codex app.**
  - Its app-server runs on stdio pipes, with nothing to attach to. AppleScript is disabled. The private `codex_app` tools pipe is off-limits. Deep links `codex://threads/<id>` open a thread; `codex://new?prompt=` only "Sets the initial composer text" of a new one. VP, from `capabilities/codex.md` B1.
  - OpenAI's own Remote Control: from the phone you can "Send follow-up instructions, answer questions, and steer active work" and "Approve commands"; the host is the ChatGPT desktop app; "you can't set it up from the Codex CLI or IDE extension". VP, https://learn.chatgpt.com/docs/remote-connections (read 2026-11-03; developers.openai.com/codex/app/mobile redirects there).
  - **Verdict:** "Reply in the app" (copy and open the thread), and point to Remote Control.

### 2.4 The other tools the roadmap names (brief; details in `docs/research/capabilities/`)

Most of them speak **ACP** or their own protocol, so for sessions AI Juggler drives, "reply with images" is part of the protocol. ACP image blocks are `{type:"image", data, mimeType}`, gated by the agent's `promptCapabilities.image` (VP, https://agentclientprotocol.com/protocol/content, read 2026-11-03).

| Tool | Reply into a driven session | Reply into the user's own live session | Terms note |
|---|---|---|---|
| OpenCode | v2 `POST /api/session/:id/prompt {text, files, …, delivery: "steer"\|"queue"}`; Desktop renders it live (VP, opencode server docs, via `capabilities/opencode.md`) | Same API, if the user runs the shared service | ToS bars multi-account limit circumvention |
| Cursor CLI | ACP `session/prompt` (`agent acp`), plus extensions `cursor/update_todos`, `cursor/generate_image` (VP, https://cursor.com/docs/cli/acp, via the repo) | None found | AUP bans "automated or non-human means": a ruling first (AJ-E1609) |
| Gemini CLI | ACP via `gemini --acp`, or `-r <id> -p` | None; no desktop app | Only the official binary; never its OAuth |
| Kiro CLI | ACP (`kiro-cli acp`) | None | "Third-party automation harnesses" are banned: ask AWS first |
| Kimi Code | ACP `kimi acp` (submit, steer, approvals) | None | "Interactive use only": a person drives every turn |
| Grok Build | ACP | None | — |
| Hermes | ACP; the gateway's `prompt.submit`/`session.steer`, including sessions the TUI or Desktop shows (VP) | Gateway or Desktop plugin (`host.composer.submit`) | One Portal account per person |
| Muse | MSP `turn/start`/`steer`, `userInput/answer` (VP) | MSP `session/resume` adds a subscriber | — |
| ZCode, Antigravity | None (no ACP for ZCode; Antigravity consumer plans: wrapping "not supported") | None | Track-only |

**Verdict:** each new 0.5 adapter declares its reply routes in the same contract (§4.2). No per-tool code reaches the UI.

### 2.5 Comparison

| Session | Route | Sends as | Answers asks | Images | Live / idle | macOS prompt | Win/Linux | Verdict |
|---|---|---|---|---|---|---|---|---|
| Claude, driven (Beta) | stream-json | The user | Yes (`can_use_tool`) | Yes (base64 blocks) | Both | None | Yes | **Recommended** |
| Codex, driven | app-server | The user | Yes (server requests) | Yes (`localImage`) | Both (steer) | None | Yes | **Recommended** |
| Claude or Codex, AI Juggler's PTY | Typed on click, state-gated | The user | No (opens the terminal) | No (U for paths) | Idle or working only | None | Yes | **Recommended** (text) |
| Idle, not held anywhere | Resume in a driven runtime | The user | Yes | Yes | Idle only | None | Yes | **Recommended** (Claude follows its Beta) |
| Claude terminal sessions, asks | PermissionRequest hook / channel relay | The user's click | Permissions; questions U | — | Live | None | Yes | **Lab first** |
| Claude Code tab or other terminal | Plugin mod `prompt.submit(asUser)` | The user | No | U | Waits for idle | None | Yes | **Possible, Beta** (AJ-E1905) |
| Same | Inbox socket (via plugin hook) | A peer, "not from you" | No | No (text only) | Both | None | Pipe and token on Windows | **Possible** (better for E18 notes) |
| Codex TUI (daemon) | `codex queue` | U | No | No | Queued | None | Yes | **Lab** |
| Claude app Chat/Cowork; ChatGPT Codex app | Copy and open (deep link / Go to it) | The user, in the app | In the app | In the app | Both | None (Apple events to the profile, as in 0.3.5) | Links exist on both | **Recommended fallback** |
| Any desktop app | Vendor Remote Control | The user | Yes | Yes | Both | — | — | **Point to it** |
| Any Electron app | AX / AppleScript UI scripting | — | — | — | — | Accessibility | No | **Not advisable** (terms, fragility) |
| Any app | Private IPC / stores (`codex_app`, the Claude app's caches) | — | — | — | — | — | — | **Off-limits** |

**The return path is already solved.** Every recommended route ends in the tool's own transcript or rollout, and the ADR-0024 feed already follows those live. So a reply sent from Needs you shows up in the session page, the rail state and notifications with no new plumbing. Driven sessions also stream deltas.

### 2.6 Where a quick reply lives in the UI (decided by the goal)

- **The session page:** one composer under the conversation, the same component for every route, saying in one line how it will arrive. Examples: "Sends as your message", "Typed into its terminal", "Continues it here, in AI Juggler", "Opens the Claude app with your reply copied".
- **Needs you (Home and the rail):** each row gets an inline "Reply…" field, plus the ask's own options when the route can answer. This is the "decision without going to the app" case.
- **A notification:** "Reply…" opens a small quick-reply sheet over AI Juggler, focused on that session. Whether macOS inline text replies (`UNTextInputNotificationAction`) work through `tauri-plugin-notification` is **U**; the sheet works either way.
- **⌘K:** "Reply to <session>…".
- **Guardrails**, extending the AJ-E2307 test:
  - only a user gesture in AI Juggler's main window can send;
  - no timer, rule, pool member or agent reaches a reply route;
  - each send is audited as sent, with no text kept;
  - every reply spends quota like any prompt, and the composer says so where it matters (a weekly window over 95 %).

---

## 3. A richer chat screen

### 3.1 What the tools' own chats show (survey)

| Capability | Claude Code tab | claude.ai / Claude app chat | ChatGPT Codex app | Cursor | AI Juggler today |
|---|---|---|---|---|---|
| Images you attach (paste or drop) | Yes: "images, PDFs, and other files via the **+** button or drag-and-drop" (VP, desktop page) | Yes (I, general knowledge) | Yes (I); screenshots in results (VP, remote-connections) | Yes (I) | Counted only |
| Screenshots and images in tool results | Yes: browser and preview screenshots (VP, desktop page, "takes screenshots") | — | "screenshots" (VP) | Inline image replies not supported (VS, MCP README via search) | No |
| Edits as diffs | Changes card, a diff pane with line comments (VP) | — | "diffs" (VP) | Yes (I) | One line ("Updated a.ts") |
| Command output | Verbose view: "Every tool call, file read, and intermediate step" (VP) | — | "terminal output" (VP) | Yes (I) | No |
| Thinking | "Thinking" view mode (VP) | Yes (I) | Reasoning hidden in the CLI (VP, repo) | — | Captured, behind a fold title |
| Plan, todos, tasks | Plan pane, tasks pane (VP) | — | `turn/plan/updated` (VP) | `cursor/update_todos` (VP) | No |
| Subagents | Subagent pane (VP) | — | Active/Done panel (VP, repo) | — | Subagents tab (good) |
| Code blocks: copy and highlight | Yes (I) | Yes (I) | Yes (I) | Yes (I) | Monospace, no copy |
| Tables | Yes | Yes | Yes | Yes | Yes |
| Mermaid | **Not rendered** in the Code tab (VS, https://claudeissues.com/issue/52517-claude-desktop-mermaid-code-blocks-not-rendered-in-claude-code-tab) | In artifacts (VS) | U | Partial and buggy (VS, https://forum.cursor.com/t/mermaid-timeline-diagrams-fail-to-render-in-cursor-chat/166749) | No |
| LaTeX | U | U (no official page found, NE) | U | U | No |
| Artifacts | No | Yes (chat only) | — | — | n/a |
| Citations | U | Web search citations (I) | — | — | No |
| Context, usage, cost | Context % card, `/context` (VP) | — | Tokens (VP) | — | Usage tab; not per turn |
| Search in a conversation | Find (I) | Yes (I) | U | U | No |
| Composer extras | Suggested prompts, `@` files, `/` commands while working, connectors (VP) | Attachments (I) | Skills, `@` (I) | `@`, images (I) | Text only, driven sessions only |

The rule from 0.3.4 still holds: *"the default view matches the tool's own app as closely as possible; the in-depth views go further"* (`native-conversation-display.md`). So the default view should look like the Code tab's Normal view: images, diffs on demand, plan and todos, thinking behind "Show thinking". Verbose detail sits one click deeper. Mermaid and LaTeX are not parity items: the Code tab itself doesn't render Mermaid.

### 3.2 What the transcripts actually carry

**Claude Code JSONL** (`projects/<cwd>/<id>.jsonl`; subagents in `<session>/subagents/agent-<id>.jsonl`).
- **Stability:** the format is "internal to Claude Code and changes between versions" (VP, sessions page, from the repo's research), so parsing stays version-tolerant and fixture-tested.
- **Content parts:** `text`, `thinking` (and the `signature` narration marker), `tool_use {id, name, input}`, `tool_result {tool_use_id, content, is_error}`, and **`image` / `document`** (VP local: the repo's own parser counts them, `claude_transcript.rs:273`).
  - An image part's shape is the Messages API one, `{type:"image", source:{type:"base64", media_type, data}}` (VP for stream-json input; that pasted images are stored inline in the JSONL the same way is **I**, consistent with the counting).
  - Tool results can carry images too, e.g. a browser screenshot or a Read of a PNG (I).
- **Tool inputs:**
  - Edit: `old_string`/`new_string` (and `replace_all`);
  - Write: `content`;
  - Bash: `command`, `description`;
  - TodoWrite: `todos[{content, status, activeForm}]`;
  - ExitPlanMode: `plan`;
  - WebSearch/WebFetch: `query`/`url`;
  - Agent: `description`, `subagent_type`.

  The repo already reads several of these (`CallInput`). The rest is **I** from the tools' published schemas.
- **`toolUseResult`:** the repo reads its `type` (create/update). Whether it carries a `structuredPatch` for edits is **U**.
- **Assistant `usage`** per message: input, output and cache tokens (VP local: `claude_transcript.rs` `extract_usage`).
- **Queued messages:** `attachment.queued_command` (verified in 0.3.5).
- **Size risk:** a base64 screenshot makes one line several MB. Lines over the feed's **8 MiB cap are skipped whole**, so a big pasted image can hide the whole message today. Fix this in the line reader (§3.4).

**Codex rollouts** (`sessions/YYYY/MM/DD/rollout-*.jsonl[.zst]`; "isn't a stable interface", VP).
- User images: `response_item` `message` parts of `input_image` (VP local, `codex_transcript.rs:296`). The `image_url` as a `data:` URL is **I** (the Responses API shape).
- Calls: `function_call`/`custom_tool_call` (`apply_patch` carries the patch text), `*_call_output`, the plan, and `token_count`.
- Through app-server (better, and paged): `ThreadItem`s. VP, https://learn.chatgpt.com/docs/app-server (read 2026-11-03):
  - `agentMessage {text, phase?}`;
  - `reasoning {summary, content}`;
  - `commandExecution {command, cwd, status, aggregatedOutput?, exitCode?, durationMs?}`;
  - `fileChange {changes[{path, kind, diff}], status}`;
  - `mcpToolCall {…, result, error}`;
  - `webSearch {query}`;
  - **`imageView {path}`**;
  - `plan {text}`;
  - `collabToolCall` (subagents);
  - `contextCompaction`;
  - `turn/diff/updated {diff}` (the turn's unified diff);
  - `turn/plan/updated {plan[{step, status}]}`.

**ACP tools.** Text, image, audio, resource and resource_link blocks (VP, ACP content page). Cursor adds `cursor/update_todos` and `cursor/generate_image`.

### 3.3 Gap list against AI Juggler today

| # | Gap | Where it shows today | Source data exists? |
|---|---|---|---|
| G1 | Images (yours, tool screenshots, Codex `imageView`) aren't shown | `Block` has no image kind (`conversation_feed.rs:239`) | Yes, both tools |
| G2 | Lines over 8 MiB (big images) are dropped whole | ADR-0024 bounds | — |
| G3 | Edits aren't diffs; there's no turn diff | `StepResult {failed, kind}` | Claude Edit inputs; Codex `fileChange.diff`, `turn/diff/updated`, `apply_patch` |
| G4 | Command output isn't shown | Same | Claude `tool_result`; Codex `aggregatedOutput` / call output |
| G5 | No todo, plan or tasks card | — | TodoWrite, ExitPlanMode; Codex plan |
| G6 | Thinking isn't readable as text | `thoughts` are kept but only fold the title | Yes, when the transcript keeps it |
| G7 | No tokens or cost per turn | `FeedTurn` has `model` only | Claude `usage`; Codex `token_count`; `docs/research/model-pricing.md` |
| G8 | Subagent calls don't open their thread inline | A separate tab | `.meta.json` `toolUseId`; Codex `collabToolCall` |
| G9 | Code blocks: no copy, no highlight | `Markdown.tsx` | — |
| G10 | No find in a conversation | — | The outside-search index; AJ-E1007's |
| G11 | The driven panel draws its own item list, not the feed | `DrivenPanel.tsx` `view.items` | Driven sessions write the same transcript |
| G12 | The composer is single-line text, driven sessions only | `DrivenPanel.tsx` | Protocols take images, skills, steer |
| G13 | Links can't be opened (on purpose) | `Markdown.tsx:132-160` | — |
| G14 | Mermaid, LaTeX | — | Text only; not parity items |

### 3.4 One rendering pipeline: the design

**Backend (adapters fill typed blocks; core and UI stay tool-blind).**
1. **`Block::Image(ImageRef)`.** The ref holds `{line_offset, part_path, media_type, bytes_estimate, width?, height?, origin: you|tool|assistant, call_id?}` and **no pixels**.
   - Width and height come from decoding only the first base64 bytes (the PNG IHDR; the GIF and WebP headers; the JPEG SOF scan bounded to 64 KiB). The page can then reserve the space and the scroll anchoring (`useStickToBottom`) doesn't jump.
   - Codex `imageView {path}` and `localImage` refer to a file path. The ref carries the path, and it is read only if it lies under the session's cwd or the tool's home (I: a security bound).
2. **The line reader stops dropping long lines.** In `conversation_lines.rs`, a line over the cap is scanned with a tolerant tokenizer that records each `"data":"…"` base64 span's byte range and skips it. Everything else is still parsed, so an 8 MB screenshot costs no memory.
3. **Call details by reference.**
   - Each call gains `detail: Option<DetailRef {line_offset, kind: output|diff|plan|todos|web}>` plus a tiny summary (exit code; `+12 −3`; "4 of 6 done").
   - The full output or diff is fetched on expand with **`conversations_call_detail(follow_id, turn_id, call_id)`**: it re-reads that line by offset, applies D-14 redaction, and caps the result at 256 KiB with head and tail kept.
   - Diffs: Claude Edit/MultiEdit/Write inputs are turned into a unified diff in Rust (a few hundred lines; no new crate needed for a line diff over bounded strings, I). Codex's `diff` and `apply_patch` text are passed through.
4. **Plan and todos as a block** (`Block::Plan {items[{text, status}], explanation?}`), from TodoWrite, ExitPlanMode, Codex `plan` and `turn/plan/updated`, and Cursor `update_todos`. The latest one is pinned in the session header, as the Code tab's tasks pane does.
5. **`FeedTurn.usage`** `{input, output, cache_read, cache_write}`, with cost from the pricing table when the model is known. It shows as "1.2k in · 340 out · $0.02" on hover.
6. **Images served by `conversations_image(follow_id, turn_id, image_index)`.**
   - Bytes go out as a raw IPC response (`tauri::ipc::Response`), shown through a `blob:` URL. The CSP becomes `img-src 'self' data: blob:`.
   - Formats are sniffed by magic bytes: PNG, JPEG, GIF and WebP only. **Never SVG** (it's a document), and **never a remote URL**: a markdown `![](https://…)` or a Codex `image {url}` is shown as a link that isn't followed.
   - Cap: 20 MB decoded per image. Bigger ones get "Too large to show here" plus "Open in Preview", which writes nothing and opens the tool's own file path when it's a path (I).
7. **Driven sessions use the feed too** (closes G11). The driven panel keeps only its live overlay (streaming text, asks, limits), so there is one renderer.
8. **Find** (`conversations_find(conversation, query) → [{offset, snippet}]`) uses the contentless index when it's on, else a bounded streaming scan of the file (newest first, at most 64 MiB per call, "Search further"). A hit jumps through `conversations_earlier` around the offset. ADR-0024's keyset paging already supports this.

**Frontend.**
- **One `ConversationView`** for watched, own-terminal and driven sessions.
- **New renderers:**
  - `ImageThumb`: lazy through IntersectionObserver, `img.decode()` off the main thread, at most 4 decoding at once, blob URLs revoked when off screen; a click opens a lightbox with zoom, ⌘C and Save as….
  - `DiffView`: unified diff; split view when wide; at most 400 lines, then "Show all".
  - `OutputBlock`: monospace, ANSI colours parsed (a small hand-written SGR parser, no HTML), head and tail with "Show all".
  - `PlanCard`.
  - `ThinkingFold`, behind "Show thinking" with a per-user default.
  - `SubagentCard`, which opens the child's feed in a side panel.
- **Code blocks** get Copy and a language label. Syntax highlighting is loaded lazily, after first paint and only for blocks in view (lazy-load rule).
  - Options: a ~200-line hand-written tokenizer for the top 10 languages, or a small library loaded on demand.
  - A new UI dependency needs a decision-log entry (E23 law), so the hand-written route is the default (I).
- **Mermaid and LaTeX:** later, lazy, off the main bundle. They are not needed for parity.
- **Links:** offer "Open link" with a confirmation that shows the full address. ADR-0003 doesn't forbid it; today's "never followed" was a safety default. This is a maintainer call only if they want to keep it strict.

**Performance (budgets to test, I).**
- **Feed sizes stay inside ADR-0024's bounds.** Refs are about 100 bytes, details are lazy, and `content-visibility` stays off.
- **Rows have fixed-size placeholders.** Image and diff rows reserve their space up front, so a virtualizer is still not needed at ≤300 turns. Revisit if long-turn pages pass about 2,000 DOM rows.
- **Images are held only while useful.**
  - Decoded images stay in memory in a per-tier LRU (ADR-0023): 32, 64 or 128 MB by tier.
  - Thumbnails could be kept on disk in a registered "Conversation images" cache (D-58): owner-only, rebuilt from the source, dropped with Last messages off.
  - D-37 promises conversation content is "kept nowhere", so disk thumbnails need a ruling (§7). The default is memory only.
- **Interaction budgets.**
  - Opening a session with 20 screenshots: first paint ≤ 150 ms with placeholders, each thumbnail ≤ 100 ms after it scrolls into view.
  - Expanding a 10k-line output: the head and tail render in ≤ 50 ms, and "Show all" streams.

**Privacy and security.**
- Screenshots can show secrets that D-14's text redaction can't touch.
- Images are therefore never put in notifications, previews, Home rows or the rail. They show on the session page only.
- A Privacy setting, "Show images in conversations" (on), lets the user turn them off.
- The Privacy page lines are tied to tests, as `trust-claims.json` already does.

### 3.5 The composer

- **One `Composer`:**
  - multi-line text (⌘↩ sends);
  - pasted and dropped images, shown as chips and validated as in §3.4;
  - a `/` picker fed by the adapter: Claude's `init.slash_commands` and skills, Codex `skills/list`, ACP commands. It follows the rule "suggested, then all".
  - an `@` file picker over the session's cwd: git-tracked files first, then all, with search. Inserted as `@path` text for Claude; for Codex, as text, or a `mention` input if one turns out to exist (U).
  - a send-mode hint: Send, or "Add to what it's doing" (steer) while it works.
- **Attachments per route:**
  - Claude driven: base64 image blocks inline, so nothing touches disk.
  - Codex driven: the image is written to the session's `0700` runtime folder, passed as `localImage`, and deleted once the turn is accepted.
  - PTY route: text only (images U).
  - Copy-and-open route: text only, with the image put on the clipboard as a second step (I).
- `TurnInput` gains `attachments: Vec<Attachment>` with `Image { media_type, bytes | path }` and `File { path }`. An adapter that can't take one says so (§4.2), and the composer greys it out with the reason.

---

## 4. Proposed 0.5 scope

The current 0.5 table has no conversation or reply item. This is planned work, so it belongs in 0.5 (a minor), filed as **E34 "Reply and read in AI Juggler"**. It fits the 0.5 intent ("make AI Juggler a macOS release people can rely on") and the Floor's "who needs you".

### 4.1 Order (each step ships value alone; all within 0.5)

| Step | What you get | Effort (I) | Depends |
|---|---|---|---|
| **E34-1 Typed conversation** | Images (yours, tool screenshots, Codex views); long lines no longer dropped; diffs for edits and per turn; command output; plan and todo card; thinking on demand; tokens and cost per turn; subagent cards inline; Copy on code blocks. Driven sessions read the feed | L | ADR-0024 amendment (new block kinds, detail-by-offset commands, CSP `blob:`) |
| **E34-2 Find in a conversation** | ⌘F with hits that jump, older turns paged in | S–M | E34-1; the indexes |
| **E34-3 One composer** | Multi-line text, images, `/` and `@` pickers, steer. Used by driven Codex (default) and driven Claude (Beta) | M | `TurnInput.attachments`; codecs (stream-json image blocks; `localImage`) |
| **E34-4 Quick reply everywhere AI Juggler owns the session** | Reply from the session page, a Needs you row, a notification sheet or ⌘K. Routes: driven; typed into AI Juggler's own terminal (state-gated); "Continue here" for idle sessions nobody holds | M | AJ-E2306 exact state; the one-writer check |
| **E34-5 Reply in the app** | For sessions a desktop app holds (Claude Code tab and Chat, ChatGPT Codex): copy the reply, open the exact session; the user pastes and sends. Plus a one-line pointer to the vendor's Remote Control | S | Go to it (AJ-E2102) |
| **E34-6 (Beta, lab-gated) The Claude Code plugin's reply mod** | Typing a reply in AI Juggler for a Code-tab or other-terminal session delivers it *as the user* when the session is idle (`prompt.submit asUser`), plus exact state for the Code tab through plugin hooks | M | AJ-E1905 (D-43 point 6); lab L6 |
| **E34-7 (Beta, lab-gated) Answer Claude terminal asks from Needs you** | Permission prompts answered first-answer-wins, through a PermissionRequest hook if the lab proves the TUI closes its dialog; else the channel relay once allowlisted | M | Labs L4 and L5 |
| Later (0.6+) | Mermaid and LaTeX (lazy); syntax highlighting if the hand-written tokenizer isn't enough; `codex queue` for daemon TUIs; inbox-socket delivery for E18 notes | — | — |

### 4.2 The architecture: what each adapter declares

**`juggler-adapter`: `HarnessAdapter::reply_routes(&SessionContext) -> Vec<ReplyRoute>`**, best first. `SessionContext` holds who runs it, its places, its runtime and its exact state. The routes:
- `Driven`: through the runtime that exists.
- `OwnTerminal`: typed into AI Juggler's PTY; needs exact state.
- `ResumeHere`: idle, held by no place.
- `Plugin`: AI Juggler's plugin is installed in that profile and reported this session.
- `OpenWithCopy { link }`: the deep link or Go to it.
- `None { why }`.

**`ComposeCaps`** is returned per route:
- `images: Option<{max_bytes, media_types}>`;
- `files`;
- `slash: Option<CommandSource>`;
- `mentions`;
- `steer`;
- `answers: {permissions, questions}`;
- `voice: User | Peer`, so the UI says "Sends as your message" or "Arrives as a note from AI Juggler".

Each is data, not a branch. The existing `Capabilities` bitflags stay for discovery-time facts, plus one new flag (`REPLY_ROUTES`) so the conformance kit runs the reply specs.

**`RenderCaps` are implicit.** The feed's blocks are typed, and an adapter emits only what its transcript has. The `layout` (`stretches` | `folded`) stays the only "how its app draws it" hint (ADR-0024 §7).

**The conformance kit adds:**
- no route sends without a user gesture (the guardrail extends `runtime_guardrails.rs`);
- the PTY route refuses unless the state is exactly `idle`/`working` and fresh (≤ 2 s);
- the PTY route strips C0 and C1 control characters and ESC;
- `ResumeHere` refuses when a place holds the conversation;
- image refs carry no pixels;
- detail fetches re-redact.

**Cross-platform.** Every route is behind the supervisor and platform seams. Windows notes:
- ConPTY;
- the inbox needs the token (only through a hook);
- deep links work through registered schemes;
- notification reply sheets are AI Juggler's own UI.

---

## 5. Risks

| Risk | Effect | Mitigation |
|---|---|---|
| Typing into a TUI while a dialog is open approves something | A security bug | Exact-state gate (hooks or OSC 7501, fresh within 2 s, re-checked just before writing); no route while `asks_you`; control characters stripped; a fake-TUI test that opens a dialog mid-reply |
| Claude transcript format changes (internal) | Blocks missing | Version-tolerant parsing with recorded fixtures per Claude Code version; unknown parts counted, never crash; driven and app-server paths preferred where they exist |
| Huge base64 lines | Memory and latency | The span-skipping tokenizer (§3.4.2); pixels only on demand, capped |
| Screenshots leak secrets into more surfaces | Privacy | Session page only; never in notifications, previews or the rail; a Privacy toggle; memory-only by default |
| `-p` terms or billing move (Claude driven Beta, `ResumeHere` for Claude) | The route closes | ADR-0022's four gates; terms rechecked each release; the route degrades to OwnTerminal or OpenWithCopy |
| Mods disabled by an org, or the guard skips hooks | The plugin route is silently dead | The plugin reports it's alive on SessionStart; without that report the route isn't offered |
| The plugin route mistaken for automation | Trust | It submits only what the user typed, once; never `tool.check`/`tool.call` hooks; the source is public and its manifest names that |
| Two writers (resume while an app holds the session) | A corrupted conversation | `ResumeHere` only with no places and no live pid; re-checked at send |
| Channel relay stays preview, or the dev flag is too noisy | E34-7 slips | It's Beta and lab-gated; the fallback is "Answer in the terminal or app" |
| Scope creep in 0.5 (already large: new tools, the Floor, Stable) | Stable slips | E34-1…5 are the core; E34-6/7 are Beta and can trail into 0.5.x as previews (like the roadmap's "marked as a preview" tools) |

---

## 6. What to validate live (AI Juggler Dev, throwaway lab profiles and a lab project only)

Every item uses one or two real tools on spare accounts. No real data, no sign-ins, no Allow on the maintainer's own sessions.

- **L1 PTY quick reply (Claude TUI and Codex `--no-daemon`):**
  - idle → new turn;
  - working → queued;
  - a dialog open → AI Juggler refuses;
  - a 900-char reply → `[Pasted text]` behaviour;
  - typing an image path: does it attach? (U)
- **L2 Claude stream-json image block:** round trip, then check what the transcript stores (inline base64?) and how big the line is.
- **L3 Codex `turn/start` with `localImage`:** check the rollout's `input_image`, then `imageView` from `view_image`.
- **L4 PermissionRequest hook:** blocking with a delayed `allow` in the TUI. Does the dialog close? Who wins when the user answers in the terminal first? Repeat in the Code tab with a plugin hook.
- **L5 Channel relay:** with `--dangerously-load-development-channels server:ai-juggler` on an AI Juggler terminal session. Does AskUserQuestion relay?
- **L6 Plugin mod in the Desktop Code tab:**
  - `$.prompt.submit({asUser:true})` from a queued reply: what the app shows, the idle wait, and after Stop;
  - `$.process.spawn` of `juggler-agent watch`: its lifetime, and leftover processes (reap them).
- **L7 Inbox socket:** a hook-reported path, then a post into a TUI and a Code-tab session. Check how each shows it and the `crossSessionInbound` behaviour.
- **L8 `codex queue`** into a daemon TUI thread.
- **L9 Copy and open:**
  - `claude://resume?session=` on a session that is already a Code-tab session (U);
  - `codex://threads/<id>`;
  - timing until the composer has focus.
- **L10 `ResumeHere`:** reply to an idle Claude Code tab session that the app no longer holds, then reopen it in the app. Does the app show the new turn? (U)
- **L11 Performance:** a synthetic 300-turn transcript with 30 images and a 50k-line output, against the budgets in §3.4.

---

## 7. Open questions and the maintainer's decisions

The design choices above are mine (decide by the goal). These are the ones that are truly yours:

1. **Disk cache for image thumbnails.**
   - D-37 says conversation content is kept nowhere.
   - D-58 allows registered, clearable caches, and already holds subagent prompts.
   - The question: may "Conversation images" be one of them? My default is memory only.
2. **Publishing the Claude Code plugin with a reply mod** (D-43 point 6 / AJ-E1905), and whether to ask Anthropic for channel allowlisting of it. Their docs point partners to an official-marketplace listing.
3. **Following links from conversations,** with a confirmation that shows the full address. Today links are never followed.
4. **Any new UI dependency.** My plan needs none, with a hand-written highlighter and diff renderer. Mermaid and LaTeX would need one each, loaded lazily.
5. **E34's place:** in 0.5 as above, with E34-6 and E34-7 as Beta previews that don't hold the Stable.

**Unverified items, to settle in the labs:**
- whether a TUI's dialog closes on a late hook decision;
- whether AskUserQuestion relays through channels;
- image attachment through the PTY;
- `codex queue` visibility;
- `claude://resume` on a Code-tab session;
- whether the Claude app shows turns written elsewhere;
- inline notification replies through `tauri-plugin-notification`;
- the exact on-disk image shape in the Claude JSONL and in Codex rollouts.

---

## Sources

**Read 2026-11-03:**
- Remote Control: https://code.claude.com/docs/en/remote-control
- Cross-session messaging and the inbox socket: https://code.claude.com/docs/en/cross-session-messaging
- Mods API: https://code.claude.com/docs/en/plugins/mods/api
- Mods events (tool.call, tool.check, prompt.submit, the built-in guard): https://code.claude.com/docs/en/plugins/mods/events
- Channels reference (permission relay): https://code.claude.com/docs/en/channels-reference
- The Desktop Code tab's features, Dispatch and Remote Control settings: https://code.claude.com/docs/en/desktop
- Streaming input, with image blocks: https://code.claude.com/docs/en/agent-sdk/streaming-vs-single-mode
- `claude://` links: https://support.claude.com/en/articles/14729294-open-claude-desktop-with-a-link
- Codex app-server (input items, ThreadItems, turn diff and plan, terms): https://learn.chatgpt.com/docs/app-server
- Codex Remote Control and mobile: https://learn.chatgpt.com/docs/remote-connections (redirected from https://developers.openai.com/codex/app/mobile)
- ACP content blocks: https://agentclientprotocol.com/protocol/content
- Secondary, VS:
  - Mermaid not rendered in the Code tab: https://claudeissues.com/issue/52517-claude-desktop-mermaid-code-blocks-not-rendered-in-claude-code-tab
  - Cursor Mermaid limits: https://forum.cursor.com/t/mermaid-timeline-diagrams-fail-to-render-in-cursor-chat/166749
  - Codex mobile coverage: https://www.igeeksblog.com/openai-codex-chatgpt-mobile-app and https://codex.danielvaughan.com/2026/05/15/codex-mobile-chatgpt-app-relay-architecture-remote-agent-control/

**From the repo's research (accessed 2026-10-25/27, not re-fetched):**
- Claude Code, from `docs/research/capabilities/claude-code.md` and `docs/research/session-runtime.md`:
  - agent view: https://code.claude.com/docs/en/agent-view
  - hooks: https://code.claude.com/docs/en/hooks
  - deep links: https://code.claude.com/docs/en/deep-links
  - interactive mode: https://code.claude.com/docs/en/interactive-mode
  - terminal config (OSC 7501): https://code.claude.com/docs/en/terminal-config
  - sessions: https://code.claude.com/docs/en/sessions
  - legal and compliance: https://code.claude.com/docs/en/legal-and-compliance
  - Agent SDK overview: https://code.claude.com/docs/en/agent-sdk/overview
  - headless: https://code.claude.com/docs/en/headless
- Codex, from `docs/research/capabilities/codex.md`: D/reference/commands (deep links), D/developer-commands (`codex queue`), D/hooks.
- Other tools:
  - OpenCode server docs, from `capabilities/opencode.md`;
  - Cursor ACP: https://cursor.com/docs/cli/acp;
  - Kiro ACP: https://kiro.dev/docs/acp/;
  - Gemini, Kimi, Grok, Hermes and Muse, from `capabilities/*.md`.

**Repo files read:**
- docs:
  - `docs/plan/ROADMAP.md`
  - `docs/adr/0003-*`, `0022-*`, `0024-*`
  - `docs/tasks/E23-session-runtime.md`
  - `docs/research/native-conversation-display.md`, `session-runtime.md`, `capabilities/*`
  - `docs/review/decision-log.md` (D-37, D-58)
- UI:
  - `ui/src/pages/session/{DrivenPanel,ConversationTurn,TurnWork,Markdown}.tsx`
  - `ui/src/types/generated/TurnBlockInfo.ts`
- crates:
  - `crates/juggler-adapters/src/{conversation_feed,claude_transcript,codex_transcript,claude_env,claude_live}.rs`
  - `crates/juggler-adapter/src/{capabilities.rs,runtime/events.rs}`
  - `crates/juggler-app/src/{terminal.rs,features/conversations/dto.rs,features/places/handoff.rs,features/outside_search/mod.rs}`
  - `crates/juggler-app/tauri.conf.json`
