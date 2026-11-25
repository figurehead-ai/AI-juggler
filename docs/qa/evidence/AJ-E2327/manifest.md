# Evidence manifest: AJ-E2327

**Task:** `docs/tasks/E23-session-runtime.md`, AJ-E2327: Add on a running desktop profile brings in its Claude Code folder by evidence, so its sessions show at once; the rail keeps its place and never contradicts itself.

**Branch:** `release/0.5.1`. Root-caused read-only against the installed 0.5.0 Nightly's data (`sqlite3 -readonly`) and the process table (`ps`), never written, nothing launched or quit. Paths and names here are placeholders (`/Users/you`, profiles `work`, `team-a`, `lab`).

## Root cause (written before the fix)

### What the data shows

- `apps`: six desktop Claude App rows, every `folder_id` NULL; three with `added = 1` (the default instance and two profiles). `accounts`: empty.
- Each running profile's Code child (`…/ClaudeProfiles/<profile>/claude-code/<ver>/…/claude`, under `Claude.app/Contents/Helpers/disclaimer`, under the profile's `Claude` main process) wrote its pid file in exactly one tool folder: `…/ClaudeCodeProfiles/<folder>/sessions/<pid>.json`. That folder is its `CLAUDE_CONFIG_DIR` as the child itself wrote it (checked by pid, not by reading any environment: D-37).
- The folder names don't follow the profile names: profile `team-a` writes in `ClaudeCodeProfiles/client-a`, profile `lab` in `ClaudeCodeProfiles/acme-north`; the app's default instance (`…/Application Support/Claude`) writes in `~/.claude`.

### Why 0.5.0 left `folder_id` NULL after Add (bde5fa4d8)

The rail's Add is `places_offer_answer` → `answer_offer(state, id, true)` (`places/mod.rs` at bde5fa4d8, `link_its_folder(state, id, true)`), so `may_bring_in` was true and the import was never reached, let alone refused. Discovery did find the folders (`claude_code.rs` `scan_environment`: `~/.claude` and every `ClaudeCodeProfiles/*`). The link failed on the **name match**:

- `places/link.rs:13-20` (bde5fa4d8) `folder_for_profile` takes only the discovered folder whose file name equals the profile folder's name. `team-a` ≠ `client-a`, `lab` ≠ `acme-north`: `None`.
- `places/link.rs:74-75` (bde5fa4d8): the app's default instance has an `instance_dir` (`…/Application Support/Claude`), so it takes the `(Some(name), _)` arm with the name `Claude`, and the `(None, Some(default_folder))` arm that knows `~/.claude` is never reached: `None`.
- `None` → `continue` → `false`: no Account is brought in, `account_id` stays `None` (`folder_id` NULL).

With no folder linked, nothing of the profile's is read: the conversation index reads Accounts' folders only, so no session appears anywhere, and its App has nothing under it ("nothing open", the calm grey of D-117 with no words saying why).

### Would HEAD (AJ-E2315 part 2, 00179c60a) still?

Partly. `places/link.rs:74` (HEAD) returns `false` whenever the profile isn't running (`running_now` is `None`), and `places/link.rs:105` links a profile only by `folder_of_its_sessions` (its running sessions' pid files under its own main process). So:

