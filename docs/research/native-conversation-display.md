# How the tools' own apps show a conversation and its subagents (0.3.4)

The maintainer tested 0.3.3 on a long Claude Code session run from the Claude app, and compared our session page with the app's own window. They found three gaps:

- **The conversation shows more than the app does.** Our Conversation card shows every note written between steps ("Ran 3 commands", paragraphs of narration). The app folds each stretch of work into one titled line and shows the final reply.
- **The app lists fewer subagents than we do.** The app "only shows a certain amount of subagents". Our Subagents tab lists 259 and opened on an empty "Running".
- **Two filter counts disagree.** The tab's header said "18 failed" beside "Needs attention 19".

The rule the maintainer set: *the default view matches the tool's own app as closely as possible; the in-depth views (the Subagents tab) go further than any native app does, and must be right.*

The research was read-only:
- the tools' own records (keys and counts only, never conversation text);
- the tools' docs and source;
- our code.

Each claim below is marked **verified** (seen in records, source or docs) or **inferred**.

## 1. What the Claude app (Code tab) and Claude Code show

**Conversation.**
- **Verified:** the app folds each run of work into one line with a short past-tense title, puts a one-sentence summary above it, and shows the reply in full.
- **Verified:** the titles are not in the session's transcript or in the app's per-session records. Claude Code has a small-model helper that writes such labels (`tool_use_summary`, about 30 characters, past tense, per batch of tool calls). It is generated live and kept out of the transcript.
- **Inferred:** the one-sentence summary is the model's summarized thinking (`thinking` blocks marked `thinkingDisplay: "updates"`). It is stored only while the session is on screen in the app.
- **So:** the app's exact titles can't be rebuilt from files. Anything we show in their place must come from data that is there: the steps, their times and their own descriptions.

**Subagents.**
- **Verified:** each subagent has its own transcript under `<session>/subagents/agent-<id>.jsonl` with a `.meta.json`. The meta holds `toolUseId` (the parent's Agent call), `parentAgentId`, `agentType` and `description`.
- **Verified:** a background start shows in the parent as `status: "async_launched"`. Its end arrives later as a task notification.
- **Verified (docs):** the app's tasks pane lists "the background work running inside the current session". The CLI's subagent panel drops a finished row at once and keeps a failed or stopped one for 30 seconds.
- **So:** the app shows **what runs now**, not the session's history of subagents. Our 259 is the right count for an in-depth list. 263 were on disk; the 4 extra were still running.

**The nested entries in the app's sidebar are not subagents.**
- **Verified:** they are separate sessions this session started. Each has its own chat, branch or PR.
- **Verified:** the link is in the child's app record: `spawnedFrom.sessionId` (the parent), with `sideSession: true` and the task's title. The parent's `sideSessionNotes` holds the children's reports, not the link. Archived children are hidden from the sidebar.

## 2. What ChatGPT's Codex, the Codex CLI and the other tools show

**ChatGPT's Codex (verified: the app's default `conversationDetailMode`).**
- A finished turn shows the person's message, then one folded row ("Worked for {time}", with an activity line such as "Ran commands and edited files"), then the final answer.
- Notes between steps, commands, patches, reasoning and subagent activity are inside the fold.

**Codex marks its messages (verified).**
- Every assistant message carries a `phase`: `commentary` or `final_answer` (rust-v0.105.0 and later).
- Across 63 real threads there were 693 turns, 688 final answers and 5,472 commentary messages: about 8 notes per turn.
- We don't read `phase` yet.

**The Codex CLI (verified).**
- Commentary stays inline as bullets. Reasoning is hidden, and reads and searches group as "Explored".
- `/subagents` lists Main and each subagent, green while running and dimmed when closed.

**Codex's subagents (verified).**
- Each child has its own rollout (`source.subagent.thread_spawn`).
- The app's side panel splits a session's subagents into **Active** and **Done · N**.

**The other tools (verified, docs).**
- **Kimi Code:** keeps a finished turn's last two messages and folds the rest ("… thinking N times, call N tools, N messages").
- **Grok Build:** folds runs of reads and searches.
- **OpenCode:** hides thinking.
- **All three:** list subagents inline as cards or rows that open the child.

**The pattern across tools: fold a finished turn's work, show its reply, keep a running turn open.**

## 3. What we did (0.3.3) and why it differed

**Conversation (verified).**
- `last_turns.rs` joins one speaker's consecutive messages into a turn of text and step blocks. `LastMessages.tsx` drew every text block in full and folded only runs of tool calls.
- No subagent content is ever merged in: Claude sidechain lines are dropped, Codex children are their own rollouts. The extra text was the session's own notes.

**Subagents tab (verified).**
- **Why it opened on Running:** the tab mounts hidden when the page opens, and chose its first view then, from a cached count. It never chose again, so it could open on an empty Running. That breaks AJ-E2810 requirement 2.
- **Why "18 failed" beside "Needs attention 19":** Needs attention counts failed, stopped and with no recorded end, but the summary said only "failed".
- **Type:** subagents with no recorded type could not be picked.
- **Other apps' session pages:** they lacked D-48's "N subagents · M running › Subagents" line.

## 4. Decisions and where each lands

| What | Release | State |
|---|---|---|
| A conversation reads on a near-black, neutral surface in the dark theme (white in the light one), as the tools' windows do | 0.3.4 | done |
| A finished turn's work is one row, "Worked for 4 min · Ran 3 commands, edited a file · 2 notes", built from the turn's own times and steps, opening onto everything as written. The last text is the reply. The latest turn of a working session shows open. | 0.3.4 | done |
| The question card reads only the reply, never a "?" in a note | 0.3.4 | done |
| Subagents tab: opens on Running only when something runs, else All, decided each time it's shown; the summary names failed, stopped and "with no recorded end"; "No type recorded" can be picked; other apps' pages get the D-48 line | 0.3.4 | done |
| Codex: read `phase`, so a turn's final answer is the one Codex marked, and commentary after it still folds | 0.4 | planned |
| Overview, beside the conversation, matches the apps' side lists: subagents **running now** (Claude's tasks pane) and **Active / Done** (Codex's panel), each opening its conversation in place; and **the sessions this one started** (`spawnedFrom`), each opening its page. Built with E31's related-items list. | 0.4 | planned |
| A step's own description as its line ("Run the full preflight" instead of `ran "just preflight…"`), the label Claude Code itself shows | 0.4 | planned |

**Not done, on purpose:**
- We don't invent the app's titles or summaries. They aren't stored, and making them would mean sending conversation text to a model. AI Juggler reads a conversation on this Mac and keeps nothing of it.
- The fold's words come only from the turn's own record.
