# Evidence manifest — AJ-E2108

Task: `docs/tasks/E21-desktop-first.md` AJ-E2108, the session page: where
it runs and its last messages (D-35 to D-37, the approved canvas).
Re-audited against the code on `release/0.4.1` (HEAD 051e71e4b),
2026-11-07. The page was built in 0.1.3 (ac0e41dc0) and has since been
reworked: the conversation feed (0.3.5, ADR-0024, 0238b1bda), the header's
two chips (D-62, a6756c0e1), one mark per App (ec1fb9437) and the 0.4
design review (e2476f451).

Since the 2026-10-24 manifest: the direct move from a session another app
runs is built (Continue on lists every tool's profiles and switches from
the conversation itself, `switchSourceOf` → `conversation:<account>:<id>`).
Still open: Window preview and History for another app's session, and the
two budgets.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 header: app icon and profile badge, title, chips (state, place, profile, account, folder), Go to it | `cd ui && pnpm -s exec vitest run src/pages/session/session-chips.test.tsx src/components/one-mark-per-app.test.tsx src/components/outside-conversations.test.tsx`: "shows two chips: the state, and the App menu named by its App", "opens to the App, its account, the working folder and a door to each of their pages (A3)", "says where it runs on the facts line", "the session header's mark and its App chip", "AJ-E2108: while it's open in its app, Go to it leads and its own profile waits, saying why"; `SessionHead.tsx` (`SessionMark`, `SessionChips`, `SessionWhere`, `GoTo`); gallery `_gallery/92-session-pages/outside-asks-1280x800-dark.png`, `account-menu-open-1280x800-light.png` (looked at) | every named part on the header, Go to it while open | all pass. App mark with the profile's badge beside it (`session-mark`, `session-mark-profile`, the profile's colour), title, the state chip ("Asks you · 3 min"), the place on the facts line ("Running in the Claude app · viewed 2 min ago"), and profile, account and folder in the App chip's menu (D-62 folded the three chips into one, 0.3 polish), Go to it ↵ as the primary. The captures (a7d543b10, 2026-11-01) predate the D-62 chip label; the gallery gate re-shoots them | PASS | a6756c0e1 | 2026-11-07 | Claude (self) |
| R2 last messages: the last 2–3 turns read when the page opens, secrets redacted, kept nowhere; a Settings switch (on); a Claude app chat shows its title only; never in rows, the tray or notifications | `cd ui && pnpm -s exec vitest run src/pages/session/last-messages.test.tsx src/screens/settings/desktop-apps.test.tsx`: "is followed only while its page is shown: once, and let go when it isn't", "off in Settings: says so, with the way to turn them on", "a chat with no transcript here shows its title only", "a paired Mac's session is never read here", "Last messages are on until you turn them off, and only off is kept"; cargo (runs in CI): `juggler-adapters` `src/last_turns_tests.rs::only_the_end_is_read_and_a_turn_begun_before_the_window_is_dropped`, `::a_compressed_file_is_not_read`; `juggler-app` `src/features/conversations/last_messages.rs::an_api_key_in_a_turn_shows_redacted`, `::a_private_key_block_is_taken_out_whole`, `::off_in_settings_no_transcript_is_looked_up_or_read`, `::nothing_is_kept_written_or_logged`, `::no_transcript_or_a_compressed_one_says_so_and_reads_nothing`; `feed/tests.rs::a_key_in_an_appended_turn_arrives_redacted_and_counted`, `::an_unfollowed_feed_is_kept_a_minute_then_let_go`; gallery `_gallery/92-session-pages/last-messages-off-1280x800-*.png`, `outside-asks-1280x800-dark.png` | read on open only, redacted, nothing written, off reads nothing, title only without a transcript | vitest pass; the cargo tests exist at those names and back the `last-messages` trust claim. The reader (`useConversation`) is used only by `LastMessages`, `ConversationSwitcher` and `SubagentsTab` on a page, never by a row, the tray or a notification; the command is classed Write (`lib.rs` 549) so the read-only bridge can't serve it. Since 0.3.5 (ADR-0024) the page shows up to 30 turns (`TURNS`) and follows the conversation, more than the 2–3 asked; "kept nowhere" now means memory only, let go a minute after the page closes | PASS | 0238b1bda | 2026-11-07 | Claude (self) |
| R3 Where it runs; Runway; Continue on another account (accounts with their profiles, a one-line setup check each, same-account profiles marked "no new room") | `cd ui && pnpm -s exec vitest run src/pages/session/continue-on.test.tsx src/pages/session/continue-first-run.test.ts src/pages/session/session-page-once.test.tsx src/screens/accounts/accounts-grouped.test.tsx`: "a Claude-app session offers every tool's profiles, each one click away while it stays open there", "a session another app ran, closed now, can also be continued on its own profile", "a target the ranking holds back says why and can't be picked", "Where it runs has no App row (the chip's), and names the model as people say it", "leaves the Runway card out while nothing is measured, unless it has more to say", "AJ-E2108: a profile on the session's own account gives no new room", "the setup check in one line"; `WhereItRuns.tsx`, `Runway.tsx`, `ContinueOn.tsx` (`byAccount`) | all three cards; grouped by account; one check line per profile; no-new-room line | all pass: targets grouped by account (`continue-account`), one `continue-check` line each, "Work 2 shares this account, so it gives no new room"; a session another app runs now moves straight from the conversation (`conversation:work:c-1`), closing the old "not done" | PASS | e35197711 | 2026-11-07 | Claude (self) |
| R4 tabs (amended): AI Juggler's own sessions keep Terminal (or Window preview) first with Overview, Usage, Processes, History; another app's session has Overview · Subagents · Usage · Processes, no Window preview (its app's window shows whichever conversation it has open) and no History (no record of a session AI Juggler didn't run) | `pages/registry.ts` `tabsOf` (session, desktop session, conversation); vitest `screens/nav-screens.test.tsx` and `components/desktop-surface.test.tsx` "its page's body is the Window preview (labelled honestly, not 'Terminal'), and opens it"; budgets: vitest `components/session-page-timing.test.tsx` "shows another app's session's head inside 150 ms" (median 4.7 ms, jsdom, the preview's data); cargo `juggler-adapters` `the_last_turns_of_a_long_transcript_read_inside_the_budget` (4 MB, median 3.9 ms, debug) | page < 150 ms; tail read < 30 ms | pass; the page time is the UI's own (a live backend's answer adds to it: the hand-timed K1 page 138–163 ms in a debug build) | PASS | 8ac8f4d0b | 2026-11-07 | Claude (self) |
| Gate | `cd ui && pnpm -s exec vitest run src/pages/session/session-chips.test.tsx src/components/one-mark-per-app.test.tsx src/components/outside-conversations.test.tsx src/pages/session/last-messages.test.tsx src/screens/settings/desktop-apps.test.tsx src/pages/session/continue-on.test.tsx src/pages/session/continue-first-run.test.ts src/pages/session/session-page-once.test.tsx src/components/inspector-shell.test.tsx src/components/desktop-surface.test.tsx src/pages/session/history-tab.test.tsx src/pages/session/subagents-tab.test.tsx` (`accounts-grouped.test.tsx` ran with AJ-E2109's command) | all pass | 12 files, 111 tests pass (accounts-grouped: pass). Cargo and Playwright (`session-pages-gallery.spec.ts`) cited as written: they run in CI / the gallery gate | PASS | 051e71e4b | 2026-11-07 | Claude (self) |

## R4's gap, exactly

- **Window preview for another app's session.** `ui/src/pages/registry.ts`
  `tabsOf`, `case "conversation"`, never adds `TAB.window`; the preview
  backend (`crates/juggler-app/src/features/preview/mod.rs`
  `preview_start_impl`, `ui/src/lib/ipc/desktop.ts` `previewStart`) is keyed
  by an AI Juggler session id. Needed: a preview keyed by the conversation's
  place (its `places[].pid` / `app_pid` from `features/conversations/places.rs`),
  a `desktop` fact on the conversation's `pageTabs` call in
  `components/ConversationPage.tsx` (line 167), and the tab's body there.
  Test: `pageTabs("conversation", { desktop: true })` includes `"window"`,
  and `outside-conversations.test.tsx` renders the tab for a conversation
  open in the Claude app and starts the preview with that place.
- **History for another app's session.** `useSessionHasHistory(sessionId)`
  and `HistoryTab.tsx` read AI Juggler's own events by session id; a
  conversation has none. Needed: a source of a conversation's own events
  (its switches and carry-ins, keyed by `conversation:<account>:<id>`, the
  key `switchSourceOf` already gives it), a `hasHistory` fact for it
  passed from `ConversationPage.tsx`, and `case "conversation"` adding
  `history` when it holds. Test: `pageTabs("conversation", { hasHistory: true })`
  includes `"history"`, and the tab lists a switch made from that conversation.
- **Budgets.** Page < 150 ms: measured only by hand in a debug build
  (K1 step 3: 138, 147, 163 ms); the release-build timing is AJ-E3306's.
  Tail read < 30 ms: no measurement anywhere. Needed: a timed test beside
  `last_turns_tests.rs` (or a bench) reading the 64 KB window of a large
  fixture transcript and asserting under 30 ms, and the page timing from the
  release build recorded here.

## What exists now

- **Header** (`ui/src/pages/session/SessionHead.tsx`): the App's mark with
  the profile's badge beside it (one colour per App, AJ-E3206), the title,
  the state chip with how long it has asked, the App chip whose menu names
  the App, its Account and the working folder with a door to each page,
  where it runs on the facts line, and **Go to it ↵** while it's open
  (Enter on the page does the same).
- **Overview**: the conversation (`LastMessages.tsx`, followed while shown,
  ADR-0024), Where it runs, Runway (left out while nothing is measured),
  Continue on another account (`ContinueOn.tsx`: every tool's profiles by
  account, a setup line each, "no new room" for the session's own account,
  straight from another app's conversation while it stays open there).
- **The read**: the end of the transcript only (`last_turns.rs`), the
  tools' own parsers, redacted by D-14's patterns before it leaves the
  backend; off in Settings › Desktop apps reads nothing; no file, setting,
  log line or index entry.

## Binary artifacts

None added; cited: `_gallery/92-session-pages/outside-asks-1280x800-dark.png`,
`account-menu-open-1280x800-light.png`, `last-messages-off-1280x800-*.png`
(captured 2026-11-01, a7d543b10).

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (fixture text only)
