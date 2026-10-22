# Evidence manifest — AJ-E1529

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1529, sessions started outside AI
Juggler (a 0.1 blocker, from the maintainer's real-setup test). Status:
**IN-REVIEW**: everything is built and passes on the replica. The
real-data pass (qa-real, read-only, with redacted counts) comes before
DONE-EVIDENCED.

Gallery: `docs/qa/evidence/_gallery/78-outside/` (Home and the rail, an
outside conversation's page, Accounts and a project's page, in both
themes, all from the replica fixture).

## What exists now

**Each tool's own files, through its adapter (ADR-0019).** Three
defaulted methods on the adapter contract:

- `conversation_files` lists a setup's conversations: Claude's
  `projects/*/*.jsonl` (a subagent's file is part of its parent, so
  it isn't listed); Codex's `sessions/Y/M/D/rollout-*.jsonl`.
- `read_conversation` reads one: its id (what `--resume` takes), its
  folder, its last git branch, a title (the tool's summary, else the
  first real ask, skipping slash commands and the context Codex
  injects), its first and last activity, and its tokens, each message
  counted once.
- `session_process` tells a CLI session from helpers (ChatGPT's
  `app-server` and `exec-server`) and from a desktop app's internals,
  and reads which conversation it resumes.

No tool's layout is left in shared code: the backfill's Claude-only
`projects/` helpers are gone.

**One index in memory** (`features/conversations`), keyed by path, size
and mtime:

- only changed files are read again, and nothing is written (no
  migration);
- a tick every 15 s refreshes it, re-runs the projects backfill when
  files changed, and works out which conversations are live;
- it emits `conversations://changed` only when something changed.

**Live.** A running session is on:

- the conversation its `--resume` id names;
- else the newest conversation in its folder, on its account, written
  since it started.

Its account is the folder its environment points the tool at (only that
one variable is read), else the tool's own default. A session on a
folder that isn't one of the user's accounts is attributed to none.

**Projects.** `project_root` folds every worktree shape seen on a real
Mac into its repository:

- agent worktrees `<repo>/.claude/worktrees/<name>`, by path, whether
  present, removed or pruned;
- linked worktrees elsewhere, through their `.git` file;
- a folder inside a repository, into the repository.

The backfill counts both tools' conversations from the index.

**UI**, all from one `conversations_list` (a `ConversationsProvider`):

- **The rail.** Under each project: every live outside session, then the
  newest 3, then "N more outside AI Juggler". Each is a row built like a
  session row, with a quiet "outside AI Juggler" mark and the account's
  dot; a screen reader hears ", Running, outside AI Juggler, on Work
  Max". A project where only an outside session runs gets its group, and
  the rail's empty state shows only when there is truly nothing. The
  filters apply: "Running" shows and counts live outside sessions,
  "Ended" the ended ones, and the other filters hide them.
- **Home.** Its Running list and its "N running" include them, and the
  headline no longer offers a first session while eight run.
- **An outside conversation's page.** What it is, where, which account,
  tokens; **Continue in AI Juggler** (a resume launch on its account, on
  the user's click; it waits, saying why, while it's open elsewhere) and
  **Copy resume command** (the adapter's line). Its ⋯ menu comes from
  the AJ-E1537 registry and is read-only (D-08).
- **Accounts.** Each card's "N running", and the account page's
  "Running sessions", count them.
- **The go-to palette.** It lists them, and opens their page.
- **A project's page.** It lists every one of its outside conversations.

The backend refuses "Continue in AI Juggler" in words when the
conversation is live elsewhere, its folder is gone, or the account has
no launch settings. The command takes ids only (SECURITY C1).

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 list each account's conversations from its own files, never writing | adapter tests: `claude_conversations` (id, folder, branch, summary title, times, tokens once by message id; first real ask; empty file; subagents not listed) and `codex_conversations` (thread, folder, branch, ask past the injected context, times, tokens once); `tests/conversations.rs` (listed, folded, resume line, refusals) | PASS |
| Req.2 live by running process and recent writes | `live.rs` tests (resume id; newest since start; none written yet; a folder that's no account's; two sessions in one folder); `session_process` tests (CLI vs node package vs app internals vs Codex helpers) | PASS |
| Req.3 in the rail under their project, quietly marked, read-only; open, Continue in AI Juggler | vitest `outside-conversations` (the rail's slice, groups for live-only projects, the row's mark and name, the read-only menu with Continue disabled while live, the page's actions and its gone state); Playwright `replica-journey`; gallery `78-outside` | PASS |
| Req.4 the rail updates when the backfill finishes, by a typed event | the tick re-runs the backfill on file changes and emits `conversations://changed`; App and the provider read again on it | PASS |
| Req.5 worktrees fold into their repo, even removed or pruned; our agent worktrees never projects | juggler-core `project_root_tests` (present, removed, pruned, a subfolder of a repo, a plain folder, a worktree path without its repo); `projects_backfill_worktrees` | PASS |
| Req.6 a project's page lists its conversations | Playwright `replica-journey` (the count matches the fixture); gallery `project-page-*` | PASS |
| Req.7 within the rail's budget at 50+ | live + 3 recent per project, the rest on the project's page; the replica's 346 conversations render without a long list (gallery `home-rail-*`) | PASS |
| Req.8 journeys read a fixture made by the backend's readers | `just replica-fixture` (`scripts/qa/replica_fixture.py`, which refuses a non-replica HOME) wrote `ui/tests/visual/fixtures/replica.json`: 9 accounts, 16 projects, 346 conversations, 8 live. `replica-journey` and `outside-gallery` read it | PASS |
| Oracle comparison on the replica | `just replica-oracle` with QA's oracle (claude/qa-real-walk-clean eb7ecdd4, plus the two counting fixes below): 60 OK and no MISMATCH for setups, imports, projects (each with its conversations and accounts), conversations per account, or live per account; this Mac's own 17 sessions are left out as INFO. The remaining MISMATCH lines are Codex window tokens and pricing, which belong to AJ-E1530 and #120. [oracle-replica.txt](oracle-replica.txt) | PASS |
| One conversation per thread | a resumed Codex thread is a rollout per resume, all with one id. The list showed it twice, and both copies counted as live, while projects counted it once. `ConversationIndex::all` now merges them: the newest file stands for the thread, from its earliest start, with the most tokens any one file holds (never the sum: a resumed rollout repeats its parent's rows). Test: `a_thread_written_to_several_files_is_one_conversation` | PASS |
| Oracle comparison on the real data | qa-real runs the command below, read-only, under its snapshot check; redacted counts only | PENDING |
| P22: Accounts and the palette count outside sessions | vitest (`activityWords`, the palette's outside rows); `replica-journey` (each card's count against the fixture) | PASS |
| P23: a pruned worktree is no longer MOVED; the rail's count matches its rows | `project_root_tests` (pruned); the replica's oracle line "projects: backend 16 rows, oracle 16" | PASS |

The oracle (`scripts/qa/real_truth.py`) is QA's. It is copied as a file
from claude/qa-real-walk-clean, and QA keeps the same changes there. It
learnt these here, all true of real Macs too:
- folders compare resolved (`/tmp` is `/private/tmp`);
- a Codex CLI is matched by its folder, like Claude's;
- a Codex thread counts once, however many rollouts it has;
- a live session it can't put on an account is a MISMATCH;
- a CLI run from a temp folder is a test's stand-in, unless it is under
  the HOME being examined (the replica's own home is in `/tmp`);
- on a replica, this Mac's own sessions working outside that HOME are
  left out as INFO. The last rule never applies when HOME is the user's
  own, so the real-data pass leaves nothing out. The replica's
live sessions run as the real CLIs do (`claude`, `claude --resume
<id>`, `codex`), in a clean environment: a fake once inherited this
shell's own `CLAUDE_CONFIG_DIR`, and the backend correctly attributed it
to no account.

## Walked on the replica: the three questions

The replica has 9 accounts (7 Claude, 2 Codex), 16 projects, 346
conversations and 8 sessions live outside AI Juggler, plus 2 desktop
stand-ins. It was walked through the preview with its own fixture, and
through `just replica-oracle`.

1. **Does it solve the real problem?** Someone with several
   subscriptions opens the app and finds all their sessions: Home says
   "8 running" and lists each one with what it is, its account and how
   recently it worked. The rail shows them under their projects, live
   first, and a project's page lists every one. Before this, the same
   Mac said "No sessions yet". The first walk found the account wasn't
   obvious at a glance in the rail (only a coloured dot, with the name in
   a tooltip), so it was reworked before closing: every rail row, AI
   Juggler's and outside ones, now ends in the account's disc (the I-02
   `AccountDisc`). Each of the user's accounts gets its own of twelve
   hues (`lib/palette.ts`, brought forward from AJ-E1532), the same on
   every screen, so the disc matches the account's dot on Home and in
   Runway. Its initials come from the first and last words of the label,
   so `claude-work`, `claude-personal` and `claude-client-acme` read CW,
   CP and CA rather than all "CL". On the replica all 9 accounts are
   different colours. The name is still in the tooltip and what a screen
   reader hears ("on claude-work"). AJ-E1532 adds project marks and
   clones on top.
2. **Is it clear without explanation?** A row says "Running · outside
   AI Juggler · 1 min"; "20 more outside AI Juggler" says what's behind
   it. The page says, in one line, why it's read-only. The two actions
   say what they do, and Continue says why it waits while the session
   is open elsewhere. The menu has no Stop and no tidying.
3. **Is it consistent with its siblings?** The rows are built like
   session rows (the same shape, state glyph, type, dot and selection),
   and Home's rows like Home's. The counts agree everywhere: Home, the
   Running chip, Accounts and the palette all said 8 on the replica
   (pinned by `replica-journey`). Two inconsistencies remain, both owned
   by the next items: the header still says "Home" on a conversation's
   page (AJ-E1534, orientation), and the page is laid out on its own
   rather than in the shared page shell (AJ-E1533, one page model, which
   takes it in).

## The real-data pass (for qa-real)

Read-only: it brings every setup in where it is, into a fresh, empty
data folder of its own, and launches, signs in and writes nothing in a
setup. It prints only redacted OK and MISMATCH lines.

```sh
cargo test -p juggler-app --test qa_real_readonly --no-run
env -i HOME="$HOME" USER="$USER" TMPDIR="$TMPDIR" PATH=/usr/bin:/bin:/usr/sbin:/sbin \
  AIJ_QA_REAL_DATA=<a fresh, empty folder> AIJ_QA_REAL_OUT=<out.json> \
  <the qa_real_readonly test binary> --ignored --nocapture
python3 scripts/qa/real_truth.py --compare <out.json>
```

The acceptance is no MISMATCH line for setups, projects, conversations
or live sessions. Lines about tokens, resets, pricing and Codex windows
belong to AJ-E1530 and #120.

## Not here

- History search across every transcript (QA P20) needs History to list
  conversations as well, and is built with AJ-E1536.
- A live outside session's own terminal or window can't be focused: AI
  Juggler didn't open it. Continue in AI Juggler and Copy resume command
  are the ways in.
