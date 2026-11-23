# Claude app: the formats a move relies on

AJ-E2328 req 1 and 9. A move to another account (ADR-0031) and the worktree
ownership Repair rewrite a few of the Claude app's own files in each app
profile. This page says what those files are, how AI Juggler knows it still
understands them, and how to check and update that when the Claude app ships
a new version.

Everything here was read from files on disk and the app's own log lines,
never from the app's code. The one source of truth is the manifest
`crates/juggler-adapters/src/claude_move/known_formats.json`; the Claude
adapter (`claude_move/worktrees.rs`) and `scripts/claude_compat_check.py`
both read it.

## What a profile keeps

A Claude app profile is one app data folder (the default one, or one
started with its own `--user-data-dir`).

- **Conversation records** (`session_record`):
  `claude-code-sessions/<account>/<org>/local_<id>.json`, one per
  conversation. AI Juggler needs the record's own id and the Claude Code
  session id. For a conversation that works in its own worktree, the record
  also names the worktree (its folder name and full path) and the branch.
  It lists folders the conversation was given access to and folders its
  shell used, by full path; a move points those that are inside the old
  profile at the new one.
- **The worktree file** (`worktree_file`): `git-worktrees.json`, one per
  profile, at schema version 2. Its `worktrees` map has one entry per
  worktree the profile created and owns: the folder, the branch it was made
  on, and which conversation record holds it. Other top-level parts
  (remote URLs and pins, pending removals) are kept exactly as they are,
  as is any field AI Juggler doesn't know.
- **The cleanup** (`cleanup_sightings`): inside the worktree file, the
  profile's tidy-up of unused worktree folders: the folders it looks after,
  and the worktree folders it has seen there without owning them (with when
  it saw them). A folder it saw but doesn't own is removed after about a
  day.
- **Side-session notes** (`side_session_notes`):
  `side-session-notes/<record id>/`, one folder per conversation, shared
  with the conversation's side sessions.
- **Log lines** (`log_lines`): the lines in the app's own log that showed
  what goes wrong when ownership is left behind: a worktree made afresh
  because the profile didn't own the old one, an unowned folder removed by
  the cleanup, and a write refused because the worktree belongs to another
  profile. They are listed so a later version can be checked for the same
  behaviour; AI Juggler never reads the logs at run time.

Each manifest entry names the Claude app versions it was read on
(`verified_on`) and when (`read`).

## What a move and Repair do with them

A move takes each moved conversation's worktree entry from the old
profile's worktree file to the new one's (the new profile owns it), names
the branch the worktree is really on (read from its `HEAD`, never checked
out), drops the old profile's cleanup sighting of it, moves the notes
folder, and points the record's paths at the new profile. Both worktree
files are rewritten whole, backed up first, so Undo puts them back byte for
byte. Nothing inside a worktree is ever written.

The ownership check (Settings › Desktop apps › Moving conversations) finds
conversations an older move left split: the worktree still owned by the
profile it left, or recorded on another branch than it's on. Repair takes
the move's path: the involved profiles quit by their own process, the same
rewrite, both started again, and the check run again. When a conversation
has two worktrees (the app made a fresh one), the stale one moves too and
both are kept: a folder no profile owns would be removed by a cleanup and
may hold unpushed work. A worktree another profile's cleanup has seen is
told as an info line ("… also tidies this folder; keep … running in it or
commit your work"), never changed.

## The gate

Before a move or a Repair writes anything, two things must hold:

1. The installed Claude app's version (its bundle's
   `CFBundleShortVersionString`, read the same way the move reads it) is in
   the manifest's accepted range (`app.accept`). Otherwise the move and
   Repair refuse in words ("AI Juggler hasn't checked moves on the Claude
   app <version> yet …; update AI Juggler"), and Copy still works. Settings
   › Desktop apps shows "Claude app <version> · moves checked" or
   "· not yet checked".
2. Each profile's worktree file is the schema version the manifest names,
   with every required field of the right type. Otherwise the move refuses,
   naming the profile and what didn't fit.

The gate asks through the adapter (`InPlaceMove::moves_checked`), so the
app never names a tool.

## When the Claude app ships a new version

1. Install the new version, open each profile once, and run
   `python3 scripts/claude_compat_check.py`. It is read-only, makes no
   network call and reads no process's environment; it prints a short JSON
   verdict and never a value, title, id or path:
   - `ok` (exit 0): the version is accepted and every file fits.
   - `drift` (exit 1): a file has a key or type the manifest doesn't know;
     the verdict names the entry and the key.
   - `unknown_version` (exit 2): the version isn't accepted yet (drift, if
     any, is listed too).
   - `error` (exit 3): the manifest can't be read.
2. For `unknown_version` with no drift: re-read the files and logs the
   entries describe on the new version (a lab profile, never the
   maintainer's own), then edit the manifest: add the version to
   `app.checked`, widen or add a span in `app.accept`, and add it to each
   entry's `verified_on` with today's `read` date.
3. For `drift`: work out what the new key means before accepting it. A key
   AI Juggler only needs to keep goes in the entry's `typed` list; a change
   to a key the move rewrites needs the adapter changed and a matrix case
   first.
4. Run `python3 scripts/claude_compat_check.py --selftest` and
   `cargo test -p juggler-adapters --test claude_move_matrix`, and walk a
   move of a conversation in its own worktree in the lab (AJ-E2323 PL1).
