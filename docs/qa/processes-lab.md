# Processes lab: the E24 live checks (AJ-E2406)

> **Superseded for accounts (2026-11-11, ADR-0032's evidence):** a lab
> starts a tool only on the sanctioned test profile (`aij-test`), AI
> Juggler Dev's own Lab App folder, or a folder under a throwaway lab home,
> never on one of the maintainer's own tool folders (`~/.claude`,
> `~/.codex`, `ClaudeCodeProfiles/*`). Labs run here from 10-25 to 11-07
> started 62 Claude CLI sessions on two real profiles' folders, which then
> showed as "CLI" Apps on the maintainer's screens. AI Juggler Dev now
> refuses such a start (`lab_scope::refuses_real_folder`), and so does the
> switch lab.

The runnable checklist for what only a real Mac shows about the Processes
view (E24): AI Juggler's own sessions and what they leave, another app's
session and its leftover (D-41), a zombie, a process that won't stop, and
that the counts agree everywhere. It is written to be run as is, by the
maintainer or by an agent the maintainer allowed to run live lab tests in
AI Juggler Dev with one or two real tools (2026-10-25).

**Rules while running it**
- AI Juggler Dev only, in a throwaway lab project. An agent runs it on a
  lab HOME (`scripts/qa/lab_scenario.sh`); the tools' sign-ins there are
  the maintainer's one step. Never the installed AI Juggler, never a
  project with real work, never the maintainer's own processes.
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
- Settings › Sessions › **Restart sessions that crash** stays **off**: the
  leftovers below come from a stopped tool's crash (see "The crash route").

**The crash route (rewritten 2026-11-07 from lab 1).** Claude Code stops
its own background tasks when a session is stopped or `/exit`ed, so a
clean end leaves nothing behind. What leaves a task running is the tool
crashing: end the lab's **own** `claude` with `kill -KILL PID`, where PID
is the pid on its "the session itself" row in Processes, checked first
with `ps -o pid=,comm= -p PID` (it must say `claude`). A signal end counts
as a crash, so the second rule ("after a crash") is the one that applies.
- Check nothing is left at the end (step 12).

**Fill in first**

