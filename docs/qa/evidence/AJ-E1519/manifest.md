# Evidence manifest — AJ-E1519

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1519, migration guides and importers (P2).
Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR). 2026-10-23, claude-opus.

## What a user coming from another tool was missing

The tools' own files already bring most of it across. AI Juggler reads
every Claude Code and Codex conversation, so sessions started in Claude
Squad, Vibe Kanban or Crystal are listed, and each row shows its branch,
which carries the other tool's session name. The projects backfill makes a
project of every repository they ran in, and a live worktree folds into
its repository through its `.git` file.

Two things didn't come across:

1. **A removed worktree became a project of its own.** Claude Squad
   removes a session's worktree on pause or kill, and macOS clears Vibe
   Kanban's temporary worktrees. Once the `.git` file was gone, the
   worktree's past conversations made a project named after its folder
   (`fix-login_18a3b2c4`), split from their repository.
2. **Accounts set by their profiles weren't found.** A Claude Squad
   profile that starts `CLAUDE_CONFIG_DIR=~/.claude-work claude`, or a
   Vibe Kanban executor profile with `CLAUDE_CONFIG_DIR` / `CODEX_HOME` in
   its `env`, is an account the user has, and E307's scan didn't look
   there.

Renaming projects after the other tool's sessions, which was the first
plan, was dropped: a worktree's sessions belong to the repository's
project, so there is no worktree project to rename, and the branch already
shows the name on each row.

## What exists now

1. **The importers**, `crates/juggler-adapters/src/other_managers.rs`.
   - **Claude Squad:** `~/.claude-squad/state.json` gives each worktree's
     folder and repository. `config.json` gives the programs its profiles
     start; only their leading `CLAUDE_CONFIG_DIR=` / `CODEX_HOME=` words
     are read, since tmux runs the program through the shell.
   - **Vibe Kanban:** `db.v2.sqlite` (else `db.sqlite`) is opened read-only
     and immutable, and one named-column query joins `workspaces`,
     `workspace_repos` and `repos`. The worktree is
     `<container_ref>/<repo name>`. `profiles.json` gives each variant's
     `env`, and only those two variables are read.
   - **Read-only, and only those fields.** The JSON is read into types that
     hold nothing else. Another variable in `env` (an API key), a prompt or
     a diff is skipped by the parser and never kept. A shape or schema we
     don't know reads as nothing. A file over 8 MiB isn't read. A folder
     that doesn't exist isn't offered.
   - **Never opened:** Vibe Kanban's `credentials.json`,
     `relay_host_credentials.json`, `server_ed25519_signing_key` and
     `config.json` (its GitHub token). The module names none of them.
2. **Worktree homes, remembered** (schema v32, `worktree_homes`: folder,
   repo, manager, seen_at).
   - `projects_backfill_with` reads both tools' worktrees before each
     backfill, which runs whenever conversation files change, and
     remembers them.
   - `project_root_with` checks the remembered homes before the ordinary
     rules. A folder matches when it is inside a remembered worktree; it
     compares by whole path segments, and `/private/var` and `/var` count
     as the same folder. A repository that's gone leaves the folder to the
     ordinary rules.
   - It's used at every place a conversation's project is worked out: the
     backfill, the cleanup of earlier worktree rows, the conversations
     list, and Continue in AI Juggler.
3. **Accounts, through E307's own path.** The other tools' hints are
   `AliasHint`s with `set_by: Some("Claude Squad" | "Vibe Kanban")`.
   - Each adapter's alias scan includes them, for its own variable only.
   - `alias_installs_of` merges a known folder as before. A new one becomes
     a setup with origin `other-manager`, in **Other Claude setups**, with
     the note "Set by CLAUDE_CONFIG_DIR in Claude Squad's settings —
     <file>."
   - The scan's S5 row still counts the shell's assignments only.
   - Stats: the origin counts as `onboarding.source_detected{source:
     other_manager}`, a value the closed schema already has, so no schema
     change.
   - **No new screen.** A found setup shows in Add account like any other,
     with its path. The spec's "Import from another tool, in Add account's
     More" became this, because the scan already offers every setup it
     finds, and a second way in would duplicate it (progressive
     disclosure, UI consistency). The gallery's first-run states cover the
     rows unchanged.
4. **The guide**, `docs/guide/coming-from-other-tools.md`, linked from the
   guide index under Getting started. It covers Claude Squad, Vibe Kanban,
   Crystal (no longer maintained) and cc-switch: what comes across, what
   AI Juggler reads, and what to do about the rest. Each line was checked
   against the code:
   - Worktree folding is `project_root` / `project_root_with`.
   - Continue in AI Juggler is refused while the conversation is open
     elsewhere, so a running Claude Squad session is stopped there first.
   - cc-switch is read for provider names and a count only
     (`scan_cc_switch`).
   - `statusLine` is the one setting written for measuring.

## Tests

- `crates/juggler-adapters/tests/other_managers.rs`, a fixture per
  importer and state:
  - `claude_squad_worktrees_come_with_their_repositories`: a live worktree,
    a removed one and a gone repository; a prompt and a diff in the file
    never come back.
  - `a_claude_squad_state_we_dont_know_reads_as_nothing`
  - `claude_squad_profiles_offer_the_folders_they_point_at`
  - `vibe_kanban_workspaces_come_with_their_repositories`: two
    repositories, plus a workspace not started.
  - `a_vibe_kanban_schema_we_dont_know_reads_as_nothing`: an older schema,
    and a file that isn't a database.
  - `vibe_kanban_profiles_offer_the_folders_they_point_at`: API keys beside
    the variable never come back.
  - `the_importers_never_name_a_sign_in_or_key_file`: the static guardrail.
    *Planted:* a `"credentials.json"` constant fails it.
- `other_managers::tests::a_program_sets_only_its_leading_known_variables`
- `juggler-core` `project_root_tests::another_managers_worktree_belongs_to_its_repository_after_removal`
  (both temporary-folder spellings, a sibling folder with the same prefix,
  a gone repository) and `worktree_homes_are_remembered_once_and_refreshed`.
  `storage_roundtrip` covers the upgrade to v32.
- `crates/juggler-app/tests/projects_backfill_worktrees.rs`
  `a_worktree_claude_squad_removed_stays_with_its_repository`: end to end,
  over a fixture home.
  - An earlier stray project folds away.
  - The worktree's conversation counts for the repository.
  - After Claude Squad forgets the session, the conversation still belongs
    to the repository.
- `crates/juggler-app/tests/onboarding.rs`
  `alias_and_pattern_merge_into_one_candidate`: a Claude Squad
  hint becomes an `other-manager` setup whose note names Claude Squad's
  settings, not the shell.

## Sources for the formats

Read on 2026-10-23 from each project's own source on GitHub:

- **Claude Squad** (`smtg-ai/claude-squad`):
  - `config/config.go`: `Config` and `Profile`, `~/.claude-squad/config.json`.
  - `config/state.go`: `State.instances`.
  - `session/storage.go`: `InstanceData` and `GitWorktreeData`.
  - `session/tmux/tmux.go`: the program is tmux's shell command.
- **Vibe Kanban** (`BloopAI/vibe-kanban`):
  - `crates/executors/src/command.rs`: `CmdOverrides.env`.
  - `crates/executors/default_profiles.json`: the `executors` shape.
  - `crates/db/src/models/workspace_repo.rs`: the `workspace_repos` join.

Not checked against a real Claude Squad or Vibe Kanban install: neither is
on this Mac. The fixtures follow the formats above, and a format that
differs reads as nothing, never as a wrong project.
