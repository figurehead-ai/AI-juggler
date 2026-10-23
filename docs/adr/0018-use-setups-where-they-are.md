# ADR-0018: Use setups where they are (in-place accounts); partial bring-in is the user's call

- Status: Accepted
- Date: 2026-10-20
- Deciders: the maintainer (0.0.7 dogfood), Vuxtra engineering
- Supersedes: ADR-0006 §2 *as the only mode* (copy stays, as an option);
  E3 AJ-E307 Req.7 ("no adopt-in-place"); SPEC §12 "Nothing is ever
  written inside `~/.claude`…"
- Relates to: ADR-0003 (automation policy), ADR-0006, D-25 (calm UI)

## Context

ADR-0006 made bringing in a setup a byte copy into AI Juggler's own
folder. Using it every day on a Mac with six Claude desktop windows
showed what that costs the person it is for:

- **A second world.** Each copy is a new place with the same name.
  Sessions started in AI Juggler don't show in that Claude window, and
  the reverse. Settings, skills and MCP servers drift apart.
- **Signed out on arrival.** Claude Code keeps its sign-in in the
  Keychain under a name derived from the config folder's path, so a copy
  never has it. Every bring-in ends in "sign in again", once per copy.
- **Twice the disk**, and copies that can fail halfway (0.0.7: a linked
  `skills/` folder).

The maintainer's words: most people "don't want 10s of different
profiles confusing everywhere". They want AI Juggler to manage what they
have, and to be the one who decides.

Two more problems from the same session:

- **The importer was all-or-nothing in the UI.** When one of six setups
  failed, the five that came in could not be used without retrying the
  failed one or unticking it. "Try again" reran everything and lost the
  sign-ins of the ones that worked. Closing the screen mid-way kept
  importing out of sight.
- **AI Juggler already wrote into its account folders.** A profile with a
  model replaced the whole `settings.json` with `{"model": …}` on every
  launch, which wiped hooks and permissions in copies. The quota
  pipeline rewrote `settings.json` with a status line wrapper. That was
  tolerable only because the folders were ours. It is a bug either way.

## Decision

1. **Two ways to bring a setup in; the user picks, every time.**
   - **Use it where it is** (the default). AI Juggler remembers the
     folder and works with it there. Your Claude app and AI Juggler share
     one sign-in, one history and one set of settings.
   - **Make a separate copy.** The ADR-0006 copy into AI Juggler's
     folder, for people who want an isolated sandbox. The original is
     never touched.

   The choice sits in plain view on the bring-in screen, with one line
   on what each means. A failed copy offers "Use it where it is
   instead".

