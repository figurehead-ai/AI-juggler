# E34 — Reply and read in AI Juggler: a richer conversation, and an honest reply area

> Source: the maintainer, 2026-11-03, on the research
> ([`docs/research/reply-and-read.md`](../research/reply-and-read.md)),
> verbatim: "Plugin route is not worth it. We should have explanation that
> desktop apps have this limitation , and if they wanted to chat from
> within here they need to start sessions as CLI apps. Which we can offer
> that with our switch capabilities. But what we can and should do improve
> our chat content experience like we were proposing, so images, links and
> other content from chat is rendered even better. And right there where
> chat input is supposed to be we should have , a button like go to app
> and respond ...etc". Earlier the same day: "you can cache conversations,
> images everything locally … it really comes down to managing the cache
> and cleaning it up once it gets big". Planned for 0.5 (D-66).
>
> **The rule this epic builds.** A conversation in AI Juggler reads as the
> tool's own app shows it, or better: images, links, diffs, command output,
> plans and to-dos, thinking, tokens and cost, subagents. Where the chat
> input would be, a session says honestly how you can answer it: here
> (sessions AI Juggler runs, or runs in its own terminal), or in its app
> (a desktop app holds it: Go to app and respond), with Continue here as
> CLI (the switch) for chatting from AI Juggler from then on.
>
> **A standing rule (D-66).** Whenever a desktop app offers a sanctioned
> way to reply into its sessions (a documented protocol, an app-server,
> an MCP channel, an official URL scheme or SDK), AI Juggler supports it
> as that tool's reply route, and the reply area offers it instead of Go
> to app. Every tool's reply surfaces are checked again each release in
> `docs/research/capabilities/`; a new one becomes a task in the next
> release.
>
> **Order.** AJ-E3401 first (the typed feed every other task renders),
> then AJ-E3402 and AJ-E3403 beside each other, then AJ-E3404 and
> AJ-E3405. Each names its scenarios in [`docs/qa/0.4-validation-plan.md`](../qa/0.4-validation-plan.md)'s
> form and isn't done until they pass by hand.

### AJ-E3401 The typed conversation: images, links, diffs, output, plans, thinking, tokens, subagents | Status: TODO | Priority P0 | Effort L | Depends: —
**Goal**: The conversation shows what the transcripts already hold, drawn as the tool's app draws it.
**User story**: As someone reading a session in AI Juggler, I see the screenshot I pasted, the diff Claude made, the command's output and the plan, without opening the app.
**Requirements**:
1. The feed (ADR-0024, amended) SHALL carry typed blocks: image references (no pixels in the feed; size read from the header), call details by byte offset (output, diff, plan, web), plans and to-dos, thinking, tokens and cost per turn, subagent cards, for Claude Code transcripts and Codex rollouts.
2. Images SHALL load lazily from the transcript or the tool's file by offset, PNG, JPEG, GIF and WebP only (never SVG, never a remote URL), into a registered, clearable local cache (D-58; the 2026-11-03 ruling: cache freely, manage its size), with a lightbox (zoom, copy, save).
3. Links SHALL open after a confirmation showing the full address; a remote image is a link, never loaded.
4. Code blocks SHALL have Copy and a language label; highlighting loads lazily, for blocks in view.
5. Driven sessions SHALL read the same feed (one renderer).
6. A message whose line is long only because of inline image data SHALL show (0.4 keeps it; this task draws its image).
**UI/UX reasoning**: the default view matches the tool's own app; depth one click deeper (`native-conversation-display.md`).
**Performance budget**: opening a session with 20 screenshots paints in ≤ 150 ms with placeholders, each image ≤ 100 ms after it scrolls into view; a 10 000-line output's head and tail in ≤ 50 ms.
**Quality gates**: fixtures per tool version; image refs carry no pixels; details re-redacted (D-14); the cache registered and cleared by the user.
**Evidence/validation proof**: a lab walk on spare profiles with pasted images, edits and long outputs.
**Visual testing**: a conversation gallery, both themes, 1024 and 1280.
**Files to touch**: `crates/juggler-adapters/src/conversation_feed.rs`, `conversation_lines.rs`, `ui/src/pages/session/`.
**Research basis**: `docs/research/reply-and-read.md` §3.
**Handover note**: —

