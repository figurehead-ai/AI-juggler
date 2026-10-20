# E13 — Codex, the second Provider

> Source: the release notes promise Codex; the AJ-E808 fidelity register
> and `fidelity.rs` already count Codex pairs; no Codex adapter existed.
> Historical spec: [EPICS.md](../plan/EPICS.md) M2-1 / M2-2 (Codex adapter
> and watcher), M3-4 (Codex estimation), M4-3 (transplant) and
> [SPEC §4.2](../SPEC.md). **The contract is the code**:
> `crates/juggler-adapter/src/adapter.rs` (`HarnessAdapter`) and
> `handover.rs` (`HandoverSupport`); the reference is the Claude Code
> adapter in `crates/juggler-adapters/src/claude_*.rs`.
>
> Verified facts about Codex live in
> [research/codex.md §10](../research/codex.md) (codex-cli **0.163.0**,
> source tag `rust-v0.163.0`). Every task re-checks the behaviour it
> relies on against the current CLI and records the version.
>
> Law for every task here: one isolated `CODEX_HOME` per Account; the
> user's own `~/.codex` and every real credential are never read or
> written (fixtures only — synthetic rollouts, configs and `auth.json`
> files holding no real secret); copies never touch the original;
> switching to or from Codex is **labelled beta and never called
> lossless** (D-04, D-11); fidelity cells stay UNVALIDATED until a live
> run on real binaries (AJ-E808).

## Order and shape

| Task | What a user can do afterwards | PR |
|---|---|---|
| AJ-E1301 | Add a Codex account, sign in, launch it | 1 (+ a small UI PR) |
| AJ-E1302 | Find and bring in existing Codex setups | 2 |
| AJ-E1303 | Watch Codex sessions; the app reads their conversations | 3 |
| AJ-E1304 | Switch Codex↔Codex and Claude↔Codex (beta) | 4 |
| AJ-E1305 | Live validation on real accounts; docs | maintainer-run |

ZCode follows the same five steps later (its own epic).

