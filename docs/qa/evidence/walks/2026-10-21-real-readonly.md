# Real setup, read only — AI Juggler Dev at main `3ccf2bd9`, 2026-10-21

The first walk on the maintainer's real setup (allowed for read
operations only), after the Beta 0.0.12 reports: fabricated "100% left",
Codex limits unread, live sessions not listed, the memory alert repeated
with clashing figures, and Screen Recording not detected until a relaunch.

**Status: batches 1–5 from the backend (no clicks); batch 6 walks the real UI through the read-only bridge.** The Dev window could be seen but not
clicked (see W0), so journeys 2–6 could not be walked in the UI. What
this pass gives instead: the real setup counted on disk (the ground truth
the fixes must match), the first-run screen, and the code behind each
reported bug, checked against the real data's shape. Findings marked
**predicted** come from code plus real data, not from the UI; each one
must be re-checked in the window before it is called fixed or not.

Redaction: accounts are Account A (the default `~/.claude`), B–F (the
five Claude profiles) and G (Codex, `~/.codex`). Paths start at `~/`. No
project names, no transcript content: only counts, mtimes and usage fields.

## Setup

- **Build**: `just app` with `CARGO_TARGET_DIR` shared; the bundle step is
  off (`bundle.active: false`), so the raw `juggler-app` binary (identifier
  `ai.figurehead.juggler.dev`, so Dev edition and the `AI Juggler Dev`
  data folder) was copied out and run.
- **Started like Finder does**: `env -i HOME USER LOGNAME TMPDIR SHELL LANG
  PATH=/usr/bin:/bin:/usr/sbin:/sbin`. The agent's own shell carries
  `CLAUDE_CONFIG_DIR` and ~30 other `CLAUDE_*` variables, which would have
  skewed discovery.
- **Not touched**: the real AI Juggler (running, pid left alone), its
  data and WebKit state. No vendor CLI was run by the walker; no sign-in,
  no Allow, no write action in the app.
- **Afterwards**: the Dev app was quit and its data folder renamed to
  `~/Library/Application Support/AI Juggler Dev (qa-real-walk 2026-10-21)`.

### Snapshot diff (paths, sizes, mtimes; before launch → after quit)

197 entries: the top levels of `~/.claude`, `~/.codex`, ClaudeCodeProfiles
and each profile, plus 11 config files (`~/.claude.json`, each
`settings.json`, `.claude.json`, `config.toml`).

- Config files: **all 11 unchanged** (size and mtime).
- Paths added or removed: **none**.
- Changed mtimes/sizes, none caused by the walk:
  - `~/.codex`: `state_5`, `logs_2`, `goals_1` and `thread_history_1` (`.sqlite` / `-wal`), plus
    `models_cache.json`, `.codex-global-state.json(.bak)`, `session_index.jsonl` and
    `thread-writer-locks`. These are the live Codex inside the ChatGPT app (2 `codex`
    processes); the same files were already changing between the first snapshot and launch.
  - Account E's `plugins/plugin-directory-cache-v2.json`: the Claude app's
    plugin catalog cache, in the profile this agent session itself runs
    under. No other profile's `plugins` changed.

## Ground truth on disk (read-only counts at 15:20)

| | Top-level transcripts | Projects | Changed in 1 h | in 24 h | Assistant messages in 5 h | Output tokens in 5 h |
|---|---|---|---|---|---|---|
| Account A | 20 | 10 | 1 | 2 | 738 | 600,359 |
| Account B | 3 | 2 | 1 | 1 | 77 | 35,439 |
| Account C | 2 | 2 | 1 | 1 | 858 | 896,653 |
| Account D | 2 | 2 | 1 | 1 | 36 | 29,115 |
| Account E | 15 | 15 | 6 | 13 | 2,033 | 1,432,005 |
| Account F | 2 | 2 | 0 | 0 | 0 | 0 |
| Account G (Codex rollouts) | 249 | — | 4 | 43 | — | — |

Messages are deduplicated by `message.id` + `requestId` and include `subagents/` transcripts
only in the 24-hour shape check below.

- **Codex limits** (newest `token_count` event, 19:18:59Z):
  - `rate_limits.primary`: `used_percent` 68.0, `window_minutes` 10080 (a weekly window),
    `resets_at` present. That reads as 32% left.
  - `secondary`: null.
  - `plan_type`: pro.
  - `credits`: `has_credits` true, a balance present.
- **Running tools** (`ps`):
  - 6 Claude desktop app instances, one per profile data folder plus the default.
  - 1 ChatGPT app.
  - 16 `claude` CLI processes under the Claude app, 0 in a terminal.
  - 2 `codex` processes under ChatGPT.
  - RSS by group: Claude app 5.9 GB (91 processes), claude CLI 4.0 GB, ChatGPT 2.6 GB,
    codex 0.2 GB.
  - `ps` CPU summed: about 78% of one core (30 cores, so about 3% of all cores).
- **Memory** (`vm_stat`, 16 KB pages; `memory_pressure`):
  - 103 GB total.
  - Free 0.3 GB, active 21.9 GB, inactive 21.8 GB, wired 13.6 GB.
  - System-wide free: 42%.
- **Transcript shape** (all Claude transcripts changed in 24 h):
  - 160,181 lines carry `message.usage`; 0 carry a top-level `usage`.
  - 150,150 distinct line `uuid`s, but only 72,200 distinct `message.id`s.
  - Output tokens summed per line: 140.8 M; per message: 53.3 M.
  - Claude Code repeats one message's usage on every content-block line.

## Findings

Severity: **blocker** (stops the walk or a 0.1 gate), **high** (a wrong
number or a missing core feature), **medium**, **low** (wording, layout).

### W0 — The Dev window can be seen but not driven — blocker (for this walk)

- **Did**: launched the Dev build. Captured its window with `screencapture -l` (this works now,
  unlike the 0.0.11 walk). Then posted a test click (CGEvent) on the read-only "Where we
  looked" link.
- **Expected**: the click lands.
- **Happened**: `AXIsProcessTrusted()` is false for the session, so synthetic input is
  dropped. Granting it is the maintainer's call; the walker never presses Allow.
- **The workaround was refused**: a local, uncommitted, env-gated hook that would have
  evaluated scripted clicks in the Dev webview. The session's permission classifier
  blocked it as an RCE surface, and it was not pursued further.
