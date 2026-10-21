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
These come from reading the source at that tag, not from a live run.

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
| Req.3 nothing written: the home's snapshot (every path's size and mtime, the root's own entry too) is identical across bring-in, a launch with a model (a flag), the usage pass, the checklist and a second bring-in attempt | `codex_in_place` (snapshots in every test that touches a home) | PASS |
| Req.3 guard for desktop adapters to come (no real adapter declares DESKTOP_APP today, so no user could hit this): a desktop launch never prepares an in-place folder and refuses a gone one instead of recreating it. The fake now prepares like the real adapters (folder + marker); the test snapshots the folder, root included | `desktop::a_desktop_launch_never_prepares_a_folder_used_where_it_is` — fails on the old code (checked by reverting the guard) | PASS |
| Req.4 the in-place list comes from the backend: `providers_capabilities` (registry only — no install probe, cheap for First run and Accounts) returns `can_use_in_place` per Provider, tied to each registered adapter's `adopt_account` (a refusal other than Unsupported means supported). If the call fails the choice stays, in place preselected (the user decides; the backend refuses what it can't adopt) | `codex_in_place::the_in_place_flag_matches_what_each_adapter_supports`, `providers_capabilities_reports_the_flag_without_probing`; vitest `providers.test.ts`, `first-run.test.tsx` ("a tool the backend can't use in place comes in as a copy, with no choice", "when the capabilities call fails, the choice stays and in place is preselected") | PASS |
| Req.4 one home lookup: "use the original" finds the default folder through the adapter's own home (the one its launch rule uses), so a copy of `~/.codex` switches back to `~/.codex` exactly and runs with its variables unset | `codex_in_place::a_copy_of_the_default_home_switches_back_to_it_exactly` — fails on the old `dirs::home_dir()` lookup (checked) | PASS |
| Req.4 per-tool choice: Codex setups get "Use them where they are" (worded for Codex), in place by default; a failed Codex copy offers "Use it where it is instead"; a Codex copy offers "Use the original instead" (the same command) and the original stays untouched | vitest `first-run.test.tsx` (Codex block, 4 rewritten), `providers.test.ts`; `codex_in_place::a_codex_copy_can_switch_to_its_original_home`; Playwright `accounts — a Codex copy switches to its original home` (gallery `68-accounts/use-original-codex-1280x800-{dark,light}.png`); hand walk in the preview (Codex-only first run; mixed machine, failed Codex copy → in place) | PASS |
| Req.5 the checklist is honest per Provider: "CODEX_HOME and CODEX_SQLITE_HOME are this folder, exactly" / "the default folder runs as Codex runs it (… unset)"; Claude's rows unchanged in substance | `codex_in_place` (both env rows asserted); `in_place_accounts` (5) | PASS |

Gates (2026-10-20, this branch after review): `just verify` green —
fmt, clippy with default features AND `--no-default-features`, 1032
workspace tests, docs, evidence and the other gates; `cargo deny check`;
vitest 541; `tsc -b`; build; full Playwright 903; `./scripts/a11y_gate.sh`
PASS (0 critical/serious); `./scripts/visual_gate.sh` 73/73. CI and
`just verify`/`just lint` now run clippy with `--no-default-features`
too (before, only the default features were linted).

## Not claimed

- No real Codex binary or home was used (fixtures and a stub). That a
  session started in place shows in `codex resume` outside AI Juggler is
  an AJ-E1305 live check.
- A copy whose copied `config.toml` sets `sqlite_home` would still point
  at the original's database (config beats the variable) — a copy-path
  question, not this task's; noted for follow-up.
