# Evidence manifest — AJ-E1306

Task: `docs/tasks/E13-codex-adapter.md` AJ-E1306 (use Codex homes where
they are, ADR-0018). Status: **DONE-EVIDENCED** on fixtures and stub
binaries; a live check on a real `~/.codex` belongs to AJ-E1305.
2026-10-20, reviewer claude-opus.

## Basis

Codex `rust-v0.163.0`
(`https://github.com/openai/codex/blob/rust-v0.163.0/codex-rs/`):

- `utils/home-dir/src/lib.rs` `find_codex_home`: `CODEX_HOME` set → it
  must exist and is canonicalised; unset → `~/.codex`. So `~/.codex` in
  place launches with `CODEX_HOME` unset, as a plain `codex` does.
- `login/src/auth/storage.rs` `compute_store_key`: the Keychain item is
  `cli|<sha256(canonical home)[..16]>` whether or not the variable is
  set — the sign-in carries over either way.
- `core/src/config/mod.rs`: `sqlite_home` from `config.toml` beats
  `CODEX_SQLITE_HOME`, which beats the home; the only warning is for a
  managed requirement (`config/requirements.rs`). Pinning the variable
  to the folder is what Codex does by default and never overrides the
  user's own setting.

## Requirements

| Requirement | Proof | Result |
|---|---|---|
| Req.1 adopt: a Codex home (config, `auth.json` or sessions) is accepted with its sign-in from `auth.json`'s shape; our own copy, a non-home and a missing folder are refused; nothing is written | `juggler-adapters` `codex_adapter::adopting_a_codex_home_checks_it_and_writes_nothing`; `juggler-app` `codex_in_place::ai_juggler_s_own_codex_copy_or_a_non_home_cannot_be_used_in_place` | PASS |
| Req.2 exact env: another home → `CODEX_HOME` and `CODEX_SQLITE_HOME` are the folder exactly; `~/.codex` → both unset; inherited `OPENAI_API_KEY` unset; the copyable command matches | `codex_in_place::a_codex_home_used_where_it_is_is_never_written_by_ai_juggler`, `codex_s_default_home_runs_with_its_variables_unset` (stub records the env); `codex_adapter::the_default_home_runs_with_its_variables_unset` (snippet `env -u CODEX_HOME -u CODEX_SQLITE_HOME -u CODEX_API_KEY -u OPENAI_API_KEY codex`) | PASS |
| Req.3 nothing written: the home's snapshot (every path's size and mtime, the root's own entry too) is identical across bring-in, a launch with a model (a flag), the usage pass, the checklist and a second bring-in attempt; a desktop launch never prepares (marks) an in-place folder and refuses a gone one instead of recreating it | `codex_in_place` (snapshots in all four tests); `desktop::a_desktop_launch_never_marks_a_folder_used_where_it_is` | PASS |
| Req.4 per-tool choice: Codex setups get "Use them where they are" (worded for Codex), in place by default; a failed Codex copy offers "Use it where it is instead"; a Codex copy offers "Use the original instead" (the same command) and the original stays untouched | vitest `first-run.test.tsx` (Codex block, 4 rewritten), `providers.test.ts`; `codex_in_place::a_codex_copy_can_switch_to_its_original_home`; Playwright `accounts — a Codex copy switches to its original home` (gallery `68-accounts/use-original-codex-1280x800-{dark,light}.png`); hand walk in the preview (Codex-only first run; mixed machine, failed Codex copy → in place) | PASS |
| Req.5 the checklist is honest per Provider: "CODEX_HOME and CODEX_SQLITE_HOME are this folder, exactly" / "the default folder runs as Codex runs it (… unset)"; Claude's rows unchanged in substance | `codex_in_place` (both env rows asserted); `in_place_accounts` (5) | PASS |

Gates (2026-10-20, this branch): vitest 533; `tsc -b`; build; full
Playwright 874; `./scripts/a11y_gate.sh` PASS (0 critical/serious);
`./scripts/visual_gate.sh` 73/73; `cargo test -p juggler-app --test
codex_in_place --test desktop --test in_place_accounts`,
`-p juggler-adapters --test codex_adapter`. Clippy found one `cmp_owned`
in the new helper (fixed); the workspace clippy (both feature sets) and
`just verify` run on the PR's CI — the local disk was under the 20 GB
floor.

## Not claimed

- No real Codex binary or home was used (fixtures and a stub). That a
  session started in place shows in `codex resume` outside AI Juggler is
  an AJ-E1305 live check.
- A copy whose copied `config.toml` sets `sqlite_home` would still point
  at the original's database (config beats the variable) — a copy-path
  question, not this task's; noted for follow-up.
