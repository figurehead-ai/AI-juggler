# Runtime lab: the E23 live checks (AJ-E2301)

The runnable checklist for the five things only a real run shows (ADR-0022
Consequences; E23 AJ-E2301). It is written to be run as is: by the
maintainer, or by an agent the maintainer allowed to run live lab tests
with one or two real tools on the maintainer's accounts (2026-10-25). The
maintainer's longer note is `docs/internal/MAINTAINER-STEPS.md` §18.

**Rules while running it**
- AI Juggler Dev only (`just dev`), in a throwaway lab project. Never the
  installed AI Juggler, never a project with real work.
- Allow only the lab commands below (`ls`, `echo`). Sign in to nothing,
  click no Allow that isn't in this list, read no credential file, the
  Keychain or a process environment.
- Nothing in an account's own folder is edited. The one hook the merge
  check needs lives in the lab project, and is removed at the end.
- Snapshot the tool folders before and after (metadata only:
  `scripts/qa/runtime_lab_snapshot.sh` lists names, sizes and times, never
  contents).
- Stop everything you start, and check nothing is left (step 7).

**Learned in lab 2 (2026-10-25, `docs/qa/evidence/AJ-E2301/lab-2-2026-10-25.md`)**
- **Codex's trust screen.** `codex resume` in a folder Codex hasn't trusted asks "Trust this folder?", and the answer is saved to the user's `config.toml`. Quit rather than trust, or start with `-c 'projects={"<lab>"={trust_level="untrusted"}}'`, which saves nothing and offers "Open restricted". Skip Codex's update prompt with Esc.
- **Claude's onboarding.** A profile that has never run Claude Code in a terminal shows a theme picker and security notes first; the theme choice creates its `settings.json`. On the trust dialog the default is **"No, exit"**: move to "Yes" before Enter.
- **Claude's self-update.** The first terminal start can update Claude Code for the whole Mac. Set `DISABLE_AUTOUPDATER=1` for the lab's terminal runs.
- **A dialog needs a write.** In auto mode (some accounts' default), and for read-only commands, Claude asks nothing. Use `echo lab > lab.txt` to bring up a dialog.
- **`/hooks` doesn't list `--settings` hooks.** Check the "running … hooks 0/2" line and the hook logs instead.
- **Process listings.** Print `pid` and `comm` only; `pgrep -fl` can show other processes' environment text.

**Fill in first**

```sh
export LAB="$HOME/lab/aj-runtime"                       # the throwaway project
export CODEX_DIR="<the Codex profile's folder>"         # e.g. ~/.codex
export CLAUDE_DIR="<the Claude Code profile's folder>"  # e.g. ~/.claude
export AJ_LAB_OUT=/tmp/aj-runtime-lab                   # snapshots and notes (0700)
export AJ_DATA="$HOME/Library/Application Support/AI Juggler Dev"
```

## 0. Set up and snapshot (5 min)

- [ ] `mkdir -p "$LAB" && cd "$LAB" && git init -q && echo "lab" > README.md`
- [ ] `scripts/qa/runtime_lab_snapshot.sh take before "$CODEX_DIR" "$CLAUDE_DIR" "$AJ_DATA"`
      (run from the repo; it prints the file counts and the lab processes,
      which must be none).
- [ ] `just dev` and wait for AI Juggler Dev's window. In it, the two
      profiles above exist (add them "where they are" if not).

## 1. Codex driven through app-server (10 min)

- [ ] New session → the Codex profile → folder `$LAB` → Start. The session
      page says **Runs through Codex's app-server**.
- [ ] `pgrep -fl 'codex app-server --listen stdio://' | grep -v /Applications/`
      shows one server for the session (the Codex app's own server, inside
      `/Applications`, isn't the lab's). Over stdio there is no socket
      anywhere, and nothing new under `/private/tmp/codex-daemon-*`.
- [ ] Before any turn, the session page shows the limits Codex reports,
      **each with its reset time**, named by length (5 hours, weekly). An
      account that reports only the weekly window shows "no 5-hour limit
      reported", never a made-up one (record what came).
- [ ] Send "Run `ls` in this folder." Needs you shows **Asks you: ls** with
      Codex's own options (only the decisions it lists). Click **Allow
      once**: the turn completes and the reply is on the session page.
- [ ] Send "Run `ls` and then `pwd`." Two small approvals in a row: Needs
      you lists them together; answer both from the keyboard (Tab to the
      ask, `1` for its first option, `Enter` for the focused one).
- [ ] Send "Ask me which colour I like, with two options, using your
      question tool." Record what came: a question with two choices (answer
      it in AI Juggler; the turn carries on), or plain text (then questions
      arrive as text on this version).
