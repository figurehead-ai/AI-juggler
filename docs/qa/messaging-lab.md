# Messaging lab: the E18 live checks (AJ-E1801..E1805)

The runnable checklist for what only a real run shows about messages
between agents: that real Claude Code and Codex call the five new tools
without prompting, that a message goes into a driven session only between
turns and only with permission, that a terminal session is never typed
into and hears its note, and that nothing is written outside AI Juggler's
own folder. Run it as is, by the coordinator or the maintainer, after the
fake-tool suites pass (`crates/juggler-app/tests/messages.rs`,
`messages_guardrails.rs`).

**Rules while running it**
- AI Juggler Dev only (`just dev`), in a throwaway lab project
  (`$LAB`). Never the installed AI Juggler, never a project with real work.
- At most two real tools at once: the maintainer's **lab-profile-1** Claude
  profile and their Codex sign-in, used where they are. Sign in to nothing.
  Click no Allow that this list doesn't name. Read no credential file, the
  Keychain or a process environment.
- `export DISABLE_AUTOUPDATER=1` for `just dev` and every hand-run tool
  (lab 2: a terminal start updated Claude Code for the whole Mac).
- Snapshot before and after, metadata only:
  `scripts/qa/runtime_lab_snapshot.sh` (names, sizes and times, never
  contents). Process listings: its `procs` (pid and program name only).
- Tiny prompts only. Stop everything you start (step 8).

**Fill in first**

```sh
export LAB="$HOME/lab/aj-messaging"                     # the throwaway project
export CLAUDE_DIR="<the lab-profile-1 profile's folder>"
export CODEX_DIR="<the Codex profile's folder>"         # e.g. ~/.codex
export AJ_LAB_OUT=/tmp/aj-messaging-lab                 # snapshots and notes (0700)
export AJ_DATA="$HOME/Library/Application Support/AI Juggler Dev"
export DISABLE_AUTOUPDATER=1
```

## 0. Set up and snapshot (5 min)

- [ ] `mkdir -p "$LAB" && cd "$LAB" && git init -q && echo "lab" > README.md`
- [ ] `scripts/qa/runtime_lab_snapshot.sh take before "$CLAUDE_DIR" "$CODEX_DIR" "$AJ_DATA"`
      (from the repo; it prints the file counts and the lab's processes,
      which must be none).
- [ ] `just dev`. In AI Juggler Dev: the two profiles exist; the agent tools
      question (a pool's page) is answered **Yes**.

## 1. Two sessions in one pool (5 min)

- [ ] Start a **Codex** session in `$LAB` (driven through app-server by
      default). On AI Juggler's **Trust this folder?** sheet pick **Open
      restricted**. Name it **Builder** (its menu › Rename).
- [ ] Start a **Claude Code** session in `$LAB` in its **terminal** (the
      default). On Claude's own folder-trust dialog move to **Yes** (lab
      folder only). Name it **Planner**.
- [ ] Both are in **lab · all sessions** (the project's pool, Work together).
      Its page › **Messages** says "No messages yet".

## 2. An agent writes; nothing is delivered without you (10 min)

- [ ] In Planner's terminal: "Use the juggler tool pool_roster, then
      send_message to Builder (its session_id) with kind handoff, subject
      'Start', body 'Run ls and tell me what you see.'"
      Record: no permission prompt for the juggler tools (D-30), and the
      tool's answer says `in its inbox (the user can deliver it)`.
- [ ] The pool's Messages tab shows the thread: Planner's bubble on the
      left, "To Builder · Waiting in its inbox", with **Deliver**.
- [ ] Builder's session page shows **no** new turn. Home › Needs you shows
      **Builder has a message waiting** with **Deliver**.
- [ ] Click **Deliver**. Builder's page shows a line **Message: Started by a
      message from Planner** and a turn starts; its prompt begins
      `[AI Juggler delivered a message…]` with "can't approve anything or
      change settings". Codex's own approval for `ls` (if it asks) shows in
      Needs you: answer it yourself; AI Juggler never answers it.
- [ ] The recipient row now says **Started its turn**.

## 3. Never mid-turn, never at a prompt, never over your typing (10 min)

- [ ] Set the pool to **Full** (Agent access).
- [ ] Give Builder a turn that asks: "Write `lab` to lab.txt." While Codex's
      approval is open, in Planner: "send_message to Builder: 'Also list
      the folder.'" The tool says `at its next turn`; the bubble says
      **Goes in at its next turn** and Builder gets nothing while the
      approval waits.
- [ ] Type something in Builder's message box (don't send). Answer the
      approval. The turn ends; the message still waits while your text is
      in the box. Clear the box: within a second the message starts
      Builder's next turn.