### AJ-E3402 Find in a conversation | Status: TODO | Priority P0 | Effort M | Depends: AJ-E3401
**Goal**: ⌘F in a conversation finds words in all of it, not just what's loaded.
**User story**: As someone looking for what was said yesterday, I search and jump to it.
**Requirements**:
1. ⌘F SHALL search the whole conversation (the outside-conversation index when on, else a bounded scan, newest first) and jump to each hit, paging older turns in.
**UI/UX reasoning**: instant and paged.
**Performance budget**: first hit ≤ 300 ms on a 50 MB transcript.
**Quality gates**: hits agree with the words on screen.
**Evidence/validation proof**: a walk on a long lab session.
**Visual testing**: the find bar, both themes.
**Files to touch**: `crates/juggler-app/src/features/conversations/`, `ui/src/pages/session/`.
**Research basis**: `docs/research/reply-and-read.md` §3.4.
**Handover note**: —

### AJ-E3403 One composer for the sessions AI Juggler can answer | Status: TODO | Priority P0 | Effort M | Depends: AJ-E3401
**Goal**: Where AI Juggler can send to a session, it has a real message box.
**User story**: As someone working in AI Juggler, I reply with several lines, a pasted screenshot, a slash command or an @file.
**Requirements**:
1. Multi-line text (⌘↩ sends), pasted and dropped images, a `/` picker fed by the adapter and an `@` file picker (suggested, then all), steer while it works.
2. Routes, declared per adapter (no per-tool branches outside adapters): driven sessions (Codex app-server, Claude stream-json); AI Juggler's own terminal sessions, typed only when the exact state is idle or working, never while a question or permission is open, control characters stripped; Continue here for an idle session nothing holds.
3. Only a user's gesture in AI Juggler sends (the runtime guardrails test extended).
**UI/UX reasoning**: progressive disclosure: one box, its extras on demand.
**Performance budget**: the box answers a keystroke in ≤ 16 ms.
**Quality gates**: a fake TUI with a dialog open refuses the typed route.
**Evidence/validation proof**: lab walks for each route.
**Visual testing**: the composer, both themes.
**Files to touch**: `crates/juggler-adapter/src/runtime/`, `ui/src/pages/session/`.
**Research basis**: `docs/research/reply-and-read.md` §2, §3.5.
**Handover note**: —

### AJ-E3404 Where the input would be: how this session can be answered | Status: TODO | Priority P0 | Effort S | Depends: AJ-E3403
**Goal**: A session always says how you can answer it, and gives the one click that does it.
**User story**: As someone reading a Claude app session in AI Juggler, I see why I can't reply here, and I can go to the app to answer or continue it here as a CLI session.
**Requirements**:
1. A session a desktop app holds (the Claude app, Cowork, ChatGPT's Codex) SHALL show, in place of the composer: one line on the limitation (the apps take replies only in their own window), **Go to app and respond** (opens that exact conversation; anything typed is copied first), and **Continue here as CLI** (the switch to a CLI App, after which the composer works).
2. A Serious-choice explanation (consequence, an example, a docs link) behind "Why?".
3. The same choice from a Needs you row and a notification ("Reply…").
**UI/UX reasoning**: honest about limits; one click to either way out.
**Performance budget**: Go to app brings the window forward in ≤ 1 s.
**Quality gates**: never a scripted click or keystroke into another app (terms; ADR-0003).
**Evidence/validation proof**: a walk with a Claude app session on a lab profile.
**Visual testing**: the reply area for each kind of session, both themes.
**Files to touch**: `ui/src/pages/session/`, `ui/src/components/Home*`.
**Research basis**: `docs/research/reply-and-read.md` §2.2, §2.3.
**Handover note**: —

### AJ-E3405 Quick reply from Needs you and a notification | Status: TODO | Priority P0 | Effort S | Depends: AJ-E3403, AJ-E3404
**Goal**: A decision without opening the session.
**User story**: As someone told a session needs me, I answer from the notice.
**Requirements**:
1. A Needs you row and a notification SHALL open a quick-reply sheet over AI Juggler for the sessions AI Juggler can answer, and AJ-E3404's choice for the rest.
**UI/UX reasoning**: the shortest path to a decision.
**Performance budget**: the sheet opens in ≤ 150 ms.
**Quality gates**: as AJ-E3403.
**Evidence/validation proof**: a lab walk from a notification.
**Visual testing**: the sheet, both themes.
**Files to touch**: `ui/src/components/`.
**Research basis**: `docs/research/reply-and-read.md` §2.6.
**Handover note**: —
