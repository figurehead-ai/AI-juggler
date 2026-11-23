# Evidence manifest: AJ-E3824 (Search (⌘K) takes over Bot's look-ups; D-144)

Built 2026-11-23 on release/0.5.1 (Opus). Backend: `bot::search::bot_search`, a Read
command over `bot::lookup::find_records` (Bot's one look-up, no copy); unit tests on a
fixture world (generic names). UI: the ⌘K palette's own rows and groups
(`ui/src/components/palette/`), vitest with the `window.__AIJ_SEARCH__` seam; the
99-palette gallery retaken (preview build, mocked backend), both themes at 1280, 1024
and 1440, axe 0 critical/serious at 1280, opened by eye in both themes. Live walk on
the real Mac: still to do (the persona walk, Sonnet; Opus verifies).

| Req | What | Observed | Verdict | Proof |
|---|---|---|---|---|
| 1 | Sessions | found by name ("login") and by state ("What needs me?"); a session already listed under Sessions shows once; ↩ opens its page by id | PASS | `a_session_is_found_by_its_name`, `a_session_is_found_by_its_state`; vitest "a session already listed shows once", "each record opens its page by id" (49a9a2809, 33ddb995f) |
| 1 | Tasks | found by name, with its pool; ↩ opens its pool's board by the pool's id | PASS | `a_task_is_found_with_its_pool`; vitest "each record opens its page by id" |
| 1 | Pools | a pool named ("release pool") is listed once with its found tasks' count; ↩ opens its board | PASS | `a_pool_is_found_by_its_name_once`; vitest |
| 1 | Usage | "usage" lists every account's use; an account's name finds it; ↩ opens its page | PASS | `usage_is_found_by_the_word_and_by_an_account_name`; vitest; `_gallery/99-palette/records-1280x800-{light,dark}.png` |
| 1 | One look-up, nothing written; no noise | the command goes through `find_records`; no turn written; a greeting, one letter or "remember" finds nothing; "I don't know" finds nothing | PASS | `the_command_reads_the_records_and_writes_nothing`, `words_that_ask_for_nothing_find_nothing`, `what_no_record_holds_finds_nothing` |
| 2 | Suggested entries | empty query: Suggested holds What needs me, What finished today, Runway by account, What’s running; picking one searches its words in Search; each finds its records | PASS | vitest "Suggested holds…", "a Suggested entry searches its words here…"; `each_suggested_entry_finds_its_records`; `_gallery/99-palette/empty-*.png` |
| 3 | Watch a session… | a command under Actions on the empty query and found by typing "watch"; opens Watches' form | PASS | vitest "Watch a session… is a command under Actions…", "typing “watch” finds Watch a session…" |
| 4 | One search, no duplicate | Bot's no-AI panel: no box, no results, no shortcuts, no "Or go straight to", no "Watch a session…"; ⌘K holds them once | PASS | bot.test.tsx "no AI ready (D-144): only Set up Bot…" (4 states); records de-duplicated against Sessions and Accounts |

## Usability answers (PAT-USE), matched on the built screen

1. Who/expect: ⌘K finds sessions, tasks, pools and usage; Bot is conversation only. Matches (records groups; Bot's no-AI panel has none).
2. Empty query: the four suggested entries and Watch a session…, each named plainly. Matches (`empty-*`).
3. One search, no second one in Bot. Matches (R4).
4. No results: "No matches for “…”.", "Looked in sessions, tasks, pools, usage, apps, projects, accounts and settings.", then Suggested; no setup state (Search needs no AI). Matches (`no-matches-*`).
5. None: no OS prompt. Matches.
6. Watch-fors: a record shown twice (de-duplicated, tested); Search and Bot both offering look-ups (Bot has none, tested); a shortcut leading to an empty page (Suggested stays in Search and says "No matches" with where it looked). Matches.

## Design critique

`docs/qa/evidence/_gallery/99-palette/critique.md`, section "0.5.1: Search takes over Bot's look-ups" (stamped).

## Checks run

- `cargo test -p juggler-app --lib bot::search` (11 passed), `command_types` (bridge routes, commands.ts), clippy `-p juggler-app --all-targets -D warnings` clean, rustfmt on the new files.
- vitest: palette, ⌘K and Bot tests; tsc clean for these files; `ui_lint.py` no finding in the touched files.
- `plain_words_lint`, `private_data_lint`, `png_metadata_lint`, `design_tokens_lint`, `critique_check`, `terminology_lint`: OK.
- Playwright `tests/visual/palette-gallery.spec.ts` (12 passed, axe gate at 1280).