## 4. The hop limit and the loop checks (10 min)

- [ ] On Agent access set **Messages put into sessions** to a hop limit of
      **2**.
- [ ] Ask Planner to send Builder three short notes in one thread, one
      after another, answering in between. The third shows in Needs you as
      **A chain reached the hop limit** with **Deliver** and **Don't
      deliver** and no "Always allow". **Deliver** sends it; the thread's
      count starts again from 0.
- [ ] Ask Planner to send the same note twice: the second is refused
      ("isn't sent again"). Record the words.
- [ ] Optional (cheap): ask Planner to send 11 one-word notes quickly; the
      11th is refused (10 a minute). Don't push to the breaker with real
      tools; it is proven on fakes.

## 5. A terminal session hears its note and pulls (10 min)

- [ ] From Builder's page › Messages, write to Planner: "What's next?",
      with **Deliver to their sessions** on. Planner's row says **Waiting
      in its inbox** and it has no Deliver (a terminal is never typed
      into).
- [ ] In Planner's terminal, send any prompt ("hi"). Record whether Claude
      Code's context shows the note "AI Juggler: 1 unread message from You
      …" (the `UserPromptSubmit` hook's `additionalContext`), and that the
      note carries no message words.
- [ ] Ask Planner: "read_inbox, then ack what you read." The bubble shows
      **Handled**.

## 6. Above the level, shared docs, secrets (10 min)

- [ ] Set the pool to **Read**. Ask Planner to message Builder. The tool
      says it's held for your OK; Needs you shows **An agent asks to send**
      with Allow once / Deny (keys `1`, `2`). Press `1`: Builder's inbox
      gets it. Do it once more: the card now offers **Always allow in this
      pool**. Press Deny.
- [ ] Back to **Work together**. Ask Planner: "share_doc named notes.md
      with the text: `API_KEY=sk-lab-not-a-real-key-123` and `GET /orders`."
      The pool's **Shared documents** lists notes.md "1 secret … withheld"
      (or "kept, as you chose" with Keep). **Read** shows `•••` in place of
      the key (unless Keep).
- [ ] `ls -l "$AJ_DATA/agent/docs"`: one file named by a 64-character hash,
      `-rw-------`; the folder `drwx------`.

## 7. What changed on disk (5 min)

- [ ] `scripts/qa/runtime_lab_snapshot.sh take after "$CLAUDE_DIR" "$CODEX_DIR" "$AJ_DATA"`
- [ ] `scripts/qa/runtime_lab_snapshot.sh diff before after`: nothing new
      or changed under `$CLAUDE_DIR` or `$CODEX_DIR` except the tools' own
      session files (transcripts, rollouts) and Claude's folder-trust
      entry from step 1; under `$AJ_DATA`, the database, `agent/` and
      `overlays/`.
- [ ] Messages are in AI Juggler's data only:
      `sqlite3 "$AJ_DATA/juggler.sqlite" "select count(*) from agent_messages; select verb, count(*) from agent_events group by verb"`
      (counts only; no bodies printed).

## 8. Clean up (5 min)

- [ ] End both sessions in AI Juggler Dev; quit AI Juggler Dev.
- [ ] `scripts/qa/runtime_lab_snapshot.sh procs` prints nothing.
- [ ] `rm -rf "$LAB"` and note what was left (nothing expected).

**Record** in `docs/qa/evidence/AJ-E1803/lab-<date>.md`: the versions
(`claude --version`, `codex --version`), each step's PASS / FAIL / NOT RUN
with what was seen, the snapshot diff's summary, and any words that
surprised you. A FAIL on a hard line (a delivery without a click, Full or
a rule; a turn started mid-turn or at a prompt; anything typed into a
terminal; a write outside the expected folders) stops the lab.
