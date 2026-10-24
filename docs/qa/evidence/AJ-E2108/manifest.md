# Evidence manifest — AJ-E2108

Task: `docs/tasks/E21-desktop-first.md` AJ-E2108, the session page: where
it runs and its last messages (D-35 to D-37, the approved canvas).
Branch `e21-pages` (off `feat/desktop-first` with AJ-E2101..E2103 and
AJ-E2110 merged in), 2026-10-24.
Status: **IN-PROGRESS**: the page, the read and its privacy are built
and tested in the browser preview and by unit tests. Open: the pinned
container's gallery run, a walk on the maintainer's real setup, and a
move straight from a session another app runs (today it's carried into
AI Juggler first; see "Not done").

Gallery: `docs/qa/evidence/_gallery/92-session-pages/` (captured on this
Mac by a local Playwright run of `ui/tests/visual/session-pages-gallery.spec.ts`;
the container run replaces them).

## What exists now

- **One header for every session** (`ui/src/pages/session/SessionHead.tsx`,
  `PageHeader layout="session"` in `pages/PageShell.tsx`):
  - the app's mark at 52 px with the profile's badge (`AccountAvatar`) on
    its corner;
  - the title at 24 px bold;
  - chips: the state in E21's words with how long when it asks you
    ("Asks you · 3 min", amber), where it runs ("Running in the Claude app
    · viewed 2 min ago", never "in front"; "Last active 5 h ago" once it
    ended), "Profile: Work" bordered in the profile's colour, "Account:
    Claude Max · o…@work.com", the folder;
  - **Go to it ↵** (accent, the up-right arrow) while it's open, and Enter
    on the page does the same; the ⋯ menu from the registry.
  - Sessions AI Juggler starts keep their header's Switch account…,
    Stop and Relaunch; Go to it shows for one in a desktop app's window.
