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
| Req.4 (revised after review) `auth.json` is never copied — a ChatGPT refresh token is single-use: codex-rs `rust-v0.163.0` `login/src/auth/manager.rs:207` ("your refresh token was already used. Please log out and sign in again"), `:1688` (`refresh_token_reused`), `:1604-1618` (a refresh stores the NEW refresh token); the skip is listed with its reason, the copy reads signed out, the note says it signs in once | `import_copies_config_and_history_and_leaves_the_source_identical`, `settings_only_import_…`; app `codex_provider::importing_a_codex_home_…` (`logged_out`, no `auth.json` in the managed home) | PASS |
| Req.5 links are listed and never followed (a link to a folder inside, a folder outside, a file outside, a dangling link: none copied, no target bytes in the copy, import succeeds); also for Claude imports | `codex_environment::links_are_listed_and_never_followed`, `claude_environment::claude_import_skips_links_instead_of_failing_or_following` | PASS |
| Req.5 Codex skip names apply to the top level only (`skills/x/packages/`, `log/`, `tmp/`, `*.sqlite` inside a skill are copied) | `skip_names_apply_to_the_top_level_only` | PASS |
| Scan size is exact (files only, never a folder's own size, links skipped) | `scan_finds_default_override_and_home_patterns` asserts the byte count | PASS |
| A failed prepare never removes a folder it did not create (and removes one it did) | `claude_environment::a_failed_prepare_never_removes_a_folder_it_did_not_create` | PASS |
| Claude's import unchanged by the shared `Layout` refactor | `claude_environment` suite (12) | PASS |

Commands: `cargo test -p juggler-adapters --test codex_environment` (6),
`--test claude_environment` (12), `--test codex_adapter` (11);
`cargo test -p juggler-app --test codex_provider` (4), `--test onboarding`
(11); `just verify` on the PR.

## Review (2026-10-20)

The first version copied `auth.json` and called the result "shared";
review found ChatGPT refresh tokens are single-use, so sharing would sign
one home out — now kept back. The same review found the shared copy walk
followed symlinks (a linked folder failed the import, a link to a file
outside was copied in — Claude imports too), skip names applied at every
depth, a vacuous size assertion, and a prepare that could leave an empty
folder behind. All fixed above; AJ-E806 ledger records it.

## Not claimed

- A real `~/.codex` import was not run (the maintainer's data). AJ-E1305.
- Codex memories and queued messages live in SQLite and do not travel
  with the copy; the import lists those files as skipped.
