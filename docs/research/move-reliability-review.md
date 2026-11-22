# Move to account: reliability and architecture review

> Committed 2026-11-22 for 0.5.1 (AJ-E2315 and AJ-E2317..E2324). An independent Opus review, read-only, of `origin/release/0.5` at `bde5fa4d8`. Profile names in its observations are replaced with generic ones (`work`, `personal`).

Reviewed: `origin/release/0.5` at `bde5fa4d8` (fetched 2026-11-22). Read-only. Every `file:line` below is on that commit. Process shapes were observed on this Mac with `ps` and `lsof` only (nothing killed, no profile data read).

Short names used below:

- `moving.rs` = `crates/juggler-app/src/features/handover/moving.rs`
- `engine` = `crates/juggler-handover/src/moving/mod.rs`, `state.rs` = `crates/juggler-handover/src/moving/state.rs`
- `tree.rs` = `crates/juggler-app/src/features/places/tree.rs`
- `places/mod.rs` = `crates/juggler-app/src/features/places/mod.rs`
- `conv/places.rs` = `crates/juggler-app/src/features/conversations/places.rs`, `procs.rs` = `.../conversations/procs.rs`
- `attest.rs` = `crates/juggler-app/src/features/agent_bridge/attest.rs`
- `claude_move.rs` = `crates/juggler-adapters/src/claude_move.rs`
- `MoveChoice.tsx` = `ui/src/components/switch/MoveChoice.tsx`

## The one-paragraph diagnosis

The move's file engine is careful: journaled steps, identity-checked renames, clone backups, and a read-only recovery reader. The weak part is the process side. It decides who must stop by **loose evidence** ("any program of the bundle that names a path in the folder", "the profile running = a main process whose `--user-data-dir` string matches"). It decides that they stopped by **waiting and hoping** (poll every 250 ms, 30 s per profile, then "files hold still for 1.5 s"). And it keeps its "don't touch these profiles" promise **in memory only**, in one AI Juggler process. So the same code fails in three opposite directions:

- it waits on processes that will never end and refuses wrongly (AJ-E2315);
- it does not quit a profile that is running, when the running profile is spelled differently, or is a second profile of the same account;
- after a crash it lets keep alive restart apps onto a half-moved folder, with no door back to Undo.

The codebase already has the right primitives in `attest.rs` (strict `--user-data-dir` parse, canonical paths, a profile proven by the process holding its lock, a pid proven by start time and version). The move, the places tree, running-app detection and keep alive don't use them. Part B proposes one shared ownership module, an adapter "writers" hook, and a durable, resumable move.

---

## A. Failure catalogue

Severity scale: **Loss** (data lost or split), **Integrity** (data intact but in a wrong or broken place, or Undo unreachable), **Hang** (the dialog or the app waits long with no way out), **Wrong refusal**, **Slow**, **Disruption** (apps quit and restart for nothing).
Likelihood is for a real multi-profile Mac like the maintainer's (eight Claude profiles, Code-tab sessions, a Dev and an installed AI Juggler side by side).

### A1. Process ownership

| # | Mechanism | Concrete trigger | Current behaviour (cite) | Severity | Likelihood |
|---|---|---|---|---|---|
| A1.1 | The third mark counts any bundle program whose **any** argument value is inside the profile folder. Everything below a marked process is pulled in too. | Another profile's Code-tab session runs `disclaimer … claude --add-dir=…/ClaudeProfiles/personal/…` (AJ-E2315). | `tree.rs:76-84` (any arg, `split_once('=')`), descendants walk `tree.rs:99-106`; `rest_ended` re-reads marks every 250 ms `moving.rs:836`, refuses at 30 s `moving.rs:796-813`. | Wrong refusal, plus Disruption (both profiles quit and restart on every retry) | High. Being fixed in AJ-E2315; listed here for the class. |
| A1.2 | The main pid's **whole subtree** is snapshotted before quit, and every member must end. Anything the Code tab started that outlives it blocks: a dev server started with `nohup`/`&`/`run_in_background`, an MCP server that ignores SIGHUP, `caffeinate`, a `tail -f`. | A Code-tab session started `pnpm dev` in the background an hour ago. | `before = system.tree(state, place, &[pid])` `moving.rs:773`; each must be dead `moving.rs:831-835`. | Wrong refusal (every retry, forever, until the user finds and kills a process they can't see) | High for developers. |
| A1.3 | Mark 1 counts any process whose **executable** lives inside the profile folder. | The user links the app-bundled `claude` (`<profile>/claude-code/<ver>/…/claude`) into `PATH` and uses it in terminals on other accounts. | `tree.rs:67-68` | Wrong refusal | Low. |
| A1.4 | Electron's crash reporter starts as a child of launchd (ppid 1), not of the app. It is found only by the third mark (`--database=<profile>/Crashpad`). | Observed on this Mac: `chrome_crashpad_handler` pids 4202 and 28686 have ppid 1 and `--database=…/ClaudeProfiles/work/Crashpad`. If a version keeps it alive after quit, every move of that profile is refused. | Only `tree.rs:76-84` finds it. Once A1.1 is narrowed, it may also be missed. | Wrong refusal, or (if missed and it writes) none: it writes no conversation files | Medium. It depends on the Electron version. |
| A1.5 | A process descended from the profile is **AI Juggler itself, or the other AI Juggler**. Only this process is left out. | The Dev build is run from a Code-tab shell (`cargo tauri dev`). This happened in the rc3g walk, see `conv/places.rs:219-228`. Quitting that profile ends the Code-tab `claude` and its `disclaimer --pgroup` group, which can take AI Juggler down mid-move. If the installed AJ moves that profile, the Dev AJ is in the tree and never ends. | `own` exclusion only, `tree.rs:100`. There is no check that the move would quit its own ancestors. | Hang/crash mid-move (Integrity), or Wrong refusal | Medium for the maintainer and agents; low for users. |
| A1.6 | Pid reuse. Start times are compared **to the second, with ±1 s slack**. | A pid reused within a second. | `same_start` `crates/juggler-adapter/src/live.rs:421-429`; used by `alive` `crates/juggler-app/src/features/places/start.rs:84-88`. `attest.rs` uses micros plus the audit-token pid version (`attest.rs:122-127`). | Wrong quit or wrong wait | Very low. |
| A1.7 | **TOCTOU between checking and signalling.** `alive(pid,start)` is checked, then `kill(pid, SIGTERM)` is sent by bare pid. | The pid ends and is reused between the two calls. | `moving.rs:765-774`; `crates/juggler-platform/src/process.rs:145` | Wrong quit (another app gets SIGTERM) | Very low. |
| A1.8 | Sandboxed or other-user helpers have no readable args. `sysinfo` drops them from the table. | A sandboxed XPC helper of the app writes into the profile. | `procs.rs:193-199` ("not ours to read … isn't in the table") | It is missed, so no wait | Low for Claude (its helpers are the same user). |
| A1.9 | **Other apps, terminals or editors** hold or write the moved files: VS Code with the jsonl open, `tail -f`, a sync tool (Dropbox, iCloud Drive), Time Machine, Spotlight. | The user has `~/.claude` in a synced folder, or an editor open on the history. | Nothing looks at open files. Only `hold_still` (size and mtime) `moving.rs:854-868` catches *active* writes. Readers are harmless (the clone is consistent). | Loss is possible only with a sync tool that rewrites after the move (it recreates the source file) | Low. |
| A1.10 | **Claude Code doesn't keep its history open.** It appends and closes. An `lsof`-only writer check would see nothing. | Observed: `lsof -p 29599` (a live Code-tab `claude` on this very session) lists no `.jsonl` and no `projects/` path. | — (this bears on proposal B3) | — | Certain. Open-file checks can only be a supplement. |

