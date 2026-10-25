# Evidence manifest — AJ-E2308

Task: `docs/tasks/E23-session-runtime.md` AJ-E2308, Run without a terminal: answer from AI Juggler (Beta). Branch `e23-runtime`. Status: **DONE on fakes**, off by default, per session. The lab showed it viable (2026-10-25); its live checks are AJ-E2301 Req.3–4.

## What exists now

`crates/juggler-adapters/src/claude_stream/` (`codec.rs`, `session.rs`, `mod.rs`); `src/bin/fake_claude.rs`, `src/bin/support/stream.rs`; `tests/fixtures/claude-stream/`.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 stream-json with `--verbose` and stdio prompts | `cargo test -p juggler-adapters --test claude_stream` (`recorded_messages_make_the_right_events`, `answers_go_back_in_claude_codes_shapes`, `a_headless_claude_session_is_driven_end_to_end`); `cargo test -p juggler-adapters --test adapter_conformance` (flag rule `stream-json` → `--verbose`) | fixtures → events; answers in Claude Code's shapes | pass | PASS | 186cbe75 | 2026-10-25 | agent |
| Req.2 hand back with `--resume`, one writer | `cargo test -p juggler-app --test runtime` (`claude_runs_without_a_terminal_only_as_a_per_session_beta`) | handed over; one writer | pass | PASS | 74324796 | 2026-10-25 | agent |
| Req.3 no forbidden flag; only a trusted folder; never spoofed | `cargo test -p juggler-adapters --test claude_stream` (`the_fake_refuses_what_claude_code_refuses`, `only_in_a_trusted_folder_and_a_new_enough_claude_code`); `cargo test -p juggler-app --test runtime_guardrails` (`no_launch_bypasses_a_tools_prompts`) | refused | pass | PASS | 74324796 | 2026-10-25 | agent |
| Req.4 off by default, Beta, per session; the four gates written | `cargo test -p juggler-app --test runtime` (same test); `docs/adr/0022-session-runtime.md` decision 4; `docs/guide/sessions-and-the-terminal.md` | terminal unless picked with the switch on | pass | PASS | 74324796 | 2026-10-25 | agent |
| Live: initialize, an approval, a question, set_model, a rate_limit_event | [lab 2](docs/qa/evidence/AJ-E2301/lab-2-2026-10-25.md), "Claude Code without a terminal" (the codec's launch and shapes, by hand) | live | `permissionMode: default`, `apiKeySource: none`; Allow once and the AskUserQuestion answer carried on; `set_model` switched the model; one five-hour `rate_limit_event` per turn, utilization a fraction | PASS | 2414a4bd | 2026-10-25 | agent |
| Live: the round trip to the terminal and back | [lab 2](docs/qa/evidence/AJ-E2301/lab-2-2026-10-25.md) | one transcript | one file, one `sessionId`, sdk-cli → cli → sdk-cli in order | PASS | 2414a4bd | 2026-10-25 | agent |
| Live: an "add directory" suggestion's label | [lab 2](docs/qa/evidence/AJ-E2301/lab-2-2026-10-25.md), bug 3 | names the folder | `rule_words` gives "Always allow this, for this session" for `addDirectories` | FAIL | 2414a4bd | 2026-10-25 | agent |
| Live: from the app (the Beta switch, the session page) | docs/qa/runtime-lab.md step 4 | seen in the app | BLOCKED: AI Juggler Dev not built (disk < 25 GB), no desktop control | UNPROVEN | — | 2026-10-25 | agent |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