```sh
export LAB="$HOME/lab/aj-processes"          # the throwaway project (HOME is the lab's)
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
3. Open **Processes** (More › Processes, or ⌘⇧C). Choose **Everything
   running**.

- [ ] Under **AI Juggler**, the lab session's row unfolds to its own
      process, a shell (`zsh`), and under it the server with **:8765**:
      `python3 (http.server)`, or `Python` when the interpreter runs from
      Python.app (Homebrew, the command-line tools), its role "probably a
      dev server", CPU with a line, memory, age, and **Started by AI
      Juggler**.
- [ ] Where the tool says what it ran, the **shell's** row says **Started
      by Bash: …** (Claude Code's hooks; the server under it doesn't) or
      **Started by a command: …** (Codex).
- [ ] `juggler-agent` processes don't show; **Show AI Juggler's own** in
      the filters shows them as "AI Juggler hook" / "AI Juggler's tools",
      with no stop.
- [ ] A healthy session folds to one line with its count, memory and CPU.

## 2. The session ends and leaves it behind

1. Turn on Settings › Notifications › **A session leaves something
   running** first if you want to see the notification.
2. End the session by the crash route (its `claude`, `kill -KILL`). A
   **Stop** from its page would stop the server too (lab 1).

- [ ] Within about 40 s (30 s of grace, then two agreeing ticks), the
      shell and the server are **Left behind** in **Needs attention**,
      under the session (now ended).
- [ ] The top bar's "things to clean up" button, This Mac's **Left behind**
      and the page's "to clean up" say the same number (2).
- [ ] With the notification turned on (and AI Juggler not in front), one
      notification: "Left 2 processes running".

## 3. Keep and Remind me later

- [ ] **⋯ › Keep (not a leftover)** takes it out of Needs attention and the
      counts (it stays in Everything running). Undo it (**Don't keep**).
- [ ] **⋯ › Remind me in 1 h** does the same until then. Undo it (**Remind
      me now**).

## 4. Stop it, then see it in Ended and the history

1. **Stop…** on the left-behind server.

- [ ] The sheet checks it now (started by AI Juggler, the same process, its
      session has ended) and names what stopping frees (":8765").
- [ ] After **Terminate …**, it says how it ended ("exited on TERM").
- [ ] **Ended** lists it with **Ran for** and its peak memory; **Cleanup
      history** lists the stop under the lab session (by its folder's name
      while it has no title), and the lab session's own **Processes ›
      Cleanup history** shows only its stops. Stop the left-behind shell
      the same way.

## 5. A process that won't stop politely

1. Write the lab's own script once:

   ```sh
   cat > "$LAB/stubborn.py" <<'PY'
   import signal, socket, time
   signal.signal(signal.SIGTERM, signal.SIG_IGN)
   s = socket.socket()
   s.bind(("127.0.0.1", 8767))
   s.listen()
   time.sleep(900)
   PY
   ```

2. In a new lab session, ask it to run `python3 stubborn.py` in the
   background.
3. End the session by the crash route; when the script shows **Left
   behind**, **Stop…** it.

- [ ] The sheet says it ignored the polite request and needed a force stop
      ("needed KILL"), and :8767 is free after (`lsof -nP -iTCP:8767 -sTCP:LISTEN` prints nothing).

## 6. The automatic rule's dry run (the lab account only)

1. Start a lab session, run `python3 -m http.server 8765` in the
   background, and end the session by the crash route so the shell and
   the server are left behind.
2. Under **Automatic cleanup** on the Processes page, for the **lab's
   account only**, choose **Preview** for each rule.

Claude Code starts each command in a process group of its own, so its
leftovers are tidied only after a crash, by the second rule (ADR-0010,
its AJ-E2405 amendment):

- [ ] "Tidy left-behind dev servers" (the first rule) says it would never
      tidy them: "Its session crashed (the second rule)".
- [ ] "Also after a crash" (the second rule) says it would tidy the server
      (and the shell, when its command line names the server): now, or
      after their wait (five minutes after the crash, one notice, then the
      Undo window). Nothing is stopped by a preview.
- [ ] Mark the server **Keep**: the preview says it would never tidy it
      ("You chose Keep"). Clear Keep, then stop both by hand. Leave the
      rules off.

## 7. Another app's session and its leftover (D-41)

1. In Terminal (or the Claude app, in `$LAB`), start `claude` in `$LAB`
   and ask it to run `python3 -m http.server 8766` in the background.

- [ ] Processes shows **Terminal** (or **Claude**) → the session (its
      title) → `claude` → `python3 (http.server)` **:8766**, owner **In a
      session Terminal runs**. There is no Stop while that session runs
      ("Its session is still running in its app").
- [ ] The session's page in AI Juggler has the same rows on its
      **Processes** tab.

2. Let the server run at least 10 s (a process is sighted only past
   10 s), then end that session by the crash route on **its** `claude`
   (`/exit` stops the server too).

- [ ] Within about 10 s, it is **Left behind** under the ended session
      (its title may be empty for a session with no title yet).
- [ ] **Stop…** opens the same sheet, with no scope to choose ("Only this
      process: the app's session and its other processes are never
      touched."): it checks the process is the one it saw (pid, start time,
      program, this boot) and that its session ended. After **Terminate …**,
      Cleanup history lists it as "You (another app's session)", named by
      its program only (`python3`, or `Python`).
- [ ] The app itself and the session's own process never offer **Stop**.

## 8. Finished, waiting for its parent (a zombie)

1. Write a script that starts a short command and doesn't wait for it
   (as the supervisor's own zombie tests do):

   ```sh
   cat > "$LAB/waits.py" <<'PY'
   import subprocess, time
   subprocess.Popen(["/usr/bin/true"])
   time.sleep(180)
   PY
   ```

2. In a lab session, ask it to run `python3 waits.py` in the background.

- [ ] Within about 10 s (a full scan), `true` shows as **Waiting for its
      parent**, naming its parent as the process table does (`python3` or
      `Python`: "Finished; only … can clear it"), with the plain
      explanation above the list and no Stop. After a minute it is in
      Needs attention.
- [ ] **Stop…** on the parent while the session runs (or the 180 s
      running out) clears it. Once the session ends its tree isn't watched,
      so the waiting child no longer shows.

## 9. Picked up after restart

A session in AI Juggler's own terminal ends with the app, so there is
nothing to pick up; this step needs a session AI Juggler started that
keeps running without it (a desktop app's). Skip it, and say so in the
record, when the lab has none.

1. With such a session running a `python3 -m http.server 8765`, quit AI
   Juggler Dev and start it again.

- [ ] The session's processes are **Picked up after restart**, and nothing
      was stopped.

## 10. This Mac lists the other app's session

1. With the step 7 session running again in Terminal (or the Claude app):

- [ ] This Mac › What's using it counts it under its app's group: a
      `claude` inside the Claude app in the "Claude app" row, a `claude` in
      a terminal as "claude in a terminal"; another tool's session (a Codex
      session in Terminal, ChatGPT) is listed by its title as "Codex in
      Terminal". Each is marked "not started by AI Juggler", with no
      controls.
- [ ] **Filters › Include other apps** in Everything running adds "Other
      apps on this Mac": programs by name and memory only, with no Stop.

## 11. What was kept of other apps' processes

```sh
sqlite3 "$AJ_DATA/juggler.sqlite" \
  "select pid, exe_name, length(cmd_hash), session_root from process_sightings order by last_seen desc limit 10;"
sqlite3 "$AJ_DATA/juggler.sqlite" "pragma table_info(process_sightings);"
```

- [ ] Program names and 32-character hashes only; no column holds a command
      line, a folder or an environment.

## 12. Nothing left

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
