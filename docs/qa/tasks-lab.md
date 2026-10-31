# Tasks lab: the E19 live checks (AJ-E1901..E1906)

The runnable checklist for what only a real run shows about tasks shared
by a pool: that real Claude Code and Codex call the six task tools without
prompting, that two real agents claiming one task can't both get it, that a
change above the pool's level waits for you, that no agent starts a session
or picks an account, that "Start a session for it" starts one only on your
pick and hands it the task, that a switch keeps the task, and that nothing
is written outside AI Juggler's own folder. Run it as is, by the
coordinator or the maintainer, after the fake-tool suites pass
(`crates/juggler-app/tests/tasks.rs`, `tasks_switch.rs`,
`tasks_guardrails.rs`, `crates/juggler-agentbridge/tests/bridge.rs`,
`plugin.rs`).

**Rules while running it**
- AI Juggler Dev only (`just dev`), in a throwaway lab project
  (`$LAB`). Never the installed AI Juggler, never a project with real work.
- At most two real tools at once: the maintainer's **lab-profile-1** Claude
  profile and their Codex sign-in, used where they are. Sign in to nothing.
  Click no Allow that this list doesn't name. Read no credential file, the
  Keychain or a process environment.
- `export DISABLE_AUTOUPDATER=1` for `just dev` and every hand-run tool.
- Snapshot before and after, metadata only:
  `scripts/qa/runtime_lab_snapshot.sh` (names, sizes and times, never
  contents). Process listings: its `procs` (pid and program name only).
- Tiny prompts only. Stop everything you start (step 9).

**Fill in first**

```sh
export LAB="$HOME/lab/aj-tasks"                         # the throwaway project
export CLAUDE_DIR="<the lab-profile-1 profile's folder>"
export CODEX_DIR="<the Codex profile's folder>"         # e.g. ~/.codex
export AJ_LAB_OUT=/tmp/aj-tasks-lab                     # snapshots and notes (0700)
export AJ_DATA="$HOME/Library/Application Support/AI Juggler Dev"
export DISABLE_AUTOUPDATER=1
```

## 0. Set up and snapshot (5 min)