2. **In place, AI Juggler writes nothing of its own into your folder.**
   No marker, no permission change, no `settings.json` edits, no status
   line wrapper, no spool. Its bookkeeping lives in AI Juggler's data
   folder: the account row, and per-launch settings files under
   `overlays/<account-id>/`.

   Your folder still changes in two ways:
   - Claude Code itself writes to it as it runs: history, its own state,
     and what you change in Claude.
   - An explicit switch of a session to that account adds the session's
     transcript files. These are new files only; the installer never
     replaces one (see #71).

3. **Launch with the folder's exact identity.**
   - The account's folder is stored as the exact path string the scan
     found, which is what the profile manager sets. Claude's Keychain
     name hashes that string, so the sign-in carries over.
   - The default `~/.claude` is launched **without** `CLAUDE_CONFIG_DIR`,
     as Claude itself does, so it finds the same sign-in.
   - Per-launch settings (a profile's model, the quota status line) go
     in with `claude --settings <file in AI Juggler's data>`. AI Juggler
     never edits `settings.json` for this. The same applies to copies;
     that fixes the overwrite.

4. **Removing an in-place account only stops managing it.** The folder
   is never deleted, moved or changed. Any code that deletes an account
   folder refuses an in-place one.

5. **Each setup comes in, or doesn't, on its own.**
   - The result lists every pick: in, or couldn't (in plain words why).
   - For the failed ones the user can **Try again**, **Use it where it
     is instead** (for a failed copy), or **Continue with the ones that
     came in**.
   - Sign-ins of the ones that came in are kept across retries.
   - **Stop** during a bring-in finishes or undoes the setup in progress
     cleanly and keeps the rest. The next visit shows those as "Already
     in AI Juggler" and preselects the others.
   - A copy whose account record can't be saved removes its copied
     folder. Copied folders don't accumulate silently.

## Alternatives considered

- **Copy only (ADR-0006).** Rejected as the default for the reasons in
  Context. Kept as an option.
- **In place only.** Rejected: some people want a sandbox, and "always
  let the user decide" was the maintainer's explicit ask.
- **In place, but install the quota status line into the user's
  `settings.json`.** It would capture quota for sessions started outside
  AI Juggler too, but it edits the user's own configuration and replaces
  the status line they see. Rejected. Quota for those sessions comes
  from their transcripts.

## Consequences

- Bringing in is instant for in-place setups: nothing is copied and
  nothing fails halfway. Signed-in setups stay signed in.
- The scan recognises in-place setups by AI Juggler's account list, not
  by a marker file. They show "Already in AI Juggler" like copies do.
- The isolation checklist describes in-place accounts honestly. "Owner
  only" and "marker present" are AI Juggler's guarantees for its own
  folders, not for yours.
- Measured quota samples from the status line come only from sessions
  AI Juggler starts. Sessions from the Claude app on the same folder
  still count, through their transcripts.
- Two programs use one folder at once: the Claude app and sessions AI
  Juggler starts. That is what Claude Code already supports between its
  desktop app and the terminal.
- New code paths: account placement (`copy` | `in_place`) on the
  account row, the launch overlay file, and the result screen.
  `account_import` takes the mode, and Codex setups get the same choice
  when their adapter supports it.

## Update — Codex (2026-10-20, AJ-E1306)

Codex homes get the same choice. In place, `CODEX_HOME` and
`CODEX_SQLITE_HOME` are the exact folder; Codex's own default `~/.codex`
launches with both unset, as a plain `codex` does (the counterpart of
`~/.claude` without `CLAUDE_CONFIG_DIR`). Nothing is written into the
home — the model goes in as a flag. For Codex, in place is also the
safer default: its refresh token is single-use, so a copied `auth.json`
would sign one of the two out, and copies never carry it.

## Amendment — measuring limits for sessions started elsewhere, opt-in (2026-10-22, AJ-E1530 Req.1)

The maintainer's decision, 2026-10-22: **"Offer it, opt-in."**

§2's rule, "in place, AI Juggler writes nothing of its own into your
folder", had a cost. Claude reports its limits only through the status
line. So the windows of an account whose sessions are all started in the
Claude app or a terminal stay "Not measured yet", even while that is all
the account does. The alternative this ADR first rejected (installing
the capture into the user's `settings.json`) is now offered, and never
done unasked.

- **Off by default.** It is offered per account, for an account used
  where it is, as "Measure limits for sessions started elsewhere". A
  diff preview of the exact change comes first, and nothing is written
  before the user confirms it.
- **One value changes.** Only the top-level `statusLine` value in
  `settings.json` is replaced, or added when there is none. It is
  spliced into the text, so every other byte stays as it was: comments,
  key order and line endings included.
  - The write is atomic.
  - A full copy of the file from before the change is kept in AI
    Juggler's own data.
- **Their own status line keeps working.** The user's command is
  chained after the capture, and its output is shown unchanged.
- **Our files stay ours.** The wrapper and the spool live in
  `overlays/<account-id>/`. Nothing else is ever written to the user's
  folder.
- **Turning it off is in the same place.**
  - It puts their original `statusLine` value back, byte for byte.
  - If they edited the file since, only AI Juggler's value is spliced
    out. The old backup is never restored over their edits.
- **Unsafe states are refused in plain words.** That covers a
  `settings.json` that is missing, unreadable as JSON, or changed
  between the preview and the change. If their status line becomes
  something else later, AI Juggler says so and leaves it alone.
- **The isolation check** allows exactly that one key, while the user
  has it on. Any other write still fails it.

Trade-off: the user's own configuration now holds one value that points
into AI Juggler's data. That is the price of measured limits for
everything they run. It stays their choice: off unless they turn it on,
visible as a diff before it happens, and undone exactly.

## Amendment: setting up what a switch's target lacks, on approval (2026-10-22, AJ-E1542)

The maintainer (2026-10-22), on switching: AI Juggler "should check MCP
server and capabilities, and our system should recognize if there are any
MCP servers missing or skills missing and offer to initialize to those other
accounts. All this should be presented to user, and user has control if
they choose to do it or not. Least amount of work for the user, but user
oversees that and views it."

So before a switch, AI Juggler compares what the source account has set up
with the target's, and offers what's missing. A write into the target
account (in place or a copy) happens only for an item the user picked:

- **What is read.** Each tool's adapter reads its own files only, never
  starting the tool or a server:
  - MCP servers;
  - skills, commands and subagents;
  - the account's own rules;
  - plugins.

  Only the keys it needs are parsed. A value that looks like a secret is
  dropped while it's read, so it can't be shown or copied.
- **What is offered.** Everything the source has and the target doesn't,
  with the target tool's verdict:
  - set up as it is;
  - translated (and how);
  - needs you (a login, or a variable to provide), with what to do;
  - can't be set up there, and why. This covers claude.ai-synced skills,
    plugins across tools, and SSE servers on Codex.

  A target that has its own rules is never given more.
- **How it is written.**
  - **A Claude MCP server:** through Claude's own `claude mcp add-json`, so
    Claude writes its own file.
  - **A Codex MCP server:** one new `[mcp_servers.<name>]` table appended
    to `config.toml`, checked to still parse. Before the first set-up the
    file is copied to `config.toml.before-ai-juggler`; the last undo that
    needs that copy removes it, so nothing is left behind.
  - **Skills, commands, subagents and rules:** copied as new files.
  - **Secrets:** a secret becomes a reference to an environment variable
    (`${NAME}`, `env_vars`, `bearer_token_env_var`), never its value.

  Nothing the target has is ever replaced.
- **Undone from the same place.** Each set-up is recorded with how to undo
  it. Undo removes only files still exactly as they were made, takes out
  exactly the text that was appended, or runs the tool's own removal. If
  something changed since, it's left as it is and the user is told.

Trade-off: the target account's own configuration gains what the user
picked. That's the point of the feature. It stays their choice, item by
item, shown before it happens and undoable.
