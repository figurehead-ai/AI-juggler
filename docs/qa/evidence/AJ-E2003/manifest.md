# Evidence manifest — AJ-E2003

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2003, the webview
lockdown. Branch `release/0.4.1` (D-83). Status: **DONE-EVIDENCED** (agent
self-check, 2026-11-06). Every check below was run on 2026-11-06 at commit
9415f2442 or later, and passed.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | `build.rs` generates each command's permission from the `app_commands!` registry in `src/lib.rs` (renames kept). The main window's set is `main-window`; a terminal window's is `permissions/terminal-window.toml`, granted by `capabilities/*.json` per window label. `tests/webview_acl.rs` builds the real capability files on Tauri's mock runtime: `the_main_window_may_call_the_registered_commands`, `a_terminal_window_may_call_only_its_own`, `an_unknown_window_may_call_nothing`, `the_dev_server_page_counts_as_the_apps_own` (all pass). |
| R2 | PASS | `scripts/webview_lint.py` (in `just verify` and CI's static job; its `--selftest` passes) fails on these. In a capability file: a shell, fs, http or opener permission, a plugin's whole default set, `core:default`, a remote URL or a `"*"` window. In the CSP: a lost directive. Elsewhere: remote IPC, a window from the config, a browser opened outside `webview_guard.rs`, or a page `invoke` the main window can't call. |
| R3 | PASS | Every window is built in `src/webview_guard.rs` with `on_navigation(page_allowed)` (only the app's own origin, plus the dev server in a Dev build) and `on_new_window(… Deny)`. Outside links go through the Rust opener: the app opens only https; a link the user clicked opens any http(s) address and nothing else (`webview_guard::tests::a_window_stays_on_the_apps_own_pages`, `a_clicked_link_opens_any_web_address_and_nothing_else`, `the_app_opens_only_https`). The terminal's links use xterm's explicit handler (`TerminalView.tsx`: `WebLinksAddon` with ⌘/Ctrl-click, then `openLink`). |
| R4 | PASS | `tauri.conf.json`'s CSP: `default-src 'self'; script-src 'self'; … object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'`. The Dev CSP adds only the Vite server and its inline bootstrap. The lint fails if a required directive is lost. |
| R5 | PASS | `core:default` is gone. The main window has only what the page uses: event listen/unlisten, the native menu (new, append, popup), resource close, window dragging and the title-bar maximize. A terminal window has events only. The page has no dialog permission: pickers open from Rust (`pick_path`). |
| R6 | PASS | Commands take ids, or a path the user chose. Each command with a path-like argument asks `picked::vouch` whether the path was picked in a native dialog (by kind), dropped, offered by AI Juggler, or known to it (`picked::tests::*`, 4 pass). `webview_lint.py` fails any new path argument that does neither, unless `scripts/path_params.allow` names it with its reason (9 entries: ids, keep or trash words, and paths checked against AI Juggler's own records). |

Hand check: not done in a running window. A fresh AI Juggler Dev build
asks macOS for its keychain item on every read, and these walks never
press Allow. The mock-runtime tests above use the app's real capability
files and CSP, so the access list itself is what's tested. The click-through
(a link in a transcript, ⌘-click in a terminal, a dropped folder, a picked
backup) is folded into the 0.4.1 walk that needs the maintainer's keychain
Allow.