- [ ] `mkdir -p "$LAB" && cd "$LAB" && git init -q && echo "lab" > README.md`
- [ ] `scripts/qa/runtime_lab_snapshot.sh take before "$CLAUDE_DIR" "$CODEX_DIR" "$AJ_DATA"`
      (from the repo; the lab's processes must be none).
- [ ] `just dev`. In AI Juggler Dev: the two profiles exist; the agent tools
      question (a pool's page) is answered **Yes**.

## 1. A plan, two sessions (10 min)

- [ ] Start a **Claude Code** session in `$LAB` in its **terminal**. Name it
      **Planner**. Start a **Codex** session in `$LAB` (driven). On AI
      Juggler's **Trust this folder?** sheet pick **Open restricted**. Name
      it **Builder**.
- [ ] On **lab · all sessions** › **Edit pool**, make Planner the **Lead**.
- [ ] Its **Tasks** tab says "No tasks yet". **New task** › kind **Epic**,
      "Lab epic"; then **New task** › kind **Story**, **More** › **Part of**
      "Lab epic", "Lab story". **Plan** shows the epic, the story under it,
      "0 of 1 done".

## 2. An agent plans; the list stays honest (10 min)

- [ ] In Planner's terminal: "Use the juggler tools: task_create a task
      'Write hello.txt' under Lab story with acceptance '- hello.txt says
      hello', then task_create 'Write bye.txt' blocked_by the first."
      Record: no permission prompt for the juggler tools (D-30); both ids
      (`aj-…`).
- [ ] The Tasks tab lists **Write hello.txt · Ready · Anyone can take it**
      first and **Write bye.txt · Blocked · Waits on aj-…** below it. The
      task view says "Written by Planner (an agent: its words, not yours)".
- [ ] In Planner: "tasks_list ready: true". Only hello.txt is listed.

## 3. Two agents claim one task (10 min)

- [ ] In Builder (its page's message box): "Use the juggler tool task_claim
      on aj-<hello>." At about the same time, in Planner: "task_claim
      aj-<hello>." Exactly one of them gets it; the other's tool says it's
      the first one's already. Record who won and the words.
- [ ] The task view says **In progress with <winner>, running now** and
      **Open its session**; its history says "<winner> took it" once.
- [ ] The winner: "write hello.txt saying hello, then task_update its task
      to in_review with a note." The row says **In review**; the history has
      the note. On the pool's **Members**, the winner's card says
      **Task: aj-… · Write hello.txt (in review)**.
- [ ] You set it **Done** (Status). **Write bye.txt** turns **Ready** by
      itself.

## 4. Above the level, it waits for you (10 min)

- [ ] In Planner (the lead): "task_assign aj-<bye> to Builder (its
      session_id from pool_roster)." The tool says it's held for the user's
      OK. Home › Needs you: **An agent asks about a task** with Allow once /
      Deny (keys `1`, `2`). Press `1`. The task says "With Builder".
- [ ] Do it once more with another task: the card offers **Always allow in
      this pool**. Press **Deny**: the task is as it was; its history says
      "you said no".
- [ ] Set the pool to **Read**. In Builder: "task_create 'x'". Held for
      your OK; press **Deny**. Set the pool back to **Work together**.
- [ ] In Builder (a member): "task_assign aj-<bye> to nobody." Refused:
      only the pool's lead gives tasks out. Record the words.

## 5. No agent starts a session (10 min)

- [ ] In Planner: "task_assign aj-<bye> to new_session with note 'needs its
      own session'." The tool says the user decides and nothing starts by
      itself. Count the sessions: unchanged (`scripts/qa/runtime_lab_snapshot.sh procs`).
- [ ] Needs you shows **«Planner» asks for a new session for aj-…** with
      the launch settings picker (nothing picked unless you set the lab
      project's default account), **Start session** greyed until you pick,
      and **Not now**.
- [ ] Ask Planner to do it again: still one request.
- [ ] Pick the **lab-profile-1** launch settings, **Start session**. A new
      Claude session starts in `$LAB`; its first prompt begins
      `[AI Juggler: you started this session for the task aj-…`. The task
      says **In progress with** the new session; **Sessions** lists it as
      "started for it". Stop it at once (its menu › Stop) after its first
      reply.

## 6. A switch keeps the task (10 min)

- [ ] Give Builder a task (the task's **With** › Builder), and in Builder:
      "task_claim it." Then continue Builder on another account or tool
      (its menu › **Switch account…**, the maintainer's choice of a second
      lab profile; skip this step if there is none).
- [ ] The new session's page has a **Tasks** tab with the same task, in
      progress. In it: "tasks_list mine: true": the task, `yours: true`.
- [ ] The project's **Overview** › **How this work travelled** shows the
      task under both sessions.

## 7. An issue link (5 min, needs `gh` signed in to github.com)

- [ ] Edit a task (or create one) with the link
      `https://github.com/figurehead-ai/AI-juggler/issues/1`. The task shows
      `figurehead-ai/AI-juggler#1` and **Look up its title**; nothing was
      looked up yet.
- [ ] Click it: the title and state appear with "looked up just now".
- [ ] Turn Wi-Fi off and click **Look it up again**: it says it couldn't
      (offline), and the link still opens.

## 8. What changed on disk (5 min)

- [ ] `scripts/qa/runtime_lab_snapshot.sh take after "$CLAUDE_DIR" "$CODEX_DIR" "$AJ_DATA"`
- [ ] `scripts/qa/runtime_lab_snapshot.sh diff before after`: nothing new
      or changed under `$CLAUDE_DIR` or `$CODEX_DIR` except the tools' own
      session files and Claude's folder-trust entry from step 1; under
      `$AJ_DATA`, the database, `agent/` and `overlays/`.
- [ ] Tasks are in AI Juggler's data only:
      `sqlite3 "$AJ_DATA/juggler.sqlite" "select status, count(*) from tasks group by status; select verb, count(*) from task_events group by verb"`
      (counts only; no titles printed).

## 9. Clean up (5 min)

- [ ] End every session in AI Juggler Dev; quit AI Juggler Dev.
- [ ] `scripts/qa/runtime_lab_snapshot.sh procs` prints nothing.
- [ ] `rm -rf "$LAB"` and note what was left (nothing expected).

**Record** in `docs/qa/evidence/AJ-E1902/lab-<date>.md`: the versions
(`claude --version`, `codex --version`, `gh --version`), each step's
PASS / FAIL / NOT RUN with what was seen, the snapshot diff's summary, and
any words that surprised you. A FAIL on a hard line (a session started
without your click; an account picked for you by usage; two agents holding
one task; a change above the level applied without your OK; a write outside
the expected folders) stops the lab.
