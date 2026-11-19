# F-rc3c-3 → 0.5: each App on its own sign-in's Account (a folder that holds two sign-ins)

For rel-05. Main's ruling (2026-11-19): "0.5: a per-App Account needs a
design that reconciles D-63." Found on the 0.4.6 rc3c walk (real HOME,
Dev data folder). rel-05 gives it its task number (E32 is the Account
and App epic: `docs/tasks/E32-account-and-app.md`).

## The problem, in plain words

A Claude app profile and its Claude Code CLI can share one Claude Code
folder: AI Juggler starts the profile with `CLAUDE_CONFIG_DIR` set to the
CLI's folder (0.3.5). That folder then holds **two sign-ins**: the CLI's
(what a terminal `claude` there uses) and the Claude app's own (what the
app's Code tab uses). They can be different Accounts. On the sanctioned
test profile aij-test, the app is signed in to the personal Max Account
and the CLI to the Team Account, on the same email.

AI Juggler stores one Account per **folder**, read from one file that
both programs write. So the Account flips to whichever program ran last.

What the user saw (rc3c, 2026-11-18):

1. Before: Home's Runway put both aij-test Apps ("Claude app · aij-test",
   "Claude Code CLI · aij-test") under the Team Account (93% left), with
   the default Claude app.
2. The aij-test Claude app was started from AI Juggler and a Code
   conversation was picked in its sidebar. Within a minute both aij-test
   Apps moved to the personal Max Account, which is out of weekly room
   until 07:00, and the rail marked both Out. No `account_audit` row or
   notice recorded the move. The link stayed after the app was quit.
3. The app's own UI said "… · Max". A terminal `claude --resume` that AI
   Juggler ran in the same folder minutes earlier said "Claude Team" in
   its banner. Both are right: one folder, two sign-ins.
4. (Fixed in 0.4.6, acadb1b67: the default Claude app also wore an Out
   mark, because the rail's mark read the Accounts once and the readings
   every 30 seconds.)

**What is wrong for the user:** the CLI App is shown on an Account it
doesn't use, and is marked Out while its terminal would run on the
Team Account. Resume on a terminal session there is greyed as "out of
room" for the wrong reason. The desktop App is right only by accident
(after the app ran). After the next terminal run it is wrong the other
way. Nothing tells the user that the folder moved, or why.

## Evidence (read-only, the Dev database)

- `accounts`: folder `claude-code-20253843000` (aij-test, in place) is on
  the personal Max Account (`…5569ee52`), the same as the "Lab" folder
  and the personal-max profile. Before, it was on the Team Account
  (`…35347e41`) with `~/.claude`.
