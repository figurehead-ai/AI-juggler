# Moving a Claude conversation to another account (research, 2026-11-08)

What it takes to move a saved Claude conversation and its whole family from one
account to another on the same Mac, instead of rebuilding a copy: the
maintainer's case of 2026-11-08 (Claude app profile to Claude app profile, two
accounts of one organization), read from the installed app and the files it
writes. It is the evidence behind D-93 and ADR-0031, and the acceptance matrix
the feature is tested against.

Everything here uses placeholders: `<A>` and `<B>` for the two accounts, `<org>`
for the organization, `local_<id>` for an app record, `<uuid>` for a
conversation. The real case's identifiers, emails and paths stay on the
maintainer's Mac.

## What ties a saved conversation to an account

| Part | Where | What it means for a move |
|---|---|---|
| App profile | `~/Library/Application Support/<profiles>/<name>` (the app's `--user-data-dir`) | Its own sign-in and app state. Never moved, never edited. |
| The app's list | `<profile>/claude-code-sessions/<account>/<org>/local_<id>.json` | One record per conversation, under the account the profile is signed in to. A record in another account's folder isn't listed. |
| A record | `sessionId` (`local_<id>`), `cliSessionId` (the conversation's `<uuid>`), `cwd`, continuation fields (`lastAssistantUuid`…), `isArchived`, `title` | **The record's id isn't the conversation's id.** A child's `local_<id>` can be unrelated to its `<uuid>`; deriving one from the other loses sessions. No account or sign-in field was found in a record. |
| The family | a record's `spawnedFrom.sessionId` and `forkedFromSessionId` (they name **records**, not conversations) | The app builds its sidebar tree from these. Reached recursively, archived ones included. |
| Claude Code folder | the profile's `CLAUDE_CONFIG_DIR` | Each profile runs Claude Code with its own folder; that folder's sign-in sends the requests. |
| History | `<folder>/projects/<project>/<uuid>.jsonl` | A worktree has its own `<project>`: find the history by name under every project, never assume one. |
| Its folder | `<folder>/projects/<project>/<uuid>/` | `subagents/` (nested agents' histories and their `.meta.json` sidecars) and `tool-results/` (outputs too large for a message). |
| Rewind | `<folder>/file-history/<uuid>/` | Per conversation. |
| Task list | `<folder>/tasks/<uuid>/` | Per conversation. |
| Project notes | `<folder>/projects/<project>/memory/` | Shared by **every** conversation of the project on that account: changing it is project-wide. |
| Sign-in | the app's own OAuth state, the keychain, cookies | Never touched. A process can't change account by having files moved under it: the move restarts both profiles, and each comes back on its own sign-in. |

The histories aren't free of account identifiers: old commands, outputs and
attachments mention account and organization ids. They are history, not a
sign-in, and are left exactly as they are.

## What the app does with links and files (installed app, read)

- Its resume link (`claude://resume?session=<uuid>`) opens a conversation in
  the profile that receives it, adopting a history already in that profile's
  Claude Code folder. macOS gives an app's links to whichever instance it
  picks, so the link goes to the target profile's **own process** (a `GURL`
  Apple Event to its pid), never through `open`.
- It reads the records when it starts and writes them from memory while it
  runs and when it quits. Records changed while it runs are overwritten, so
  both profiles are quit before anything changes and planned again after
  they quit (a clean quit appended ~1 KB to the main history in the real case).
- Its private file reads refuse a symlinked history (`O_NOFOLLOW`), and
  adoption refuses a history with more than one name (a hard link). A
  directory link is tolerated by some reads and refused by others. **So a
  move uses no links.**
- A history pointer to a path outside the folder makes the app copy the
  history into its own folder: not a zero-copy design.

## Designs considered

| Design | Verdict |
|---|---|
| Copy into the target (today's switch) | Works across tools and disks; a second conversation with new ids, the family re-created. Kept as **Copy**. |
| Symlink or hard-link the history | Refused by the app's own checks; also couples the two accounts' writers. |
| Share one Claude Code folder | Moves every conversation and lets two writers at one folder. |
| **Rename on one disk, clone backups** | The same files (same inodes) in the target's folders; backups are APFS clones (no room until either side writes). **Chosen.** A move across disks is refused, never turned into a copy. |

## The case that proved it (sanitised)

A main conversation in `<A>`'s profile with two current side sessions; `<B>`'s
profile already had an older record of the same conversation (an alias, with
its own diverged history) and 20 archived side sessions hanging from that
alias.

1. **First pass (incomplete).** Moving the main conversation alone left the
   two side sessions in `<A>` and the 20 archived ones pointing at a record
   that was put aside. The main conversation opened; the family didn't.
   *Lesson: a move is the whole family, found before anything changes.*
2. **Family pass.** Recursive descendants of the canonical record and of the
   alias; the two side sessions moved; the 20 archived records re-linked to the
   canonical record by their structured fields only, archive flags kept; their
   routing snapshots (`promptAppendSnapshot`, `toolSurfaceSnapshot`) cleared so
   the app makes them again; the target's diverged branch kept aside whole.
3. **Checks.** 750 before reopening (every hash, same inode at the target,
   every link, archive states, nothing left claiming the moved conversation on
   `<A>`); 1,430 after (every original byte still first in each history,
   every nested history file, the family of 23 records, both side sessions
   opened in `<B>`'s window, found by the window's process id, not the app's name).
4. **What the app added after reopening.** Bookkeeping lines, and one
   `type:user` line marked `promptSource:system`, `origin.kind:task-notification`,
   `queueTranscriptOnly:true`: a background command that stopped when the app
   quit. It is a notice, not a turn; anything else of type `user` is a turn.
5. **Not established by the case:** a new model reply on `<B>` (left to the
   maintainer after a usage reset), server-side billing, other organizations,
   personal accounts, other app versions, the Claude app to a terminal or back.

Stale runtime fields of a moved record (`error`, `errorAt`, `errorRows`,
`priorErrorMark`, `lastTurnCutShort`) were cleared, originals kept in the
backup; the error cards in the history itself stay.

Out-of-line outputs: old messages name `tool-results/` files by their full
path in **both** accounts (the alias's history named `<B>`'s, the moved one
`<A>`'s). Both output folders stay where they are, and cleanup must keep them.

## Failure modes the design closes

| Failure | Closed by |
|---|---|
| Children left behind, archived ones orphaned | recursive family over both profiles before any change; closure checked after |
| Record id taken for the conversation id | the plan maps records and conversations explicitly |
| A history in a worktree's project folder missed | every `projects/*/<uuid>.jsonl` found; two claims refuse |
| The target's own version overwritten or concatenated | it's put aside whole in the move's holding place; Undo restores it |
| App rewrites records during the move | both profiles quit by exact pid and start time; plan made again after; a changed family refuses |
| Keep alive or AI Juggler's own automation restarts or writes a profile mid-move | a move hold both stand down for |
| A reused pid quit | pid with its start time, checked before quitting |
| Running work killed | busy work in either profile refuses before anything is quit; never a forced quit |
| Cross-disk copy of gigabytes | same-disk renames only; refused otherwise |
| A crash half way | a synced journal per step; recovery reads disk and journal together and never guesses |
| Undo destroying later work | backups checked first; what's there now is kept aside, then the baseline restored |
| A sign-in carried | none read or written, ever |

## Acceptance matrix

| Case | Required outcome | Test |
|---|---|---|
| Simple family | main, two children, a grandchild and nested sidecars move; an unrelated conversation is untouched | `claude_move_matrix::row01_simple_family` |
| Distinct ids | a child whose record id differs from its conversation id opens | `row02_distinct_ids` |
| Existing alias | the target's alias and its archived children re-link, archive flags and continuation kept | `row03_existing_alias` |
| Divergent destination | both branches kept, never joined, one owner | `row04_divergent_destination` |
| Project/worktree roots | every history found; a gone folder named, nothing recreated | `row05_worktree_roots` |
| Output references | both output folders stay resolvable; cleanup keeps them | `row06_output_references` |
| Shared notes conflict | an explicit choice, backed up, undoable project-wide | `row07_shared_notes_conflict` |
| Busy profile / pid reuse | nothing changes before a safe stop; no reused pid or other app quit | `moving_tests::a_busy_profile_…`, `a_reused_pid_is_never_quit`, `a_profile_that_wont_quit_…` |
| Unsupported disk, link, version | refused before anything; no copy fallback | `row09_unsupported_version_link_and_device` |
| Restart append | the original prefix kept, bookkeeping accepted | `row10_restart_append` |
| System task notification | a notice told apart from a turn | `row11_system_task_notification` |
| Corrupt backup / interrupted | no unsafe restore; resume or undo deterministic | `row12_…`, engine `moving::tests` |
| Undo after more work | originals back, newer work kept apart | `row13_undo_after_more_work` |
| Repeated switch / third account | the current owner and whole family stay authoritative | `row14_repeated_switch_and_third_account` |
| Claude Code folder to folder | the same on terminal folders | `cli_folder_to_folder` |
| The app shows it in the right window | the target profile's own window, by pid | live check on the `aij-test` profile (AJ-E212 evidence) |
| A real reply on the new account | the user's to check | never claimed by AI Juggler |

## Sources

- The installed Claude app 4.x (its bundled code, read on 2026-11-08) and the
  files it writes; the maintainer's case and its journal (local, not in the repo).
- [Claude Code: sessions](https://code.claude.com/docs/en/sessions),
  [authentication](https://code.claude.com/docs/en/authentication),
  [subagents](https://code.claude.com/docs/en/sub-agents) (background; they
  don't document moving between accounts);
  [APFS clones](https://developer.apple.com/documentation/foundation/about-apple-file-system).
