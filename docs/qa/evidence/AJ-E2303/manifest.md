# Evidence manifest — AJ-E2303

Task: `docs/tasks/E23-session-runtime.md` AJ-E2303, Codex through app-server, by default. Branch `e23-runtime`. Status: **DONE on fakes**, and the installed Codex 0.163.0 passes the surface check offline; live parts are AJ-E2301's.

## What exists now

`crates/juggler-adapters/src/codex_appserver/` (`rpc.rs`, `codec.rs`, `asks.rs`, `session.rs`, `surface.rs`); `src/bin/fake_codex.rs`, `src/bin/support/app_server.rs`; `tests/fixtures/codex-app-server/`; `scripts/codex_app_server_surface.py`.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 one stdio server per session, owned; never ephemeral; windows by length; Codex's own decisions | `cargo test -p juggler-adapters --test codex_app_server` (`a_codex_session_is_driven_end_to_end_through_its_app_server`) | start, turn, approval, question, limits, interrupt, steer, compact, release | pass | PASS | f659cf83 | 2026-10-25 | agent |
| Req.2 honest clientInfo; another codexHome refused | `cargo test -p juggler-adapters --test codex_app_server` (`every_request_the_codec_sends_has_its_required_fields`, `a_server_on_another_home_is_refused`) | `ai_juggler` / `AI Juggler`; refused | pass | PASS | 1c593149 | 2026-10-25 | agent |
| Req.3 surface pinned from 0.163.0, checked at discovery | `cargo test -p juggler-adapters --test codex_app_server` (`the_recorded_surface_has_everything_the_codec_uses`, `only_a_pinned_protocol_is_driven`); `--ignored the_installed_codex_has_the_pinned_surface` with a scratch CODEX_HOME | no gap | pass; the installed 0.163.0 has no gap | PASS | 1c593149 | 2026-10-25 | agent |
| Req.4–5 the mapping; the tool's own options only | `cargo test -p juggler-adapters --test codex_app_server` (`recorded_messages_make_the_right_events_and_answers`, `answers_are_codexs_shapes_and_only_to_open_asks`) | fixtures → events; answers only to open asks | pass | PASS | f659cf83 | 2026-10-25 | agent |
| Default for Codex sessions in the app | `cargo test -p juggler-app --test runtime` (`codex_runs_through_its_own_app_server_by_default`) | driven, exact state | pass | PASS | 4749d6fc | 2026-10-25 | agent |
| No bypass or inherited key in the launch | `cargo test -p juggler-adapters --test codex_app_server` (`the_driven_launch_never_carries_a_bypass_or_an_inherited_key`) | none | pass | PASS | 1c593149 | 2026-10-25 | agent |
| Live: a real approval; limits before the first turn | [lab 2](docs/qa/evidence/AJ-E2301/lab-2-2026-10-25.md), "Codex" (by hand in the codec's shapes) | live | `account/rateLimits/read` before any turn: weekly only (`windowDurationMins` 10080) with its reset, no 5-hour; `requestApproval` with `availableDecisions` [accept, acceptWithExecpolicyAmendment, cancel]; `accept` carried the turn on | PASS | 2414a4bd | 2026-10-25 | agent |
| Live: a question | [lab 2](docs/qa/evidence/AJ-E2301/lab-2-2026-10-25.md), "Codex" | `item/tool/requestUserInput` or text | plain text on 0.163.0 with `experimentalApi: false` (no requestUserInput) | PASS | 2414a4bd | 2026-10-25 | agent |
| Live: the approval's summary names the command | [lab 2](docs/qa/evidence/AJ-E2301/lab-2-2026-10-25.md), bug 2 | "ls" | Codex sends `command: "/bin/zsh -lc ls"` (the real command is in `commandActions`); `command_ask` would show the wrapper | FAIL | 2414a4bd | 2026-10-25 | agent |
| Live: in the app (the session page, Needs you, the audit) | docs/qa/runtime-lab.md step 1 | seen in the app | BLOCKED: AI Juggler Dev not built (disk < 25 GB), no desktop control | UNPROVEN | — | 2026-10-25 | agent |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
