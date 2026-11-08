# ADR-0031: A switch can move: same-tool accounts on one disk move the whole family, journaled and undoable

- Status: accepted (2026-11-08; D-93, the maintainer: "for Claude -> Claude accounts offer that as a default option, keep other options … Make sure implementation is high quality and done well")
- Amends: ADR-0028 (a tenth switching capability, `in_place_move`), SPEC §7.7
- Research: `docs/research/claude-move-switch.md`

## Context

A switch copies: the target gets a second conversation, the family is re-created
under it, and the original stays (ADR-0028). For two accounts of the same tool on
one Mac there is a better move. On 2026-11-08 the maintainer and an agent moved a
Claude app conversation and its family into another profile's folders by hand:
same ids, full history, sub-sessions and rewind, opened under that account's own
sign-in, checked by 750 checks before and 1,430 after reopening. The first attempt
left the side sessions behind and orphaned 20 archived ones: a move is the whole
family or nothing. D-93 made it the default for Claude → Claude, with Copy kept.

## Decision

1. **A switch mode, in the contract.** `SwitchCapability::InPlaceMove`
   (`in_place_move`) joins the nine. Every adapter answers it; it is never
   required. An adapter that answers Supported returns a planner from
   `ToolHandover::in_place_move()`, and the conformance kit (check 14) holds the
   answer and the hook together. Claude answers Supported; Codex NotSupported (its
   threads are also in its own database); the rest move nothing.

2. **The adapter plans; AI Juggler applies.** `juggler_adapter::moving::InPlaceMove`:
   - `plan(request)` reads both sides and returns a `MovePlan`: the family
     (`MoveMember`s, record id and conversation id kept apart), ordered
     `MoveStep`s (`MakeDir`, `Rename`, `Hold`, `WriteRecord`, `Clone`), what stays
     on purpose (`Retained`), the roots every step must stay inside, notes, a notes
     conflict, and refusals in words. It never writes.
   - `verify(request, plan)` checks the tool's own records after the move.
   - `appended(line)` sorts what the app adds to a moved history (bookkeeping, a
     history-only notice, a turn).
   - `resume_link(conversation)` opens it in the target app.

   No tool's layout or name appears outside the adapter.