- added while running **with a Code session open**: linked correctly (the pid-file evidence works on this Mac's shapes, the default instance included);
- added while **not running**, or running with no Code session open: never linked, so the same blank row. Its own records (the app's `claude-code-sessions/**/local_*.json`, each naming its `cliSessionId`) were not used, and nothing told the user why.

### The rail's contradictions

- "Open now 3" beside "Nothing open yet": `ui/src/components/PlaceBoard.tsx:389` shows the empty state when `items.length === 0` (no sessions), while Open now's head counts running Apps by App (`ui/src/lib/rail-lines.ts:246`, `runningGroups`). Three running Apps with no session read as both.
- Add scrolls the top out of view: `ui/src/components/rail-hold.tsx` `hold()` keeps the line under the pointer still while the pointer is in the list. Add removes the offer line under the pointer and puts the App's header into Open now above it, so the hold scrolls the list down by the new line's height (and pads below to make room): the user's own change was treated like a notice arriving.
- The grey icon is D-117's calm tone at rest (`ui/src/lib/attention.ts` `appTone`, `.app-tone` in `styles.css`), shown with no words; on an App that can't show its sessions it read as broken.

## The fix

### Evidence order, as built (`ownership::sessions::decide`, gathered in `places/link.rs` `look`)

D-37 stands: no process's environment is read (the Claude app's sessions carry a sign-in token there; `tests/trust.rs` `no_process_environment_is_ever_read` is unchanged). What the Code child was given as `CLAUDE_CONFIG_DIR` is read from the pid file the child itself writes in that folder.

1. **Running**: its running sessions' pid files (`<folder>/sessions/<pid>.json`), each pid's parents only the app's own programs up to a main process running this instance (`ownership::sessions::folder_of_sessions`, AJ-E2315).
2. **Records**: its app's own records (`claude-code-sessions/**/local_*.json`, each `cliSessionId`) found as transcripts (`<folder>/projects/*/<id>.jsonl`) in every discovered folder and every Account's folder; the folder holding most of them wins; a tie (a switch copies a conversation under the same id) is confirmed only by its link so far or the folder named like it, else nothing is decided (`folder_of_records`).
3. **Declared** (apps with one instance only, ADR-0032): the folder the app declares (`~/.codex`).
4. **Recorded**: AI Juggler's own record: the folder it starts the profile with (its link so far), or the one the user chose with "Choose its folder…".
5. **Named**: the folder named like the profile, a fallback only, and never once its records name conversations; looked at again (throttled) until 1–4 confirm or correct it.

The source is kept for each App (`app_settings` `places.folder_by:<id>`, plus the last look's note in memory) and said on its row (`KnownPlaceInfo.folder`). A look that finds nothing (or only a recalled or guessed folder) is tried again after 60 s while it runs, 10 min while closed, and at once when it starts. A new folder is brought in (in place, never copied) only with the user's consent: Add's yes, or an App the user added (D-84).

### For AJ-E2317

`places::link::folder_mismatch(state, id) -> Option<(PathBuf, LinkSource)>`: the folder the App's own evidence (running sessions, records, declared folder) says it uses when that isn't the folder it's linked to, and what says so; `None` when it agrees, nothing proves otherwise, or it isn't linked. Built on the pure `ownership::sessions::contradicts(&Evidence, linked)`. Reads folders and the process table: call it off the window's thread.

### For first run's "Your apps" (AJ-E1545)

IPC `places_pairings` → `PlacePairingInfo[]`: each desktop profile known (Apps and offers, not declined) with `folder` (`PlaceFolderInfo`: `tool`, `name`, `path`, `by`, `why`), `certain` (proof: running, records or declared) and `added`. Reads only: nothing is added, brought in or linked.

## By requirement

| Req | Result | Evidence | Commit | Date |
|---|---|---|---|---|
| R1 evidence-first linking through the ownership module; a name match only as a fallback the evidence confirms or corrects; a profile added while closed still linked | PASS | `ownership/sessions.rs` (`LinkSource`, `Evidence`, `decide`, `contradicts`, `folder_of_records`); `places/link.rs` (`look`, `link_by_evidence`, `correct_by_records`). Tests: `ownership::tests::link_evidence::*` (5), `places::link_tests::add_on_a_running_profile_links_the_folder_its_sessions_write_in`, `a_profile_added_while_closed_is_linked_by_its_records`, `a_name_guess_is_corrected_by_evidence` | 56856d10d | 2026-11-23 |
| R2 every Add path brings the folder in place and links it; the confirmation names it | PASS | `answer_offer` returns what it linked (`places_offer_answer` → `PlaceAddedInfo`). Every Add in the window goes through `usePlaceOffers().answer` (the rail, Settings › Desktop apps, first run's found apps, an App page's Find them), which shows `sayAdded`: "Added Claude app · team-a — Also uses its Claude Code folder client-a, where it is, so its sessions show here (its open sessions write there)." ⌘K has no Add of its own (its Find my apps opens first run). Tests: `app-folder.test.ts`; the Accounts made are `in_place` (link_tests) | 56856d10d | 2026-11-23 |
| R3 sessions, live state and subsessions at once; the icon not greyed | PASS (see the usability line on the icon) | Linked on Add, its records are read for its folder in the same call (`record_index(..).listed` in link_tests); its sessions then show where every App's do. The calm grey is D-117's tone at rest, now explained (R5) | 56856d10d | 2026-11-23 |
| R4 no evidence: the row says why and offers "Choose its folder…" | PASS | `PlaceHeader` "sessions not shown" (its title the reason), `AppFolderNote` under it with "Choose its folder…", the header menu's item; `places_choose_folder` (a picked folder only, `picked::vouch`; refused when it isn't a Claude Code folder or the App's own evidence says otherwise). Tests: `no_evidence_says_why_and_choose_its_folder_links_it`, `place-header-folder.test.tsx` | 56856d10d | 2026-11-23 |
| R5 the rail keeps its scroll on Add and brings the added row into view; "Nothing open yet" only when Open now is empty, worded for its lens; the grey icon explained | PASS | `rail-hold.tsx` `ownChange`, `rail-add.ts` `useAddOffer`; `lib/rail-empty.ts` + `RailEmpty`; `toneTip` on the App icons of the rail's headers and Home's App tiles. Tests: `rail-hold-own.test.tsx`, `rail-add.test.tsx`, `rail-empty.test.ts`, `app-folder.test.ts`, `place-header-folder.test.tsx` | 56856d10d | 2026-11-23 |
| R6 tests, failing on the old code | PASS | The table below | 56856d10d | 2026-11-23 |

### Each test against the old rules

The new tests were run with `look` reduced to each old rule (a temporary edit, reverted): HEAD before this change (AJ-E2315 part 2: running pid files only) and 0.5.0 (the folder named like the profile only).

| Test | HEAD before | 0.5.0 | Now |
|---|---|---|---|
| `add_on_a_running_profile_links_the_folder_its_sessions_write_in` (fresh data folder, running profiles, folder names unlike theirs) | passes | fails (took the decoy `team-a` by its name) | passes |
| `a_profile_added_while_closed_is_linked_by_its_records` | fails | fails | passes |
| `restarting_it_starts_it_with_its_folder` | fails | fails | passes |
| `no_evidence_says_why_and_choose_its_folder_links_it` | passes (nothing was linked either; the words and Choose its folder… are new API) | passes (nothing linked either) | passes |
| `a_name_guess_is_corrected_by_evidence` | fails | fails | passes |
| `a_mismatch_between_the_link_and_the_evidence_is_said` | passes (new API) | fails | passes |
| `pairings_pair_each_profile_by_evidence_and_add_nothing` | fails | fails | passes |
| `rail-hold-own.test.tsx` "before (no own change)" | the old hold scrolls the top out (asserted) | same | Add keeps it |
| `rail-empty.test.ts` "never shows under Open now" | 0.5.0's `items.length === 0` shows it | same | null |

## Usability answers, on the built screen

1. A new user's Add on a running profile: the App appears with its sessions, as in the app. Built: yes, its folder is linked on Add by its own pid file and its records are read at once.
2. The confirmation names what comes with it: built ("Also uses its Claude Code folder client-a, where it is…"); a row that can't show its work says why ("sessions not shown", the reason in its tooltip and the note under it).
3. Useful at once, nothing new on screen: built; the only new words appear when no folder could be found.
4. No folder: the row says so and offers "Choose its folder…": built (row, note, menu, and the Add toast's action); the grey is explained wherever an App's icon is drawn in the rail's headers and Home's tiles.
5. Reading a running process's environment: not done (D-37); the pid file and the tools' folders are read instead, the same reads AI Juggler already makes, primed by AJ-E1547's gate.
6. Watch-fors: "Open now 3" beside "Nothing open yet" can't happen (the empty state needs Open now empty); the rail doesn't jump on Add; a name guess is said as a guess and corrected by evidence; nothing is added that the user didn't click (D-84: offers only, pairings read only).

## Close-out (0.5.1, 2026-11-23, Opus)

Each test named below was found by name and run, targeted, on `release/0.5.1` (vitest by file; cargo through `scripts/heavy.sh`, `--lib` with module filters). An OPEN line says where it closes.

R1: PASS — cargo `ownership::tests` link evidence (`a_closed_profile_is_linked_by_where_its_records_are`, `a_tie_is_confirmed_by_its_link_or_left`, `the_folder_holding_most_of_its_sessions_wins`, `the_strongest_evidence_decides`, `only_proof_contradicts_a_link`); `places::link_tests::add_on_a_running_profile_links_the_folder_its_sessions_write_in`, `a_profile_added_while_closed_is_linked_by_its_records`, `a_name_guess_is_corrected_by_evidence` (56856d10d).
R2: PASS — `places::link_tests::add_on_a_running_profile_links_the_folder_its_sessions_write_in`, `pairings_pair_each_profile_by_evidence_and_add_nothing`; the confirmation's words: vitest `lib/app-folder.test.ts`.
R3: PASS — `add_on_a_running_profile_links_the_folder_its_sessions_write_in` (its records listed in the same call), `restarting_it_starts_it_with_its_folder`; the icon in full colour: vitest `one-mark-per-app.test.tsx` (AJ-E1550's `.app-tone`).
R4: PASS — `places::link_tests::no_evidence_says_why_and_choose_its_folder_links_it`; vitest `components/place-header-folder.test.tsx`.
R5: PASS — vitest `components/rail-hold-own.test.tsx`, `components/rail-add.test.tsx` "lets its change through and brings the added App into view", `lib/rail-empty.test.ts`, `lib/app-folder.test.ts`.

Every add path walked live from an empty Dev folder (the task's quality gate, not a numbered requirement): PL1 steps C3–C6 and D1–D7.

## 0.5.1 PL1 F18/F19 (2026-11-23, Opus)

F18: the lab's running profiles were never offered: `Book::observe` dropped any profile under the temporary folder, and a lab's HOME is `/tmp/aij-lab…`. A profile inside the run's own home is kept (`places::book::temporary_not_home`); tests `places::d84_tests::a_profile_inside_a_temporary_home_is_offered` (fails before), `temporary_not_home_spares_only_the_runs_own_home` (6aa61ed48).
F19: every add path after Remove: `a_removed_running_profile_is_offered_again` (the rail's Running, not added, Settings' Found, the wizard; 6aa61ed48); Settings › Desktop apps › Add apps › Find my apps… (`keep-alive.test.tsx` "F19 …"), ⌘K Find my apps… (`palette-commands.test.tsx` "F19 …"), the shell's event (`app-find-accounts.test.tsx`) (585871f39); one tool's Add an app and its Look again list its app's offers (`first-run-add-again.test.tsx`, fails before; 3ade82664).
