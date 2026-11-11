# ADR-0032: Presence comes from the process table: one reconciler, one owner per session, one section per App

- Status: accepted (2026-11-11; D-100, the maintainer: "active and inactive apps, sessions and subsessions must match reality 100%. This is the trust foundation")
- Amends: D-64 (what "nothing open" means), D-62 (labels), ADR-0013's rail grouping where it computed owners in the UI
- Ships: 0.4.2 (the release's whole point)

## Context

On the installed 0.4.1 beta (2026-11-11), against `ps` at the same moment
(the Claude app running in six profiles, ChatGPT.app running on Codex):

- Open now held four Apps. Two running profiles sat under **Inactive apps**
  as "Running · 2 min ago", and Codex sat there as "8 h ago".
- The same Account showed twice: "Claude app · acme-south" under Open
  now, and "Claude Code · acme-south" under Inactive apps. Nothing said
  one is the desktop app and the other the terminal tool.
- A terminal session ("Jive CLI") was filed under the Claude **app**
  profile's group.
- The App lens's "Last 3 inactive" rows mixed in another grouping.

The causes, from the code:

1. **Open now is built from sessions, not processes.** `buildLines`
   (`ui/src/lib/rail-lines.ts`) makes Open now's groups from open sessions
   only; `idleApps` puts every App whose key no open session has under
   Inactive apps, running or not. D-64's "nothing open" was read as "no
   open session".
2. **Running comes from a watch that is fed on the side.** `places::known`
   reads an App's pid only from `places.watch`, filled by the live view's
   full scans (`observe`), by keep alive, and by Open. Between those, a
   running App reads as closed, and a closed one can keep a stale pid
   until the next full scan.
3. **A session's owner is worked out twice, with guesses.** The backend
   (`places::app_of`) and the rail (`appKeyOf`, `theirs` in `buildLines`)
   each decide which App a session belongs to, and both fall back to "the
   one desktop profile on this folder". So a terminal session whose host
   wasn't recognised as a terminal is filed under a Claude app profile.
   The two copies also drift: the rail moves a row the backend counted
   elsewhere.
4. **The App lens nests sessions by relation** (`nestRows`: a session
   under the one it was started from), across Apps, inside an App's
   "Last 3 inactive".
5. **Labels name the tool, not the kind.** "Claude Code · acme-south"
   and "Claude app · acme-south" are two Apps on one Account (the
   entity model allows many), but nothing says desktop or terminal.

## Decision

### 1. One reconciler, in the backend

A new `presence` feature in `juggler-app` owns presence. Every screen reads
its snapshot. Nothing else decides what runs or who owns a session.

- **Input:** the process table (`procs::table`: pid, ppid, start, exe, args,
  tty), the adapters' host apps (`HostApp`: bundle, bundle id, instance
  argument), the adapters' process readers (which process is a session,
  and its config folder: `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, or the
  default), and the user's Apps (the `apps` table, D-84).
- **Identity:** each App instance has exactly one key, built by its
  adapter:
  - a desktop App is `app:<host app id>:<instance folder>`, from its main
    process's bundle and its `--user-data-dir` (or the app's default
    folder). Codex is `app:codex-app:`, from `ChatGPT.app`'s main process
    through the Codex adapter's `HostApp`. A process the adapter reads as
    its own (the bundled `codex` app-server) belongs to that App, not to a
    CLI App.
  - a CLI App is `cli:<tool>:<config folder>`, from the session process's
    tool and the config folder it runs with.
- **Running:** a desktop App runs while its main process is alive with
  the same start time (pid reuse can't fool it). A CLI App runs while one
  of its session processes is alive. Nothing else counts: not a
  session's last activity, not a watch from an earlier scan.
- **Owner of a live session:** walk its process's ancestors. The first
  desktop host app of the session's own tool owns it, as that instance;
  with none, its CLI App owns it (the terminal it runs in is a detail of
  the row, as D-62 says). No folder guesses for a live session: its
  process says.
- **Owner of an ended session:** the owner recorded when it was last seen
  live (stored per conversation when the reconciler binds it). Without a
  sighting, the tool's own records (a Claude app profile's local session
  files name the profile; `started_in` from the tool's data). Else its CLI
  App. "The one profile on its folder" is no longer a rule.
- **Subsessions** (subagents, and sessions started from another) belong
  to their root session's owner.
- **Snapshot:** `presence_snapshot` returns `{ apps: [{ key, kind:
  desktop|cli, tool, label, account, running, pids, last_seen }],
  owner: { <session or conversation key>: <app key> } }`, cached per
  process-table read (at most once a second), with a change event when
  it differs. `places_list`, the conversation list's places, Home's Your
  apps and the switch targets read the same snapshot.

### 2. The rail renders, it doesn't decide

- In the App lens, **Open now** has one group per running App, with or
  without sessions (an App with none says "Nothing open in it"), and
  **Inactive apps** has one row per known App that isn't running. An App
  is in exactly one of them. "Running" never appears under Inactive.
- A row sits under the App the snapshot names as its owner. `appKeyOf`,
  `theirs` and the UI's copy of `app_of` go.
- **Each lens groups strictly by its own entity.** App by App, Project by
  Project, Account by Account. In the App lens, "Last N inactive" lists
  only that App's own sessions, flat, newest first. Nesting by relation
  stays inside one App's open sessions only.
- **Labels say the kind.** A desktop App reads "Claude app · acme-south",
  a CLI App "Claude Code (terminal) · acme-south". The kind also
  shows as an icon on the header. Two Apps on one Account never look the
  same.

### 3. Invariants, enforced

- **In code:**
  - no App key in two sections;
  - the set of Apps in Open now equals the running set;
  - every session and conversation has exactly one owner, and that owner
    is a known App key;
  - a live session's owner is running.

  The backend checks them on every snapshot in debug builds and logs any
  mismatch (`presence: invariant …`). The rail checks the section ones in
  its unit tests.
- **Tests:**
  - property tests (proptest) over generated process snapshots: desktop
    apps with and without sessions, several profiles, the CLI and desktop
    app on one Account, Codex in ChatGPT.app with its bundled `codex`,
    terminals, tmux, quits, crashes, and pid reuse with a new start time;
  - a fuzz-style round of random process trees against a reference model
    written separately;
  - fixtures shared by the backend and the rail (`fixtures/presence/*.json`).
- **On the real Mac:** `scripts/qa/presence_check.sh` is read-only. It
  reads `ps` itself, reads the Dev build's rail through the accessibility
  tree (by pid, never by name), and lists every App running that Open now
  lacks, every App in two sections, and every Inactive App that runs.
  It gates the release (release scope and preflight-light's release
  mode), and runs before and after on the maintainer's setup.

## Consequences

- One place to fix presence bugs, and its property tests run in CI.
- The rail gets simpler: it renders the snapshot.
- Keep alive, Open and Start read running from the snapshot too; the
  watch keeps only keep alive's own state (misses, restarts, pauses).
- Persona labs add "an App open with no session" and "Codex in
  ChatGPT.app" before persona walks resume (on 0.4.2).
- Cost: one process-table read a second while the window shows, as the
  live view already does; the snapshot reuses that read.

## Rejected

- **Patching `buildLines` to add running Apps to Open now.** It fixes
  one symptom and leaves the second copy of ownership, which caused the
  CLI-under-desktop filing.
- **Inferring running from recent session activity.** That's the bug.
