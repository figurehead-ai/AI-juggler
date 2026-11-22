# Evidence manifest — AJ-E1610 Desktop apps: open, keep alive, focus, and reply where a route exists

Task: `docs/tasks/E16-more-tools.md` AJ-E1610. Built on `release/0.5`,
2026-11-15. Status: **IN-PROGRESS** — built and proved on fixtures; every
live part is PENDING on the lab (AJ-E1612). Research basis:
`docs/research/desktop-apps.md` and the desktop sections of `cursor.md`,
`kiro.md`, `antigravity.md` (all 2026-11-15).

No real app was launched, driven or typed into for this task: the tests
use made-up bundles (an `Info.plist` and an empty program), made-up
processes, a starter that records and a desktop that only records links.
The only app installed on the build Mac in scope (the Claude app) was not
touched.

## What was built

- One table of the AI desktop apps, `crates/juggler-adapters/src/desktop_apps.json`
  (`desktop_apps.rs`): the Claude app, the Codex app, Cursor, Kiro IDE,
  the Antigravity app and Antigravity IDE, OpenCode, Kimi Code and Hermes,
  each with its bundle id (or, for Kimi Code, its own state folder), its
  routes and how sure each is (`documented`, `live_checked`,
  `unverified`), its track-only parts with one plain line each, and
  whether its tool may be driven. Rows marked `"seen": "source"` were
  never read from an installed bundle: confirm on a real install.
- The seam, `juggler_adapter::desktop_app` (`DesktopFacts` on `HostApp`):
  only a documented or live-checked route is ever used; an unverified one
  is declared and Go to it falls back to plain focus.
- Each adapter returns its own app (`host_apps`: Claude Code, Codex,
  OpenCode, Kimi Code, Hermes); the host registry adds the rows of tools
  with no adapter yet (Cursor, Kiro, Antigravity). No per-tool branch
  outside the adapters.
- Detection by bundle id: an app bundle the registry doesn't know by its
  folder name is matched by the `CFBundleIdentifier` its own XML
  `Info.plist` declares (a renamed app, a beta build), read once a run.
- An editor that is also an AI app (Cursor, Kiro IDE, Antigravity IDE) is
  an App (offered, opened, kept alive, focused) while a session in its
  terminal keeps the editor's Go to it; it is not measured as a tool tree
  on This Mac, because the CLIs in its terminal would then count twice.
- "Open here" (`places_open` with an action): the app's own documented link for a
  new chat or a new session in a folder, sent to the running profile's
  process alone, never asking macOS for consent from there; a folder must
  be one the user picked or a project's (`picked::work_folder`).
- The App page's "What works from here" section: those doors, and each
  track-only line (`AppReach.tsx`).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 detected by bundle id: each app found running by its bundle, or by the bundle id its `Info.plist` declares when the folder is renamed; Kimi Code by its bundle name, its own folder as the install sign | cargo `juggler-app` `features::places::desktop_apps_tests::each_desktop_app_is_found_by_its_bundle_or_its_bundle_id`, `kimi_code_is_known_installed_by_its_own_folder_alone`; `juggler-adapters` `desktop_apps::tests::each_app_is_found_by_its_bundle_id_or_its_own_folder`, `the_table_reads_and_every_id_is_once`, `kimi_code_is_found_by_its_own_folder_which_is_never_written`; `juggler-platform` `apps::tests::a_bundles_id_is_read_from_its_xml_plist_and_a_strange_one_is_not` | all 8 single-instance apps found from renamed bundles, each once, by its main process alone; an unknown id is no App | pass on fixtures | PASS | a5e2acc60 | 2026-11-15 | Claude (self) |