- `quota_samples` for the aij-test folder, source `desktop_app`: 06:19 →
  36%/96% (the personal Max's numbers, the same timestamps as its folders);
  17:27 and 17:38 → 6%/7% (the Team Account's, the same timestamps as
  `~/.claude`); 22:51 and 22:59 → 0%/100% (the personal Max's again).
  Those samples are picked by the folder's `.claude.json` sign-in at read
  time, so the folder's sign-in changed at least three times in one day.
- `instances`: AI Juggler's terminal run in that folder at 22:48:19–22:48:37
  (the "Claude Team" banner). The app's Code tab ran right after it; the
  100% readings start at 22:51.
- `account_audit`: no row for any of the moves.

## Root cause (file:line on release/0.4.6)

1. **One file decides a folder's Account.** `probe_claude_identity`,
   `crates/juggler-adapters/src/claude_code.rs:1269`, reads only
   `oauthAccount.accountUuid` and `organizationUuid` from
   `<folder>/.claude.json`.
2. **Two programs write that file.** `places/start.rs:130` starts a
   profile with `CLAUDE_CONFIG_DIR=<its CLI folder>`. The Code tab runs
   that folder's `claude` with the app's environment
   (`docs/research/desktop-pool-members.md` §2) and the app's own
   sign-in, and that `claude` writes the app's sign-in into `oauthAccount`.
   A terminal `claude` there writes the CLI's.
3. **Every read is taken as final.** `store_accounts`,
   `crates/juggler-app/src/features/subscriptions/mod.rs:894`, upserts the
   Account the read names. Then `pools::sign_in::set_folder`
   (`mod.rs:941` → `crates/juggler-app/src/features/pools/sign_in.rs:48`
   → `crates/juggler-core/src/storage/entities.rs:395`) points
   `accounts.subscription_id` at it. No audit row or notice is written
   (only a pool notice, when open sessions change pools).
4. **The Account belongs to the folder, so every App on the folder moves.**
   `with_apps` (`mod.rs:504`) lists every App whose `folder_id` is the
   folder's.
5. **Readings follow the same file.** `read_claude_records`
   (`crates/juggler-app/src/features/quota/records.rs:155`) takes the
   app instances' meter samples for the sign-in in `.claude.json`
   (`records.rs:202-208`; org filter in
   `crates/juggler-adapters/src/claude_limits.rs:341`). It stores them on
   the folder (`quota_samples.account` = the folder id). An Account's
   room is then the newest reading among its folders
   (`ui/src/lib/subscriptions.ts:194`, `accountDisplay`). After a move,
   for a few minutes, the folder carries its old Account's last reading
   into the new one.
6. **Every consumer reads room by folder.** These include:
   - the rail and Home marks (`ui/src/components/RoomChip.tsx:82`);
   - Resume (`ui/src/pages/session/ResumeButton.tsx:94`, through
     `useTargetRoom`, `ui/src/components/switch/TargetRoom.tsx:19`);
   - switch room and the backend's out-of-room notices;
   - the presence check's out rule (`scripts/qa/presence_sessions.py:330`:
     a folder out puts its whole Account out).

Not new in 0.4.6: the probe dates from 0.1 (8d9b4318d), the profile
started on its CLI folder from 0.3.5 (ad461cf53), and the stored folder
link from 0.4.0 (f2ec31297).

## Why no 0.4.6 mitigation

The cheap idea: keep the stored link when the desktop app wrote last, and
mark it "unclear". It is not small or safe, for three reasons:

1. **Readings.** They follow the same file (root cause 5). A folder kept
   on the Team Account would carry the personal Max's 100% readings
   there. The Team Account would show Out, which is the same wrong mark
   by another road: the default app, the presence check's rule, the
   backend's "out of room" notices and switch room. Gating the readings
   too means changing the records pipeline and the readings already
   stored.
2. **Claude-app-only folders.** Every folder already has a `cli:<folder>`
   App row (the v40 migration), so "a folder held by a desktop App and a
   CLI App" covers the maintainer's normal profiles. If one of those apps
   is signed in to another Account, a "keep the link" rule would leave
   the folder stuck on the old Account. Telling "this folder has its own
   CLI sign-in" apart needs `login_state`'s existence check of the
   folder's Keychain item. Nobody has checked whether the app's `claude`
   creates that item, and it can't be checked without a lab run.
3. **Knowing who wrote last after quit.** Claude Code removes the pid
   files (`sessions/<pid>.json`) on exit (`claude_live.rs:3-6`), so after
   the app quits only the transcripts' `entrypoint` can say who wrote
   last.

Also, an "unclear" state needs a new field through the backend, the
generated types, the marks, Resume and switch. That is M, and it touches
other agents' 0.4.6 areas.

## Design questions (the ruling comes first)

### Q1 — D-63: may AI Juggler know a desktop profile's own sign-in?

D-63 (`docs/review/decision-log.md:2202`) says: "a desktop app's own
sign-in isn't read (its data is the app's)". `docs/qa/0.4-walks/design.md`
item 20 builds on it ("A desktop App's Account isn't named"). But since
0.1.5 (7a84fdf4f) AI Juggler has already read the **names** of an
instance's `claude-code-sessions/<account>/<org>/` folders
(`claude_limits::instance_sign_ins`, `claude_limits.rs:303`; nothing
inside them is opened). It uses them to assign the app's meter samples
and to plan Move (`claude_move.rs:294`).

- **A. Amend D-63: folder names only.** The desktop App's sign-in is the
  `<account>/<org>` of its newest Code record (by `lastActivityAt` or
  mtime). Metadata already read; nothing inside a credential or the app's
  storage is opened.
  - Pro: no new source, and it works while the app is closed.
  - Con: a newly signed-in app has no record until its first Code
    session ("not known yet"). An instance with records under two
    sign-ins needs the newest one.
  - Recommended.
- **B. The meter's `org` only** (`plan-usage-history.json`, also read
  already).
  - Con: it gives the organisation, not the account, so two accounts in
    one organisation can't be told apart. Use it only as a cross-check
    for A.
