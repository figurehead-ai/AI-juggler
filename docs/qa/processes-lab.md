# Processes lab: the E24 live checks (AJ-E2406)

The runnable checklist for what only a real Mac shows about the Processes
view (E24): AI Juggler's own sessions and what they leave, another app's
session and its leftover (D-41), a zombie, a process that won't stop, and
that the counts agree everywhere. It is written to be run as is, by the
maintainer or by an agent the maintainer allowed to run live lab tests in
AI Juggler Dev with one or two real tools (2026-10-25).

**Rules while running it**
- AI Juggler Dev only (`just dev`), in a throwaway lab project. Never the
  installed AI Juggler, never a project with real work.
- **Only stop what this lab started.** Every process stopped below is one
  the lab itself started (a `python3` on a lab port, a lab script). Never
  stop, mark or touch any other process on the Mac, even if the page
  offers it.
- List processes by pid and program name only (`ps -o pid=,comm=`, or
  `scripts/qa/runtime_lab_snapshot.sh procs PATTERN`): never `pgrep -fl`
  or `ps eww`, which can print other processes' environments.
- Sign in to nothing; allow only the lab's own commands below in the
  tools' dialogs; read no credential file, the Keychain or a process
  environment.
- Leave the automatic rules off for every real account. Step 6 turns one
  on only for the lab's own account, and turns it off again.
- Check nothing is left at the end (step 11).

**Fill in first**

```sh
export LAB="$HOME/lab/aj-processes"          # the throwaway project
export AJ_DATA="$HOME/Library/Application Support/AI Juggler Dev"
export AJ_LAB_OUT=/tmp/aj-processes-lab       # notes and listings (0700)
mkdir -p "$LAB" "$AJ_LAB_OUT" && chmod 700 "$AJ_LAB_OUT"
```

