# Evidence manifest — AJ-E1301

Task: `docs/tasks/E13-codex-adapter.md` AJ-E1301 (Codex Provider:
discovery, isolated home, launch, sign-in, registration). Status:
**IN-PROGRESS** — the Rust half is proven here; Req.8 (First run and Add
account offering Codex) lands in a separate UI change. 2026-10-20,
reviewer claude-opus.

## What was checked, and against what

- **Codex CLI 0.163.0** (`/opt/homebrew/bin/codex`, `codex --version` →
  `codex-cli 0.163.0`), run only with a throwaway `CODEX_HOME` in the
  session scratchpad and synthetic `auth.json` files holding no real
  token. The user's `~/.codex` was not read or written.
- **Source** at tag `rust-v0.163.0` (commit `83086188ae06`, released
  2026-10-15): `codex-rs/utils/home-dir/src/lib.rs`,
  `login/src/auth/storage.rs`, `login/src/auth/manager.rs`,
  `config/src/types.rs`, `state/src/lib.rs`, `tui/src/daemon_startup.rs`,
  `tui/src/session_resume.rs`, `cli/src/main.rs`. Findings with paths:
  [research/codex.md §10](../../../research/codex.md).
- **Docs**: `developers.openai.com/codex/*` → 308 to
  `learn.chatgpt.com/docs/*` (auth, config reference, non-interactive mode).

## Live checks on the real binary (throwaway home)

| Check | Observed | Result |
|---|---|---|
| `CODEX_HOME` pointing at a missing dir | "CODEX_HOME points to … but that path does not exist", exit 1 → `prepare_account` must create it | PASS |
| `codex login status`, empty home | "Not logged in", exit 1 | PASS |
| synthetic ChatGPT `auth.json` (`auth_mode: chatgpt`, fixture tokens) | "Logged in using ChatGPT", exit 0 — default store reads the file | PASS |
| synthetic API-key `auth.json` | "Logged in using an API key - …", exit 0 | PASS |
| `codex login status` cost | ≈0.5 s wall → not used on hot paths; the adapter reads field names instead | PASS |
| every argv the adapter builds (`--no-daemon`; `--no-daemon --model m <prompt>`; `exec --json <prompt>`; `resume <id> -C <dir> --no-daemon <delta>`; `login`) parsed by clap | exit 0 each; control `--bogus-flag` → exit 2 | PASS |
| `--no-daemon` on `resume` and `fork` | listed in each subcommand's help | PASS |

## Requirements

| Requirement | Proof | Result |
|---|---|---|
| Req.1 discovery, `codex-cli X.Y.Z` parsing, drift note | `codex_adapter::version_parsing_takes_the_number_after_the_name`, `discover_verified_and_drifted_versions`, `discover_missing_binary_is_not_found` | PASS |
| Req.2 the home is created 0700 with the `codex` marker and nothing else | `prepare_account_creates_the_codex_home_codex_requires`, `two_accounts_never_share_a_home` | PASS |
| Req.3 `CODEX_HOME` and `CODEX_SQLITE_HOME` pinned after overrides | `launch_specs_per_mode` (an override of both is ignored) | PASS |
| Req.4 argv per mode, model flag, `--no-daemon` | `launch_specs_per_mode`, `model_override_rides_the_command_line`; real-binary parse above | PASS |
| Req.5 sign-in state through `HarnessAdapter::login_state`, field names only, `auth_mode` first, damaged = unknown, Keychain item naming | `login_state_reads_field_shape_only`, `keychain_account_matches_codex_naming`; Claude's suite unchanged (`claude_code_adapter`) | PASS |
| Req.6 vendor `codex login` in a PTY; snippet `CODEX_HOME='…' codex --no-daemon` | `login_spec_runs_the_vendor_flow_in_the_isolated_home`, `isolation_keys_and_snippet` | PASS |
| Req.7 registry and `providers_list` include Codex; isolation check names `CODEX_HOME` and passes; `CODEX_HOME` aliases are Codex candidates; Keychain-bound inference uses `auth.json` | `juggler-app` `codex_provider` (3 tests) | PASS |
| Req.8 First run and Add account offer Codex | UI change pending (coordinated with the D-25 screens work) | UNPROVEN |

Commands: `cargo test -p juggler-adapters --test codex_adapter` (11
passed), `cargo test -p juggler-adapters --test claude_code_adapter` (8
passed), `cargo test -p juggler-app --test codex_provider` (3 passed);
the full gate is `just verify` on the PR.

## Not claimed

- No real Codex account was signed in or launched: that is AJ-E1305
  (maintainer-run). Concurrent browser sign-ins share the vendor's
  localhost callback (port 1455, with a fallback port) — unexercised live.
- Session watching, import and switching are AJ-E1302–E1304.
