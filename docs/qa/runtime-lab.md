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
- Stop everything you start, and check nothing is left (step 6).

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
- [ ] `ls "$AJ_DATA/runtime"` lists one `.sock`; `stat -f '%Sp %N' "$AJ_DATA/runtime" "$AJ_DATA"/runtime/*.sock`
      shows `drwx------` and `srw-------`.
- [ ] `pgrep -fl 'app-server --listen unix://'` shows one `codex app-server`
      whose socket is that file (and nothing under `app-server-control/`).
- [ ] Before any turn, the session page shows the 5-hour and weekly limits
      **with their reset times** (record them).
- [ ] Send "Run `ls` in this folder." Needs you shows **Asks you: ls** with
      Codex's options (Allow once, Allow for this session, …, Deny, Deny
      and stop). Click **Allow once**: the turn completes and the reply is
      on the session page.
- [ ] Send "Ask me which colour I like, with two options, using your
      question tool." Record what came: a question with two choices (answer
      it in AI Juggler; the turn carries on), or plain text (then questions
      arrive as text on this version).
- [ ] Settings › Activity (the audit) lists the answer: session, kind,
      the option, the time; never a reply's text.

## 2. Codex: the real TUI on the same thread (5 min)

- [ ] On the session page, **Open in terminal**. A terminal opens with
      `codex --remote unix://… resume <thread>`: the same conversation,
      live. (If it doesn't, record the exact error: `--remote` before
      `resume` is the spike's question.)
- [ ] In that TUI, send "Run `echo two-clients`." **Who is asked**: the
      TUI, AI Juggler, or both? If both, answer in the TUI and check AI
      Juggler's ask disappears (`serverRequest/resolved`); record it.
- [ ] Close the TUI (Ctrl-C twice). AI Juggler's session carries on.
- [ ] **Stop** the session in AI Juggler. `pgrep -fl 'app-server --listen unix://'`
      prints nothing; `ls "$AJ_DATA/runtime"` has no socket left.
- [ ] Optional, by hand, without AI Juggler (the protocol itself):
      ```sh
      mkdir -p /tmp/aj-rt && chmod 700 /tmp/aj-rt
      CODEX_HOME="$CODEX_DIR" codex app-server --listen unix:///tmp/aj-rt/s.sock &
      nc -U /tmp/aj-rt/s.sock
      {"id":1,"method":"initialize","params":{"clientInfo":{"name":"ai_juggler","title":"AI Juggler","version":"lab"}}}
      {"method":"initialized"}
      {"id":2,"method":"account/rateLimits/read"}
      ```
      Record both answers (`codexHome` must be `$CODEX_DIR`), then Ctrl-C
      `nc`, `kill %1`, `rm -rf /tmp/aj-rt`.

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
  - [ ] `/hooks` in the session lists both; the status line shows what the
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

## 5. Snapshot after and compare (5 min)

- [ ] `scripts/qa/runtime_lab_snapshot.sh take after "$CODEX_DIR" "$CLAUDE_DIR" "$AJ_DATA"`
- [ ] `scripts/qa/runtime_lab_snapshot.sh diff before after > "$AJ_LAB_OUT/diff.txt"`
      and read it. **Expected** in the tool folders: new conversation files
      (`$CODEX_DIR/sessions/…/rollout-*.jsonl`, `$CLAUDE_DIR/projects/…/*.jsonl`),
      their history and log lines, Codex's state database. **Never**:
      a changed `settings.json`, `config.toml` or sign-in file, or any file
      of AI Juggler's (`juggler-*`, `launch-settings.json`, a socket) in a
      tool folder. In `$AJ_DATA`: `overlays/runtime/<session>/settings.json`
      for the Claude sessions and nothing under `runtime/` after the stops.
- [ ] The processes section is empty.

## 6. Clean up (2 min)

- [ ] Stop any session still running in AI Juggler Dev, quit it.
- [ ] `pgrep -fl 'app-server --listen unix://|claude -p --input-format stream-json|juggler-agent'`
      prints nothing (if it does: `kill <pid>` each, and record it as a bug).
- [ ] `rm -rf "$LAB" /tmp/aj-lab-hook.log /tmp/aj-rt`
- [ ] The lab's conversations stay in the tools' history (they are the
      account's own); archive the Codex thread in `codex resume` if wanted.
- [ ] Keep `$AJ_LAB_OUT` until the evidence is written, then `rm -rf "$AJ_LAB_OUT"`.

## 7. Write the evidence

- [ ] `docs/qa/evidence/AJ-E2301/manifest.md`: each checkbox above with
      what was seen, the versions (`codex --version`, `claude --version`),
      the date, and the diff's tool-folder section (paths only).
- [ ] Captures (if any) go to `docs/research/evidence/`, redacted with the
      fail-closed tool and checked by eye.
- [ ] Turn the "unverified" lines in `docs/qa/evidence/AJ-E2302`…`AJ-E2308`
      into what was measured. A difference is a bug in the adapter's
      `codex_appserver/`, `claude_hooks.rs` or `claude_stream/` only.