### A2. Which profile and which account: attribution before the quit

| # | Mechanism | Trigger | Current behaviour | Severity | Likelihood |
|---|---|---|---|---|---|
| A2.1 | **A profile spelled differently reads as not running.** Its place id comes from the raw `--user-data-dir` string. Equality is not canonical: a symlink, a different letter case on case-insensitive APFS, `..`, or the two-word `--user-data-dir X` form. Then `running_now` is `None`, so `was_running = false`, so **the profile is never quit** and its files are moved under a live app. | The user's own launcher starts a profile as `…/ClaudeProfiles/Work`, while the book has `…/work`, or a symlinked folder. | `running_apps` uses `arg_value` (loose, `conv/places.rs:339`, `crates/juggler-adapter/src/conversation.rs:186-200`), then id equality `places/mod.rs:1114-1116`; `stop_places` `moving.rs:759-760` treats `None` as closed. Only `hold_still` stands between this and a live writer. | **Loss/Integrity.** The live app rewrites its record and appends to a recreated source history, so the conversation splits. | Low–medium. |
| A2.2 | **Profile chosen by account, not by evidence.** `profile_for(account)` takes the *running* profile, or the one seen last. With two profiles on one account (supported by the entity model), the source may be the wrong one. The record filter then drops it, `source_place = None`, and the move becomes a **terminal-folder move**: no app is quit, and the app profile that really lists the conversation keeps a record pointing to a history that left. | Account "Work" has `ClaudeProfiles/work` and `ClaudeProfiles/work-2`. The conversation lives in `work-2`, and `work` is running. | `crates/juggler-app/src/features/places/handoff.rs:164-173`; `moving.rs:441-455`; `places_on` `places/mod.rs:1473-1492`. The planner allows folder→folder when both host dirs are `None` (`claude_move.rs:606-614`). | **Integrity** (orphaned record in a running app, which can write into the old path) | Medium wherever an account has 2+ profiles. |
| A2.3 | **The target App is resolved from the account, not from the App the user picked.** The command takes an AI Juggler profile id and maps it to an account (`crates/juggler-app/src/commands.rs:83-97`). The desktop target is then `profile_for(account_b)`. | The switch list (D-143: "the ranked list of Apps") shows "Claude · Work-2". The move quits, writes into and opens "Claude · Work". | `moving.rs:422-423, 453-455` | Integrity (the wrong App gets the conversation) | Medium with 2+ Apps per account. |
| A2.4 | **Profile↔tool-folder link by name convention.** `ClaudeProfiles/<name>` is assumed to go with `ClaudeCodeProfiles/<name>`. The move writes the history into the *linked* folder, and restarts the profile with `CLAUDE_CONFIG_DIR` = the linked folder. If the profile actually ran on another folder (the user's launcher), it is restarted on a different folder than before. | A profile renamed or launched by a hand script with a different `CLAUDE_CONFIG_DIR`. | `crates/juggler-app/src/features/places/link.rs:13-20`; start env `crates/juggler-app/src/features/places/start.rs:134-139`; move side `moving.rs:456-458` | Integrity (the history lands where the app doesn't read; "no conversation found") | Low (D-102's record check exists), but high impact. |
| A2.5 | **Terminal moves trust the live model's account attribution.** A `claude` process not attributed to either account (missing pid file, `-p` headless, a background `kind`) isn't counted as "open elsewhere". | A cron or script `claude -p --resume <id>` on the source folder. | `moving.rs:372-393` (filters on `entry.account_id`); folder moves have no stop at all (`places` empty, `moving.rs:1020-1024`) | Loss is possible (a writer during apply), mitigated by `hold_still` | Low. |

### A3. Quit semantics

| # | Mechanism | Trigger | Current behaviour | Severity | Likelihood |
|---|---|---|---|---|---|
| A3.1 | **The app asks a question at quit.** The main pid doesn't end. | A quit-confirm dialog, or an unsaved-input prompt. | 30 s, then "It may be asking something: look at its window" `moving.rs:778-785`. The source restarts, and Retry is offered (`MoveChoice.tsx:142`). | Hang (30 s) then a correct but un-actionable refusal | Low–medium. |
| A3.2 | **SIGTERM, not a polite quit.** Electron handles SIGTERM as a quit. A future AppKit host (or a native rewrite) does not by default: the default action is immediate death, with no save. | Any non-Electron host added later. | `crates/juggler-platform/src/apps.rs:246-255` → `process.rs:145` | Loss (a torn record) on non-Electron hosts | Low today. Grows with adapters. |
| A3.3 | **The app relaunches itself.** Squirrel's `ShipIt` installs a pending update on quit (observed: ShipIt pid 14738, ppid 1, running since yesterday). It can swap the bundle while the move restarts it; the restarted version differs from the version the plan's shape check read. | An update downloaded and pending. | Restart `moving.rs:1200-1205` and `open_in` `moving.rs:933-958` don't wait for ShipIt. The shape is checked only from records (`claude_move.rs:615-619`). | Integrity (the move is checked against the old version), or the start fails mid-swap | Low. |
| A3.4 | **Our own keep alive in *another* AI Juggler.** `MoveHold` is an in-memory set in this process (`places/mod.rs:141-170`). The installed AJ's keep alive relaunches a profile the Dev AJ just quit, about 9 s later (`keep.rs:26-29`: 2 misses + 5 s). | The maintainer runs Dev and installed side by side, both with keep alive on a profile. | `still_down` catches it, then undo, then "was started while the move ran" `moving.rs:553-568, 1072-1081`. Retry repeats the same thing. | Wrong refusal (forever), Disruption | **High for the maintainer.** |
| A3.5 | **launchd / login-item relaunch.** A user LaunchAgent with `KeepAlive`, or a watchdog script. | As above. | Same as A3.4. It is never told apart. | Wrong refusal | Low. |
| A3.6 | **Sequential quits share a 30 s budget per profile, counted from before the quit.** Two profiles can take 60 s+ before a refusal. | Two slow-closing profiles. | `until` set before `quit` `moving.rs:757`; places in a loop `moving.rs:750` | Slow / Hang | Medium. |
| A3.7 | **The link is sent the moment a pid appears.** The app may not have a URL handler ready yet. "Opened" is reported as **verified** on delivery alone. | A cold start of a large profile. | `open_in` `moving.rs:944-957`; outcome `moving.rs:1281-1284` | Over-claim (not data). The user sees "verified" and the conversation isn't open. | Medium. |
| A3.8 | **A fixed 6 s settle before the "after" checks.** A slow start rewrites records later, which gives a false "catalog failed" or a premature pass. | A cold start, or a busy Mac. | `SETTLE` `moving.rs:61, 1221` | Wrong outcome words | Medium. |

### A4. File stillness vs. real writers

| # | Mechanism | Trigger | Current behaviour | Severity | Likelihood |
|---|---|---|---|---|---|
| A4.1 | **Stillness is not absence of a writer.** An idle writer (an open session between turns) looks still, then writes after the backup. | A terminal session on the moved conversation that the live model didn't attribute (A2.5). | `hold_still` compares size and mtime only `moving.rs:854-868`, `state.rs:199-237` | Loss is possible (a write between backup and rename lands in the renamed inode; the cold check then fails and undoes) | Low. The engine's identity and byte checks (`engine:767-779, 844-858`) make it a failed move, not silent loss. |
| A4.2 | **There is always a 1.5 s floor, and up to 9 s plus another 9 s** (the retry's settle). | Every move. | `STILL`, `STILL_LOOKS` `moving.rs:66-68`; settle on retry `moving.rs:1157-1159` | Slow | Certain. |
| A4.3 | **The settle result is ignored on the second backup try.** | A writer still running. | `let _ = hold_still(..)` `moving.rs:1158` | Slow (a doomed second full read) | Low. |

### A5. Crash or power loss mid-move, recovery, Undo

| # | Mechanism | Trigger | Current behaviour | Severity | Likelihood |
|---|---|---|---|---|---|
| A5.1 | **There is no recovery at startup.** It only sweeps the backups of undone moves (`moving.rs:1423-1433`, `crates/juggler-app/src/lib.rs:934`). `recover()` is read-only and called only to list moves (`engine:955-1022`, `moving.rs:1438-1473`). | AI Juggler is killed, crashes, is ⌘Q'd, or the Mac loses power during `apply`. | The move stays `Applying`/`BackedUp`/`Verified` in the journal. | Integrity | Low per move, but certain over many moves. |
| A5.2 | **The "held closed" promise isn't durable.** `MoveHold` is in memory. After a restart, keep alive (after its 60 s settle, `keep.rs:32`) **starts both profiles onto the half-moved folders**. A durable `paused` is written only when an undo *fails* (`moving.rs:571-581, 1097`). | As A5.1 with keep alive on. | The apps write into a half-applied state. Undo later puts their work aside. | Integrity (the user sees a broken conversation; work is not deleted) | Low–medium. |
| A5.3 | **No door to Undo or Finish.** Undo is reachable only from the done screen or the failure screen of the same dialog (`MoveChoice.tsx:90-99`, `MoveDone.tsx:87`). Nothing lists moves anywhere else. The pause text says "undo it from the switch screen first" (`moving.rs:576`), and that screen has no such control. | Any interrupted move, or any committed move after its dialog closed. | Stuck: profiles paused or half-moved, no UI path. | **Integrity, stuck** | Low, but it breaks the "every object needs an obvious door" rule. |
| A5.4 | **A move that passed the cold checks but crashed before reopening is offered only Undo**, though it is complete on disk (`Verified`) and could be rolled forward. | A crash during restart/settle. | `moves()` labels it "interrupted … Undo puts both back" `moving.rs:1446-1449` | Disruption (a good move gets undone) | Low. |
| A5.5 | **The UI picks "the" interrupted move by state, not by id.** `left` = the newest *any* interrupted move. | An older interrupted move of another conversation exists, and this one fails part way. | `MoveChoice.tsx:94-98` | Wrong target (Undo of another conversation's move) | Low. It breaks the "actions target by id" rule. |
| A5.6 | **A Planned move left by a crash** counts as "previous" for the next move of the same conversation. | A crash between `begin` and stop. | `previous` filter `moving.rs:1030-1037` (any phase ≠ Undone) | Cosmetic (lineage) | Low. |

### A6. Disk full, large histories, APFS

| # | Mechanism | Trigger | Current behaviour | Severity | Likelihood |
|---|---|---|---|---|---|
| A6.1 | **The room check counts only record writes plus 256 MB.** Undo needs room too (it makes `preserved/` folders and clone restores). A full disk mid-apply makes the undo fail, which leaves profiles closed and paused, which leads to A5.3 (no door). | A nearly-full disk, with a sync or download running beside it. | `engine:326-343`; fail path `moving.rs:1093-1105` | Integrity, stuck | Low. |
| A6.2 | **Every byte is read and hashed 4 times:** the baseline (`state.rs:85-109`, `engine:658`), the clone check (`engine:680`), the cold check (`engine:853`), and the after-open prefix (`engine:924`). Plus `hold_still`'s tree walks every 1.5 s. | A 406 MB family reads about 1.6 GB. A 4 GB family reads about 16 GB. | — | Slow (seconds at 406 MB; tens of seconds at multi-GB) | Medium (long-lived projects with big `file-history/`). |
| A6.3 | **The manifest lists every file's hash and is rewritten whole at each phase.** A family with tens of thousands of files (`file-history/`) gives a multi-MB manifest written 6+ times. | Large rewind histories. | `engine:467-471, 483-487`; `DirState` `state.rs:20-28` | Slow | Low–medium. |
| A6.4 | **Committed moves keep their clone backups forever.** Clones share blocks until the live file changes. A history the app *rewrites* (not appends), or records, then costs full size. | Many moves over months. | Backups are dropped only for Undone moves (`engine:698-717`, `sweep_undone` `engine:353-360`) | Slow disk growth | Low. |
| A6.5 | **Clone of a dataless (iCloud-evicted) or compressed file**, or a family on a non-APFS volume. | `~/Library` on an external HFS+ disk. | It is refused up front: `preflight` clone check `engine:321-325`, planner `claude_move.rs:620-631`. Good. | — | — |

### A7. Concurrency and a second AI Juggler

| # | Mechanism | Trigger | Current behaviour | Severity | Likelihood |
|---|---|---|---|---|---|
| A7.1 | **The cross-process lock is keyed by the raw path string** (not canonical). Two spellings of one folder give two locks. | A symlinked `~/.claude`, or one AI Juggler with a trailing slash. | `engine:189-214` (sha of `to_string_lossy()`) | Concurrent moves of one folder | Very low. |
| A7.2 | **Each AI Juggler has its own journal folder.** Undo's "a later move owns these files" check sees only its own moves. | The installed AJ undoes a move made before a later Dev AJ move of the same family. | `moving.rs:1334-1339` reads `moves_dir(state)` only; the lock is shared (`engine:175-178`) but the journals are not. | Integrity (old copies restored over a newer owner; the newer work is put aside, not deleted) | Low; higher for the maintainer. |
| A7.3 | Keep alive, Open, Restart and automation respect only **this** process's hold. | See A3.4. | `places/mod.rs:174-182, 755, 1215, 1340`; `automation.rs:664` | Wrong refusal / Disruption | High for the maintainer. |
| A7.4 | A second move of the same accounts while one runs is refused, not queued. | Two dialogs. | `Lock::take` `engine:206-210` | Fine. Retry helps (transient). | — |

### A8. Target running or not; CLI vs desktop; adapters

| # | Point | Cite |
|---|---|---|
| A8.1 | A target that was **not** running is still started at the end to open the link. That's by design (the four outcomes), but it starts an App the user had closed. | `moving.rs:1207-1212`, `open_in` `moving.rs:940-942` |
| A8.2 | A target signed in to a different account than AI Juggler thinks: records are written under the profile's *actual* signed-in folder (good, ADR-0031 §3), but the history goes to the *linked* tool folder (A2.4). | `claude_move.rs` records; `moving.rs:456-458` |
| A8.3 | **Terminal-folder moves have no stop phase.** Safety rests on `open_elsewhere` (live-model attribution) and `hold_still`. | `moving.rs:500-505, 1020-1024` |
| A8.4 | **Codex and the other adapters don't move.** Only Claude implements `InPlaceMove` (`crates/juggler-adapters/src/claude_code.rs:900`; ADR-0031 §1: Codex NotSupported). Their switch is Copy. The ownership issues in Part E still apply to their live views and Go to it. Codex already has strong writer evidence (`thread-writer-locks/<id>.lock`, `crates/juggler-adapters/src/codex_live.rs:90-112`), which the generic engine could use once Codex moves. |

### A9. Timeouts and Retry

| # | Mechanism | Cite | Severity |
|---|---|---|---|
| A9.1 | **Retry is offered for every "nothing changed" failure**, whatever the cause. It is useless for A1.1–A1.5, A2.x, A3.1, A3.4, A3.5 and A5.x. Each Retry quits and restarts both profiles again and kills their idle Code-tab work. | `MoveChoice.tsx:142-146`; `nothingMoved` `ui/src/components/switch/MoveRunning.tsx:25-28`; words "Try again in a moment" `moving.rs:801, 1088` | Wrong refusal plus Disruption |
| A9.2 | **The offer can't predict process refusals.** Ownership conflicts that are visible *before* quitting (the other profile's `--add-dir`, the Dev AJ in the tree, a nohup server) are found only after both apps were quit and waited on for 30 s. | `offer_of` `moving.rs:679-728` has no tree look | Disruption |
| A9.3 | **No cancel while running.** The dialog has no footer during `moving` (`MoveChoice.tsx:122`). The backend thread can't be interrupted. | — | Hang (up to about 70 s worst case) |

---

## B. Architecture proposal

### B0. Principles

1. **Ownership by evidence the OS vouches for, never by "names a path".** Every process the move waits on, quits or blames carries a *reason*: `MainOf(instance)`, `HelperOf(instance)`, `DescendantOf(main, snapshot)`, or `WriterOf(conversation)`. Each reason has a named evidence source.
2. **Block only on writers.** A process blocks the move only if it can write a file the move touches: the profile's main process (its storage), and the adapter-named writers of the family. Everything else in the old tree is *reported* ("a dev server your session started keeps running") and never waited on.
3. **Decide before you disrupt.** Anything that can be known without quitting is known in the offer, under 0.5 s. Quitting happens only when the remaining risk is "the apps take a moment to close".
4. **A move is a durable state machine.** The "held closed" state is on disk, it is shared by every AI Juggler on the Mac, and it is resumed or rolled back automatically at startup. There is always a visible door to the state of every move.
5. **Every refusal has a class** (transient / needs the user / setup / bug) and names *which* app, session or process holds *what*. Retry appears only for transient.

### B1. A shared ownership module (`features::ownership`, OS parts behind `juggler_platform`)

Extract and generalise what `attest.rs` already proves, and make it the only way any feature answers "whose is this process / this profile / this conversation":

```text
ProcId        { pid, start_micros, pid_version? }      // proc_list + audit-token version where known
InstanceKey   canonical real path of an app instance folder (realpath; case-folded on case-insensitive volumes)
instance_of(main: &Proc, host: &HostApp) -> Result<InstanceKey>   // attest.rs:338-402: strict `flag=value`, once, before `--`
proven_running(instance, host) -> Option<ProcId>       // the holder of host.instance_lock (F_GETLK; attest.rs:422-439, safe_fs.rs:504)
helpers_of(instance, table) -> Vec<ProcId>             // strict instance arg on helpers (mark 2, canonical)
snapshot(main) -> Vec<(ProcId, ancestry)>              // descendants with their chain, taken before quit
writers(tool_dir, conversations) -> Vec<Writer>        // adapter hook, below
conflicts(...)                                          // a process two places claim: resolved by the strongest reason
```

- **Instance proof = the lock holder.** The Claude adapter already declares `INSTANCE_LOCK = "Local Storage/leveldb/LOCK"` (`crates/juggler-adapters/src/claude_peer.rs:63`). Observed: the main process 28682 holds that LOCK in `work`. The fcntl lock holder is independent of how `--user-data-dir` was spelled, so "is profile X running" becomes "does anyone hold X's lock", and "is X closed" becomes "nobody holds it" (A2.1 gone). Put the lock path in `HostApp` data (an adapter fact, no per-tool branch).
- **The conflict rule.** When two places would claim one process, the reason ranks `MainOf > HelperOf (strict arg) > WriterOf (pid file) > DescendantOf`. A process that is a descendant of profile P's main process (by snapshot) is never counted as profile Q's because of an argument value (AJ-E2315 as a class).
- **The self rule.** If AI Juggler's own ancestry, or the other AI Juggler's, intersects the set that must end, refuse up front: "AI Juggler runs inside Claude app · work (started from its Code tab); start it from Finder to move this profile" (A1.5).
- **OS seam** (`juggler_platform::procs`): list with start micros (macOS `proc_listallpids`/`proc_pidinfo`; Linux `/proc/<pid>/stat`; Windows Toolhelp32 + creation time); lock holder (macOS/Linux `F_GETLK`/`F_OFD_GETLK`; Windows Restart Manager `RmGetList`); open files of a pid (macOS `proc_pidinfo(PROC_PIDLISTFDS)` + `PROC_PIDFDVNODEPATHINFO`; Linux `/proc/<pid>/fd`; Windows Restart Manager).

### B2. The adapter "writers" hook (stays inside the adapter architecture)

Add to `juggler_adapter::moving::InPlaceMove` (or `live::HostFamily`):

```rust
/// The processes that can write `conversations` under `tool_dir` now, each with its evidence.
fn writers(&self, tool_dir: &Path, conversations: &[String], table: &dyn ProcView) -> Vec<Writer>;
struct Writer { pid: u32, start: i64, conversation: Option<String>, evidence: WriterEvidence }
enum WriterEvidence { SessionFile /* <tool_dir>/sessions/<pid>.json, procStart matches */,
                      ResumeArg   /* --resume=<id> in its args */,
                      HeldLock    /* Codex thread-writer-locks/<id>.lock */,
                      OpenFile    /* holds a touched file open for writing */ }
```

- Claude: `<tool_dir>/sessions/<pid>.json` with `sessionId` and `procStart` (`crates/juggler-adapters/src/claude_live.rs:3-7, 76-112`) plus `--resume=<id>` (`crates/juggler-adapters/src/claude_conversations.rs:365-373`). Observed: the Code-tab `claude` carries `--resume=<conversation id>` in its args. This is **the** evidence for "this process writes this conversation", and it doesn't need `lsof` (A1.10).
- Codex (when it moves): `thread-writer-locks/<id>.lock` held open (`codex_live.rs:90-112`).
- The generic engine waits on `writers` ∪ `{main process of each instance}`, and nothing else.
- Run `writers` for **both accounts' folders whatever the place**. That closes A2.5 (headless and unattributed CLI sessions) and A8.3, with no live-model dependency.

### B3. Open-file guard as a supplement (not the primary signal)

Right before `back_up` and again right before `apply`, ask the OS which processes hold any *touched path* open with **write** access (`proc_pidfdinfo`, about 1–3 ms per process over about 900 processes; restrict to same-user processes). Writers block, with their names; readers (editors, Spotlight, `tail`) don't. This catches unknown writers (sync tools, other apps, A1.9) at no cost to the common case. It can't catch Claude Code's append-and-close, which is why B2 is primary.

### B4. Event-driven waits instead of polling

- **Process exit:** after the `ProcId` checks, register `kqueue EVFILT_PROC NOTE_EXIT` on each pid in the wait set (then re-check the start time to close the register race). Wake on the last exit, or on the deadline. Linux: `pidfd_open` + `poll`. Windows: `OpenProcess(SYNCHRONIZE)` + `WaitForMultipleObjects`. That removes the 250 ms polls of `alive` (a fresh `sysinfo::System` per call, `procs.rs:217-229`) and the full table re-read every 250 ms (`moving.rs:836`).
- **File stillness:** `EVFILT_VNODE NOTE_WRITE|NOTE_EXTEND|NOTE_DELETE|NOTE_RENAME` on the touched files and their folders (FSEvents for trees). "Still" = no event for a short quiet window (300 ms) *after* every writer has exited. There's no 1.5 s floor. Linux: inotify. Windows: `ReadDirectoryChangesW`.
- **Quit in parallel:** signal both profiles' main processes, then wait on the union. Today they are sequential (`moving.rs:750`).
- **Graceful quit through the seam:** `juggler_platform::apps::quit_politely(pid)` = Apple Event `aevt/quit` addressed by pid (the same Automation consent the GURL already needs), falling back to SIGTERM. Linux SIGTERM. Windows `WM_CLOSE` to the pid's top-level windows (A3.2).

### B5. Durable, shared, resumable move (journal + two-phase commit)

Phases (keep the engine's, and add the process side to the journal):

```text
Planned → Holding (hold marker written, shared) → Quiet (writers proven gone, instance locks free)
 → BackedUp → Applying(step n) → Applied → Verified (cold) → Reopened → Committed | RolledBack
```

1. **One shared move area for every AI Juggler on the Mac**, e.g. `~/Library/Application Support/AI Juggler Shared/moves/` behind a platform path. It holds the lock files (canonical roots, A7.1), the journals, and a **`held/<place-key>` marker per profile**. Keep alive, Open, Restart and the automation watch in *every* AI Juggler check the marker (A3.4, A7.2, A7.3). Each data folder keeps only an index pointing at it.
2. **The hold is written before the first quit and removed only on Committed/RolledBack.** It names the move id and the process (pid, start) that holds it. A hold whose owner is gone is a *crashed* move, not a free profile.
3. **Startup recovery (automatic, before keep alive's first tick):**
   - Planned / Holding / Quiet → roll back (nothing changed): drop the hold, restart what was running.
   - BackedUp → nothing changed on disk: the same, and drop the backups.
   - Applying(n) → the journal + disk classification you already have (`engine:955-1022`). Roll **back** by default (Undo). Roll **forward** when every step is done and the cold checks pass now.
   - Verified / Reopened → finish forward: re-run the after-open checks, commit.
   - Unclear → keep the hold, notify (bell + the conversation's page) with "Finish or Undo", never silently.
4. **Block AI Juggler's own quit during `Applying`** (a few hundred ms): the quit waits for the step loop to reach a phase boundary.
5. **A door for every move:** a "Moves" row on the conversation's page and in Settings › Desktop apps, with Undo (committed), Finish/Undo (interrupted), the journal's timeline in Copy details. Every action carries `move_id` (A5.5). The failure error carries the move id too, instead of the UI guessing.

### B6. Refusal classes and words

Replace `AppError::Move { unchanged }` with:

```rust
enum MoveStop { class: Transient | NeedsUser | Setup | Bug,
                holders: Vec<Holder>,   // (app or session name, what it holds, its evidence)
                move_id: Option<String>, unchanged: bool, words, detail }
```

- **Transient** (a writer still finishing within the grace; another move running): **Retry**, auto-retried once.
- **NeedsUser** (the app is asking something; a session is working; a terminal session holds the conversation; a launch agent relaunches it; the Dev AJ runs inside the profile): no Retry button. Instead "Check again", which reads the offer (no quit), and the named holder with a door ("Go to it" for a session or window).
- **Setup** (a disk without clones, an unknown shape, two folders sharing a profile): Move hidden, Copy offered (as now).
- **Bug** (an invariant broken: a process claimed by two profiles with equal evidence, the journal disagreeing with the disk): Copy details plus "Report", never Retry.

The offer computes everything except "they take a while to close", so most refusals happen **before** anything quits (A9.2).

### B7. Diagnostics and telemetry

- In the journal: a `timeline` event per phase with ms timestamps. Per wait: the wait set (pid, program name, reason, evidence), who ended when, and what remained. Per refusal: its class. There are no paths in the UI words. Copy details is the redacted journal summary.
- A local `moves/diagnostics.jsonl` ring (64 entries) with durations and classes, readable by `scripts/gate_watch.py` like `compat/shape-mismatches.json` (`moving.rs:595-644`).
- Through the stats service (if the user consented): counts of refusal classes, p50/p95 per phase, family bytes bucket. No ids, no names.

### B8. What stays where (adapter architecture and OS seams)

| Concern | Owner |
|---|---|
| Family, steps, records, shape, resume link, **writers**, appended-line sorting | Adapter (`InPlaceMove`) |
| Instance flag, default instance, **instance lock path**, link scheme | `HostApp` data (adapters / `desktop_apps.json`) |
| Journal, phases, backups, apply, verify, recovery, shared move area | `juggler-handover::moving` (generic) |
| Ownership reasons, conflicts, self rule | `juggler-app::features::ownership` (generic) |
| Process list, exit events, lock holder, open files, polite quit, clone, rename, file events | `juggler_platform` (macOS now; Linux/Windows implementations behind the same functions) |

No tool name appears outside its adapter.

### B9. Ranked proposals (impact per effort)

| Rank | Proposal | Fixes | Effort | Release |
|---|---|---|---|---|
| 1 | **Wait only on writers + the main process** (B0.2, B2 for Claude: pid files + `--resume`; drop marks 1 and 3 as blockers, report them instead) | A1.1–A1.4, A9.1 (most), A3.6 partly | S–M | **0.5.1** (fold into AJ-E2315's follow-up, don't duplicate its patch) |
| 2 | **Refusal classes, Retry only for transient, holders named, move id in the error** (B6) | A9.1, A5.5, A3.1 words | S | **0.5.1** |
| 3 | **Shared hold marker checked by every AI Juggler's keep alive / Open / Restart**, plus the self rule (B5.1, B1) | A3.4, A7.3, A1.5 | S | **0.5.1** |
| 4 | **Instance proof by lock holder + canonical `InstanceKey`** in `running_now` and the move (B1) | A2.1, A2.3 partly | S (reuse `attest.rs`) | **0.5.1** |
| 5 | **Source profile by the conversation's record, target by the App the user picked** (B1 conflict rules; pass the place id through `switch_move_execute`) | A2.2, A2.3 | S | **0.5.1** |
| 6 | **Durable hold + automatic startup recovery + a Moves door** (B5.2–B5.5) | A5.1–A5.4, A6.1's dead end | M | 0.5.1 for the hold and door; the roll-forward later |
| 7 | **Decide before you disrupt:** run ownership + writers in the offer; refuse pre-quit (B0.3) | A9.2, Disruption | S (once 1 and 4 exist) | 0.5.1 |
| 8 | **Parallel quit, per-class deadlines, polite quit seam** (B4) | A3.6, A3.2 | S | 0.5.1 (parallel); polite quit later |
| 9 | **Event-driven waits** (kqueue exit + vnode) (B4) | A4.2, perf | M | 0.5.x |
| 10 | **Open-file write guard before backup and apply** (B3) | A1.9, A4.1 | M | 0.5.x |
| 11 | **I/O halving:** clone first, hash the clone once, cold check by identity + size + mtime, keep the after-open prefix hash (C) | A6.2, A6.3 | M | 0.5.x |
| 12 | **Accurate "opened":** verified only when the adapter's after-open check sees the conversation focused/opened; wait for the records' rewrite (event, cap 15 s) instead of a fixed 6 s | A3.7, A3.8 | S | 0.5.1 |
| 13 | **Shared journals** (one move area for every AI Juggler) and canonical lock keys | A7.1, A7.2 | S–M | 0.5.x |
| 14 | **Room for Undo** in preflight (the family's metadata plus the largest record set), and backup retention for committed moves (drop after N days, or once the next move of the family commits) | A6.1, A6.4 | S | 0.5.x |
| 15 | Diagnostics timeline + stats (B7) | future reports | S | 0.5.1 (journal timeline); stats later |

---

## C. Performance

### C1. What a move costs today (406 MB family, both profiles running, idle)

| Step | Today | Cite |
|---|---|---|
| Offer | records read for both profiles, plan, preflight. No process look. | `moving.rs:679-728` |
| Quit source, wait for its tree | main ends in about 0.1 s; `disclaimer` in about 6.3 s (live trace, `tree.rs:6-10`); polled every 250 ms with a **full table read + a fresh `sysinfo::System` per member** each poll | `moving.rs:778-787, 830-848`; `procs.rs:217-229`; `tree.rs:134-146` |
| Quit target, wait | the same again, **sequentially** | `moving.rs:750` |
| Re-plan | full records read | `moving.rs:1110` |
| Hold still | ≥ 1.5 s (up to 9 s) of tree walks of every touched folder | `moving.rs:854-868` |
| Back up | full read + SHA-256, clone, full read + SHA-256 of the clone (812 MB) | `engine:626-687` |
| Apply | renames + fsyncs: fast | `engine:738-756` |
| Cold check | full read + SHA-256 again (406 MB) | `engine:844-858` |
| Restart source, start target, link | `open` per app; poll for the pid every 250 ms up to 30 s | `moving.rs:933-958` |
| Settle | **fixed 6 s** | `moving.rs:1221` |
| After-open check | prefix SHA-256 (406 MB) | `engine:903-946` |

Estimated today: typical **about 20–25 s** (two 6 s tree waits + 1.5 s + about 2 s I/O + about 3 s start + 6 s settle). Worst case before a refusal: **about 70 s** (30 + 30 + 9 + backup).

Process-table scans: about 944 processes here (`ps -ax`, 2026-11-22). The incremental table (`procs.rs:89-97, 133-211`) makes a steady-state read cheap (one listing syscall, a few ms), but `rest_ended` calls it 4 times a second per profile, plus one `sysinfo` refresh per tree member per poll (about 15 members), for up to 30 s × 2. That is roughly 240 table reads + about 3,600 single-pid reads, about 1–2 s of CPU on a shared machine for a wait that kqueue does for free.

### C2. Targets

| Measure | Target |
|---|---|
| Offer, including the ownership + writers look and every pre-quit refusal | **≤ 500 ms** (p95), no process quit |
| Refusal decision for anything knowable before quitting | **≤ 500 ms** (in the offer) |
| Refusal decision after quitting ("a writer didn't end") | **≤ 10 s** after the main process ended; "the app is asking something" at **15 s** |
| Quit both profiles (parallel, writers + main only) | **≤ 2 s** typical |
| Quiet confirmed (events) | **≤ 0.5 s** after the last writer exits |
| Backup at 400 MB (clone, one hash) | **≤ 1.5 s**; at 4 GB **≤ 10 s**, with progress |
| Apply + cold check (identity) | **≤ 0.3 s** |
| Start both + link + records rewritten | **≤ 8 s** typical, **15 s** cap |
| **Whole move, typical (≤ 500 MB, two running profiles)** | **≤ 10 s** p50, **≤ 20 s** p95 |
| CPU during the waits | **≤ 50 ms** total (event-driven) |

### C3. I/O plan (proposal 11)

- Clone first (`clonefile`, metadata only). Record the source's (inode, size, mtime_ns) before and after the clone. They must be equal, and the writers must be proven gone.
- Hash the **clone** once (private, immutable). That is the baseline used for Undo and the after-open prefix.
- Cold check: the renamed file is the same inode with the same size and mtime (a rename keeps both). No hash.
- After open: keep the prefix hash. It is the proof that nothing the user had was lost.
- Hash files in parallel (2–4 threads; mind the shared-machine rule).
- Result: 4 full reads become 2. The manifest keeps per-file hashes but is written at phase changes only, and the journal holds per-step lines.

---

## D. Verification plan

### D1. Unit tests with fixtures (pure; `procs::fixture`, no OS)

Ownership shapes (`features::ownership`), each asserting *which* processes block, which are reported, and the refusal class:

1. The live trace (main, 9 helpers, disclaimer, Code-tab claude, its node server); main gone; helpers reparented to launchd (already in `tree.rs:226-289`; keep it).
2. **AJ-E2315:** another profile's disclaimer + claude with `--add-dir=<this profile>/…`. It is never this profile's.
3. A `nohup` dev server below the Code-tab claude: reported, not waited on.
4. `chrome_crashpad_handler` with ppid 1 and `--database=<profile>/Crashpad`: reported, not waited on.
5. **The Dev AI Juggler descended from the profile's Code tab** → the self-rule refusal, before any quit.
6. The other AI Juggler descended from the profile → the same.
7. Instance spellings: a symlink, upper/lower case, a trailing slash, `--user-data-dir X` two-word form, a second `--user-data-dir`, a flag after `--` (the strict parse refuses; the lock holder decides).
8. Two profiles on one account; the conversation in the non-running one → the source is the one holding the record; it is quit.
9. Two Apps on the target account; the user picked the second → it is the target.
10. Pid reused between the snapshot and the wait (same pid, a new start) → not waited on, not signalled.
11. A pid file in `sessions/` with a stale `procStart` → not a writer.
12. A `claude -p --resume <id>` headless process on the source folder, with no live-model entry → a writer, blocking (NeedsUser, named).
13. A process claimed by two profiles with equal evidence → class Bug, never a wait.

Refusal classification: a table test from each failure to its class and Retry visibility (Rust), plus a UI test that Retry shows only for `transient` (`switch-move.test.tsx`).

### D2. Engine fault injection (`juggler-handover`, temp dirs, APFS on CI macOS runners)

Some exist already (`crates/juggler-handover/src/moving/tests.rs`). Add:

- A crash after each phase boundary and after each step's intent → run **startup recovery** → assert rolled back or rolled forward, the hold released, backups as specified.
- A crash between `Verified` and reopen → it rolls forward and commits.
- ENOSPC injected (a seam on `transfer::write_atomic`/`clone_file`) during apply and during undo → the state stays recoverable; recovery after room frees finishes it.
- A writer appending between clone and the post-clone stat → refused as Transient, no clone left.
- Two AI Juggler data folders sharing one move area: a later move by "B" blocks Undo of an earlier move by "A".
- Lock keys: two spellings of one root take one lock.
- A large family (10k files in `file-history/`, 2 GB sparse files) → time and read-count budgets (count bytes read through a seam; assert 2× not 4×).

### D3. App-level integration (scripted `MoveSystem`, runs in CI, no accounts)

Extend `crates/juggler-app/src/features/handover/moving_tests.rs`:

- Keep alive in a **second AppState** sharing the move area relaunches the source at +9 s → it is held, no relaunch.
- A LaunchAgent-style relaunch (the scripted system restarts the pid) → NeedsUser "something restarts it", no Retry.
- An app asking at quit (the pid never ends) → NeedsUser at 15 s; the other profile restarted; no Retry.
- Parallel quit: two profiles each taking 3 s → the total stop ≤ 3.5 s.
- Startup with a held profile and an interrupted journal → keep alive's first tick starts nothing until recovery finishes; the Moves door lists it.
- The error carries `move_id`; the UI's Undo targets it (no `find(interrupted)`).
- The "opened" outcome is `pending` when the link was delivered but the after-open check doesn't see it.

A real-process integration (CI macOS, no accounts): the `fake-desktop` fixture already used by `places_real` (see `scripts/leaked_fixtures.sh:2-32`), extended to (a) hold an instance LOCK with fcntl, (b) spawn a fake `claude` writing `sessions/<pid>.json` + a jsonl, (c) optionally spawn a nohup child, (d) optionally refuse SIGTERM for N s. That exercises kqueue waits, the lock-holder proof, pid files and the self rule, with no real app.

### D4. Live lab walks (PL1 lab profiles only; never real profiles)

Only what fixtures can't prove. Each runs on the Dev identity in lab profiles started through `lab_env.sh`, with every process verified by `ps` (memory: lab launches isolated by ps):

1. The real Claude app: quit timing per process class (main, helpers, crashpad, disclaimer, Code-tab claude), with the new wait set → the typical-move target.
2. A Code-tab session with a background dev server → the move succeeds; the server is reported, not waited on.
3. Lab profile A's Code tab with `--add-dir` into lab profile B → moving B isn't blocked by A (the AJ-E2315 regression).
4. Keep alive on in the installed-identity lab replica + the Dev move → held.
5. Kill the Dev AJ during apply (from outside, by pid) → relaunch → automatic recovery + the Moves door.
6. A pending ShipIt update in the lab bundle copy, if reproducible: the move restarts onto the new version; the after-open checks run against it.
7. The lock-holder check across a real Claude update (LOCK_ELSEWHERE fail-closed words, `attest.rs:410-411`).

---

## E. The same bug class across the codebase: loose evidence attributing to the wrong owner

Each row: where, the wrong-attribution scenario, the consequence, severity, and the fix under the shared ownership principle (B1/B2).

| # | Site | Loose evidence | Scenario | Consequence | Sev | Fix |
|---|---|---|---|---|---|---|
| E1 | `tree.rs:76-84` | Bundle program + *any* arg value under the profile folder | AJ-E2315 (`--add-dir`), crashpad `--database`, any future flag naming a path | Wrong refusal and repeated disruption | High | Being fixed (AJ-E2315). Long term: `ownership` reasons; arg values never confer ownership. |
| E2 | `tree.rs:99-106` | "Everything below a marked process" | One wrong mark pulls in a whole other session's subtree | Wrong waits; the words name the wrong programs | High | Descendants only of the *main* snapshot, plus strict-arg helpers. |
| E3 | `tree.rs:67-68` | Executable inside the profile folder | The app-bundled `claude` used from terminals on other accounts | Wrong waits | Low | Writers by pid file; exe location only as a hint. |
| E4 | `tree.rs:70-74`, `conv/places.rs:262, 339`, `places/mod.rs:1114-1116` | `arg_value` (accepts two-word, repeats, after `--`); raw path equality | A symlinked or case-different profile folder; a flag repeated | **The move doesn't quit a running profile** (A2.1); keep alive starts a second instance; Open now shows it as two or none | High (Move), Med (others) | `instance_of` strict (`attest.rs:338-372`) + `InstanceKey` canonical + lock-holder proof (`attest.rs:422-439`). **One module for `attest.rs`, `running_apps`, `host_of`, `tree.rs`, `running_now`.** |
| E5 | `crates/juggler-app/src/features/places/handoff.rs:164-173` (`profile_for`), used by `moving.rs:441, 455` | The account → "the running profile, else the one seen last" | Two profiles on one account | Move from or into the wrong profile; a hand-off or "Open in" opens the conversation in the wrong profile | High (Move), Med (hand-off) | Resolve by the **record** (the profile whose records list the conversation) and by the **App id the user picked**; refuse when two profiles claim it. |
| E6 | `moving.rs:422-423` + `commands.rs:83-97` | The target picked as an AI Juggler profile → account → `profile_for` | The switch list shows Apps (D-143); the user picks App 2 of an account | The move targets App 1 | Med | Carry the place id end to end ("actions target by id"). |
| E7 | `crates/juggler-app/src/features/places/link.rs:13-20` (`folder_for_profile` by name) | `ClaudeProfiles/<name>` ↔ `ClaudeCodeProfiles/<name>` by folder name | A profile launched by the user with another `CLAUDE_CONFIG_DIR` | The move writes the history where the app doesn't read; a restart puts the profile on another account's folder | Med | Evidence: the profile's Code-tab `claude` pid file sits in `<tool_dir>/sessions/` (pid with the profile's main as ancestor) → that tool_dir. Check it before every move and every restart; refuse on a mismatch. |
| E8 | `moving.rs:372-393` (`open_elsewhere`), `places/mod.rs:1145-1160` (`working_in`) | The live model's `account_id` and host pid | An unattributed or headless `claude`; a stale live model (2 s tick, 30 s app records, `scan.rs:33-52`) | A writer runs during the move | Med | Ask the adapter's `writers()` for both tool folders at decision time (B2), not the cached model. |
| E9 | `MoveChoice.tsx:94-98` | "The newest interrupted move" | Another conversation's move is interrupted | **Undo of the wrong move** | Med | The error carries `move_id`. |
| E10 | `places/mod.rs:141-182` (`MoveHold`), `keep.rs` | An in-memory hold, per process | Dev + installed AI Juggler | The other app's keep alive restarts a profile mid-move | High (maintainer) | A shared hold marker (B5.1). |
| E11 | `crates/juggler-handover/src/moving/mod.rs:189-214` (`Lock`) | A lock keyed by the raw path string | Two spellings of a root | Two moves of one folder | Low | `InstanceKey`/canonical roots. |
| E12 | `crates/juggler-app/src/lab_scope.rs:83-114` | **String** prefix `starts_with(lab)` and `contains("={lab}")` on any arg | (a) `/tmp/aij-lab1` matches `/tmp/aij-lab10/…`; (b) a **real** Code-tab session with `--add-dir=<lab>` (an agent driving the lab) is kept in the lab's table | The lab's screens and captures show a real session (privacy); lab actions (Stop, Move's tree) can see a real process | Med | Component-wise canonical path check; an arg *value* never makes a process the lab's. Use ancestry from the lab's own launches + instance-lock proof for lab profiles. The same class as E1. |
| E13 | `crates/juggler-app/src/features/conversations/places.rs:230-286` (`host_of`) | The first known bundle on the chain; the instance from the main's args or the default | A `claude` started *inside* a Code-tab session's shell (a nested CLI) reads as running in that Claude profile; a tmux session reads as no host | Go to it brings the wrong window; Open now and the profile's session list count it under the profile | Low–Med | Keep the chain rule, but take the session's *account* only from its pid file's folder (B2), and its instance from the strict + lock proof. |
| E14 | `crates/juggler-adapters/src/codex_conversations.rs:400-412` | Any arg **containing** `@openai/codex`; `HELPERS` word anywhere | `npm i -g @openai/codex`, `npx @openai/codex …`, a prompt word `login` | A false "Codex session" in the live view; for a future Codex move, a false "open elsewhere" | Low | Match the program (exe or the node entry script path), not substrings; positional parse. |
| E15 | `crates/juggler-adapter/src/conversation.rs:186-200` (`value_after`) used for `--resume` by every adapter (`claude_conversations.rs:371`, `codex_conversations.rs:410`, `grok_build.rs:259`, `hermes.rs:341`, `kimi_code.rs:226`, `opencode.rs:784`, `zcode.rs:473`) | The first `flag=value` or `flag value` anywhere, after `--` too | A wrapper's own `--resume` vs the tool's; `codex … "resume" x` in a prompt | The session attributed to the wrong conversation; Go to it or switch on the wrong one | Low | Per-adapter strict parse (`--` ends options; one occurrence), the same shape as `attest.rs:338-372`; confirm with the pid file's `sessionId` where the tool writes one. |
| E16 | `moving.rs:873-903` (`still_writing`) | Path prefix → "this profile was writing" | Both sides' folders nest (a target tool_dir under a source host_dir) | Wrong words only | Low | Name the writer from B2/B3 evidence. |
| E17 | `crates/juggler-app/src/features/places/start.rs:84-88` + `moving.rs:774` | Check then `kill(pid)` by bare pid | Reuse between the two | SIGTERM to another program | Very low | Signal through a kqueue-registered `ProcId` (re-check start after registering); polite quit by Apple Event to the pid. |
| E18 | `scripts/leaked_fixtures.sh:30` | `pgrep -f '--user-data-dir=…/Lab X profile'` | Reporting only | A false "leftover" line | Very low | Fine as is. Noted because it is the same pattern. |

**Good patterns to copy (no change needed):** `attest.rs` (strict instance, canonical, lock holder, start + version, `still_the_same`); `features/kill` and `features/processes/stop.rs` (verify pid + start + program + boot at click time, never the app or AJ itself, `stop.rs:1-14`); `procs.rs`' identity (start secs + micros + name, `procs.rs:115-117`); Codex's writer locks (`codex_live.rs:90-112`); `first_open_window_check.sh:70-76` (filters `pgrep -x juggler-app` by full path, so the maintainer's app is never touched).

**One shared module.** E1–E4, E5–E7, E11–E13 and E17 should all go through `features::ownership` (B1). E8 and E15 go through the adapter `writers()` / strict session parse (B2). E9 goes through ids end to end. The kill and stop paths already follow the principle. They should consume `ownership::ProcId` too, so there is one identity type.

---

## What I did not verify

- Whether the current Claude/Electron version keeps `chrome_crashpad_handler` alive after the main process quits (A1.4). The lab walk item D4.1 answers it.
- Whether `disclaimer --pgroup` actually signals its group on exit (A1.5's crash variant). The descendant-refusal half of A1.5 holds either way.
- Whether the live model includes `background`/headless `claude` sessions in `entries` (A2.5).
- Electron's exact behaviour on SIGTERM (`before-quit` and dialogs). A3.1's handling is correct either way; only how often it happens is unknown.