- **C. Keep D-63. Infer it from `.claude.json` when a `claude-desktop`
  process wrote it** (by transcript `entrypoint`).
  - Pro: no ruling change.
  - Con: it is known only after the app ran a session, it races with the
    CLI's writes, and the pid files vanish on exit. Fragile.
- **D. Keep D-63 strictly.** A desktop App has no Account of its own
  ("as its folder" or "Account unknown"); only CLI Apps are grouped.
  - Pro: simplest.
  - Con: a desktop App can't show its room or Out, and the Runway shows
    desktop Apps apart. This works against the product's point (Account =
    subscription, desktop-first).

### Q2 — How is the CLI's own sign-in known when the app wrote last?

- **a.** A read moves the CLI App only when a CLI wrote it. That means
  the newest transcript line with entrypoint `cli` (or AI Juggler's own
  run) is newer than the newest `claude-desktop` line, before the state
  file's mtime. Otherwise, keep the CLI App's stored Account.
- **b.** First ask whether the folder has a CLI sign-in at all:
  `login_state` (`claude_code.rs:916`) reports whether
  `.credentials.json` or the folder's namespaced Keychain item exists
  (existence only, as today). With none, the CLI App reads "Not signed
  in" (the maintainer's app-only profiles: the CLI's first run was never
  done). **Lab check needed:** does the app's `claude` create that
  Keychain item? Verify on the aij-test lab through AI Juggler's own
  `login_state`, never by reading the Keychain.
- Recommended: b to gate, then a to decide.

**Answer (AJ-E3210, 2026-11-19): b gates, a decides, and only where the
question arises.** By the same standard as Q1 (read nothing new; correct
over clever):
1. Only a folder a desktop App shares is in question. Every other folder
   reads its state file as today, so no CLI-only setup changes.
2. Gate (b): the folder's CLI sign-in, by `login_state`'s existence check
   (`.credentials.json` or the folder's namespaced Keychain item, never
   opened). With none, the CLI App is "Not signed in": the app-only
   profiles' CLI Apps stop wearing the app's Account.
3. A read that names a sign-in no desktop App on the folder is on was
   written by a CLI: the app's `claude` only ever writes the app's own.
   It moves the CLI App.
4. A read that names the desktop App's own sign-in moves the CLI App only
   when the newest `claude` run in the folder was a CLI's (the adapter's
   `sign_in_writer`: a live pid file's `entrypoint`, else the newest
   transcript's last `entrypoint`; both already read for sessions).
   Otherwise the CLI App keeps its Account; one that never had one reads
   "Account unknown", saying the Claude app ran there last.
5. Why not a alone: without the gate an app-only folder never has a CLI
   writer, so its CLI App would keep an old Account forever (acceptance
   test 3). Why not b alone: two sign-ins in one folder both have the
   gate's credential.
- The lab check stays: if the app's `claude` creates the folder's Keychain
  item, an app-only folder passes the gate, and rule 4 keeps its CLI App
  where it was (never wrong-moved, but not "Not signed in"). The live
  walk records which.

### Q3 — Where the Account is stored

- **i.** `apps.subscription_id` for every App: the CLI App's comes from
  Q2, the desktop App's from Q1. `accounts.subscription_id` stays as "the
  folder's CLI sign-in" for older readers, then goes.
  - Pro: one rule for every surface.
  - Con: a migration (take the next free number on release/0.5), and
    every by-folder reader must move to by-App.
- **ii.** Keep the per-folder link and add `apps.subscription_id` for
  desktop Apps only.
  - Pro: smaller.
  - Con: two rules, and the same bug class returns in code that reads
    the folder.
- Recommended: i.

