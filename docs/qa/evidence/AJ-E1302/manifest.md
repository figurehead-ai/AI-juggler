# Evidence manifest — AJ-E1302

Task: `docs/tasks/E13-codex-adapter.md` AJ-E1302 (find and bring in
existing Codex setups). Status: **DONE-EVIDENCED** on fixtures.
2026-10-20, reviewer claude-opus.

## Basis

Codex CLI 0.163.0, source tag `rust-v0.163.0`
([research/codex.md §10](../../../research/codex.md)): what a home holds,
that `state_5.sqlite` records absolute rollout paths and is rebuilt from
`sessions/` by a filename scan with read-repair. Every fixture home is
built in a temp dir by the tests; every `auth.json` is synthetic. No test
scans the real `$HOME` with the Codex adapter (the only real-machine scans
are `#[ignore]`d operational tests).

## Requirements

| Requirement | Proof | Result |
|---|---|---|
| Req.1 scan: `~/.codex`, `$CODEX_HOME`, `~/.codex*`, config evidence only, metadata only; `CODEX_HOME=` rc targets | `codex_environment::scan_finds_default_override_and_home_patterns` (an evidence-less `~/.codex-empty` is ignored); rc aliases: `codex.rs` `scan_shell_aliases` filters `CODEX_HOME`, mapped to Codex by `codex_provider::a_codex_shell_alias_becomes_a_codex_candidate` | PASS |
| Req.2 byte copy, rebuildable state skipped (`*.sqlite*`, `.tmp/`, `log/`, `app-server-*`, `packages/`); settings-only import leaves `sessions/`, `archived_sessions/`, `history.jsonl`, `session_index.jsonl` with sizes listed | `import_copies_config_and_history_and_leaves_the_source_identical`, `settings_only_import_leaves_history_behind_and_lists_it`, `inspect_reports_sizes_without_writing` | PASS |
| Req.3 the source is byte-identical (sha256 of every file before/after) and never marked | the same tests' `snapshot` comparisons; app level `codex_provider::importing_a_codex_home_copies_it_and_leaves_the_original_alone` | PASS |
| Req.3 refusals: not a Codex home, existing destination, an AI Juggler-managed home | `import_refuses_what_it_must_not_copy` | PASS |
| Req.4 the carried sign-in says it is shared; `auth.json` stays 0600 in the copy; a Keychain-only sign-in says it does not move | `import_copies_…` (note + mode); the Keychain note is `CODEX_LAYOUT.login_stays_note`, shown when the source probes signed in and the copy does not | PASS |
| Claude's import unchanged by the shared `Layout` refactor | `claude_environment` suite (12) | PASS |

Commands: `cargo test -p juggler-adapters --test codex_environment` (6),
`--test claude_environment` (12), `--test codex_adapter` (11);
`cargo test -p juggler-app --test codex_provider` (4), `--test onboarding`
(11); `just verify` on the PR.

## Not claimed

- A real `~/.codex` import was not run (the maintainer's data). AJ-E1305.
- Codex memories and queued messages live in SQLite and do not travel
  with the copy; the import lists those files as skipped.
