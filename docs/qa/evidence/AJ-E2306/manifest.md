# Evidence manifest — AJ-E2306

Task: `docs/tasks/E23-session-runtime.md` AJ-E2306, Claude terminal sessions with hooks and the status line. Branch `e23-runtime`. Status: **DONE on fakes**: the per-launch `--settings` overlay, `juggler-agent hook`, the report key and exact states. The merge with the user's own hooks is AJ-E2301 Req.2.

## What exists now

`crates/juggler-adapters/src/claude_hooks.rs`; `crates/juggler-agentbridge/src/hook.rs`, `wire.rs`; `crates/juggler-app/src/features/runtime/reports.rs`; `features/agent_bridge/calls.rs`.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 per-launch settings in AI Juggler's folder, folding the account's status line | `cargo test -p juggler-adapters --lib claude_hooks` (`the_overlay_settings_are_folded_in_once`, `a_settings_file_elsewhere_is_left_alone`) | one `--settings`, nothing in the setup | pass | PASS | 3840317a | 2026-10-25 | agent |
| Req.2 the hook reads only its fields, prints nothing, exits 0 within 2 s | `cargo test -p juggler-agentbridge --lib hook`; `cargo test -p juggler-agentbridge --test reports` (`the_hook_command_is_silent_and_never_fails`) | silent, exit 0 | pass | PASS | 3840317a | 2026-10-25 | agent |
| Req.3 its own per-launch key; a stranger's refused | `cargo test -p juggler-agentbridge --test reports` (`a_report_with_a_stranger_key_is_refused`) | refused | pass | PASS | 3840317a | 2026-10-25 | agent |
| Req.4 exact states end to end through the real hook command | `cargo test -p juggler-agentbridge --test reports` (`the_tools_own_hooks_make_the_state_exact`) | Asks you, Working, Idle, ended | pass | PASS | 3840317a | 2026-10-25 | agent |
| Req.4 kit: state reports added honestly | `cargo test -p juggler-adapters --test adapter_conformance` (`adds_state_reports_honestly`) | only added args checked | pass | PASS | 3840317a | 2026-10-25 | agent |
| Req.5 hooks merge with the project's; PermissionRequest timing | [lab 2](docs/qa/evidence/AJ-E2301/lab-2-2026-10-25.md), "Claude Code in the terminal" | both run; the dialog doesn't wait | both `UserPromptSubmit` hooks ran; the dialog was drawn 53 ms before the 3 s hook started; SessionStart didn't run on a first start trusted after launch (ran next time); `/hooks` lists only the project's | PASS | 2414a4bd | 2026-10-25 | agent |
| Req.5 a launch that carries the user's own `--settings` | [lab 2](docs/qa/evidence/AJ-E2301/lab-2-2026-10-25.md), bug 1 | both apply | Claude Code applies only the last `--settings`; `injection` puts the overlay first, so its hooks are lost | FAIL | 2414a4bd | 2026-10-25 | agent |
| Req.5 merge with the account's own hooks and status line | [lab 2](docs/qa/evidence/AJ-E2301/lab-2-2026-10-25.md) | both run | not observable: the lab profile has no `settings.json` (key names checked) | UNPROVEN | — | 2026-10-25 | agent |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
