# Processes

AI sessions start other programs: dev servers, watchers, test runners,
MCP servers. Some keep running after the session ends. Processes shows
what each of your sessions runs, and each app's, what it started, and
what it left behind. You can stop a process there, after a check.

Open it from More › Processes, from the "things to clean up" button in the
top bar, from This Mac › Left behind › **Clean up**, or with ⌘⇧C. A
session's page has the same list for that session alone, under
**Processes**.

## What it shows

The page opens on **Needs attention**: only what needs you. The other
views are:

- **Everything running**: every session's processes, grouped by app and
  then by session. A session with nothing wrong is one line, for example
  "web-app · 6 processes · 1.2 GB · 4 %"; open it to see its processes.
- **Ended**: processes that ended in the last 7 days, how long each ran
  and the most memory it used.
- **Ports**: every port open on this Mac, what holds it, and whether a
  session left it running.
- **Cleanup history**: each process that was stopped, by whom (you, or an
  automatic rule), and how it ended. **Export the history** saves it.

The figures at the top count what there is to clean up, what didn't stop
when asked, what is waiting for its parent, and what is running. The
"things to clean up" button in the top bar and This Mac's **Left behind**
show the same count.

Each process shows:

- its name, and what it probably is ("probably a dev server") and what
  started it, when the tool said so ("Started by Bash: npm run dev");
- its state, in the words below;
- its CPU now, with a line of the last minute (100% is one core busy, as
  Activity Monitor counts it);
- its memory now (point at it for the most it has used), the ports it
  listens on, and how long it has run;
- whose it is: "Started by AI Juggler", or "In a session ChatGPT runs" for
  a session another app runs.

AI Juggler's own helpers, such as the hook that tells it what a session is
doing, are hidden. **Filters › Show AI Juggler's own** shows them.

One kind of process isn't shown under its session: a server an agent
starts in the background from a one-line command (`nohup npm run dev &`).
The command's shell ends at once and the server is handed to macOS, so
nothing left says which session started it. A tool's own background tasks
(Claude Code's "run in background") are shown.

## The states

| State | What it means |
|---|---|
| Working | It is running and busy. |
| Idle | It is running and doing nothing right now. |
| Left behind | Its session ended, and it is still running. |
| Left behind while closed | Its session ended while AI Juggler was closed. |
| Picked up after restart | AI Juggler was closed while it ran, and took it back when it opened again. |
| Waiting for its parent | It has finished, but its parent hasn't collected it yet. It uses no memory or CPU, and only its parent can clear it: by collecting it, or by ending. Some tools call this a "zombie". |
| Didn't stop | You asked it to stop, and it is still running. |
| Won't be touched | AI Juggler can't be sure it is the process the session started, so it offers no way to stop it. |
| Ended | It has ended. |

## Find something

Type in the search box to find a process by its name, what it probably
is, its session or its app. `port:5173` finds what listens on port 5173,
and `pid:4242` finds one process.

**Filters** narrows the list by state, app, profile or project, or to
processes listening on a port or using a lot. All filters are off until
you turn one on.

**Include other apps** adds the rest of this Mac to Everything running:
each program by name, with how many of it run and their memory. Nothing
else is read about them, and nothing offers to stop them; **Show in
Activity Monitor** opens Activity Monitor.

## Stop a process

Choose **Stop…** on a process. AI Juggler checks it again: that it is
the same program, started at the same time, as the one it recorded. The
sheet says so in one line, with what stopping it will free, such as a
port; **Details** shows its command, program and folder. A process that
may hold unsaved work (an MCP server, a shell, a database) is named as
such. Nothing happens until you confirm. Stopping asks the process
politely first, and forces it after 5 seconds only if it ignored that.

For one of AI Juggler's own processes you can stop more at once: **This
process and its helpers** (the ones AI Juggler started; any other process
in its group keeps running) or **Whole session tree**. Each process is
checked the same way just before it is stopped, and one that isn't the
process AI Juggler recorded is left alone; the report says so. Every stop
is saved in **Cleanup history**, which the report links to.

A session that left something behind offers **Clean up (N)…**: each of
its leftovers is checked, then stopped on its own, with the outcome beside
it.

A session's own process, while the session runs, offers **Go to session**
instead: you stop a session from its page, with its own **Stop**.

From a process's **⋯** menu you can also:

- **Keep (not a leftover)**: it stays, and stops counting as something to
  clean up.
- **Remind me in 1 h**: it leaves Needs attention for an hour.
- **Go to session**, **Reveal folder** and **Copy PID**.

### Another app's leftovers

A session the ChatGPT app or a terminal runs can leave processes behind
too. AI Juggler can stop one of those only when you ask, and only after
that session has ended. Before it does, it checks that the process is the
one it saw in that session (same process number, start time and program,
since this Mac last started). It stops only that process: never the app,
never the session itself, and never anything else of it. No automatic rule
ever stops another app's process. Each stop is in Cleanup history, as
"You (another app's session)".

## Automatic cleanup

Under the list, **Automatic cleanup** shows each profile's setting. It is
off until you turn it on, for each profile:

- **Tidy left-behind dev servers** stops dev servers and watchers AI
  Juggler started, after their session ended.
- **Also after a crash** does the same for what a session left when AI
  Juggler closed unexpectedly, after telling you, with a moment to undo.

**Preview** shows what a setting would stop right now, and what it would
leave alone, before you turn it on. **Pause all** stops every automatic
rule until you resume it. Automatic rules only ever stop what AI Juggler
started.

To be told when a session leaves something running, turn on **A session
leaves something running** in Settings › Notifications.

## Export snapshot

**Export snapshot** saves what is running now: each app, session and
process by name, with its state and figures, and a hash of each command
line in place of the line itself. It holds no titles, folders, command
lines or environments.

## What AI Juggler keeps

For sessions other apps run, AI Juggler keeps each process's program name,
start time, the session it was in, when it was seen, and a hash of its
command line, on your Mac only, for 7 days. Never its environment, and
never the command line itself. See [What AI Juggler touches](trust.md).