- **Tabs** (`pages/registry.ts`): AI Juggler's sessions keep their body
  first (Terminal, or Window preview for a desktop app's), then
  Overview, Usage, Processes, History; a session another app runs has
  Overview and, when its tokens are known, Usage.
- **Overview, three columns** (`pages/session/SessionOverview.tsx`; a
  container query folds it to one column on a narrow page):
  - **Last messages** (two columns, `LastMessages.tsx`): the last 2–3
    turns as bubbles labelled You / Claude / Claude asks (amber, when the
    session asks you), the note "read from the session's transcript when
    you open it · nothing kept" (and how many keys were taken out), then
    "Answer it in the Claude app. Go to it brings Claude · Work to the
    front; if another session is showing there, pick this one in its
    sidebar." (the sidebar half only for a tool's own desktop app).
  - **Where it runs**: App, Window (the app's instance), Profile, Account,
    Model, Folder — only the ones known; "Also open in …" when it's open
    in two places.
  - **Runway**: the account's 5-hour and week meters (one runway for all
    the account's profiles, AJ-E2109).
  - **Continue on another account** (`ContinueOn.tsx`): accounts with
    their profiles beneath, "% left" per account, a one-line setup check
    per profile from the existing check (`switch_setup_check`: "Ready:
    same tools set up", "Needs 2 MCP servers, set up as it moves"),
    "Work 2 shares this account, so it gives no new room" for profiles on
    the session's own account (not offered), the session's own profile
    left out, and the switch-back card when the session has one. A
    profile opens the existing switch screen (`SwitchFlow`, AJ-E1542/1543)
    through RightInspector's `onHandover`: nothing duplicated.
- **The read** (`crates/juggler-adapters/src/last_turns.rs`,
  `crates/juggler-app/src/features/conversations/last_messages.rs`,
  `conversations_last_messages`):
  - only the transcript's end: a seek to `len − 64 KB` (`QUICK_TAIL`),
    whole lines from there;
  - the tools' own parsers (`parse_any`): tool calls and results, a
    tool's injected lines (`isMeta`, `<command-…>`), Codex's injected
    context and sub-agents' lines are not turns; one speaker's
    consecutive messages are one turn; each keeps its last 1,200
    characters;
  - redacted by D-14's patterns (`juggler_handover::redaction::strip_segments`:
    `sk-` keys, bearer tokens, credential-named assignments, private-key
    blocks, API-key headers) before it leaves the backend;
  - off in Settings (`show_last_messages` = "0", AJ-E2110's switch in
    Settings › Desktop apps), nothing is looked up or read; the page
    reads again when the switch changes;
  - run on `ipc::blocking`, only when the page's Overview is first shown,
    once per session;
  - a compressed transcript (a finished Codex rollout) isn't read, a
    session with no transcript on this Mac (a Claude app chat) shows its
    title only, a paired Mac's session says "Open on <Mac> to read" and
    is never read here;
  - classed Write in the command registry so the read-only test bridge
    can never serve message text.
- **Nothing kept**: no file, setting, log line or index entry; the text
  lives in the page's state only.
- **Privacy page**: the `last-messages` claim (AJ-E2110's words) now
  names this task's tests (below), in `trust-claims.json` and
  `docs/guide/trust.md`.

## Tests

| Requirement | Test |
|---|---|
| Only the end, whole lines | `crates/juggler-adapters/src/last_turns.rs::only_the_end_is_read_and_a_cut_line_is_skipped` |
| Turns in order, one per speaker | `…last_turns.rs::the_last_turns_of_a_claude_transcript_in_order_one_per_speaker` |
| A tool's own lines aren't turns | `…last_turns.rs::a_tools_own_lines_and_sub_agents_are_not_turns` |
| Codex through its own parser | `…last_turns.rs::a_codex_rollout_reads_through_its_own_parser` |
| Compressed not read | `…last_turns.rs::a_compressed_file_is_not_read` |
| An API key shows redacted | `crates/juggler-app/src/features/conversations/last_messages.rs::an_api_key_in_a_turn_shows_redacted` |
| A private key block taken out whole | `…last_messages.rs::a_private_key_block_is_taken_out_whole` |
| Off: nothing looked up or read | `…last_messages.rs::off_in_settings_no_transcript_is_looked_up_or_read` |
| Nothing kept, written or logged | `…last_messages.rs::nothing_is_kept_written_or_logged` |
| No transcript / compressed / unreadable | `…last_messages.rs::no_transcript_or_a_compressed_one_says_so_and_reads_nothing` |
| Read once, only when shown; labels; off; no transcript; remote | `ui/src/pages/session/last-messages.test.tsx` (5) |
| No new room; setup check words | `ui/src/screens/accounts/accounts-grouped.test.tsx` |
| Go to it leads an open outside session | `ui/src/components/outside-conversations.test.tsx` |
| Tabs | `inspector-shell.test.tsx`, `desktop-surface.test.tsx`, `app-keys.test.tsx` |
| Each kind of session, both themes, axe 0 | `ui/tests/visual/session-pages-gallery.spec.ts` (8 of its 14) |

## Gates run (this branch, 2026-10-24)

- `cargo test` targeted: `juggler-adapters --lib last_turns` 5/5,
  `juggler-app --lib last_messages` 7/7, `--test trust` 13/13.
- `cargo clippy -p juggler-adapter -p juggler-adapters -p juggler-app
  --all-targets -D warnings`: clean; `cargo fmt --all --check`: clean.
- `pnpm typecheck`: clean; `vitest run`: 119 files, 904 tests pass.
- `just structure`'s scripts (layering, IPC audit, size budget): OK.
- Terminology, plain-words and tool-literal lints: OK.
- Hand test in the browser preview (`vite` on this worktree): both
  kinds of page, Overview, light and dark; axe-core in the page: 0
  critical/serious on each.

## Not done

- **A move straight from a session another app runs.** The switch screen
  needs a session AI Juggler runs; such a session's Continue section lists
  the accounts and profiles and says to continue it in AI Juggler first
  (its button; greyed while it's open in its app). Chaining carry-in and
  the switch into one click would start a session only to stop it.
- **Window preview and History for a session another app runs**: no
  source yet (the preview is per AI Juggler session; history is AI
  Juggler's own record), so the tabs aren't shown (a tab shows only when
  it has something).
- **Budgets** (page < 150 ms, tail < 30 ms): not measured on the real
  setup; the read is one 64 KB seek-read and one parse.
- **The pinned container's run** of the galleries and the regression
  gate; **a walk on the maintainer's setup** (left to the maintainer: it
  reads real transcripts).
