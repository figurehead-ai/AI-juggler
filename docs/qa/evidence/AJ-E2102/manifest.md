# Evidence manifest — AJ-E2102

Task: `docs/tasks/E21-desktop-first.md` AJ-E2102, Go to it. Branch
`e21-step1`. Status: **IN-PROGRESS**: Reqs 1–5 are built and tested
below; Req.6 (a session on another Mac, "Bring forward on <Mac>",
AJ-E711) is not in this step, and no live click was made on the
maintainer's Mac (the walk is the maintainer's).

## What exists now

- `conversations_go_to(account_id, conversation_id, pid?)`
  (`features/conversations/goto.rs`, async through `ipc::blocking`,
  answering within 1.5 s): any open session, AI Juggler's or not.
- The app comes forward by **its own process**:
  `NSRunningApplication::runningApplicationWithProcessIdentifier` +
  `activateWithOptions` (safe objc2 calls, no permission), after its start
  time is checked (a reused pid is never brought forward). Never `open -b`.
- Then what the app allows, from the host registry (data: the adapters'
  `host_apps()` and `features/conversations/hosts.json`): a Codex thread
  by `codex://threads/<id>`; an editor's folder by `cursor://file/<folder>`
  or `code <folder>` (never an editor extension's session link); a
  Terminal or iTerm tab by its `tty` only when Settings › Sessions' "Go to
  a session's own tab in Terminal and iTerm" is on (off by default); the
  window count and "minimised — click it in the Dock" only when window
  focus is on in Settings (Accessibility + System Events, asked for only
  from Settings).
- The outcome names the step: "Brought Claude · Work forward; pick the
  session in its sidebar", "(3 windows)", "Claude · Work is minimised —
  click it in the Dock", "Opened the thread in ChatGPT", "Brought Terminal
  forward and selected its tab", "… isn't open now", "Still bringing …
  forward…" after 1.5 s.
- `desktop_focus` is async with the same limit, activates by pid, and
  runs the System Events tiers only with window focus on.
- UI: an open row's click and Enter go to it (a toast says "Going to it…"
  at once, then the outcome); Go to it leads its menu and the go-to
  palette.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 by process, never bundle id | `goto::tests::a_claude_instance_comes_forward_by_its_process_and_says_where_to_look`; `focus::tests::without_window_focus_only_the_process_is_activated` | activation by pid; nothing else runs | pass | PASS | 0f77987e | 2026-10-24 | agent |
| Req.2 off the main thread, 1.5 s limit, progress at once | `goto::tests::a_slow_step_answers_within_the_limit`; `ipc_audit.py` (desktop_focus left the sync list); the "Going to it…" toast | answers within the limit | pass | PASS | 0f77987e | 2026-10-24 | agent |
| Req.3 no prompt from a click | `goto::tests::window_counts_and_minimised_need_window_focus_turned_on`, `a_terminal_tab_is_selected_only_when_tabs_are_on`; `ui/src/components/terminal-tabs.test.tsx` | no AppleScript without the setting; the setting is off by default | pass | PASS | 6de05df9 | 2026-10-24 | agent |
| Req.4 honest outcome | `goto::tests::*` (7 tests) | each outcome's words | pass | PASS | 0f77987e | 2026-10-24 | agent |
| Req.5 links | `goto::tests::a_codex_thread_opens_by_its_link`, `an_editor_opens_the_folder_and_never_an_extension_link`, `the_registry_has_no_extension_session_link` | `codex://threads/<id>`, `cursor://file/<folder>`, `code <folder>` | pass | PASS | 0f77987e | 2026-10-24 | agent |
| PID reuse in focus | `WindowIdentity.start_time` (`storage_roundtrip::session_window_identity_roundtrip_and_degrade`) | a reused pid reads "not running" | pass | PASS | 3cd91602 | 2026-10-24 | agent |
| UI: primary and Enter | `ui/src/components/outside-conversations.test.tsx` "an open one's click goes to it", "Go to it leads an open one's menu" | click/Enter on an open row goes to it | pass | PASS | 525bf197 | 2026-10-24 | agent |
| Req.6 another Mac | — | — | not in this step | UNPROVEN | — | 2026-10-24 | agent |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none)