- [ ] The answers are in the account's audit, with the session, the kind
      and the option, never a reply's text:
      `sqlite3 "$AJ_DATA/juggler.sqlite" "select action, detail, actor from account_audit where action = 'answered' order by at desc limit 5"`

## 2. Codex: the real TUI on the same thread, by hand-over (5 min)

- [ ] On the session page, **Open in terminal**. It waits for the turn's
      end; then the app-server ends (`pgrep` above prints nothing) and a new
      session opens in AI Juggler's terminal with `codex resume <thread>
      --no-daemon`: the same conversation. The old session says "handed
      over".
- [ ] Send "hello" in the TUI, then exit it (`/quit`). On that session's
      page, **Run in AI Juggler**: a new driven session continues the same
      thread (its last messages include "hello").
- [ ] **Stop** the session in AI Juggler. `pgrep -fl 'codex app-server'`
      lists nothing of the lab's (the Codex app's own server, under
      `/Applications/ChatGPT.app`, is not the lab's).
- [ ] Optional, the protocol by hand without AI Juggler (stdio, the way AI
      Juggler speaks it; paste one line at a time, `Ctrl-D` ends it):
      ```sh
      CODEX_HOME="$CODEX_DIR" codex app-server --listen stdio://
      {"id":1,"method":"initialize","params":{"clientInfo":{"name":"ai_juggler","title":"AI Juggler","version":"lab"}}}
      {"method":"initialized"}
      {"id":2,"method":"account/rateLimits/read"}
      ```
      Record both answers (`codexHome` must be `$CODEX_DIR`).
- [ ] Still open from the first lab, if you want to look: two clients on
      one thread (`--listen unix://` speaks WebSocket; AI Juggler doesn't
      use it yet, ADR-0022).

## 3. Claude: hooks and the status line through `--settings` (10 min)

- [ ] The merge check's own hook, in the **lab project** (never the
      account's folder):
      ```sh
      mkdir -p "$LAB/.claude"
      cat > "$LAB/.claude/settings.json" <<'JSON'
      {"hooks":{"UserPromptSubmit":[{"hooks":[{"type":"command","command":"echo lab-hook >> /tmp/aj-lab-hook.log"}]}]}}
      JSON
      ```
