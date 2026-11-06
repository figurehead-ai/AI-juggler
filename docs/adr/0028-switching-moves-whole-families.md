# ADR-0028: a switch moves a conversation's whole family, exactly; every adapter answers for it

- Status: accepted (2026-11-05, the maintainer: "switching is the main feature… make this system super reliable and scalable… to the user all of this should be seamless")
- Amends: ADR-0002 (a tool's layout lives in its adapter), AJ-E1520/E1521 (subagents, the conformance kit), the switch pipeline (AJ-E204/E205), switch back (AJ-E1541)

## Context

A switch has always moved one conversation: its transcript, its own subagents' transcripts, its tool outputs and file history. On 2026-11-05 the maintainer switched a Claude app conversation that had started about twenty side sessions. The conversation moved exactly (933 of 933 subagent transcripts arrived). The side sessions stayed on the old account, and the switched conversation stood on its own in the target app. Many of those sessions only make sense next to the one that started them, so a family that falls apart is a broken move, even when every byte that did move is right.

How each tool records a family, read on this Mac (2026-11-05):

| Tool | Children | Where the parent link lives |
|---|---|---|
| Claude Code (terminal or app) | subagents (Task) | inside the parent's folder, `<session>/subagents/`: they move with the parent |
| Claude app (Code tab) | side sessions, forks, hand-offs: separate conversations | only in the app's record of each child, `claude-code-sessions/<account>/<org>/local_<id>.json` → `spawnedFrom.sessionId` (and `forkedFromSessionId`) |
| Codex | subagent threads (`thread_spawn`), forks, reviewer threads | inside each child's own rollout, first line: `parent_thread_id`, `forked_from_id`, `source.subagent` |

The Claude app builds its whole family tree from those records (its "linked family roster" follows `spawnedFrom` when `sideSession` is set and `lineageDetached` isn't). It reads the records once, at start, and saves them from memory after. Its resume link (`claude://resume?session=…`), which a switch uses to land a conversation in the app, imports one transcript and sets title and modes only, never a parent. Its own "start a session" tool sets a parent, but only for a new or forked session. So no supported path attaches an existing conversation under another. The only faithful way is to carry the app's own records of the family, with the account-bound parts changed, and let the app load them when it starts.

## Decision

1. **A switch moves a family.** The unit of a switch is the conversation together with every conversation it started, recursively, in the tool's own terms. A conversation with no children switches exactly as before. Children never move without their parent, and the parent never leaves them behind silently. The confirm step names the conversation and says "with its N sessions". There is nothing to tick: the family is the default, and a member that can't move is named before anything moves.

2. **Each adapter finds and places its own family.** `HandoverSupport` gains:
   - `family(reference)`: the members (the conversation and each descendant, its parent's id, its transcript), as the tool records them;
   - per member, the existing bundle and install, so every member is copied, checked and verified as a conversation is today;
   - `relink(family, target)`: whatever the tool needs so that the parent links hold on the target. This is a no-op for Codex and for Claude Code subagents, whose links travel inside the files.

   Hosts (desktop apps) that keep family links outside the files carry them through `HostApp`:
   - the Claude app carries its own `local_*.json` record of each member;
   - it changes only the account folder and each member's parent id;
   - it drops what belongs to the old account (published claude.ai pages, remote MCP configuration, peer messages);
   - it places them only while that profile isn't running, then starts it;
   - while the profile runs, the conversations move at once (resumable from AI Juggler), and the nesting is placed at the profile's next start. The screen offers Restart and never restarts by itself.

3. **Exactly means checked, member by member.** After a family move, AI Juggler verifies by id that every member is on the target:
   - its history is whole (the prefix rule);
   - its own subagents arrived;
   - its parent is the one it had;
   - it can be resumed by its own id.

   The source is never changed. A failed member stops the switch with what landed rolled back, exactly as a single conversation does today.

4. **Switch back is a family operation too.** Returning carries back, for every member, only what happened away (the AJ-E1541 delta), and always on the user's click.

5. **The capability contract.** Every adapter answers each switching capability, through `HandoverSupport::switch_capabilities()`:
   - **Supported**, naming what it reads;
   - or **NotSupported**, with a reason a user can read.

   The trait's default is *Unanswered*, and the conformance kit refuses it, as check 11 does for subagents. An exception is allowed only where the tool truly can't. Its reason is written in the adapter and in `docs/SPEC.md §7.7`, and the switch screen says it to the user. A new adapter therefore can't lose a capability by omission. The capabilities:

   | Capability | What "supported" proves (conformance check 14) |
   |---|---|
   | `history_exact` | a copied conversation's history is byte-identical, and a growing source passes the prefix check |
   | `own_subagents` | every subagent transcript arrives, and each resumes within its parent |
   | `family` | children are found recursively, each once, in two levels with 20 children |
   | `family_links` | after the move, each child's parent on the target is the one it had |
   | `resumable` | every moved member resumes on the target by its own id |
   | `source_untouched` | the source conversation's files are only read; only `automation_handover` changes the tool's own marks of it, reversibly |
   | `left_behind_named` | what can't move (published pages and the like) is listed before the move |
   | `switch_back_delta` | a return carries only the away turns of every member |
   | `automation_handover` | what wakes the conversation by itself is named before the move, taken over by the copy, turned off on the original; a return reverses it |

7. **What runs by itself moves too (2026-11-05).** After a switch, two copies exist, and the original can keep working by itself on the account the user left. On 2026-11-05 a switched conversation's original kept being woken by the Claude app's Auto-fix for a pull request, and the maintainer had to archive it by hand. A switch therefore hands that automation over, through `HostAutomation` (`juggler-adapter::automation`): read (named on the confirm screen), adopt (the copy), quiet (the original, keeping what undoes it), wake (switching back). The tools were researched in their own code:
   - **Claude app** (desktop; its records): each pull request in a record's `prs` carries `autoFix`, which the app honours only when the PR isn't `inherited`, `dismissed`, unconfirmed or closed, and only in a record that isn't archived. Resuming after a usage reset, after a quit mid-turn and after sleep all skip an archived record too. The app's own move to the cloud archives the moved session at its next start, and its own fork marks the PRs `inherited` without Auto-fix. So: carried records get their PRs as the app's fork does; the copy's record gets the original's Auto-fix; the original's record is archived with Auto-fix off (the record only: unlike the app's Archive, no worktree is touched). The app reads records only at start and rewrites them while running, so these changes are applied while that profile is closed and checked after it has run with them. The terminal (`claude`) keeps nothing that runs by itself; a switch to a terminal leaves the automation with the original and says so.
   - **Codex** (CLI and desktop share the account folder): a thread's goal keeps it working, also after a usage limit lifts. Codex's app-server owns goals (`thread/goal/get|set`, working on a cold thread, checked in a lab on 2026-11-05), so the copy gets the goal and the original's is paused through the user's own `codex`, CLI or the one the desktop app ships. A heartbeat automation aimed at the thread is kept by the desktop app with its installation and sign-in: it is named as staying.
   - **The queue** (`automation/waiting.json`) survives AI Juggler quitting; a watch applies app-profile changes when the profile is closed (AI Juggler's restart or Open, or the user's own quit), and keeps each until it has been seen kept. The pair is kept per switch edge for switching back.

6. **One fixture set for every tool.** The kit drives each adapter through the same families:
   - a lone conversation;
   - a parent with 20 children;
   - two levels;
   - an 800 MB parent;
   - a source still being written.

   Each one is checked into a second sandbox home, member by member, by id.

## Consequences

- A switch is now a family transaction. Time and disk grow with the family. Space and progress are counted over all members, and APFS clones keep the cost small on one disk.
- The Claude app's records are an undocumented, private format. AI Juggler copies records the app wrote and changes the fewest fields. It refuses, with words, any record it doesn't recognise, and never builds one from nothing. A change to the app's format turns into a refused family link, never a damaged app. The live test runs against every app release the maintainer installs.
- The nesting of a family moved into a Claude app that is open appears only at that app's next start. The switch says so, and offers Restart.
- A switch changes the original's app record (archived, Auto-fix off). That is the one change on the source side, deliberate and reversed by switching back; the user never archives by hand. A change the app's format no longer allows is refused and logged, not guessed.
- Live validation uses the `aij-test` Claude app profile (signed in by the maintainer, restartable), never a profile in daily use.
