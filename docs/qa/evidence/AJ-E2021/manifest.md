# Evidence manifest — AJ-E2021

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2021 — remote
configuration, step 1: the signed envelope and its trust rules
(P0, ADR-0035, D-129). Audited 2026-11-23 by zcode-primary against
HEAD (the epic's 2026-11-18 note predates later landings).

| Claim | Requirement | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Signature verified against pinned key ids BEFORE any parse; refuses bad signature, unknown/revoked key, lower serial, expired/future file, wrong channel, oversize | R1 | `envelope.rs`/`keys.rs`/`state.rs`: pinned-key verification precedes parsing (the security review 2bf8ad4a9 covered this; its fixes 1fa860c64/0e2c06e9c/24fe4cefd landed); the whole class is pinned by the crate's tests (101 passing at audit) | PASS | 2026-11-18 (built); 2026-11-23 (audited) | security review + zcode-primary |
| Trust state durable, checksummed, atomic, fail-safe to compiled baseline, never in cache | R2 | `state.rs` + tests (R1-R3 evidence in the epic note's SHAs); fault test `every_failure_falls_back_to_the_compiled_defaults` | PASS | 2026-11-18 | claude-opus |
| Sections tolerant: unreadable value drops only that control; kills and narrowing still apply | R3 | `schema.rs` §tolerance + `1fa860c64` (the review's fix); `sections.rs` H2 test (a rule gated to newer app versions never read) | PASS | 2026-11-18 | claude-opus |
| Fetch: no identifying header or query, at most daily, fixed URL per channel, host on the compiled allow-list; the setting on by default | R4 | `fetch.rs`: the URL type refuses a query string/fragment/userinfo at the type level (tests pin `config.figurehead.ai/v1/<channel>.json` and reject look-alikes); the production transport adds only `User-Agent: AIJuggler/<version>`; `service.rs` fetches at most daily (D-129) with `remote_config_check` OFF-switch in Settings (on by default — `!= Some("off")`) | PASS | 2026-11-23 | zcode-primary |
| The app starts and works identically with no file, a truncated file or a down host | R5 | `every_failure_falls_back_to_the_compiled_defaults` (fetch.rs:142) walks every failure mode incl. "truncated" and an unreachable host; the service logs a warning and runs compiled defaults | PASS | 2026-11-23 | zcode-primary |
| A golden file per channel and the dry run against old app versions SHALL run in CI | R6 | Channel files exist committed (`config/remote/{beta,nightly,stable}.json`) and validate against the exported schema (`config/remote/schema.json`, kept current by a test); `publish-config.yml` validates + verifies-as-the-app in CI before any publish. PARTIAL: a dry run specifically against the LAST THREE released app versions is not a separate CI job — version-gating is covered per-rule (sections.rs H2: rules gated to newer versions never read; the schema registry drops unknown values one at a time), but the three-version matrix run itself awaits the channel having real published history | PARTIAL | 2026-11-23 | zcode-primary |

## Test counts at audit

`cargo test -p juggler-remote-config`: **101 passed, 0 failed**
(fetch/trust/sections/lists/publish suites).

## Note

The UI-facing parts (Settings row copy, About) belong to AJ-E2023
(backend + data layer already landed per cd715d7bf/bed8e6a0b; its
screens note says they follow the D-126 Alert, which has since
landed — E2023's own audit comes next, not here).