| R1 offered when seen running, added only by the user (D-84) | `desktop_apps_tests::each_desktop_app_seen_running_is_offered_never_added_and_once_added_opens_and_keeps_alive` | each offered, none listed as an App, none kept alive before Add; after Add an App whose tool is the table's | pass on fixtures | PASS | a5e2acc60 | 2026-11-15 | Claude (self) |
| R1 the real bundle ids of Cursor, Kiro IDE, Antigravity app and IDE, OpenCode, Hermes; Kimi Code's bundle name | Lab (AJ-E1612): with each installed in the lab, read `Contents/Info.plist` `CFBundleIdentifier` and the bundle's name; start it and see it offered in AI Juggler Dev | the table's ids match | not run (not installed on the build Mac; installing waits for the maintainer's yes, D-107) | PENDING | — | — | — |
| R2 Open, Keep alive and focus through the existing desktop contract, for every app | `desktop_apps_tests::each_desktop_app_seen_running_is_offered_never_added_and_once_added_opens_and_keeps_alive` (Open starts the app's own bundle, no arguments, no variables, one instance; Keep alive's plan starts the same bundle behind); the existing Go to it and focus tests (`goto/tests.rs`, `focus.rs`) cover the window by its pid | its own bundle only, never a profile switch it doesn't document | pass on fixtures | PASS | a5e2acc60 | 2026-11-15 | Claude (self) |
| R2 a profile per account only where proved (the Claude app's `--user-data-dir`) | `desktop_apps::tests::only_the_claude_app_opens_per_account_and_the_rest_say_why`; conformance `describes_its_desktop_apps_honestly` (10 subjects) | only the Claude app has an instance switch; every other app says why in one line | pass | PASS | a5e2acc60 | 2026-11-15 | Claude (self) |
| R2 sign-ins never read, copied or touched; no app's data folder written | `desktop_apps_tests::each_desktop_app_seen_running_is_offered_never_added_and_once_added_opens_and_keeps_alive` (every bundle file byte-identical after detection, Add, Open and Keep alive); `kimi_code_is_found_by_its_own_folder_which_is_never_written` (the state folder is looked at, its entries unchanged); the existing guardrail `procs::refresh_kind` (no process environment read) | no write outside AI Juggler's own data folder | pass on fixtures | PASS | a5e2acc60 | 2026-11-15 | Claude (self) |
| R2 live: Open, Keep alive and focus each app in a lab profile; a second Cursor, Kiro or Antigravity window by `--user-data-dir` keeps its own sign-in | Lab (AJ-E1612) per `docs/research/desktop-apps.md` § 8 and the three tools' notes | each opens, restarts when quit, comes forward by its pid | not run | PENDING | — | — | — |
| R3 sessions through the sound routes, the CLI adapters' readers, no copies | data: OpenCode `default_folder` `.local/share/opencode`, Kimi Code `.kimi-code`, Hermes `.hermes` link each app to the Account of that folder (ADR-0032, the Codex app's rule), whose sessions the adapter already reads; the Claude app's records as before (`claude_live`); `desktop_apps::tests::replies_go_only_through_an_adapter_and_never_install_anything` (every read route goes through the adapter, no link) | reuse only | pass | PASS | a5e2acc60 | 2026-11-15 | Claude (self) |
| R3 replies only through sound routes: Hermes through its gateway (the Hermes adapter); OpenCode's declared but unverified | `desktop_apps::tests::replies_go_only_through_an_adapter_and_never_install_anything` (only Hermes has a usable reply route; every other app has a reply reason; no link contains `install` or `plugin`, so never `hermes://skill/install`); conformance `describes_its_desktop_apps_honestly` | exactly Hermes | pass. The reply box itself is AJ-E3403's (not built on this branch): this task gives it the per-app flag | PASS | a5e2acc60 | 2026-11-15 | Claude (self) |
| R3 the Claude app: documented links as "Open here"; `code/continue` and `needs-input` declared unverified and never sent | `places::reach::tests::the_claude_app_opens_a_new_chat_and_a_folder_here_and_declares_the_rest`, `open_here_sends_only_a_new_chat_or_a_folder`; `desktop_apps::tests::the_claude_apps_undocumented_links_are_declared_but_never_sent`; `desktop_apps_tests::open_here_sends_only_a_documented_link_to_the_apps_own_process` (sent to the app's own pid; unproved and session actions refused); `desktop_apps_tests::the_claude_apps_unproved_links_fall_back_to_plain_focus` | new chat, chat by id and a folder usable; continue and needs-input never | pass | PASS | a5e2acc60 | 2026-11-15 | Claude (self) |
| R3 the epitaxy session link stays in use | data: `open_session` is `live_checked`, from the AJ-E2102 live row of 2026-11-07 ("Opened the session in Claude app · aij-test"), so Go to it keeps sending it; with the route marked unverified it isn't sent (`the_claude_apps_unproved_links_fall_back_to_plain_focus`) | as AJ-E2102 | pass | PASS | a5e2acc60 | 2026-11-15 | Claude (self) |
| R3 live: `claude://code/continue?session=local_<id>` and `needs-input` in a lab profile; OpenCode, Kimi and Hermes windows show sessions the adapter reads; a Hermes gateway reply shows in its window | Lab (AJ-E1612), `docs/research/desktop-apps.md` § 8 items 1–4 | each opens the right page, or stays unverified | not run | PENDING | — | — | — |
| R4 every part with no sound route is track-only and its page says why in one line | `desktop_apps::tests::every_track_only_reason_is_one_plain_line` (ends with a full stop, one line, at most 140 characters, no internal words); `reach::tests::every_desktop_app_says_what_it_allows_and_a_terminal_nothing`; vitest `ui/src/pages/app/app-reach.test.tsx` (the lines rendered, the doors only while it runs, nothing for an app that declares nothing) | each app's lines rendered as written | pass | PASS | a5e2acc60 | 2026-11-15 | Claude (self) |
| R4 Antigravity consumer accounts: no driven runtime, no automatic switching | `desktop_apps::tests::antigravity_is_never_driven_on_a_consumer_plan`; `desktop_app::tests::a_consumer_plan_is_never_driven_nor_switched_automatically_where_the_tool_says_so`; `desktop_apps_tests::each_desktop_app_seen_running_is_offered_never_added_and_once_added_opens_and_keeps_alive` (the Antigravity app's and IDE's Open and Keep alive start only their own bundles: never `agy`, `-p` or `acp`) | the policy says no on a consumer plan; the desktop path starts nothing else | pass. No automatic switcher exists on this branch; AJ-E1613's runtime and any future one must ask `may_drive` / `may_switch_automatically` | PASS | a5e2acc60 | 2026-11-15 | Claude (self) |
| Visual: the App page's "What works from here" | gallery capture of a desktop App's Overview in both themes | calm (D-117): ghost buttons, grey lines with a quiet glyph | not captured in this batch | PENDING | — | — | — |

## Live lab steps (AJ-E1612)

1. Claude app (installed): in a lab profile, send `claude://code/continue?session=local_<id>` and `claude://code/needs-input` to its pid; mark each `live_checked` only if the right page opens.
2. Install (with the maintainer's yes) Cursor, Kiro IDE, Antigravity app and IDE, OpenCode, Kimi Code and Hermes in the lab; read each `Info.plist`; confirm the table's ids and Kimi Code's bundle name; see each offered, add it, Open, quit it with Keep alive on, Go to it.
3. Cursor, Kiro and Antigravity: a second window by `--user-data-dir` with its own sign-in; only then add the switch to the table.
4. OpenCode: whether its window shows a session another server wrote (decides its reply route). Hermes: a gateway reply shows in its window.