### AJ-E1301 Codex Provider: discovery, isolated home, launch, sign-in, registration | Status: IN-PROGRESS (claude-opus, 2026-10-20) | Priority P0 | Effort M | Depends: AJ-E301, AJ-E308
**Goal**: Codex is a registered Provider: the app finds the CLI, creates an isolated `CODEX_HOME` per Account, signs in through the vendor's own flow and launches every mode.
**User story**: As a developer with a ChatGPT Plus and a Pro subscription, I add both as Codex accounts, sign each in once, and start Codex on either without one ever seeing the other's login or sessions.
**Requirements**:
1. `CodexAdapter` SHALL implement the required `HarnessAdapter` methods with id `codex`, `verified_against = ["0.163.0"]`, and discovery that finds `codex` on PATH, `/opt/homebrew/bin`, `/usr/local/bin`, `~/.local/bin` and `~/.npm-global/bin`, parsing `codex-cli X.Y.Z`; other versions SHALL be a drift note, never a failure.
2. `prepare_account` SHALL create the Account directory (0700, the managed marker naming `codex`) because Codex refuses a missing `CODEX_HOME`; it SHALL write nothing else.
3. Every launch and the login SHALL set `CODEX_HOME` and pin `CODEX_SQLITE_HOME` to the Account directory, applied after Launch-settings overrides (an override can never repoint either; both are protected isolation keys).
4. `launch_spec` SHALL map Interactive → `codex --no-daemon`, Primed → `codex --no-daemon <prompt>`, Headless → `codex exec --json <prompt>`, Resume → `codex resume <id> -C <cwd> --no-daemon [<delta>]`; a Launch-settings model SHALL become `--model <m>`. `--no-daemon` keeps the session inside the process tree Juggler owns (the per-home background server would outlive the session, ADR-0010).
5. The sign-in state SHALL come from the Account's own adapter through a new `HarnessAdapter::login_state` (the AJ-E308 taxonomy moves to the contract); Codex's probe SHALL read only the names and presence of `auth.json` fields (`auth_mode` decides; a ChatGPT sign-in may also hold an exchanged key) and, with no file, only the existence of its Keychain item (`Codex Auth` / `cli|<sha256-16>`); a damaged file SHALL be `unknown`, never `logged out`.
6. `login_spec` SHALL run the vendor's `codex login` in the guided terminal; `shell_command` SHALL be `codex --no-daemon` so the Expert snippet reads `CODEX_HOME='…' codex --no-daemon`.
7. `default_registry()` and `providers_list` SHALL include Codex; the isolation check SHALL test the Provider's own variable; shell-alias candidates for `CODEX_HOME` SHALL be Codex candidates; the "signed in without a credentials file" (Keychain-bound) inference SHALL use `auth.json` for Codex.
8. First run's scan and Add account SHALL offer Codex beside Claude Code (UI PR, coordinated with the screens work).
**UI/UX reasoning**: Nielsen #2 (match the real world): the Provider is named "Codex", its sign-in is OpenAI's own page in a terminal the app opens; Nielsen #5 (error prevention): a missing CLI is caught at Step 0 with the install link, never after an account exists.
**Performance budget**: Sign-in probe ≤ 5 ms with a file (no process spawn; `codex login status` costs ~0.5 s and is not used); discovery ≤ 300 ms (one `--version`).
**Quality gates**: `just verify`; `cargo deny check`; UI PR: vitest, Playwright, `./scripts/a11y_gate.sh` 0 critical/serious, re-rendered baselines with notes.
**Evidence/validation proof**: `cargo test -p juggler-adapters --test codex_adapter` and `cargo test -p juggler-app --test codex_provider`; the real binary checked with a throwaway `CODEX_HOME` and synthetic `auth.json` files; recorded in `docs/qa/evidence/AJ-E1301/manifest.md`.
**Visual testing**: UI PR: First run with a Codex install found, Add account Step 0 with both Providers — 1280×800, light and dark.
**Files to touch**: `crates/juggler-adapters/src/codex.rs`, `claude_code.rs` (shared helpers, marker), `crates/juggler-adapter/src/{adapter,environment,lib}.rs`, `crates/juggler-app/src/{state,commands}.rs`, tests; UI: `ui/src/screens/{FirstRun,Profiles}.tsx`.
**Research basis**: research/codex.md §2–§3 and §10; codex-rs `utils/home-dir/src/lib.rs`, `login/src/auth/storage.rs`, `tui/src/daemon_startup.rs` at `rust-v0.163.0`.
**Handover note**: Trap — `OPENAI_API_KEY` being set does not mean an API-key account. Trap — `codex --version` prints the name first; Claude's parser takes the first token. Never run the real binary against `~/.codex`; use a scratch `CODEX_HOME`.

### AJ-E1302 Find and bring in existing Codex setups | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E1301, AJ-E307
**Goal**: First run finds the user's existing Codex homes and brings each in as a copy, the original untouched.
**User story**: As a Codex user with `~/.codex` and a `~/.codex-work` I made by hand, I see both in First run and bring them in with one click each; my originals keep working exactly as before.
**Requirements**:
1. `scan_environment` SHALL report `~/.codex`, `$CODEX_HOME`, `~/.codex*` home patterns and `CODEX_HOME=` shell-rc targets that look like a Codex home (`config.toml`, `auth.json`, `sessions/`, `history.jsonl`, `version.json`), metadata only (sizes, times, the AJ-E1301 sign-in probe).
2. `inspect_source`, `import_account` and `import_account_degraded` SHALL copy byte for byte into a fresh managed home, skipping rebuildable state (`.tmp/`, `tmp/`, `log/`, SQLite `*.sqlite*`, `app-server-*`, `packages/`) and, for the settings-only import, the history (`sessions/`, `archived_sessions/`, `history.jsonl`, `session_index.jsonl`); every skip is listed with its size.
3. The source SHALL be byte-identical before and after (fs-fixture test hashing every file) and SHALL never receive the managed marker.
4. When the sign-in carries over, the import SHALL say plainly that the copy and the original now share one sign-in and that signing in again inside AI Juggler gives the copy its own; a Keychain-stored sign-in (no `auth.json`) SHALL say it does not carry over.
**UI/UX reasoning**: Nielsen #1 (visibility): the Confirm-source step shows sizes and what stays behind; Nielsen #3 (user control): copy, never adopt in place.
**Performance budget**: Scan of a home with 10k session files ≤ 2 s (bounded walk); copy ≥ 100 MB/s locally.
**Quality gates**: as AJ-E1301 (Rust only unless UI copy changes).
**Evidence/validation proof**: `cargo test -p juggler-adapters --test codex_environment` (fixture homes, source hash before/after) and an app-level import test; `docs/qa/evidence/AJ-E1302/manifest.md`.
**Visual testing**: none unless copy changes (the existing import screens are Provider-neutral).
**Files to touch**: new `crates/juggler-adapters/src/codex_environment.rs`, `codex.rs`, tests and fixtures under `crates/juggler-adapters/tests/fixtures/codex/`.
**Research basis**: research/codex.md §3 (Codex docs sanction copying `auth.json` to another machine), §10 (home contents); `claude_environment.rs` as the pattern.
**Handover note**: Trap — Codex's `state_5.sqlite` stores ABSOLUTE rollout paths: a copied index would point the new account at the original's session files, and a resume would write into the original. It is a projection Codex rebuilds from `sessions/` (filename scan + read-repair), so every `*.sqlite*` stays behind — memories and queued messages kept in SQLite do not travel (listed in the skips). The copy machinery is shared with Claude through `claude_environment::Layout`.