- [ ] New session → the Claude Code profile → `$LAB` → Start (the
      terminal, as always for Claude). Trust the folder if Claude Code asks.
      Send "hello".
  - [ ] `cat /tmp/aj-lab-hook.log` shows `lab-hook` (**the project's hook
        still runs**), and the session went Working → Idle in AI Juggler
        with no "probably" (**AI Juggler's hooks run too**).
  - [ ] `/hooks` in the session lists the project's hook (not the `--settings` ones: lab 2); the status line shows what the
        account's own status line shows.
  - [ ] The session page shows the account's limits with resets, from the
        status line (Pro or Max).
  - [ ] Send "Run `ls`." Does Claude Code's own dialog show **at once**,
        while `juggler-agent hook` runs, or only after? Needs you shows
        **Asks you** with the tool and the command (answer it in the
        terminal: Claude terminal sessions aren't answered from AI Juggler).
- [ ] `/exit`. The session ends as Ended, exactly.

## 4. Claude without a terminal (Beta) and the hand-over (10 min)

- [ ] Settings › Sessions › **Run without a terminal (Beta)** on.
- [ ] New session → the Claude Code profile → `$LAB` → More → **Run
      without a terminal: answer from AI Juggler** → Start. The session page
      says **Runs without a terminal (Beta)**.
  - [ ] `pgrep -fl 'claude -p --input-format stream-json'` shows it with
        `--permission-prompt-tool stdio` and without `--bare` or any
        permission bypass.
  - [ ] Send "Run `ls`." The approval shows with Claude Code's own options;
        **Allow once** carries on.
  - [ ] Send "Ask me which colour I like, using your question tool." The
        question and its options show; answering carries on.
  - [ ] A limit line with its reset shows, or record that none came.
  - [ ] **Open in terminal**: it waits for the turn's end, then the TUI
        opens on the same conversation (`claude --resume <id>`). Send
        "hello again" there, `/exit`, then **Run in AI Juggler**: it
        continues headless.
  - [ ] The transcript has one writer's turns in order:
        `ls "$CLAUDE_DIR/projects/"*aj-runtime*/` and read the newest
        `.jsonl`'s `"type"` lines in order (no two turns interleaved).
- [ ] Stop the session. Settings › Sessions › Run without a terminal (Beta)
      off again.
- [ ] The account's usage page (claude.ai) shows no "extra usage" or
      "third-party apps" note for these runs (record it, with the date).

## 5. Coming back after AI Juggler quits or crashes (AJ-E2310, 10 min)

Codex first (the default); then, if there is time, the same with a Claude
session without a terminal (the Beta switch on, as in step 4).

- [ ] **Between turns, a quit.** New Codex session → `$LAB` → send
      "hello", wait for Idle. Note its thread: on the session page, or
      `sqlite3 "$AJ_DATA/juggler.sqlite" "select value from app_settings where key = 'runtime.live'"`
      (the session id, the thread id and `"in_turn":false`; no message
      text). Quit AI Juggler Dev with **Quit anyway**: the sheet says the
      session **comes back when AI Juggler opens again**. `pgrep -fl 'codex app-server --listen stdio://' | grep -v /Applications/`
      prints nothing.
- [ ] `just dev` again. The same session (same row, same id) is driven
      again within a few seconds; `pgrep` above shows one new server; send
      "what did I say first?" and the reply knows "hello" (the same thread).
- [ ] **Between turns, a crash.** `pkill -9 -f 'target/debug/juggler-app'`
      (AI Juggler Dev only: check with `pgrep -fl juggler-app` first that
      nothing else matches). Its server ends within a few seconds (`pgrep`
      above). `just dev`: the session comes back as above.
- [ ] **Mid-turn, a crash.** Send "Run `ls` in this folder." and, while
      **Asks you** shows, `pkill -9 -f 'target/debug/juggler-app'`. `just
      dev`: the session says **Its turn was cut off when AI Juggler
      stopped**, is in Needs you, and nothing runs (`pgrep` above prints
      nothing). Click **Resume**: it is driven again on the same thread,
      nothing is sent by itself; send "hello again" and it answers.
- [ ] **Stopped first never comes back.** Stop the session, quit, `just
      dev`: it stays ended, and `runtime.live` no longer lists it.
- [ ] Record what Codex says about the cut-off turn after Resume (its
      thread may show the turn as interrupted).

## 6. Snapshot after and compare (5 min)

- [ ] `scripts/qa/runtime_lab_snapshot.sh take after "$CODEX_DIR" "$CLAUDE_DIR" "$AJ_DATA"`
- [ ] `scripts/qa/runtime_lab_snapshot.sh diff before after > "$AJ_LAB_OUT/diff.txt"`
      and read it. **Expected** in the tool folders, each the tool's own
      write on every start (the lab of 2026-10-25): new conversation files
      (`$CODEX_DIR/sessions/…/rollout-*.jsonl`, `$CLAUDE_DIR/projects/…/*.jsonl`,
      `projects/<folder>/memory/`), their history and log lines; Claude
      Code's `.claude.json` (and its backup), `remote-settings.json`,
      `policy-limits.json`, `skills/synced/…`, `plugins/synced/…`,
      `cache/model-catalog/…`, `.last-cleanup`; Codex's `skills/.system/…`,
      plugin and model caches, its SQLite files, `tmp/arg0-macos`, and a new
      modification time on `config.toml` at the same size (Codex rewrites
      it itself). **Never**: a change in the *content size* of the user's
      `settings.json` or `config.toml`, a sign-in file, or any file of AI
      Juggler's (`juggler-*`, `launch-settings.json`, a socket) in a tool
      folder. In `$AJ_DATA`: `overlays/runtime/<session>/settings.json` for
      the Claude sessions.
- [ ] The processes section is empty.

## 7. Clean up (2 min)

- [ ] Stop any session still running in AI Juggler Dev, quit it.
- [ ] `pgrep -fl 'app-server --listen stdio://|claude -p --input-format stream-json|juggler-agent' | grep -v /Applications/`
      prints nothing (if it does: `kill <pid>` each, and record it as a bug).
- [ ] `rm -rf "$LAB" /tmp/aj-lab-hook.log /tmp/aj-rt`
- [ ] The lab's conversations stay in the tools' history (they are the
      account's own); archive the Codex thread in `codex resume` if wanted.
- [ ] Keep `$AJ_LAB_OUT` until the evidence is written, then `rm -rf "$AJ_LAB_OUT"`.

## 8. Write the evidence

- [ ] `docs/qa/evidence/AJ-E2301/manifest.md`: each checkbox above with
      what was seen, the versions (`codex --version`, `claude --version`),
      the date, and the diff's tool-folder section (paths only).
- [ ] Captures (if any) go to `docs/research/evidence/`, redacted with the
      fail-closed tool and checked by eye.
- [ ] Turn the "unverified" lines in `docs/qa/evidence/AJ-E2302`…`AJ-E2308`
      into what was measured. A difference is a bug in the adapter's
      `codex_appserver/`, `claude_hooks.rs` or `claude_stream/` only.