3. **Claude's plan** (`juggler-adapters::claude_move`), for Claude app profile to
   profile and Claude Code folder to folder:
   - The family: the canonical record (the app's own `local_<uuid>`, else the one
     active last) and every record reached from it or its source aliases by
     `spawnedFrom.sessionId` or `forkedFromSessionId`, recursively, archived ones
     included. The target's records of the same conversations are aliases: put
     aside, and everything hanging from them re-linked to the moved records by
     structured reference keys only (never message text), archive state kept,
     routing snapshots cleared for the app to make again.
   - The files: every `projects/*/<uuid>.jsonl` (worktree folders included; two
     claims refuse), its folder's entries but `tool-results/`, files named after
     it, `file-history/<uuid>/`, `tasks/<uuid>/`, renamed into the same places in
     the target's folder; what is there already put aside first, whole.
   - `tool-results/` stays where it is: old messages name its files by full path.
     It is a `Retained` dependency cleanup must keep.
   - The project's notes: cloned to a target with none; when both differ, the
     target keeps its own unless the user ticks to take the conversation's (the
     target's then put aside). The source's are never moved.
   - Records: written under the target profile's signed-in account folder (from
     the two ids in its Claude Code `.claude.json`, nothing else read), with the
     account-bound fields the copy drops and the last run's stale error fields
     removed, every other field kept; the source's put aside last.
   - Gates: app to app or terminal folder to terminal folder only (the app ↔
     terminal move isn't proven: Copy carries it); the proven app major (4) and
     Claude Code major (2); one disk; APFS for the clone backups; no links or
     hard-linked histories.

4. **The transaction engine** (`juggler-handover::moving`), generic:
   - `preflight`: every step inside the plan's roots; the move's own folder (in
     AI Juggler's data folder) on the same disk, cloning; room.
   - `back_up` (needs the caller's `Stopped`): every touched path read (bytes
     hashed, device and inode), everything present cloned into `backups/` and each
     clone checked. Nothing changes until all check.
   - `apply`: per step, an intent line then a done line in `journal.jsonl`, each
     synced; preconditions checked (a renamed file is still the one backed up);
     renames that never cross a disk or replace a file; holds into `holding/`;
     atomic record writes.
   - `verify_cold` before the apps start: same file, same bytes, in place; holds
     held; records as written; the adapter's checks.
   - `verify_opened` after they start: each moved file still starts with every
     byte it had (a byte-prefix hash, not a size); added lines sorted; the
     adapter's checks again (the app rewrites records when it starts). Passing
     commits.
   - `recover`: the journal and the disk together; an interrupted step is
     classified done or not done from the disk, or handed to a person when the
     disk can't tell. Never a guess.
   - `undo`: every backup checked first (a mismatch changes nothing); what is at
     each touched path now (later work included) put aside in `preserved/<at>/`;
     the baseline restored from the clones; checked. Repeatable after an
     interruption. A folder the move made goes only when empty.
   - `Lock`: one move of these folders at a time across every AI Juggler on the
     Mac (the Dev build and the installed app have different data folders), in
     the user's temporary folder.

5. **The platform seam** (`juggler_platform::transfer`): file identity, one-disk
   checks, a clone that is a clone or nothing (macOS: APFS, checked, then
   `clonefile` through `std::fs::copy`; elsewhere: unsupported), renames that never
   cross a disk or replace a file, atomic writes. **No silent full copy**,
   anywhere.

6. **The process side** (`juggler-app::features::handover::moving`):
   - The offer: both sides resolved (the conversation is an app conversation when
     the source profile has a record of it), the plan, the engine's preflight,
     busy work (a turn running in either profile; for terminal folders, the
     conversation open in a session). Read only.
   - The move: the lock; a `MoveHold` on both profiles so keep alive, Open,
     Restart and the automation watch leave them alone; both quit by exact pid
     and start time, never by name, never forced (one that doesn't quit stops the
     move and the other starts again); the plan made again after they quit and
     refused if its shape changed; back up, apply, check; both started again on
     their own folders; the resume link to the target profile's new pid; the
     checks after. A failure before reopening is undone at once.
   - Four outcomes reported apart: **its history** (payload), **the app's list**
     (catalog), **opened** (in the target profile's own process), **a reply**
     (always the user's to check: a move sends no prompt).
   - Undo: the same stop, the engine's undo, the same start, opened in the source.
     A move that a later move moved on refuses Undo until that one is undone.

7. **Switching again and automation.** A move is the conversation itself
   changing owner, so a later move (back, or to a third account) plans from the
   **current** owner's folders: its whole current family, re-linked aliases
   included; the old owner's put-aside branch is never authoritative. Each move
   records the one before (`previous`). Automation (D-82) moves with the record:
   one conversation, one owner, nothing to quiet on the source; the queue of
   waiting automation changes stands down while a move runs.

8. **The UI.** The switch screen asks **How**: **Move to account** (the default
   when the offer can happen, marked Recommended) or **Copy** (the switch as it
   was). Move's screen says what moves, what happens to the original, that both
   profiles restart and running work stops, that no message is sent, and lists
   the family, kept branches and retained outputs; a notes conflict is a
   checkbox. A Move that can't happen yet says why, with Check again. The done
   screen shows the four outcomes and Undo.

## Consequences

- Claude → Claude switches keep the conversation's identity, its sub-sessions
  and rewind, with no extra disk; Copy stays for every other case and by choice.
- Both profiles restart for a move. Live workers, terminals and pending
  approvals don't survive it; their saved history does. The screen says so.
- The app's records are private and versioned: the planner refuses shapes and
  versions it doesn't know, and a new major is gated until it's walked live.
- Moves live in `<data>/moves/<id>/` (manifest, journal, backups, holding,
  preserved). They are kept for Undo and are dependencies for cleanup, as are the
  retained output folders. Removing old moves is a cleanup action of its own,
  later.
- App ↔ terminal moves, Codex moves, other organizations and personal accounts
  need their own proof before they're offered.
- Quiet means quiet (the review of 2026-11-08): a move and its Undo refuse
  while a turn runs in either profile or any session of either account is
  open outside them (a terminal writes the same folders); both profiles are
  checked still closed before the backup and before the first change; Undo
  refuses while a later move (in any state but undone) touches the same
  files, and refuses a profile AI Juggler no longer knows; a failed undo
  leaves both profiles closed with keep alive paused; reading a move back
  (`recover`) changes nothing; a step whose path changed since the backup
  stops (Rename, Hold and WriteRecord alike); a record the planner can't read
  that names the family refuses the move.
- Known limit: the clone goes through `std::fs::copy`, which on macOS is
  `fclonefileat` first; the seam checks APFS and one disk before it, so the
  byte-copy fallback can only follow a per-file clone failure. Every backup is
  checked by hash, so correctness holds; only the room used could grow.
  Calling `clonefile` directly needs `unsafe`, which `juggler-platform`
  forbids until an audited wrapper is decided.
