# Evidence manifest — AJ-E1521

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1521 (a database busy timeout and
an adapter conformance kit). Status: **DONE-EVIDENCED** on fixtures.
2026-10-21, reviewer claude-opus.

## What the kit checks

`crates/juggler-adapters/tests/conformance/mod.rs` is the kit;
`crates/juggler-adapters/tests/adapter_conformance.rs` runs it for each
adapter (one `Subject` and one `conformance!` line per adapter). Six
checks per adapter:

1. **Isolation of the home** — every pinned variable is an isolation
   variable (so overrides can't set it), points inside its own account's
   folder, differs between two accounts, and agrees with what
   `prepare_account` returns; preparing writes nothing outside the folder
   and nothing in the user's home; the tool's own sign-in flow is pinned
   the same way.
2. **No credential copies** — a copy and a settings-only copy of a
   setup whose sign-in holds a canary never contain the sign-in file, nor
   the canary in any file; inspecting, adopting and copying leave the
   original byte-identical. An adapter that can't bring setups in must
   refuse and create nothing.
3. **Pinned env after overrides** — Launch settings that set every
   isolation variable and every removed credential: for all four launch
   modes the pinned variables keep the folder's values, the credentials
   stay removed, and an ordinary override still arrives. The copyable
   shell line keeps the same rules. Where the tool's default folder lives
   in the sandbox (Codex), it runs with every pinned variable unset.
4. **Refusing option-like resume ids** — `""`, `-`, `--last`, `-c`,
   `--continue`, `--session=other`, `-x` are refused; an ordinary id is
   one argument of its own.
5. **Sessions found and resumed** — a session written in the account's
   folder for the project is found with its own id; not from another
   account, not for another folder, not when already linked, not when
   older than the launch, never a guess between two; it resumes by id
   with the pins in place.
6. **Usage labelled honestly** — usage is claimed (`USAGE_TOKENS`) only
   with a reader; no usage is invented for an empty account; reading
   writes nothing; every event has a de-duplication key (stable across
   reads, one per message), its account, source and time; credits are
   only ever `Computed`, never `Measured`.

## Requirements

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 the core storage sets a busy timeout: `BUSY_TIMEOUT` (5 s) on the storage, process-ledger, cleanup-audit and cleanup-settings connections — set explicitly, not left to rusqlite's default | `cargo test -p juggler-core --test busy_timeout` | 4 pass | 4 pass | PASS | branch head | 2026-10-21 | claude-opus |
| Req.1 the switch to WAL waits too (SQLite answers it "busy" at once, skipping the busy handler): opening a database another connection is writing — storage and the other three — now waits | `busy_timeout::opening_the_database_waits_while_another_connection_writes` — fails on the old code with `DatabaseBusy` ("database is locked"), checked by reverting `crates/juggler-core/src/storage/` | fails before, passes after | as expected | PASS | branch head | 2026-10-21 | claude-opus |
| Req.1 a write waits behind another connection's write | `busy_timeout::a_storage_write_waits_for_another_connections_write`, `the_ledger_audit_and_cleanup_settings_wait_too` (these passed on the old code too: rusqlite already waits 5 s on ordinary statements; kept as guards) | pass | pass | PASS | branch head | 2026-10-21 | claude-opus |
| Req.2 one suite, every adapter: the six checks above for Claude Code, Codex and the fake | `cargo test -p juggler-adapters --test adapter_conformance` | 18 pass | 18 pass | PASS | branch head | 2026-10-21 | claude-opus |
| Req.2 the kit fails on the old code: 8 of 18 failed before the fixes (log below) | same command with the kit on the old adapters (`pinned_env` added) | failures name real defects | 8 failures, below | PASS | branch head | 2026-10-21 | claude-opus |
| Fixed: Claude Code's copy carried `.credentials.json` (a second folder with the same refresh token signs the first out). Now kept back with the reason listed; the new account signs in once | `adapter_conformance::claude_code::copies_no_credentials`; `claude_environment::import_copies_state_skips_caches_and_never_touches_source`; `juggler-app` `onboarding::import_makes_an_existing_setup_launchable_without_touching_it` | sign-in never in the copy | not copied, skip listed | PASS | branch head | 2026-10-21 | claude-opus |
| Fixed: the copyable shell line brought back a removed credential when Launch settings set it (`env -u OPENAI_API_KEY … OPENAI_API_KEY='…' codex`), for Claude Code and Codex | `adapter_conformance::{claude_code,codex}::pins_env_after_overrides` | no override of a removed key in the line | removed | PASS | branch head | 2026-10-21 | claude-opus |
| Fixed: the fake named no isolation variable, dropped Launch-settings overrides, resumed `""` (and would write outside its folder for `../x`), couldn't find its own sessions, and claimed token usage it has no reader for | `adapter_conformance::fake::*`; `fake_harness_e2e` still passes | 6 pass | 6 pass | PASS | branch head | 2026-10-21 | claude-opus |
| One pin list: `pinned_env` (new, default from `pinned_env_keys` + `isolation_env_value`) feeds the launch check in the isolation checklist (managed and in place) and the shell line, so a tool whose pins are different folders (OpenCode's XDG roots, AJ-E1602) is checked correctly | `juggler-app` `in_place_accounts`, `codex_in_place`, `codex_provider`, `onboarding` | pass | pass | PASS | branch head | 2026-10-21 | claude-opus |

### The kit on the old code (before the fixes)

```
test fake::isolates_each_home ... FAILED              fake: names no isolation variable
test codex::pins_env_after_overrides ... FAILED       codex: the shell line brings back a removed credential
test claude_code::pins_env_after_overrides ... FAILED claude-code: the shell line brings back a removed credential
test fake::labels_usage_honestly ... FAILED           fake: claims token usage with no reader
test fake::refuses_option_like_resume_ids ... FAILED  fake: resumed the option-like id ""
test fake::pins_env_after_overrides ... FAILED        fake: an ordinary override never reached the Interactive launch
test fake::finds_and_resumes_sessions ... FAILED      fake: the session started in the folder isn't found
test claude_code::copies_no_credentials ... FAILED    claude-code: the copy copied the sign-in file .credentials.json
test result: FAILED. 10 passed; 8 failed
```

## Not claimed

- Fixtures and stub binaries only; no real tool, home or sign-in was used.
- Claude Code's default folder (`~/.claude`) is not in the kit's sandbox
  (its adapter reads `$HOME`); `vendor_default_folder.rs` covers its
  unset rule. The kit checks the default-folder rule for Codex.
- Option-like **prompts** (a Primed prompt starting with `-` for Claude
  Code) are outside this task's rule; noted as a follow-up.

## Redaction checklist

- [x] No credentials, tokens, or keys in any file here (the kit's canary is a made-up string)
- [x] Paths generalised (temporary folders only)
- [x] No transcript content