### AJ-E1303 Watch Codex sessions: rollout parser, linking, identity | Status: TODO | Priority P0 | Effort L | Depends: AJ-E1301
**Goal**: The app sees Codex sessions appear and grow, reads their conversation into the neutral parsed form every feature uses, and knows which account a home is signed in as.
**User story**: As a user running Codex, the session shows its conversation, model and tokens like a Claude session does, and a switch can find its file.
**Requirements**:
1. A rollout parser SHALL turn `sessions/**/rollout-*.jsonl` (legacy and `paginated` history modes) into the same `ParsedTranscript` the app uses for Claude: user and assistant messages from `response_item` messages (skipping injected environment/instruction turns), the model from `turn_context`, tokens from `token_usage_record`, compaction from `compacted`; it SHALL never panic on malformed lines (counted).
2. Every app site that parses a transcript (switch preflight, switch-back delta, redaction preview, packs, replay cost, running model, quality signals, cross-machine bundles) SHALL dispatch by Provider instead of assuming Claude's shape.
3. `watcher` SHALL watch `<home>/sessions` and report the thread UUID (from the filename, matching `session_meta.payload.id`) as the native id.
4. A session launched by the app SHALL get its transcript reference linked from the rollout whose `session_meta` cwd and start time match the launch.
5. `probe_identity` SHALL return `tokens.account_id` from `auth.json` as the subject and a masked label without decoding any token; API-key accounts and Keychain storage SHALL be `Unverifiable` with a plain reason.
6. The usage pipeline's rollout reader SHALL be re-checked against 0.163 output (both `token_usage_record` and `token_count` are written; only one may be counted).
**UI/UX reasoning**: Nielsen #1: a Codex session shows the same honest fields as a Claude one (D-06 — no invented values).
**Performance budget**: Index a session from `session_meta` without a full replay; a 10 MB rollout parses in ≤ 150 ms.
**Quality gates**: as AJ-E1301; golden-file tests per history mode.
**Evidence/validation proof**: `cargo test -p juggler-adapters --test codex_transcript`; app tests for each dispatched site; `docs/qa/evidence/AJ-E1303/manifest.md`.
**Visual testing**: none (data only); the existing session view captures cover rendering.
**Files to touch**: new `crates/juggler-adapters/src/codex_transcript.rs`, `codex.rs`, `crates/juggler-app/src/{commands,crossmachine}.rs`, `crossmachine/switch_back.rs`.
**Research basis**: research/codex.md §4, §10 (rollout schema, paginated mode since 0.158); codex-rs `rollout/src/policy.rs`, `protocol/src/protocol.rs`.
**Handover note**: Trap — paginated rollouts no longer write `event_msg` `user_message`/`agent_message`; read `response_item`. Trap — the `id_token` is a credential: never decode it for an email.