- **What unblocks journeys 2–6** (maintainer's choice):
  - (a) Grant Accessibility to the Claude app for the walk, and revoke it after.
  - (b) Add a permission rule allowing the local eval hook. It needs `AIJ_QA_EVAL_DIR`, is
    only ever built locally, and is never committed.
  - (c) The maintainer clicks "Bring in 7 accounts" and the screens in the Dev window while
    the walker captures and compares.

### W1 — First run finds every real setup — pass

- **Did**: first launch of the Dev edition, empty data folder.
- **Expected**: everything on disk is found: ~/.claude, the five ClaudeCodeProfiles
  folders, ~/.codex, and the desktop apps.
- **Happened**:
  - "You already have 7 setups on this Mac."
  - Groups: "Your Claude profiles · 5", "The default setup · 1" (Default, `~/.claude`) and
    "Codex · 1" (`~/.codex`).
  - All seven show "✓ Signed in", and all are ticked.
  - "Use them where they are" is the Recommended choice; "Bring in 7 accounts" is the button.
  - Versions: "Claude Code 2.1.302 installed", "Codex 0.163.0 installed". Both were found
    from a Finder PATH.
- Screenshot: [w1-01-first-run-7-setups.jpg](2026-10-21-real-readonly/w1-01-first-run-7-setups.jpg)
  (profile names and paths redacted).

### W2 — "Open in the Claude app" reads as a button, and says the same as "In use now" in other words — low

- **Saw**: the six Claude rows say "Open in the Claude app" and the Codex row says
  "In use now".
- **Why it's wrong**:
  - All are statuses: the folder is in use right now, by the Claude app or by ChatGPT's Codex.
  - They are in the accent-coloured text also used for the links below.
  - "Open in the Claude app" reads as an action that would open it.
  - Codex's row doesn't say what has it open (the ChatGPT app).
  - D-25: one concept, one phrase.
- **Expected**: one pattern, such as "In use · Claude app" and "In use · ChatGPT app".
- **Code**: `ui/src/screens/FirstRun.tsx:126-136` (`usedWords`) and `:541` (the class).

### W3 — The five profile paths are cut where they differ — low

- **Saw**: every profile row's path ends "…/ClaudeCodeProfiles/boo…". The part that tells
  them apart is cut off.
- **Expected**: a middle ellipsis ("~/Library/…/ClaudeCodeProfiles/<name>"), or the full
  path on hover.
- **Code**: the path line of the found row in `ui/src/screens/FirstRun.tsx` (near `:541`).

### W4 — The Claude Code version sits on the default setup's header only — low

- **Saw**: "Claude Code 2.1.302 installed" is on "The default setup · 1". "Your Claude
  profiles · 5" has nothing, though the same Claude Code runs all six.
- **Expected**: the version once, above or on the first Claude group.
- **Code**: `ui/src/screens/FirstRun.tsx:86` and where it is placed per group.

### W5 — The unselected choice's radio is a solid white dot — low (visual)

- **Saw**: "Make separate copies", not chosen, shows a filled white circle. The chosen
  "Use them where they are" shows a purple dot in a ring. At a glance the white dot looks
  like the stronger mark.
- **Expected**: an empty ring for not chosen.

### P1 — Codex limits are in the rollouts but never read — high (predicted; the reported bug)

- **Truth**: the newest Codex `token_count` has `rate_limits.primary.used_percent` 68,
  a weekly window, `resets_at`, `plan_type` pro and a credits balance.
- **Code**:
  - `reports_limits` is only the `USAGE_QUOTA_FEED` capability, which only Claude declares
    (`crates/juggler-app/src/commands.rs:13574`, `:13604-13606`;
    `crates/juggler-adapters/src/claude_code.rs:130`).
  - The rollout readers take token counts only, never `rate_limits`:
    - `commands.rs:4619-4625`
    - `crates/juggler-adapters/src/quota_ingest.rs:504-575`
    - `crates/juggler-app/src/usage_pipeline.rs:206-224`
  - So Home's Codex row says "Codex doesn't report its limits to AI Juggler yet"
    (`ui/src/lib/providers.ts:41-51`).
- **Expected**: Account G shows "32% left this week · resets <date>", Measured, from
  `primary`, plus credits when `has_credits`. `secondary: null` means no 5-hour window,
  not 0.

### P2 — Live and recent sessions aren't listed — high (predicted; the reported bug)

- **Truth**:
  - 16 `claude` sessions are running in the Claude app and 2 `codex` in ChatGPT.
  - 10 Claude transcripts changed in the last hour, across 5 accounts.
  - 4 Codex rollouts changed in the last hour, 43 in 24 h.
- **Code**:
  - The rail and History list only AI Juggler's own sessions table:
    `instances_list` → `session_infos`, `crates/juggler-app/src/commands.rs:4258-4270`,
    `:5267`.
  - The empty state shows at `ui/src/components/WorkflowRail.tsx:586-593`.
  - Sessions started elsewhere never appear, running or not.
  - The project backfill (`commands.rs:5394`) reads only the Claude layout
    `projects/*/…jsonl`, so Codex rollouts add no projects.
- **Expected**:
  - Running ones listed as running, marked "not started by AI Juggler".
  - Recent ones in History.
  - The counts should match the table above.

### P3 — Claude token totals likely counted 2–2.6 times — high (predicted)

- **Truth**: Claude Code writes one line per content block, each with the message's full
  `usage`. There are 150,150 line uuids for 72,200 messages. Output tokens come to 140.8 M
  summed per line against 53.3 M per message.
- **Code**:
  - `transcript_events` dedupes by the line `uuid`: `crates/juggler-adapters/src/quota_ingest.rs:432-492`,
    with the store `UNIQUE(account, msg_uuid)` in `crates/juggler-core/src/storage/usage.rs:27-76`.
  - So do `dedupe_messages` (`crates/juggler-adapters/src/claude_transcript.rs:183-200`) and
    the window's `burned_tokens` (`commands.rs:426-433`).
- **Expected**: dedupe by `message.id` (+ `requestId`); the per-account totals then match
  the table.

### P4 — The inspector reads `usage` where Claude never writes it — high (predicted)

- **Truth**: 0 Claude lines have a top-level `usage`; all of it is under `message.usage`.
- **Code**:
  - `scan_transcript` (`crates/juggler-app/src/commands.rs:4680-4736`) skips any line
    without top-level `usage`. Claude turns, tokens and per-minute bars therefore come
    out empty.
  - It also hard-codes `cache_read: 0`.
  - Session `tokens` are set to `TokenCounts::default()` at insert (`commands.rs:3433-3509`)
    and never updated.
- **Expected**: the inspector's numbers match the session's transcript.

### P5 — "100% left" can be a stale sample from a window that has reset — high (predicted; the reported bug)

- **Truth**: in the last 5 hours, Accounts A–E sent 36–2,033 assistant messages each. Only
  Account F had none.
- **Code**:
  - The percent comes only from Measured statusline samples
    (`crates/juggler-adapters/src/quota_ingest.rs:363-387`). An absent field gives no
    sample, and there is no default of 100.
  - But `evaluate_window` (`crates/juggler-core/src/quota.rs:482-590`) keeps using the
    latest sample after its `resets_at` has passed; only the Exhausted branch checks it.
  - Accounts used outside AI Juggler send no new samples (`usage_pipeline.rs:98-104`: only
    an in-place account's overlay spool, which only AI Juggler-launched sessions feed).
  - So a reading taken at the start of an old window can show "100% left" for hours of
    heavy use elsewhere.
- **Expected**:
  - Once `resets_at` is past, the old reading is gone: "Not measured yet" (D-06), or
    "Measuring…".
  - When a reading predates use the app can see in the transcripts, it is marked stale,
    not shown as current.
- **Corroboration** (passive, not walked):
  - The screen-capture permission test caught the maintainer's open AI Juggler 0.0.12 Home.
  - All six Claude rows said "100% left", four with "5-hour window resets" at the
    capture's own minute.
  - The rail said "No sessions yet", and Codex said it "doesn't report its limits".
  - That capture holds real names and is not committed.

### P6 — This Mac shows 3 of 6 Claude app instances, and its CPU figure disagrees with `ps` — medium (predicted)

- **Truth**: 6 Claude app instances are running. `ps` puts all AI tools at about 3% of all
  cores and 12.7 GB RSS.
- **Code**:
  - `assemble` keeps `.take(3)` rows (`crates/juggler-app/src/machine_health.rs:1040-1049`),
    with no "and 3 more".
  - Memory is RSS summed per process tree (`crates/juggler-supervisor/src/supervisor.rs:1251`,
    `:1267`, `:2824`), so shared helpers count more than once ("about").
  - Only Claude tools are detected, not Codex or ChatGPT (`crates/juggler-app/src/running_setups.rs:41-62`).
  - Desktop instances map to accounts by `CLAUDE_CONFIG_DIR`, else `~/.claude`
    (`running_setups.rs:90-143`). The first match wins (`machine_health.rs:1065-1080`).
- **Corroboration** (the passive 0.0.12 capture): "AI tools: about 21 GB of 103 GB memory ·
  20% of all CPU cores", against `ps` at about 3%. Rows: Claude app 8.6 / 6.3 / 3.0 GB,
  CPU 140 / 205 / 179%.
- **To check in the walk**:
  - Which instance maps to which account.
  - Whether the CPU sum double-counts trees.
  - The alert dedupe (`crates/juggler-app/src/machine_alerts.rs:323-411`: one per kind per
    30 min, but every `this_mac_impl` caller advances the engine, `machine_health.rs:1087-1182`).

### P7 — Screen Recording state — to check (predicted)

- `os_access.rs:142-176` re-checks on every call, but Screen Recording uses
  `ScreenCaptureAccess.preflight()` (`:111`), which macOS can report stale until relaunch.
  That matches the report.
- The walk could not reach step 3 ("Computer access") to see the current wording.

## Batch 2 — the oracle, and the maintainer's newest observations

### The oracle: `scripts/qa/real_truth.py`

- **What it reads**: names, mtimes and metadata or usage fields only, from each account's
  `projects/**/*.jsonl` and `sessions/**/rollout-*.jsonl`, plus `ps` and `lsof -d cwd`.
  Never content, credentials or other processes' environment.
- **What it prints**, redacted:
  - projects, with agent worktrees (removed ones too) folded into their repo;
  - conversations per project per account;
  - live sessions mapped to an account (by `--resume <id>`, else by the transcript folder
    of the process's cwd);
  - tokens per rolling 5 h / 7 d window (by message id, with the per-line sum beside it);
  - Codex limits.
- `--key FILE` writes the real names to a private file; `--selftest` checks the pure parts.
- Run at 15:32: about 23 s. The snapshot diff after it is still clean.

The oracle at 15:32:

| Project | Conversations (oracle) | Folders | 0.0.12 rail at 15:18 (passive capture) |
|---|---|---|---|
| Project 2 (the AI Juggler repo) | 13 (Account E 13) | 13: repo, live worktrees, removed worktrees | "5 conversations", plus removed worktrees as separate "MOVED" projects of 1 each |
| Project 1 | 242 (Account A 4, Account G 238) | 2 | "3 conversations · Default" (Codex not counted) |
| Project 3 | 13 (Account A 13) | 6 | "13 conversations · Default" ✓ |
| Project 6 | 3 (Account A 2, Account F 1) | 1 | "3 conversations · 2 accounts" ✓ |
| Project 7 | 3 (Account B 2, Account C 1) | 2 | "3 conversations · 2 accounts" ✓ |
| Project 8 | 1 (Account D 1) | 1 | "1 conversation" ✓ |
| Projects 4, 5, 9 | 11 / 6 / 1 (folders gone; 4 and 9 Codex only) | 1 each | not visible ("Show all 13") |

- Truth: 9 projects once worktrees fold. The rail said "Projects 13".
- **Live** at 15:32:
  - 16 `claude` sessions in the Claude app: Account A 4, B 1, C 1, D 1, E 9. All 16 matched
    an account: 7 by `--resume`, 9 by their folder.
  - 3 Codex threads written in the last 15 minutes.
  - ChatGPT's 2 `codex` processes are helpers (`app-server`, `exec-server`), not sessions.

### P8 — Projects appear only after a reload — high (confirmed in code; observation 1)

- `ui/src/App.tsx:457-459` runs `projectsBackfill()` once, when App mounts.
- On a first run, App mounts before first run brings any account in, so the backfill sees
  no accounts and finds 0 projects. Nothing calls it again after Bring in, an import or an
  Add account.
- `crates/juggler-app/src/commands.rs:5385` is the only command; there is no event.
- **Expected**: run it again whenever the account list changes (or as an `accounts_changed`
  follow-up), then refresh the rail.

### P9 — Removed agent worktrees become their own "MOVED" projects — high (confirmed in code and on disk; observation 2)

- **On disk**: 8 Claude project folders belong to `<repo>/.claude/worktrees/<name>`
  folders that no longer exist. Another 10 belong to worktrees that still exist.
- **Code**: `worktree_repo_root` (`crates/juggler-core/src/storage/projects.rs:208-238`)
  folds a worktree only by reading its `.git` file.
  - A removed worktree has none.
  - Walking up, it reaches the repo's own `.git` folder and returns `None` ("a
    repository's own root"). So the cwd itself becomes the project root: missing on disk,
    hence "MOVED", one per removed worktree.
  - The same branch means a cwd in a repo's subfolder never folds either.
- **Expected**:
  - When walking up hits a `.git` folder, the cwd belongs to that repo.
  - `<repo>/.claude/worktrees/<name>` folds by path when the folder is gone, as the oracle
    does (`project_of`).

### P10 — The repo's count is low, and Codex never counts — high (confirmed; observation 3)

- **AI Juggler repo**: 13 conversations against "5". The 8 missing are the removed
  worktrees of P9.
- **Project 1**: 238 Codex conversations, none counted.
  - The backfill (`commands.rs:5394-5440`) reads only `<account>/projects/*/`.
  - Codex rollouts' `session_meta.cwd` is never read.
  - It also counts by the newest transcript's cwd per folder, reading only its first 25
    lines (`transcript_cwd`, `commands.rs:5536`); 1 folder on disk is lost that way.
- **Project 1's Claude count**: 3 shown against 4 on disk. Not explained yet.
- **"Opens empty"**: the project view lists only AI Juggler's own sessions table (see P2),
  so a project made only of transcripts has a number and nothing to open.
- **Expected**: counts match the oracle per account, and the project opens to its
  conversations (History rows from the transcripts).

### P2 update — "No sessions yet" against 16 live (observation 4)

16 live `claude` sessions on 5 accounts, and 3 active Codex threads. The rail's source is
still AI Juggler's own table only (P2's code).

### P11 — Usage shows a blank window — blocker (reproduced in a render; observation 5)

- **Not captured in the window** (W0). Reproduced instead with vitest, rendering
  `CostMatrix` from the payload Rust really sends.
- **Cause**: no `Option` in the cost payload has `skip_serializing_if`, so a `None` arrives
  as `null`:
  - `crates/juggler-core/src/cost_ledger.rs:83`, `:107`, `:322-326`;
  - `crates/juggler-app/src/commands.rs:71`.
- `ui/src/components/CostMatrix.tsx` guards each one with `!== undefined`.
- **Reproduced**:
  - Empty store, no fee: `TypeError: Cannot read properties of null (reading
    'toLocaleString')` at `usdText` (`CostMatrix.tsx:14:13`) ← `CostMatrix.tsx:70:13`.
  - One priced row, no fee: the same error, at `:70:57`.
  - Fee set, a Claude row with `credits` null: `… (reading 'toFixed')` at line 103.
- **Same pattern**: lines 72, 107 and 113.
- **Why the whole window goes blank, rail included**: `ui/src` has no error boundary.
- **Why tests miss it**: the mock (`ui/src/lib/bridge.ts:1620-1630`) sends
  `subscription: {}`, which is undefined, not null.

## Batch 3 — the backend against the oracle

### How

- **The harness**: `crates/juggler-app/tests/qa_real_readonly.rs`.
  - Ignored, and a no-op without `AIJ_QA_REAL_DATA` / `AIJ_QA_REAL_OUT`.
  - It does what first run's "Bring in N accounts" (where they are) does, into a fresh
    empty data folder.
  - Then it calls the same functions the UI's commands call:
    - `projects_backfill_impl` → `projects_list_impl`;
    - `sessions_list`;
    - four `usage_pipeline::ingest_pass`es → `quota_display_all`, `cost_rollups`;
    - 36 This Mac ticks at 2 s, with `this_mac_impl` and `machine_alerts_take_impl`.
  - Its JSON (real paths, kept in scratch) goes to `scripts/qa/real_truth.py --compare`,
    which prints redacted OK / MISMATCH lines.
- **Run**: the test binary with the Finder-like `env -i` environment.
- **Snapshot diffs**:
  - Run 1: one profile's `.claude.json` mtime moved (same size) and its `backups/` gained a
    file. That is Claude Code's own periodic save: that folder's backups are 41–46 minutes
    apart all afternoon (13:29, 14:10, 14:56, 15:42), and the account's live session wrote
    a transcript 17 s later. Not the harness.
  - No path added or removed, no other config file touched.
  - Run 2: no config file changed, no path added or removed.
- **Limits**:
  - Runs 1–2 called only the machine tick, so This Mac's running tools read 0. Run 3
    calls the app's own `processes_tick` (collector table, then the machine tick), and
    covers them (batch 4).
  - Rolling windows: the oracle's 5 h / 7 d are rolling; the vendor's are anchored.

### Result (run 1, 15:44)

| Check | Backend | Oracle | |
|---|---|---|---|
| Setups found / brought in where they are | 7 / 7 | 7 | OK |
| Project rows | 16 | 9 | MISMATCH (P9) |
| The AI Juggler repo | 14 conversations in 9 rows (8 removed worktrees split off) | 14, one project | MISMATCH (P9) |
| Project 1 | 4 in 2 rows (a pruned worktree split off) | 4 Claude + 238 Codex | MISMATCH (P9, P10) |
| Codex-only Projects 4 and 9 | absent | 11 and 1 | MISMATCH (P10) |
| Projects 3, 5, 6, 7, 8 | 13, 6, 3, 3, 1 with the right accounts | same | OK |
| Sessions listed | 0 | 17 CLI sessions + 2 active Codex threads | MISMATCH (P2) |
| Claude percent, all 6 accounts | none (state "tracking", no sample) | — | OK: no fabricated percent on a fresh install |
| Claude "resets at" | every one = first event in the window + its length; 8 of 12 within ±1.2 min of now | no vendor reset measured | MISMATCH (P12) |
| Claude tokens, 5 h / 7 d | ×1.8–2.5 the oracle, every account with use | — | MISMATCH (P3) |
| Codex weekly percent | none | 69% used, resets Tue 27 Oct 02:33 | MISMATCH (P1) |
| Codex 5-hour window | tracked, resets ≈ now | none (`secondary` null) | MISMATCH (P13) |
| Codex tokens, 5 h / 7 d | ×1.98 / ×2.00 | — | MISMATCH (P14) |
| Cost payload | `api_equivalent_usd`, `flat_fee_usd`, `value_ratio`, `total_usd` null; 452 of 452 rows with null credits/usd | — | MISMATCH (P11: Usage crashes on exactly this) |
| Cost rows priced | 0 of 452 | — | MISMATCH (P15) |
| This Mac memory | 81.2 of 103 GB (app + wired + compressed), pressure "warning" | `vm_stat` anon 26.7 + wired 12.7 + compressor 49.1 GB; `kern.memorystatus_vm_pressure_level` 2 = warning | OK |
| This Mac machine CPU | 33–95% over the run | `top` sampled alongside (run 2): within a few points at every tick, spikes included | OK |
| Alerts over 72 s | 1 (memory pressure) | — | OK, no repeat from the engine |

Run 2 (15:48) reproduced every line (33 mismatches both times).

### P2, P3, P9, P10, P11 — confirmed in the backend

Same causes as above, now with the backend's own numbers (table).

- P9: the 8 removed worktrees are rows with `missing: true`.
- P9 also has a second shape: a worktree folder that still exists but has no `.git`
  (pruned) splits off the same way. That explains Project 1's 3-vs-4.

### P12 — "Resets at" is made up: the window's start plus its length — high (backend)

- With no vendor sample, every Claude window's `resets_at_ms` is the first usage event in
  the rolling window plus 5 h or 7 d. Checked against the transcripts for three accounts,
  the difference is within 2 minutes.
- Under continuous use that is "now": 8 of the 12 Claude windows read within ±1.2
  minutes of the moment they were read.
- That is what the 0.0.12 Home showed: "5-hour window resets Wed 03:18 PM" at 15:18 (P5).
- **Expected** (D-06): no reset time unless one was measured (the statusline's
  `resets_at`); otherwise "Not measured yet".
- **Code**: the window evaluation in `crates/juggler-core/src/quota.rs:482-590` and
  `quota_display_inner` (`crates/juggler-app/src/commands.rs:361-480`).

### P13 — Codex gets a 5-hour window it doesn't have — medium (backend)

- Codex's rollouts report a weekly `primary` (10080 minutes) and `secondary: null`. The
  backend still tracks a `five_hour` window for Account G, with its own made-up reset.
- **Expected**: the windows the tool reports. One weekly window here, from `rate_limits`
  (P1).

### P14 — Codex tokens counted twice — high (backend; numbers and code)

- Backend 560.3 M in 5 h against the oracle's 283.3 M. A second method, the sum of
  Codex's own per-turn `last_token_usage`, gives 285.3 M. 7 d: ×2.00.
- **Cause**: Codex's `input_tokens` already include `cached_input_tokens`, and
  `output_tokens` already include `reasoning_output_tokens`. The rollout reader stores
  all four as separate classes (`crates/juggler-adapters/src/quota_ingest.rs:546-552`),
  so cached input and reasoning count twice.
- **Expected**:
  - `input = input_tokens − cached_input_tokens`
  - `output = output_tokens − reasoning_output_tokens`
  - or keep the classes and stop adding the included ones.
- The inspector's Codex scan (`commands.rs:4588-4617`) should be checked for the same.

### P15 — No real model is priced, and most rows have no model — medium (backend)

- The bundled pricing seed (`crates/juggler-core/src/cost_ledger_seed.json`) has 2 rows:
  `claude-opus-4` and `claude-sonnet-4`.
- The real rows' models are claude-opus-5-5 (121), claude-opus-4-6 (15),
  claude-haiku-4-5-20251001 (7), claude-sonnet-4-6 (6), claude-opus-4-8 (3) and
  claude-sonnet-5-5 (3). None is priced, so every USD cell is "—" (honest, but Usage
  can't show a cost).
- 297 of 452 rows have no model at all: the Codex rows, whose model is in the rollout's
  `turn_context`.
- `pricing_last_synced_ms` is null.

## Batch 4 — cost per session, and This Mac against `ps`

Run 3, 16:05:

- The harness now ticks through `commands::processes_tick`, as the app's 2 s thread does.
  Its cleanup sweep acts only on AI Juggler's own ledger orphans, and a fresh store has
  none.
- It records `ps` (pid, ppid, RSS, CPU; no arguments) at each This Mac read.
- `--compare` builds `ps` trees rooted at each desktop app's main process, and checks the
  cost rows per session against the transcripts' usage per `sessionId` (subagents
  included).
- Snapshot: no config file changed, no path added or removed. 35 mismatches (the 33,
  plus the two below).

| Check | Backend | Truth | |
|---|---|---|---|
| This Mac tools | 6 | 6 Claude app instances in `ps` (ChatGPT's Codex not counted, by design) | OK |
| This Mac tools memory | 19.2 GB | 21.1 GB `ps` RSS of the 6 trees (×0.91) | OK |
| This Mac top rows | 6.0 / 4.5 / 4.5 GB (Account E, D, A) | 7.7 / 4.5 / 4.5 GB | MISMATCH (P16) |
| Instances not shown | 3 of 6, no "and 3 more" | — | (P6) |
| Account labels on the rows | the 10-CLI instance is Account E, which has the most live sessions | — | OK |
| Cost rows per session, Claude | 42 sessions: median ×2.13, p10 ×1.86, max ×2.50 | transcripts, by message id | MISMATCH (P3) |
| Cost rows per session, Codex | 246 sessions: median ×1.93, p90 ×1.96 (the 2 at ×0.51 were oracle errors, see P17) | rollouts' per-turn `last_token_usage` | MISMATCH (P14) |
| Sessions only in the transcripts | 2 | — | noted |

### P16 — The largest Claude app tree reads 1.7 GB short — medium (backend; to investigate)

- **Truth**: Account E's instance tree is 7.7 GB and 111 processes in `ps`, taken right
  after the read.
- **Backend**: 6.0 GB with 74 helpers. The other two top rows match `ps` to 0.1 GB.
- About 37 processes in that tree aren't counted. Likely children that appear after the
  observed root was set; that instance has 10 live `claude` CLIs.
- **Code**: the observed-roots tree in the collector
  (`crates/juggler-supervisor/src/supervisor.rs:1251`, `:1267`, `:2824`) and
  `set_observed_roots` (`crates/juggler-app/src/machine_health.rs:331-410`).

### P17 — Corrected: the oracle was wrong; the backend misses only pre-turn usage — low

- **The two ×0.51 sessions** were two errors in the oracle:
  - One thread was still being written after the backend ingested it. `--compare` now
    leaves out sessions written during the run; the harness records `ingested_ms`.
  - The other is a subagent thread (`session_meta.source` subagent, with
    `parent_thread_id`). Its first `total_token_usage` carries earlier tokens: 13.9× its
    own first turn. The oracle now sums each turn's `last_token_usage` (skipping repeats
    of an unchanged total) instead of taking the final total.
- **After the fix**: 277 Codex and Claude sessions compared, minimum ×1.00, median ×1.94
  (P14 and P3). No session is under-counted.
- **What is left in the backend**:
  - In that thread, the `token_usage_record`s match `token_count` turns 2..6 exactly.
  - The first `token_count`, right after a `compacted` line and before any turn, has no
    record, so the backend doesn't count it: 16% of that thread.
  - Across all 247 rollouts, 3 have this shape: 0.003% of all Codex tokens.

### This Mac CPU — not compared

- The tools' CPU is 10.1% of all cores. `ps %CPU` is a decaying average, so the two can't
  be compared.
- The machine CPU matched a simultaneous `top` (batch 3).

## Batch 5 — Codex `rate_limits` over time (for an honest "as of")

`scripts/qa/real_truth.py --codex-history`, 16:12. It covers every `token_count`
reading in every rollout of the one Codex account, from 2026-09-22 to now.

- **How often**:
  - 74,874 readings in 246 threads; 99.99% of `token_count` events carry `rate_limits`.
  - While a thread works: a reading every 14 s (median; p90 52 s).
  - Between bursts: 12 gaps over 1 h, 7 over 8 h, the longest 72 h. With nothing running,
    the newest reading is as old as the last turn.
- **How it moves**:
  - `used_percent` is a whole number. It changed 713 times, a median 30 minutes apart
    (p90 59 min).
  - Steps are mostly 1%; bigger after idle time.
- **Its shape**:
  - Only `primary`: `window_minutes` 10080 (weekly). `secondary` never appears (so no
    5-hour window, P13).
  - Plan `pro`; the credits balance is always present; `rate_limit_reached_type` is never
    set.
  - Odd readings: 3 with a null `primary`, 8 with `window_minutes` 0, 180 with a null plan.
- **`resets_at` jitters**:
  - Up to 75 s between readings of the same window: 2,474 changes, against 27 real
    rollovers.
  - A changed `resets_at` is not a new window. Group resets within an hour (the oracle
    does).
- **Conflicting readings**:
  - The newest line is not always the newest value. 19 readings are 1% lower than the one
    before in the same window, all from another, concurrent thread.
  - 9 readings jump back to an older window. Some come up to 166 h after the newer window
    first appeared: a thread resumed days later reports its old limits.
  - Late 10-19 to early 10-20, readings alternated within minutes between "80–88% used,
    resets 10-24" and "0% used, resets 10-27".

**What the UI can honestly say** (proposal):

- **Which reading**: the window with the latest `resets_at` (grouping out the jitter), and
  inside it the highest `used_percent`, not simply the last line read.
- **The label**: "as of <that reading's time>".
  - While Codex is working that is under a minute old.
  - When it's idle, it ages: after about an hour, say "as of 3:12 PM"; after a day, the
    date.
- **After `resets_at` passes**: the reading says nothing about the new window. Show "Not
  measured since the reset" (D-06), not 0% and not the old percent.
- **Odd readings**: ignore `window_minutes` 0 and a null `primary`.

## Batch 6 — the real UI through the read-only bridge (main `e2abd09f`)

### How

- **Data**: the harness did first run's in-place bring-in into a fresh
  `AI Juggler Dev` folder (the bridge refuses imports). The accounts are labelled
  Account A–G in the oracle's order (`AIJ_QA_REAL_LABELS=redacted`), so only project
  names and folder paths are blacked out in the captures.
- **App**: the Dev build with `--features dev-bridge`. Built with the Dev identifier: a
  plain `cargo build` keeps `ai.figurehead.juggler`, and the single-instance lock and the
  hand-over are keyed by it, so it could reach the real app.
- **Started Finder-like**, with `AIJ_DEV_BRIDGE=1` and Vite on 5281. The UI ran in the
  built-in browser at 1440×900, then 760×900, dark, then light.
- **Actions**: read-only clicks only (tabs, disclosures, the More menu, typing in
  search). Every refused write showed in the bridge's strip: Home and Accounts ask for
  `fleet_remote` and `validation_matrix` on load.
- **Snapshot**: no config file changed, no path added or removed.
  - Changed entries: `~/.codex` (the live Codex in ChatGPT).
  - In two profiles: `policy-limits.json` and `remote-settings.json`, with their signature,
    stamp and synced-at files. That is Claude Code's own managed-settings sync. Each
    profile syncs on its own clock all day (:59, :00 and :34 hourly; the busiest profile
    every 15–20 min); the profile with no live sessions never changes; both walk-time
    changes fall on those clocks.
- **Afterwards**: the app was killed, so its `dev-bridge.json` stayed, with a token that
  died with the run. The Dev data folder was renamed to
  `AI Juggler Dev (qa-bridge walk 2026-10-21)`. A first bring-in with real labels is
  kept aside as `… (qa-bridge real labels …)`.
- **Known, still present** (#120 isn't merged): P1, P2, P6, P9, P10, P12, and P3/P14/P15
  behind the Usage crash.

Captures:

- [Home](2026-10-21-real-readonly/b1-home.jpg) · [light](2026-10-21-real-readonly/b9-home-light.jpg)
- [Accounts](2026-10-21-real-readonly/b2-accounts.jpg)
- [Usage before the crash](2026-10-21-real-readonly/b3-usage-before-crash.jpg) · [Usage blank](2026-10-21-real-readonly/b4-usage-blank.jpg)
- [Project view](2026-10-21-real-readonly/b5-project-overview.jpg)
- [History](2026-10-21-real-readonly/b6-history-empty.jpg)
- [Memory alert, top](2026-10-21-real-readonly/b7-memory-alert-top.jpg) · [This Mac panel](2026-10-21-real-readonly/b8-memory-alert-panel.jpg)

### P18 — "100% left" is made in the UI from a null — blocker (the maintainer's first report)

- **Saw**: "100% left" on all six Claude accounts, on every screen that shows runway.
  - **Home**: "5-hour window resets Wed 05:40 PM" at 17:40. Account F, with no use in
    5 h, reads "100% left in the 5-hour window".
  - **Accounts**: both windows, every account, e.g. "This week: 100% left · resets Wed
    05:34 PM", which moved to 05:35 a minute later.
  - **Usage**: all seven cards, Account G (Codex) included, while Home says Codex
    "doesn't report its limits".
  - **Recommendations built on it**: "Best for new work: Account A, 100% left" and
    "Most runway: Account A".
- **The backend at the same moment** (harness on `e2abd09f`): every window
  `percentage: null`, `state: "tracking"`. So nothing was measured.
- **Cause**: serde sends `null`, and the UI treats it as 0% used.
  - `ui/src/screens/Home.tsx:147` filters with `w.percentage !== undefined` (null
    passes); `:152-153` compute `100 - (percentage ?? 0)`.
  - `ui/src/components/AccountRunway.tsx:29` (`windowLine`, used by Accounts and
    Usage) does the same, and never asks whether the tool reports limits.
  - `ui/src/components/QuotaRings.tsx:229` has the same guard.
- **Seen once**: on opening Home, the Runway read "Not measured yet" on every account
  for a moment, then turned to "100% left" when the backend's nulls arrived. The honest
  state exists only until the data arrives. Sampled every 2 s for 42 s afterwards: always
  "100% left".
- **Expected** (D-06):
  - With no number, "Not measured yet".
  - A tool that doesn't report limits says so everywhere, Usage included.
  - Guard with `typeof x === "number"`, and test with serde-shaped nulls.

### P11 — confirmed in the real UI: Usage blanks the whole window — blocker

- **Saw**: Usage drew its runway cards and "Loading costs…". When the costs arrived,
  `#root` emptied: a dark, blank window, rail included.
- **Error** (captured with a debug listener):

  ```text
  TypeError: Cannot read properties of null (reading 'toLocaleString')
      at usdText (components/CostMatrix.tsx:15:38)
      at CostMatrix (components/CostMatrix.tsx:107:6)
  ```

- **The bridge's `cost_rollups` payload**: `subscription` with all three fields null,
  `total_usd` null, and `usd.usd` and `credits` null on all 455 rows.
- This run threw at line 107 (a row's USD) where the vitest render threw at line 70. The
  same `!== undefined` guard fails at both, so one fix covers both.

### P19 — One memory condition, four places, three readings — high (the maintainer's report)

- **Saw**, at the same moment:
  - **Page title**: "Your Mac is under memory pressure".
  - **Needs you**: "macOS reports memory pressure. The Claude app is using about 7.4 GB".
    That is one instance (Account E's), not all AI tools.
  - **This Mac**: "Getting tight · macOS reports memory pressure · AI tools: about 18 GB
    of 103 GB memory".
  - **Rail footer**: "this-mac · healthy". At 760 px the collapsed rail says "All clear".
- **Truth**:
  - `kern.memorystatus_vm_pressure_level` is 2 (warning).
  - The compressor holds 44–49 GB; AI tools are 18 GB of about 80 GB used.
- **"See what's using it"** only scrolls to the same three AI-tool rows. It never shows
  what fills the memory.
- **Expected**:
  - One statement of the condition, with one set of figures (pressure level + what's using
    the memory), in one place.
  - The footer and the collapsed rail agree with it.
  - An attribution that doesn't blame one Claude instance for a compressor-bound Mac.
- **Code**: `ui/src/components/ThisMacPanel.tsx`, `alertSubject` in
  `ui/src/lib/machine.ts:419-440`, Home's Needs-you item (`home-mac-pressure`), and the
  rail's machine footer.

### P20 — History and its search cover only sessions AI Juggler started — high

- **Saw**: History shows "0 found · No ended sessions yet". Searching "test" gives
  "Nothing matches 'test'", although `transcript_search` answered 200. There are about
  290 conversations on disk.
- **Cause**: `index_transcripts` (`crates/juggler-app/src/history.rs:393-401`) indexes
  only rows of AI Juggler's sessions table that have a `transcript_ref`.
- The page promises "Search by … anything said in the conversation".
- **Expected**: the brought-in accounts' conversations in History, and searchable. This is
  the History half of P2 and P10.

### P21 — A project with 14 conversations and 9 live sessions has only empty tabs — medium

The AI Juggler repo's project view:

- **Overview**: "Nothing started from AI Juggler here yet · 6 conversations · Account E".
  The oracle counts 14 (P9).
- **Usage**: "Unavailable · No usage measured for this yet", although the cost rows hold
  these sessions.
- **History**: "No history recorded yet · from the session's transcript". There is no
  session in a project view.
- **Quality / Performance**: "Unavailable".
- **Processes**: "Read-only rollup across 0 sessions … No subprocesses", with 9 live CLIs
  in this repo.
- **Actions**: "Launch with another account (ranked by headroom)" lists Account A–G in
  alphabetical order, with no headroom known, and "Main" under every one (a profile
  name, jargon).
- **Footers**: Overview's and Actions' say "from this Mac's process monitor", the wrong
  source for those tabs.
- **Expected**: the conversations listed (and openable), the live sessions shown, and
  empty tabs that say why.

### P22 — "Nothing running" everywhere while 17 run — high (more surfaces of P2)

- Accounts says "Nothing running" on every account. Account A has 4 live sessions and
  Account E has 10 (the oracle).
- The Go-to-session palette says "No live sessions — start one from the rail".
- The rail says "No sessions yet".

### P23 — "MOVED" on a folder that's there; the rail's count — low

- The pruned worktree of Project 1 (the folder exists, its `.git` is gone) shows
  "MOVED".
- The rail header says "Projects 13" while 16 rows show. It counts a group of 4 as one.

### P24 — Accessibility of the rail's projects — low

- Each project button's accessible name is its full absolute path, home folder included
  (from its `title`), not the project's name.
- The open project has a highlight but no `aria-current`.

### P25 — Permission states can't be seen after first run — medium

- Settings has no section for Screen Recording, Accessibility or Automation. Only
  `FirstRun.tsx` calls `os_access_list`.
- After "Not now", or a later change in System Settings, a user can't see where a
  permission stands. That is the 0.0.12 Screen Recording report from the other side.
- **States now**, read through the bridge: `preview` (Screen Recording) **allowed**,
  `focus` (Accessibility / Automation) **not asked**.
  - Caveat: this Dev binary was started from the agent's shell, so macOS attributes
    Screen Recording to the app that launched it.
- Nothing was granted.

### Noted

- Settings › Data says "0 sessions · 0 history events · 0 indexed lines · transcripts 0
  KB" for accounts with hundreds of MB of transcripts. It is true of AI Juggler's own
  records, but reads as if the account were empty.
- Refused calls leave "Uncaught (in promise) {code: read_only_bridge}" in the console.
  The UI doesn't catch rejected writes; harmless here, but the same code paths would
  show unhandled errors in the app.

### OK

- Light theme (⌘⇧L) and dark both render. Appearance defaults to Dark when nothing is
  stored (`theme.ts`, by design).
- At 760 px the rail collapses to icons, with no horizontal overflow.
- ⌘K focuses the session search; Ctrl+D cycles the density.
- This Mac's rows carry the right accounts. The biggest Claude app instance is Account
  E's (10 live CLIs; `ps` 7.6 GB, the app "about 7.4–8.1 GB").
- The CPU total (11% of all cores) is the sum of its rows (333% + 4% + 2% over 30 cores).

## Batch 7 — the rest of the bridge walk

Same setup as batch 6, the same Dev data folder restored, 17:45–17:52.

- **Snapshot**: no path added or removed.
  - One config moved: `~/.claude/.claude.json` (+69 bytes at 17:50:00), with its
    `backups/` and its settings sync.
  - That folder writes `.claude.json` near :50 every hour (14:49:39, 16:50:24, 17:50:00);
    its sync runs at :07 and :50 (15:07, 15:50, 17:07, 17:50). Account A has 4 live
    sessions there. The walk only read and launched nothing, so this is not the walk's.
  - `~/.codex`: the live Codex.
- **The bridge log**: no refused call reached the bridge. The UI refuses writes
  before sending them.

Captures: [Account E's view](2026-10-21-real-readonly/c1-account-e-view.jpg) ·
[New session](2026-10-21-real-readonly/c2-new-session.jpg).

### P26 — New session ranks accounts by raw IDs, and says every one "exhausts ≈0 min" — high

New session, opened and closed with Escape; nothing was launched.

- **Saw**:
  - "Ranked for you" lists `claude-code-36858693000` … `codex-36946782000`: the internal
    account IDs, not Account A–G.
  - Each row says "exhausts ≈0 min · Estimate".
  - "Which account?" shows Account A as "0% Estimate · Launch settings: Main · Room left
    unknown". That is "0%" and "unknown" in one row.
- **Cause**: `ui/src/components/RankedRows.tsx` prints `row.account` (the id, around line
  73). It guards with `row.tte_min !== undefined`, which a serde null passes, so
  `Math.round(null)` gives 0 (around line 80). This is P18's null class again.
  `SwitchTargetRows.tsx:129` has the same `exhausts ≈` line.
- **Also**:
  - The six folder choices at the top are mostly removed or pruned worktrees (P9).
  - It says "1 prior sessions".
- **Expected**: account names; no time-to-empty without a measured rate; one statement
  of room left.

### P27 — An account's own view says "0% · Estimate · last measured 1m ago" with nothing measured — high

Account E, opened with "Inspect Claude Code":

- **Saw**:
  - The rings read "0% · 5-hour window · Estimate · resets 17:47 · last measured 1m ago"
    and "0% · weekly window · Estimate".
  - The backend has `percentage: null`, and the Accounts card for the same account says
    "100% left".
  - The ring doesn't say whether 0% is used or left.
- **Also**:
  - "Running sessions (0) · None right now", with 10 live (P22).
  - "Usual projects" lists 10 rows, mostly removed worktrees, each tagged "usual".
  - A stray "reset: not known yet" sits under the live-preview checkbox.
  - Its Usage, History and Processes tabs are empty, as in P21. Usage says "No usage
    measured for this yet" while the backend holds 13.7 G tokens this week for it.
- **Code**: `ui/src/components/QuotaRings.tsx:229` (`percentage !== undefined`).

### P28 — The same launch settings are "Default" in one place and "Main" in two — low (D-25)

- Accounts › Edit launch settings names the one set "Default".
- A project's Actions tab and New session call it "Main".

### P29 — Form controls without names — medium (a11y)

On Account E's view:

- The three checkboxes have the accessible name "on".
- The flat-fee field is named "200", its placeholder. The placeholder also reads like an
  entered fee.
- The renewal date field is named, but its value field isn't labelled.
- The 10 "usual project" buttons have no name at all.

### P30 — Developer words in Accounts › "Which switches between accounts are proven (Beta)" — low

- "All 0 tool pairs ship in beta — none validated yet … a pair graduates only when its CI
  row is green AND one live dogfood cell passed on real binaries", then columns Mechanism,
  Ceiling, CI fixtures, Live cell.
- "0 tool pairs" is the bridge: `validation_matrix` isn't served. The wording is the app's.

### Also seen

- **Accounts**: Account B's "This week … resets Wed 05:48 PM" comes before its
  "5-hour window … resets Wed 05:49 PM" (P12).
- **Account menu**: Rename…, Make the default, Remove…. Read, not clicked.
- **Cleanup**: consistent (only what AI Juggler started); the filter name "Re-adopted" is
  jargon.
- **Fleet**: reads fine. Its "Loading…" is the bridge refusing the fleet calls.
- **Rail Filters**: the Project list repeats every removed worktree (P9). "Sort: Quota
  risk (quota only)" says quota twice.
- **The indicator legend** is clear. Its "APP · runs in the Claude desktop app" is what
  the 16 live desktop sessions would show if they were listed (P2).

### Process: a redaction miss, caught and fixed

- Four captures of batch 6 (b3, b6, b7, b8) were committed and pushed without their
  boxes, on the first walk branch (`claude/qa-real-walk-be289a`, commit `06a2f0bd`).
  - The box list reached the old scratch tool as one unsplit argument (zsh does not
    word-split `$VAR`), and that tool wrote the image unchanged.
  - The rail's project names were visible. Account labels and paths were not.
  - The two private project names shown were already public on `main`, in
    `ui/tests/visual/rail-projects.spec.ts` (since #84), but that doesn't excuse it.
- **The fix**:
  - The tip of that branch was fixed in `9442862f`.
  - This branch (`claude/qa-real-walk-clean`) is a fresh history from `main`, without the
    bad images.
  - The old branch is kept until the maintainer decides.
- **So it can't happen again**:
  - [`scripts/qa/redact_capture.swift`](../../../../scripts/qa/redact_capture.swift)
    fails closed. With no box, an unparsable or out-of-image box, or an unknown argument,
    it writes nothing and exits non-zero.
  - It writes `<image>.redacted.json`: the output's SHA-256, the boxes, and
    `checked_by_eye: false`.
  - [`scripts/qa/check_captures.py`](../../../../scripts/qa/check_captures.py) (in the
    evidence gate, so `just verify` and CI) refuses any image under
    `docs/qa/evidence/walks/` that lacks a sidecar whose hash matches, that has no boxes
    and no `no_real_data` reason, or that is not confirmed as checked by eye.
    `--staged` runs it as a pre-commit hook.
  - Every walk image here (12) and in the 0.0.11 walk (23 fixture captures) was opened,
    looked at, and confirmed.
  - The rule: open every image before it's staged or pushed.

## Batch 8 — the oracle against #120's head (`751fddde`, before it merges)

### How

- The harness was built on #120's head (copied in for the build; #120 doesn't carry it)
  and run Finder-like on a fresh scratch data folder.
- `real_truth.py --compare` now sums every token class, reasoning included: disjoint
  classes sum to the tool's own total.
- The compare reports Codex's missing 5-hour window as OK, and a window with no percent
  and no reset as OK.
- **Snapshot**: no config file changed, no path added or removed. The changes were
  `~/.codex` (the live Codex) and one profile's managed-settings sync, its own schedule
  as before.
- **Before**: main `e2abd09f` (the batch 6 run, 17:2x): 15 OK, 32 MISMATCH.
  **#120**: 38 OK, 10 MISMATCH.

### Fixed by #120

- Claude tokens per account: ×0.98–1.02 in every 5 h and 7 d window, against ×1.7–2.5
  on main (P3).
- Codex tokens: ×1.01, against ×2.00 (P14).
- No made-up reset times: every unmeasured window has no percent and no reset (P12).
- Codex weekly percent: 74%, the same as the newest rollout reading (P1).
- Codex has no 5-hour window (P13).
- Cost rows per session: 280 of 283 within 5% of the transcripts, median ×1.00. On
  main it was 1 of 283, median ×1.94.
- 155 of 456 cost rows are priced (0 on main). `api_equivalent_usd` and `total_usd`
  have values.

### Still open after #120

- **Not in #120's scope**:
  - projects and sessions (P2, P9, P10: 16 project rows against 9, sessions 0 against 17
    live);
  - the Codex-only projects.
- **Unpriced models** (301 of 456 rows): `codex-auto-review` (252 rows), `gpt-6.1-sol`
  (28) and `gpt-6-astra` (21). #120's note names other unpriced Codex models; these are
  the ones in use here.
- **The three per-session outliers**:
  - **Two Codex sessions, ×0.84 and ×0.93**: P17's pre-turn `token_count` after
    `compacted`, as expected.
  - **One Claude session, ×0.40**: not a count error. 58% of its messages also sit in
    another transcript of the same account (a resumed copy). The backend counts each
    message once and credits it to the other file, so the account totals stay exact
    (×1.00). Worth a decision on which session a copied message belongs to: the
    earliest is the honest choice.
- **P16, larger than it looked**: This Mac's tool memory reads 11.6 GB against
  26.9 GB of `ps` RSS for the Claude app trees.
  - The largest tree (17.2 GB at the time) isn't in the top rows at all.
  - Afterwards the largest tree held a `next-server` dev server of 5.1 GB, started
    by a session inside the Claude app.
  - The collector misses session-spawned descendants, which are what "See what's using
    it" should show.
  - On main the same tree read 6.0 against 7.7 GB, so this isn't #120's regression.
- **Cost payload**: `flat_fee_usd` and `value_ratio` are still null (no fee entered,
  honest), and so are the unpriced rows' `usd`. #120 carries the UI null guards (P11,
  P18); the UI side is for the bridge walk to re-check after the merge.

### The compare output on #120 (redacted)

```text
OK        setups found: 7 of 7
OK        brought in where they are: 7 of 7
MISMATCH  projects: backend 16 rows, oracle 9
MISMATCH  Project 1: backend 4 conversations in 2 row(s) (1 split off as their own projects), accounts ['Account A']; oracle 4 Claude + 242 Codex, accounts ['Account A', 'Account G']
MISMATCH  Project 2: backend 14 conversations in 9 row(s) (8 split off as their own projects), accounts ['Account E']; oracle 14 Claude + 0 Codex, accounts ['Account E']
OK        Project 3: backend 13 conversations in 1 row(s), accounts ['Account A']; oracle 13 Claude + 0 Codex, accounts ['Account A']
MISMATCH  Project 4: backend 0 conversations in 0 row(s), accounts -; oracle 0 Claude + 11 Codex, accounts ['Account G']
OK        Project 5: backend 6 conversations in 1 row(s), accounts ['Account A', 'Account B', 'Account C', 'Account D', 'Account E', 'Account F']; oracle 6 Claude + 0 Codex, accounts ['Account A', 'Account B', 'Account C', 'Account D', 'Account E', 'Account F']
OK        Project 6: backend 3 conversations in 1 row(s), accounts ['Account A', 'Account F']; oracle 3 Claude + 0 Codex, accounts ['Account A', 'Account F']
OK        Project 7: backend 3 conversations in 1 row(s), accounts ['Account B', 'Account C']; oracle 3 Claude + 0 Codex, accounts ['Account B', 'Account C']
OK        Project 8: backend 1 conversations in 1 row(s), accounts ['Account D']; oracle 1 Claude + 0 Codex, accounts ['Account D']
MISMATCH  Project 9: backend 0 conversations in 0 row(s), accounts -; oracle 0 Claude + 1 Codex, accounts ['Account G']
MISMATCH  sessions listed: backend 0; running 17 CLI sessions + 2 active Codex threads
OK        Account A 5h: no percent and no reset time (nothing measured)
OK        Account A 5h tokens: backend 304,149,155, oracle 299,004,847 (×1.02)
OK        Account A 7d: no percent and no reset time (nothing measured)
OK        Account A 7d tokens: backend 9,638,316,599, oracle 9,637,849,485 (×1.00)
OK        Account B 5h: no percent and no reset time (nothing measured)
OK        Account B 5h tokens: backend 152,938,939, oracle 155,434,508 (×0.98)
OK        Account B 7d: no percent and no reset time (nothing measured)
OK        Account B 7d tokens: backend 7,041,404,915, oracle 7,043,900,484 (×1.00)
OK        Account C 5h: no percent and no reset time (nothing measured)
OK        Account C 5h tokens: backend 471,010,337, oracle 472,176,078 (×1.00)
OK        Account C 7d: no percent and no reset time (nothing measured)
OK        Account C 7d tokens: backend 8,034,614,340, oracle 8,035,413,501 (×1.00)
OK        Account D 5h: no percent and no reset time (nothing measured)
OK        Account D 5h tokens: backend 492,282,927, oracle 492,282,927 (×1.00)
OK        Account D 7d: no percent and no reset time (nothing measured)
OK        Account D 7d tokens: backend 12,642,249,800, oracle 12,642,249,800 (×1.00)
OK        Account E 5h: no percent and no reset time (nothing measured)
OK        Account E 5h tokens: backend 1,159,187,206, oracle 1,150,526,346 (×1.01)
OK        Account E 7d: no percent and no reset time (nothing measured)
OK        Account E 7d tokens: backend 6,942,014,884, oracle 6,946,202,073 (×1.00)
OK        Account F 5h: no percent and no reset time (nothing measured)
OK        Account F 5h tokens: backend 0, oracle 0 (×0.00)
OK        Account F 7d: no percent and no reset time (nothing measured)
OK        Account F 7d tokens: backend 2,498,492,913, oracle 2,498,492,913 (×1.00)
OK        Account G 5-hour window: none, as Codex reports (secondary null)
OK        Account G weekly: backend 74.0; rollouts say 74.0% used, resets Tue 2026-10-27 02:33
OK        Account G 7d tokens: backend 8,025,074,783, oracle 7,929,622,129 (×1.01)
INFO      cost payload: null flat_fee_usd, value_ratio; 456 of 456 rows with null credits/usd (honest nulls; main's CostMatrix.tsx crashes on them until its guards check for null)
MISMATCH  cost rows per session: 283 sessions in both; 280 within 5% of the transcripts; ratio min ×0.40, median ×1.00, max ×1.03; 0 only in the backend, 2 only in the transcripts; 8 still being written during the run, left out
MISMATCH  cost rows priced: 155 of 456
OK        This Mac machine: CPU 96%, memory 84.7 of 103 GB (app_wired_compressed), pressure warning; tools 5
OK        This Mac tools: backend 5; ps has 5 Claude app instances + 1 ChatGPT (Codex) not counted by design
MISMATCH  This Mac tools memory: backend 11.6 GB; ps RSS of the Claude app trees 26.9 GB (×0.43)
MISMATCH  This Mac top rows (GB): backend [3.8, 3.0, 2.0]; ps trees [17.2, 3.8, 3.0]; 2 instance(s) not shown
OK        This Mac tools CPU: backend 6.6% of all cores (ps %CPU is a decaying average, not compared)
OK        alerts over the run: 1 (memory_pressure)
```

## Batch 9 — AJ-E1529 (sessions and projects started outside AI Juggler) on `release/0.1` `cc1e9321`

### How

- One run on `release/0.1` (which carries the oracle at `580521ce`): the harness, read
  only, Finder-like, on a fresh data folder.
- **Overall**: 26 OK, 35 MISMATCH, 2 INFO. The token, reset, Codex-window and cost lines
  belong to #120 and AJ-E1530, which aren't on this branch.
- **Before trusting a green result**, I reviewed the driver's oracle changes:
  - resolved folders: no effect here;
  - a Codex CLI matched by its folder: fine;
  - a Codex thread counted once: fine.
- **One gap closed in the oracle** (`5edc0070`): an unattributable live session now fails,
  instead of dropping out of both sides. Temp-folder stand-ins are left out as INFO.

### The acceptance lines (redacted)

```text
OK        setups found: 7 of 7
OK        brought in where they are: 7 of 7
OK        projects: backend 9 rows, oracle 9
MISMATCH  Project 1: backend 252 conversations in 1 row(s), accounts ['Account A', 'Account G']; oracle 4 Claude + 249 Codex, accounts ['Account A', 'Account G']
OK        Project 2: backend 14 conversations in 1 row(s), accounts ['Account E']; oracle 14 Claude + 0 Codex, accounts ['Account E']
OK        Project 3: backend 13 conversations in 1 row(s), accounts ['Account A']; oracle 13 Claude + 0 Codex, accounts ['Account A']
OK        Project 4: backend 11 conversations in 1 row(s), accounts ['Account G']; oracle 0 Claude + 11 Codex, accounts ['Account G']
OK        Project 5: backend 6 conversations in 1 row(s), accounts ['Account A', 'Account B', 'Account C', 'Account D', 'Account E', 'Account F']; oracle 6 Claude + 0 Codex, accounts ['Account A', 'Account B', 'Account C', 'Account D', 'Account E', 'Account F']
OK        Project 6: backend 3 conversations in 1 row(s), accounts ['Account A', 'Account F']; oracle 3 Claude + 0 Codex, accounts ['Account A', 'Account F']
OK        Project 7: backend 3 conversations in 1 row(s), accounts ['Account B', 'Account C']; oracle 3 Claude + 0 Codex, accounts ['Account B', 'Account C']
OK        Project 8: backend 1 conversations in 1 row(s), accounts ['Account D']; oracle 1 Claude + 0 Codex, accounts ['Account D']
OK        Project 9: backend 1 conversations in 1 row(s), accounts ['Account G']; oracle 0 Claude + 1 Codex, accounts ['Account G']
OK        Account A conversations: backend 20, oracle 20
MISMATCH  Account A live: backend 0, running 4
OK        Account B conversations: backend 3, oracle 3
MISMATCH  Account B live: backend 0, running 1
OK        Account C conversations: backend 2, oracle 2
MISMATCH  Account C live: backend 0, running 1
OK        Account D conversations: backend 2, oracle 2
MISMATCH  Account D live: backend 0, running 1
OK        Account E conversations: backend 15, oracle 15
MISMATCH  Account E live: backend 0, running 10
OK        Account F conversations: backend 2, oracle 2
OK        Account F live: backend 0, running 0
MISMATCH  Account G conversations: backend 260, oracle 261
OK        Account G live: backend 0, running 0
OK        live sessions with no account in the oracle: 0
INFO      left out: 8 stand-in CLI(s) run from a temp folder (tests, not sessions)
MISMATCH  cost rows per session: 291 sessions in both; 1 within 5% of the transcripts; ratio min ×1.00, median ×1.94, max ×2.50; 0 only in the backend, 2 only in the transcripts; 7 still being written during the run, left out
```

### What a user would see against what is on disk

- **All right**:
  - all 9 projects, each with its accounts;
  - every account's conversation count (A 20, B 3, C 2, D 2, E 15, F 2);
  - each Codex thread exactly once (260 entries for 260 ids; a13f221b holds).
- **Timing, not a bug**: Account G's 260 against 261, and Project 1's 252 against 253.
  That Codex thread was created 2.9 minutes after the backend read the files.
- **Blocker: not one of the 17 sessions running in the Claude app shows as live.** The
  backend has 0 live on every account, against A 4, B 1, C 1, D 1, E 10.
  - `session_process` (`crates/juggler-adapters/src/claude_conversations.rs:157-162`)
    drops any `claude` whose executable path contains `.app/`.
  - Every Claude-app Code session runs from
    `…/claude-code/<version>/<hash>/claude.app/Contents/MacOS/claude`: 13 under the
    profile instances, 4 under the default app.
  - The desktop app's own executable is `Claude` (capital C), which the name test
    already rejects.
  - The replica passed only because its fakes aren't inside a `.app`. Both a fix and a
    replica case are proposed.

### Snapshot

- No path added or removed. `~/.codex` changed (the live Codex), and two profiles'
  managed-settings syncs ran on their own schedule.
- **One config file**: Account C's `.claude.json` (same size) and a backup at 20:35:12.
  - That's off the profile's 6-hour backup cadence (00:00, 06:00, 12:00, 18:00).
  - Its live session wrote its transcript 76 s before and 66 s after.
  - No probe or scan passes a profile's `CLAUDE_CONFIG_DIR` to `claude`.
  - Attributed to that session: strong, not proven.

### The rail walk: blocked

- `release/0.1` doesn't carry the read-only bridge (#122).
- A local cherry-pick conflicts in the command registry, where the new conversation
  commands would need a class. That is release work, so it's left to the driver.

## Batch 10 — AJ-E1529 re-run on `release/0.1` `c89b49c6` (the live fix)

### How

- One run, read only, Finder-like, on a fresh data folder. The oracle is identical to
  this branch's.
- **Snapshot**: no config file changed, no path added or removed. Only `~/.codex` (the
  live Codex) and one profile's managed-settings sync wrote.
- **Overall**: 34 OK, 28 MISMATCH, 1 INFO. The token, cost and Codex-window lines are
  #120's, not on this branch.

### The acceptance lines (redacted)

```text
OK        setups found: 7 of 7
OK        brought in where they are: 7 of 7
OK        projects: backend 9 rows, oracle 9
OK        Project 1: backend 253 conversations in 1 row(s), accounts ['Account A', 'Account G']; oracle 4 Claude + 249 Codex, accounts ['Account A', 'Account G']
OK        Project 2: backend 14 conversations in 1 row(s), accounts ['Account E']; oracle 14 Claude + 0 Codex, accounts ['Account E']
OK        Project 3: backend 13 conversations in 1 row(s), accounts ['Account A']; oracle 13 Claude + 0 Codex, accounts ['Account A']
OK        Project 4: backend 11 conversations in 1 row(s), accounts ['Account G']; oracle 0 Claude + 11 Codex, accounts ['Account G']
OK        Project 5: backend 6 conversations in 1 row(s), accounts ['Account A', 'Account B', 'Account C', 'Account D', 'Account E', 'Account F']; oracle 6 Claude + 0 Codex, accounts ['Account A', 'Account B', 'Account C', 'Account D', 'Account E', 'Account F']
OK        Project 6: backend 3 conversations in 1 row(s), accounts ['Account A', 'Account F']; oracle 3 Claude + 0 Codex, accounts ['Account A', 'Account F']
OK        Project 7: backend 3 conversations in 1 row(s), accounts ['Account B', 'Account C']; oracle 3 Claude + 0 Codex, accounts ['Account B', 'Account C']
OK        Project 8: backend 1 conversations in 1 row(s), accounts ['Account D']; oracle 1 Claude + 0 Codex, accounts ['Account D']
OK        Project 9: backend 1 conversations in 1 row(s), accounts ['Account G']; oracle 0 Claude + 1 Codex, accounts ['Account G']
OK        Account A conversations: backend 20, oracle 20
MISMATCH  Account A live: backend 3, running 4
OK        Account B conversations: backend 3, oracle 3
OK        Account B live: backend 1, running 1
OK        Account C conversations: backend 2, oracle 2
OK        Account C live: backend 1, running 1
OK        Account D conversations: backend 2, oracle 2
OK        Account D live: backend 1, running 1
OK        Account E conversations: backend 15, oracle 15
OK        Account E live: backend 10, running 10
OK        Account F conversations: backend 2, oracle 2
OK        Account F live: backend 0, running 0
OK        Account G conversations: backend 261, oracle 261
OK        Account G live: backend 0, running 0
OK        live sessions with no account in the oracle: 0
MISMATCH  cost rows per session: 291 sessions in both; 1 within 5% of the transcripts; ratio min ×1.00, median ×1.94, max ×2.50; 0 only in the backend, 2 only in the transcripts; 7 still being written during the run, left out
```

### Result

- **The `.app/` fix works**: 16 of the 17 Claude-app sessions are live, on the right
  account (through `CLAUDE_CONFIG_DIR`, or the `~/.claude` fallback).
- **Everything else matches the disk**: projects, conversations and Codex threads.
- **One miss: Account A, 3 live against 4.**
  - `value_after` (`crates/juggler-adapter/src/conversation.rs:91-95`) reads only
    `--resume <id>` as two tokens. The Claude app passes `--resume=<id>`, so no
    Claude-app session's resume id is read.
  - Each falls back to "the newest conversation in its folder newer than the session's
    start". That's right for 16. The 17th has been idle about 34 h, in a folder with 8
    conversations, so nothing there is newer than its start.
  - Fix proposed: accept `--flag=value` too, plus a replica case.
- **The rail walk**: still waiting for #122 on `release/0.1`.

## Batch 11 — P16 re-read: part of it was the harness itself

- **The cause**:
  - The app leaves its own process tree out of This Mac, which is right.
  - The harness is the app here, and it runs inside a Claude app session (this QA
    session's), so its own memory sat in that tree on the `ps` side only.
  - On main `5675de90` that was the "missing" 2.2 GB: a 2.5 GB process three levels
    under `claude`, through a shell, gone after the run.
- **The fix**: the harness now records its own pid (`qa_pid`), plus each process's
  elapsed time and command name in the `ps` sample (scratch only). The compare leaves
  the harness's subtree out, and takes the desktop apps' main processes from the run's
  own snapshot.
- **Re-run** (the clean branch, 21:1x): This Mac tools memory 17.6 against 17.6 GB
  (×1.00); top rows [9.3, 4.7, 1.9] against [9.3, 4.7, 1.9].
- **Still unexplained**: #120's run (751fddde), where a 17.2 GB tree of 178 processes,
  without the QA session in it, was counted at about 2 GB. The new names and start
  times make a repeat identifiable.
- **Snapshot**: one profile's `.claude.json` (same size) and a backup. That's this QA
  session's own profile (Account E, 10 live sessions), whose backups run every few
  minutes (21:01, 21:11, 21:17) and whose transcripts were written within seconds.

## Batch 12 — #126 (P16, P6, P17, shared messages): before and after

Both scored with the same compare (`7673dd07`). It credits a message a resumed copy repeats
to the session with the earliest first message, as #126 does, and counts This Mac's top
rows plus `others` against the `ps` trees. Read only; both snapshots clean (only the live
Codex changed).

| | main `5675de90` (before) | #126 `a7fa83fc` (after) |
|---|---|---|
| Overall | 35 OK, 13 MISMATCH | 42 OK, 7 MISMATCH |
| Cost rows per session | 285 of 289 within 5%; min ×0.40, max ×12.49 | 292 of 292 within 5%; ×1.00–1.03 |
| This Mac top rows | [7.6, 4.9, 1.6] against [7.7, 7.1, 1.6] (the −2.2 GB was the harness) | [6.3, 3.8, 2.0] against [6.3, 3.8, 2.0] |
| This Mac every tree | 3 rows, no "others" | 5 = 3 + 2 others, sizes equal to `ps` |
| Unpriced models | codex-auto-review, gpt-6.1-sol, gpt-6-astra | codex-auto-review (258 rows) |

- **Before, explained**: ×0.40 was the original conversation, and ×12.49 was its resumed
  copy credited with the original's messages. The two Codex sessions at ×0.84 and ×0.93
  were missing their compacted pre-turn usage.
- **Left on #126, not its scope**: projects and sessions (AJ-E1529, on `release/0.1`),
  `codex-auto-review` unpriced, and honest null fee fields (INFO).
- **The UI's "and N more"** is for the bridge walk on a build with #122.

## Batch 13 — AJ-E1529 accepted on the real setup (`release/0.1` `3542530d`), and the rail walk

### The real-data pass

- One harness run, read only, Finder-like, into a fresh `AI Juggler Dev` folder (the
  same folder then seeded the bridge walk).
- **Overall**: 60 OK, 2 MISMATCH, 1 INFO. The two MISMATCH lines are #126's
  shared-message credit (not on this branch yet) and one unpriced model
  (`codex-auto-review`).
- **Snapshot**: no path added or removed.
  - `~/.claude/.claude.json` (same size) and a backup, one second after that account's
    scheduled settings sync (00:07:36). Its syncs run at :07 and :50, and it has 4 live
    sessions: Claude Code's own write.
  - `~/.codex`: the live Codex.
  - During the walk: the live Codex, and one profile's own sync.

```text
OK        setups found: 7 of 7
OK        brought in where they are: 7 of 7
OK        projects: backend 9 rows, oracle 9
OK        Project 1: backend 250 conversations in 1 row(s), accounts ['Account A', 'Account G']; oracle 4 Claude + 246 Codex, accounts ['Account A', 'Account G']
OK        Project 2: backend 14 conversations in 1 row(s), accounts ['Account E']; oracle 14 Claude + 0 Codex, accounts ['Account E']
OK        Project 3: backend 13 conversations in 1 row(s), accounts ['Account A']; oracle 13 Claude + 0 Codex, accounts ['Account A']
OK        Project 4: backend 11 conversations in 1 row(s), accounts ['Account G']; oracle 0 Claude + 11 Codex, accounts ['Account G']
OK        Project 5: backend 6 conversations in 1 row(s), accounts ['Account A', 'Account B', 'Account C', 'Account D', 'Account E', 'Account F']; oracle 6 Claude + 0 Codex, accounts ['Account A', 'Account B', 'Account C', 'Account D', 'Account E', 'Account F']
OK        Project 6: backend 3 conversations in 1 row(s), accounts ['Account A', 'Account F']; oracle 3 Claude + 0 Codex, accounts ['Account A', 'Account F']
OK        Project 7: backend 3 conversations in 1 row(s), accounts ['Account B', 'Account C']; oracle 3 Claude + 0 Codex, accounts ['Account B', 'Account C']
OK        Project 8: backend 1 conversations in 1 row(s), accounts ['Account D']; oracle 1 Claude + 0 Codex, accounts ['Account D']
OK        Project 9: backend 1 conversations in 1 row(s), accounts ['Account G']; oracle 0 Claude + 1 Codex, accounts ['Account G']
OK        Account A conversations: backend 20, oracle 20
OK        Account A live: backend 4, running 4
OK        Account B conversations: backend 3, oracle 3
OK        Account B live: backend 1, running 1
OK        Account C conversations: backend 2, oracle 2
OK        Account C live: backend 1, running 1
OK        Account D conversations: backend 2, oracle 2
OK        Account D live: backend 1, running 1
OK        Account E conversations: backend 15, oracle 15
OK        Account E live: backend 10, running 10
OK        Account F conversations: backend 2, oracle 2
OK        Account F live: backend 0, running 0
OK        Account G conversations: backend 258, oracle 258
OK        Account G live: backend 0, running 0
OK        live sessions with no account in the oracle: 0
OK        Account A 5h tokens: backend 269,974,696, oracle 273,923,349 (×0.99)
OK        Account A 7d tokens: backend 9,913,952,216, oracle 9,913,952,216 (×1.00)
OK        Account B 5h tokens: backend 231,733,657, oracle 230,287,110 (×1.01)
OK        Account B 7d tokens: backend 7,134,748,941, oracle 7,133,302,394 (×1.00)
OK        Account C 5h tokens: backend 456,391,876, oracle 463,101,019 (×0.99)
OK        Account C 7d tokens: backend 8,295,952,217, oracle 8,295,395,704 (×1.00)
OK        Account D 5h tokens: backend 14,765,003, oracle 14,765,003 (×1.00)
OK        Account D 7d tokens: backend 12,496,996,247, oracle 12,496,996,247 (×1.00)
OK        Account E 5h tokens: backend 720,813,464, oracle 726,468,681 (×0.99)
OK        Account E 7d tokens: backend 7,775,756,750, oracle 7,774,790,274 (×1.00)
OK        Account F 5h tokens: backend 0, oracle 0 (×0.00)
OK        Account F 7d tokens: backend 2,498,492,913, oracle 2,498,492,913 (×1.00)
OK        Account G 7d tokens: backend 8,310,727,296, oracle 8,211,666,536 (×1.01)
MISMATCH  cost rows per session: 289 sessions in both; 285 within 5% of the transcripts; ratio min ×0.40, median ×1.00, max ×12.49; 0 only in the backend, 2 only in the transcripts; 6 still being written during the run, left out
```

**Accepted**: setups, projects, conversations and live sessions all match the disk. That
includes A 4 of 4 live (the `--resume=<id>` fix), and each Codex thread exactly once.

### The rail through the bridge

[Home with the rail](2026-10-21-real-readonly/d1-home-running.jpg). Conversation titles and
project names are blacked out (the fail-closed tool, opened and confirmed by eye).

- **Can you find every session?** Yes, once indexed.
  - The rail says "Running 17", the same 17 the oracle sees, grouped under their
    projects. Each row reads "Running · outside AI Juggler" with its age.
  - Home's Running list shows the same 17 with the account written out.
- **Is each one's account obvious?**
  - On Home, yes: the label in full.
  - In the rail, only a two-letter chip in the account's colour (the full "on Account D"
    is in the accessible name). With real labels, several sharing the first word would
    give near-identical chips. Low.
- **First launch on a heavy setup: no live sessions for about 5 minutes.**
  - The first index pass reads the whole history (25 → 48 → 232 → 300 conversations over
    about 4 minutes) and sets liveness only when the pass ends.
  - Until then the rail and Home say nothing is running, while 17 are.
  - Medium. Suggestion: set liveness first (from the process table), or after each batch.
- **Home contradicts itself.**
  - The subtitle says "Account G has 6% left — start new work on another account".
  - The Runway footer says "Best for new work: Account G, 6% left".
  - G (Codex, 94% used: true, from its rollouts) is the only account with a measured
    percent. The Claude accounts say "Not measured yet · N tokens used" (honest), so
    "best" picks the one that's nearly out.
  - High: an account at 6% shouldn't be recommended, and "best" should say it can't tell
    when the others aren't measured.

### The oracle

This Mac lines are INFO on a replica (a HOME that isn't the user's own), since they count the
whole Mac. With the real HOME they're unchanged.

## Batch 14 — batch 13's two fixes re-checked (`release/0.1` `e385f5b9`), and a false "can't start"

### How

- The Dev build with the bridge, Finder-like, on a copy of batch 13's Dev data folder.
  The conversation index lives in memory, so every launch is a first pass.
- A poller read `conversations_list` once a second through the bridge (read only), against
  the oracle's 16 live sessions.
- **Snapshot**: no config file changed, no path added or removed. Only the live Codex and
  one profile's own sync wrote.

### Home: fixed

[Home](2026-10-21-real-readonly/d2-home-after-fix.jpg), titles and project names blacked out.

- **Subtitle**: "16 running · Account G has 0% left", with no "start new work on…".
- **Runway footer**: "Account G has 0% left, and 6 others aren't measured yet, so AI
  Juggler can't tell which has room."
- G (Codex) is 100% used per its newest rollout reading, so 0% is true.
- **Minor**: the page title says "All clear — nothing needs you" while an account has run
  out.

### Live sessions on a first launch: better, but not "within seconds"

| Launch | `conversations_list` | First live | All 16 live |
|---|---|---|---|
| 1 (the data copy, first opened by this build) | no answer (5 s timeouts) for about 94 s | 94.5 s | 121.7 s |
| 2 | answered at once, but 1 conversation and 0 live until 73.9 s | 73.9 s | 95.1 s |

- Before the fix it was about 5 minutes. Once reading starts, newest-first works: from
  the first live session to all 16 in about 21 s, and the list grows from 3 to 216
  conversations meanwhile.
- **The stall is before reading starts**: about 74–94 s in which the index reads
  nothing. In launch 1 `conversations_list` itself didn't answer.
- This points at startup contention, likely the usage pipeline's first full ingest holding
  storage while the conversation tick waits for the account list. Medium.

### A false "AI Juggler Dev can't start" after a forced quit — high

[The alert](2026-10-21-real-readonly/e1-cant-start-after-kill.jpg).

- **What happened**: the first instance was ended with SIGTERM, and I waited until no
  process was left. A relaunch seconds later stopped at a system alert: "Another AI
  Juggler Dev is already open with this data and isn't answering… Switch to it and quit
  it … then Force Quit."
- **No other instance existed.** The launch sat in that alert with 0% CPU for 7+ minutes,
  never retrying (`run → not_answering → newer_data::cannot_start`).
- **Cause**: `hold_instance_lock` (`newer_data.rs:481-500`) found `juggler.lock` locked
  (`WouldBlock`). A few minutes later the lock was free (checked with a non-blocking
  lock, released at once), and the next launch started normally. So the old instance's
  lock was still held for a short time after its process was gone, and the new launch
  treated that as "open and not answering".
- **Two more problems with the alert**:
  - Its advice can't be followed: there's nothing to switch to or force quit.
  - It outlives the process: it's a `CFUserNotification`, owned by the system, still on
    screen after the Dev app exited. Its one button, Quit, only dismisses it.
- **For a user**, this is what a crash or a force quit followed by a quick relaunch looks
  like.
- **Expected**: when the hand-over finds no live peer, wait briefly and retry the lock
  (it frees within moments), then start. Only say "isn't answering" when a process
  actually holds it.

## Batch 15 — batch 14's fixes re-checked (`release/0.1` `8293c0f3`)

Three launches of the Dev build with the bridge, Finder-like, on a copy of batch 13's data.
The 1-second poller ran against the oracle's 16 live sessions.

- **Snapshot**: no path added or removed. One profile's `.claude.json` (same size) and a
  backup: Account E, this QA session's own profile, saving every few minutes (03:01,
  03:11, 03:17). Otherwise the live Codex and that profile's sync.

### Fixed

- **Relaunch after a forced quit**: after SIGTERM, I relaunched 0.02 s after the old
  process was gone. It started in about 2 s, with no alert.
- **Home**: the headline is "Account G has run out", not "All clear". The footer still
  reads "Account G has 0% left, and 6 others aren't measured yet, so AI Juggler can't
  tell which has room." G is 100% used (true).
- **`conversations_list` answers at once** during the first pass (no 5 s timeouts).

### Still slow: about 72–75 s before anything is read

| Launch | Nothing read until | First live | All 16 live |
|---|---|---|---|
| 1 (data copy opened by this build) | 75 s | 78.1 s | 97.3 s |
| 2 (SIGTERM, immediate relaunch) | 72.8 s | 72.8 s | 95.0 s |

- **The cause, from a thread sample 20 s into launch 3**:
  - The conversation tick's thread (`spawn_read_model_ticks`, the conversations closure)
    is busy in `codex_transcript::parse_rollouts → parse_rollout → message_text`, parsing
    the message text of every Codex rollout: 6.6 GB in 265 files here.
  - Nothing else is indexed until it's done.
  - Meanwhile the This Mac emit and the 2 s process tick wait on a mutex.
- **Expected**: read only each rollout's head (its `session_meta`) to find, order and
  date the threads, and work out liveness from that. The full text (titles) can come
  later, newest first, or on demand.

## Not walked (blocked by W0, before the bridge)

These need the window:

- Bring in 7 accounts in place.
- The rail and History against the table above.
- Runway, Accounts, Usage and the inspector against the table.
- This Mac: panel, alerts, banner, "See what's using it".
- Settings and permission states.
- Every screen and disclosure, both themes, the narrow window, the keyboard.

The ground truth above is the reference for each.