**Answer: i.** `apps.subscription_id` for every App (V55). A CLI App's is
the folder's CLI sign-in (Q2); a desktop App's comes from its newest Code
record's folder names (Q1-A), and is empty ("Account not known yet")
before its first one. `accounts.subscription_id` stays the folder's CLI
sign-in, written in the same step, so older readers (notices by folder,
the pools' facts) stay right; it goes when the last of them reads the
App. Why: one rule for every surface; ii brings the bug class back in
every folder reader.

### Q4 — Readings by sign-in, not by folder

- **i.** Tag each stored sample with the Account it was read under (a
  new `quota_samples` column). A display for an Account or App uses only
  its own. Meter samples are tagged with the desktop App's sign-in, and
  the `/usage` cache with the `.claude.json` sign-in, at read time.
  - Pro: no data moves. A folder's old readings never cross Accounts.
- **ii.** Key readings by Account (subscription) instead of folder.
  - Pro: cleaner.
  - Con: a migration of history, and the per-folder caps and anchors
    move too.
- Recommended: i. Either way, `accountDisplay` and the presence check's
  `out_accounts` read the same rule.

**Answer: i.** `quota_samples.signed_in_as` (V55), the Account a reading
was taken under. The records pass reads each app meter for every Account
an App on the folder is on (an instance's record folder names hashed
into the stored Account's id: nothing new read), and tags each sample
with it; the `/usage` cache is tagged with the state file's sign-in at
read time. A folder's display uses its own Account's readings (and the
untagged ones from before V55, until a tagged one is newer); the
readings of another Account taken on the folder are shown as that
Account's own display (`QuotaDisplayInfo.signed_in_as`). `accountDisplay`
and the presence check read that one rule. Why not ii: it moves history,
caps and anchors for no gain the tag doesn't give. Known limit: a
session's token events stay by folder (they feed estimates only, never a
measured Out).

### Q5 — When an Account does change

- One `account_audit` row (actor "sign-in read", "folder X: Account A →
  B") and one notice ("Claude Code CLI · aij-test is signed in to B
  now"), once per change, never once per read.

**Answer: as written.** A change is a stored App's Account changing
(`apps.subscription_id`, compared in the update): one `account_audit`
row (action `sign_in_moved`, actor `sign-in read`, under the new
Account's id, the App and both Accounts in its detail) and one earlier
notice, kind `account-changed` ("Claude Code CLI · aij-test is signed in
to Team now"). A read that changes nothing writes neither. The first
Account an App is ever read on is recorded, not told (nothing moved).

### Q6 — Every surface reads room by the App

- Runway, rail, Home, the Accounts screen and Account page Apps lists
  (`with_apps`).
- Marks (`roomMarkOfProfile` → by App).
- Resume and switch room, by the session's App: a terminal run uses the
  CLI App's Account, a Code tab session the desktop App's.
- New session's suggestions.
- Notices and the presence check.

**Answer: as written, by the App's stored Account.** The backend lists
each Account's Apps by `apps.subscription_id` (`with_apps`), and an
Account only desktop Apps are on is listed too. The UI's one rule is
`accountOfApp` (the App's `account_id`; a CLI App's folder, as before; a
desktop App with none: "Account not known yet", no mark). The rail,
Home, Accounts and App page marks, the Runway, the session pages and
Resume/switch room read it; a terminal run is its CLI App's, a session
launched in a desktop App that App's. Notices by folder stay right
because the folder is its CLI App's. The presence check's out rule goes
by App.

## What the user should see

- "Claude app · aij-test" under the personal Max Account, marked Out
  (true: the app can't work until 07:00).
- "Claude Code CLI · aij-test" under the Team Account, 93% left, no Out
  mark, with Resume on its terminal sessions enabled.
- No change when either program runs again. A real change (the CLI
  signed in as another Account, or the app's sign-in changed) shows once,
  as a notice, and appears in the Account's history.
- A profile used only through the Claude app (no CLI sign-in) shows its
  desktop App on the app's Account and its CLI App as "Not signed in",
  never stuck on an old Account.

## Acceptance tests

1. `crates/juggler-app/tests/stored_accounts.rs:506`
   `a_profiles_own_sign_in_written_into_its_folder_doesnt_move_its_cli_app`
   (920d9666e, ignored today): un-ignored and passing. After the app's
   `claude` writes its sign-in into the shared folder, the CLI App stays
   on the Team Account and is listed under it.
2. New (Rust): the same fixture lists the desktop App under the personal
   Account (its records name its sign-in) and the CLI App under the Team
   Account, in one `list_with_apps_impl` read.
3. New (Rust): an app-only folder (no `.credentials.json`, no Keychain
   item: the fixture's stand-in) follows the app's sign-in when it
   changes, and its CLI App reads "Not signed in". It is never stuck on
   the old Account.
4. Existing (Rust): `signed_out_unlinks_and_another_account_moves_it` and
   the 0.4.1 "couldn't check" tests stay green. A CLI that signs in as
   another Account (written by `cli`) still moves the CLI App.
5. New (Rust, quota records): meter samples read while the shared folder's
   `.claude.json` holds the app's sign-in are tagged with it. The Team
   Account's display never takes them; the personal Account's display
   never takes a Team reading taken on the shared folder.
6. New (vitest): the rail marks and the top bar count follow per-App
   Accounts. The default app has no Out; the aij-test desktop App is Out
   and its CLI App isn't. Keep
   `ui/src/components/room-mark-fresh-accounts.test.tsx` (acadb1b67)
   green.
7. New (vitest): Resume on a terminal session in the shared folder reads
   the CLI App's room (enabled). A Code tab session reads the desktop
   App's (greyed, "out of room until 7:00 AM").
8. New (Rust): an Account change writes one audit row and one notice. A
   second read with no change writes neither.
9. `scripts/qa/presence_check.py`: the out rule goes by App, and it
   passes on the real setup with aij-test split. Its `--self-test`
   gains the split case.
10. Live, on the aij-test lab (sanctioned profile, the maintainer only
    signs in): start the app, open a Code conversation, read the Runway,
    the rail marks, Resume and the presence check; then do a terminal
    run in the folder and read them again. Nothing flips, and each App
    stays on its own Account.

## Size

**L** (upper end). It needs:
- a D-63 amendment;
- a schema migration;
- tagging in the readings pipeline;
- every by-folder room and grouping reader moved to by App, across the
  backend, `ui/src/lib` and the presence check;
- a live lab walk.

Suggested order: Q1/Q2 ruling and the lab Keychain check → schema and
`store_accounts` → readings tagging → UI and presence check → walk.

## Task card (E32 format)

**Status**: TODO | **Priority** P1 | **Effort** L | **Depends**: AJ-E3202
(stored Accounts and Apps), a D-63 amendment (Q1)
**Goal**: Each App is on the Account it actually uses, even when a
Claude app profile and its CLI share a folder with different sign-ins.
**User story**: As someone whose test profile's Claude app is on my
personal plan and whose CLI is on my team seat, I see the app under the
plan and the CLI under the team. Each shows its own room, and neither
moves when the other runs.
**Requirements**:
1. A desktop App's Account SHALL come from its own sign-in, by the folder
   names of its Code records only (Q1-A, after the D-63 amendment).
2. A CLI App's Account SHALL come from the folder's CLI sign-in: moved
   only by a read a CLI wrote; "Not signed in" when the folder has no CLI
   sign-in (Q2).
3. Each App SHALL store its Account (Q3-i). Every surface that groups or
   marks room SHALL read it by App (Q6).
4. Readings SHALL carry the Account they were read under. An Account's
   room SHALL use only its own (Q4-i).
5. A change of Account SHALL be recorded once and told once (Q5).
**UI/UX reasoning**: Account = subscription; an App is on the one it
uses. Never show a wrong Out mark; say a change once.
**Performance budget**: the Accounts read stays ≤ its 0.4.6 time (it now
runs every 30 seconds while shown, acadb1b67); per-App grouping ≤ 5 ms.
**Quality gates**: L1 the tests above; L2 on the replica (a split
folder); the live aij-test walk.
**Evidence/validation proof**: the walk's record, the presence check's
PASS on the real setup.
**Visual testing**: the Runway and the rail with a split folder, in both
themes; the change notice.
**Files to touch**:
- `crates/juggler-adapters/src/claude_code.rs`, `claude_limits.rs`
- `crates/juggler-app/src/features/subscriptions/`, `features/quota/records.rs`,
  `features/pools/sign_in.rs`
- `crates/juggler-core/src/storage/` (migration)
- `ui/src/lib/subscriptions.ts`, `room-mark.ts`, `room-targets.ts`
- `ui/src/components/RoomChip.tsx`, `HomeRunway.tsx`
- `ui/src/pages/session/ResumeButton.tsx`
- `scripts/qa/presence_sessions.py`
**Research basis**: this file; `docs/research/desktop-pool-members.md` §2;
`docs/research/entity-model-audit.md` (Account and App);
`claude_limits.rs` (`instance_sign_ins`, `app_usage_samples_from`).
**Handover note**: 0.4.6 shipped the stale-mark fix (acadb1b67) and the
ignored repro test (920d9666e) only. No link-keeping stopgap shipped, for
the reasons in "Why no 0.4.6 mitigation".