### AJ-E1304 Switch with Codex (beta): transplant and cross-Provider replay | Status: TODO | Priority P0 | Effort L | Depends: AJ-E1303, AJ-E204, AJ-E205
**Goal**: A session moves Codex→Codex by copying its rollout into the target home, and Claude↔Codex by replaying its conversation into a primed session — every one labelled beta.
**User story**: As a user whose Plus account hit its limit, I switch the session to my Pro account and carry on; from Claude Code I can continue the same work in Codex, knowing it is a beta replay, not the same conversation.
**Requirements**:
1. `CodexAdapter` SHALL implement `HandoverSupport` and be registered: `bundle_session` copies the rollout (refusing a torn tail), records `session_meta` cwd and caps; `install_bundle` places it under the target's `sessions/YYYY/MM/DD/` with its own name; a thread id already on the target SHALL fork a new UUID rewritten in the file name and every line; the source SHALL stay byte-identical.
2. A rollout whose `session_meta` names a `history_base` or `forked_from_id` SHALL carry an honest cap (`base-rollout-not-carried`) until a live run proves the copy resumes.
3. The local switch SHALL use the source Provider's support to read the session and, across Providers, SHALL replay (distilled turns → the target's prime prompt → a Primed launch) instead of transplanting; same-Provider switches keep transplant.
4. Switch-back forks and cross-machine bundles SHALL use the Provider's own id/cwd rewriting (Codex keeps both in `session_meta.payload`).
5. Every Codex pair's fidelity cells SHALL stay UNVALIDATED and every Codex switch SHALL show the beta label.
**UI/UX reasoning**: D-03/D-11: the hybrid door shows replay as lossy, beta, with what does not travel; never "seamless".
**Performance budget**: A 10 MB rollout bundles and installs in ≤ 500 ms.
**Quality gates**: as AJ-E1301; the copy-never-move invariant test for Codex.
**Evidence/validation proof**: `cargo test -p juggler-adapters --test codex_handover`, app tests for Codex→Codex and Claude→Codex switches against stubs; `docs/qa/evidence/AJ-E1304/manifest.md`.
**Visual testing**: none unless copy changes.
**Files to touch**: new `crates/juggler-adapters/src/codex_handover.rs`, `crates/juggler-app/src/{state,commands}.rs`, `crossmachine.rs`, `crossmachine/switch_back.rs`.
**Research basis**: research/codex.md §10 (lookup scans `sessions/` and read-repairs its DB); deep/09 §H1–§H4 (transplant rules).
**Handover note**: Trap — `switch_execute_core` used the TARGET's support on the SOURCE file and always said Transplant; a Claude→Codex switch must never hand a Claude file to Codex.

### AJ-E1305 Live validation on real accounts and the docs | Status: TODO | Priority P1 | Effort M | Depends: AJ-E1304, AJ-E808
**Goal**: The Codex claims are checked on real binaries with sacrificial projects, and the docs say what is verified.
**User story**: As the maintainer, I can see which Codex behaviours ran for real and which are still fixtures.
**Requirements**:
1. Live runs (maintainer accounts, sacrificial projects): two accounts signed in in parallel; launch, resume, a Codex→Codex switch, Claude→Codex and Codex→Claude replays; each fills its AJ-E808 register cells with the versions used.
2. SPEC §4.2, the release notes and the beta-exit register SHALL state what is verified live versus on fixtures.
**UI/UX reasoning**: Nielsen #1: badges reflect evidence, never hope (D-04).
**Performance budget**: n/a.
**Quality gates**: `just verify`; register rendering (`just fidelity-register`).
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1305/manifest.md` with dated live rows; PARTIAL until the maintainer runs them.
**Visual testing**: n/a.
**Files to touch**: `crates/juggler-handover/data/fidelity-register.json`, `docs/SPEC.md`, release notes.
**Research basis**: AJ-E808, AJ-E210.
**Handover note**: Live runs need real credentials — only the maintainer runs them; agents never sign in.
