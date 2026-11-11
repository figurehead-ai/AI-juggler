# Evidence manifest — AJ-E2013

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2013, tracing with
redaction. Branch `release/0.4.1` (D-83). Status: **DONE-EVIDENCED**
(agent self-check, 2026-11-06).

The task has one requirement paragraph; its parts are numbered here.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | Library crates use `tracing` only: the supervisor's and statistics' `eprintln!` became `tracing::warn!`; workspace lints `print_stdout`, `print_stderr`, `dbg_macro` = deny (`Cargo.toml`), with `allow-print-in-tests` (`clippy.toml`). Command-line programs (examples, the fake tools, `juggler-agent`) and test helpers allow printing at file level, each with its reason. |
| R2 | PASS | One subscriber (`debug_log::install`): the debug-mode layer and `app_log::Layer`, which writes every info-or-above event, scrubbed (home folder, user name, keys, secret shapes), to `logs/app.log` in the data folder, owner-only, rolling to `logs/app.1.log` past 4 MiB; a Dev build also shows it in its terminal. The log goes into the diagnostics zip ("The app's log"). |
| R3 | PASS | `eprintln!` replaced: the app's go through `tracing`; what runs before the log exists (the data folder, the single-instance hand-over, a fatal start) or in AI Juggler's own log-less processes (the watchdog, the alert) goes through the one allowed `early_note`. |
| R4 | PASS | Spans: every command's work runs in a `command` span named for it (`ipc::blocking`), which also logs the command's error; the usage ingest pass runs in its own span. Log lines carry their span path (`command>…`). |
| R5 | PASS | The quality gate, a redaction test over the log output: `app_log::tests::what_reaches_the_file_is_scrubbed_and_inside_its_command` (an event with a key written through the real layer; the file keeps the span and the message, never the key or debug lines), `a_line_never_keeps_a_secret_or_the_home_folder`. It found a GitHub token passing the redaction engine: `juggler_handover::redaction` now finds GitHub, GitLab, Slack, AWS and Google token shapes too (`well_known_tokens_are_found_where_a_word_starts`), which also protects context packs and backups. |