The lab's ports: 8765 (AI Juggler's session), 8766 (another app's), 8767
(the process that ignores stopping).

## 1. An AI Juggler session and what it starts

1. In AI Juggler Dev, start a session in `$LAB` (Claude Code in its
   terminal, or Codex).
2. Ask it to run, in the background: `python3 -m http.server 8765`.
3. Open **Processes** (the top bar, or ⌘⇧C). Choose **Everything running**.

- [ ] Under **AI Juggler**, the lab session's row unfolds to its own
      process, and under it `python3 (http.server)` with **:8765**, its
      role "probably a dev server", CPU with a line, memory, age, and
      **Started by AI Juggler**.
- [ ] Where the tool says what it ran, the row says **Started by Bash: …**
      (Claude Code's hooks) or **Started by a command: …** (Codex).
- [ ] `juggler-agent` processes don't show; **Show AI Juggler's own** in
      the filters shows them as "AI Juggler hook" / "AI Juggler's tools",
      with no stop.
- [ ] A healthy session folds to one line with its count, memory and CPU.

## 2. The session ends and leaves it behind

1. Stop the session from its page (**Stop**), leaving the server running.
2. Turn on Settings › Notifications › **A session leaves something
   running** beforehand if you want to see the notification.

- [ ] Within about 40 s, `python3 (http.server)` is **Left behind** in
      **Needs attention**, under the session (now ended).
- [ ] The top bar's pill, This Mac's **Left behind** and the page's count
      say the same number.
- [ ] With the notification turned on (and AI Juggler not in front), one
      notification: "Left 1 process running".

## 3. Keep and Remind me later

- [ ] **Keep** takes it out of Needs attention and the counts (it stays in
      Everything running, marked Keep). Undo it.
- [ ] **Remind me in 1 h** does the same until then. Undo it.

## 4. Stop it, then see it in Ended and the history

1. **Stop…** on the left-behind server.

- [ ] The sheet checks it now (started by AI Juggler, the same process, its
      session has ended) and names what stopping frees (":8765").
- [ ] After **Stop**, it says how it ended ("exited on TERM").
- [ ] **Ended** lists it with how long it ran; **Cleanup history**, filtered
      to the lab session, lists the stop.

## 5. A process that won't stop politely

1. In a new lab session, ask it to run in the background:
   `python3 -c "import signal,socket,time; signal.signal(signal.SIGTERM, signal.SIG_IGN); s=socket.socket(); s.bind(('127.0.0.1',8767)); s.listen(); time.sleep(900)"`
2. Stop the session; when the script shows **Left behind**, **Stop…** it.

- [ ] The sheet says it ignored the polite request and needed a force stop
      ("needed KILL"), and :8767 is free after (`lsof -nP -iTCP:8767 -sTCP:LISTEN` prints nothing).

## 6. The automatic rule's dry run (the lab account only)

1. Start a lab session, run `python3 -m http.server 8765` in the
   background, and stop the session so the server is left behind.
2. In Processes › Rules, for the **lab's account only**, choose **Preview**
   on "Automatically tidy left-behind dev servers".

- [ ] The preview says "Would tidy python3 :8765 now" (or "after their
      wait" within the first minute), and nothing is stopped by the preview.
- [ ] Mark it **Keep**: the preview says it would never tidy it ("You chose
      Keep"). Clear Keep, then stop the server by hand. Leave the rule off.

## 7. Another app's session and its leftover (D-41)

1. In Terminal (or the Claude app, in `$LAB`), start `claude` in `$LAB`
   and ask it to run `python3 -m http.server 8766` in the background.

- [ ] Processes shows **Terminal** (or **Claude**) → the session (its
      title) → `claude` → `python3 (http.server)` **:8766**, owner **In a
      session Terminal runs**. There is no Stop while that session runs
      ("Its session is still running in its app").
- [ ] The session's page in AI Juggler has the same rows on its
      **Processes** tab.

2. End that session (`/exit`), leaving the server running.

- [ ] Within about 10 s, it is **Left behind** under the ended session.
- [ ] **Stop…** opens the same sheet: it checks the process is the one it
      saw (pid, start time, program, this boot) and that its session ended.
      After **Stop**, Cleanup history lists it as stopped by you, named
      `python3` only.
- [ ] The app itself and the session's own process never offer **Stop**.

## 8. Finished, waiting for its parent (a zombie)

1. In a lab session, run in the background:
   `python3 -c "import os,time; os.fork() or os._exit(0); time.sleep(180)"`

- [ ] A child shows as **Finished, waiting for its parent**, naming its
      parent `python3`, with the plain explanation and no Stop. After a
      minute it is in Needs attention.
- [ ] Stopping the parent (it is left behind once the session ends, or
      wait 3 minutes) clears it.

## 9. Picked up after restart

1. With a lab session running a `python3 -m http.server 8765`, quit AI
   Juggler Dev and start it again.

- [ ] The session's processes are **Picked up after restart**, and nothing
      was stopped.

## 10. What was kept of other apps' processes

```sh
sqlite3 "$AJ_DATA/juggler.sqlite" \
  "select pid, exe_name, length(cmd_hash), session_root from process_sightings order by last_seen desc limit 10;"
sqlite3 "$AJ_DATA/juggler.sqlite" "pragma table_info(process_sightings);"
```

- [ ] Program names and 32-character hashes only; no column holds a command
      line, a folder or an environment.

## 11. Nothing left

```sh
lsof -nP -iTCP:8765-8767 -sTCP:LISTEN
scripts/qa/runtime_lab_snapshot.sh procs python3
```

- [ ] No listener on the lab's ports, and no lab `python3` left. Stop any
      that is, from Processes (it is the lab's own).
- [ ] The automatic rule is off for every account; delete `$LAB` if you
      like.

Record what you saw in `docs/qa/evidence/AJ-E2406/lab-1-<date>.md`
(listings by pid and program name only; captures through the redaction
tool with their sidecars).
