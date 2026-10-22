# Security guidelines

Rules for every contributor, and for agents writing code here. The repo is public, and AI Juggler sits beside vendor sign-ins, spawns processes and runs local servers, so a shortcut here is visible to everyone and can hurt users.

MUST, MUST NOT, SHOULD and MAY are used as in RFC 2119. Each rule names its enforcement: **CI** (a check that fails the PR), **Review** (sign-off on sensitive paths), **Test** (a named suite) or **Design** (an ADR is required). E20 builds the checks marked as proposed. Until they land, review enforces the rule.

Report a vulnerability through [SECURITY.md](../../SECURITY.md), never in a public issue.

## Rules by area

### C1. IPC commands (Tauri)

| Rule | Enforced by |
|---|---|
| MUST register every new `#[tauri::command]` in the `AppManifest::commands` allowlist and grant it in a capability file; a command not in a capability is unreachable | CI (build fails: unknown permission) + Review on `capabilities/**` |
| MUST treat every argument as attacker-controlled: validate type, length, charset; resolve ids to objects from AI Juggler state rather than accepting paths/URLs/programs from the UI (pattern: `expert_reveal`, `figurehead_open_page`) | Review + unit test per command |
| MUST NOT accept a filesystem path, URL, program, argv, env map or SQL fragment from the webview unless the user picked it through a native dialog in the same flow; if it must, canonicalise and check it against an allowlisted root inside the command | Review; `scripts/ipc_params_lint` (grep for `path: String` etc. in command signatures) |
| MUST keep state-changing and destructive commands behind an explicit user action (ADR-0003) and a one-time token where the action is irreversible (pattern: restore token) | Test (guardrails suite) |
| MUST NOT add plugin permissions to the webview for shell, fs, http or opener; call those from Rust with server-side allowlists | CI (capability lint: deny `shell:*`, `fs:*`, `http:*`, `opener:*` in `capabilities/*.json`) |
| MUST NOT add `remote.urls` to any capability, or new windows/webviews, without an ADR | Design + CI capability lint |
| SHOULD return errors that never echo secrets or environment values | Test (redaction test on `AppError` Display) |

### C2. Process spawning (supervisor, adapters, helpers)

| Rule | Enforced by |
|---|---|
| MUST spawn with `Command::new(program).args([...])` / `CommandBuilder` argv; MUST NOT use `sh -c`, `bash -c`, `osascript` with spliced strings, or `format!` to build a command line | CI (grep gate for `"-c"`, `"sh"`, `"bash"` in non-test code; clippy `disallowed_methods` for helpers) |
| MUST use absolute paths for system tools (`/usr/bin/security`, `/usr/bin/open`, `/usr/bin/osascript`), and a resolved, recorded path for vendor CLIs | Review + grep gate |
| MUST put `--` before any user- or data-derived positional argument when the tool supports it (git, open) | Review |
| MUST start child environments from `env_clear()` plus an allowlist and the adapter overlay; MUST unset vendor credential vars (`ANTHROPIC_API_KEY`, `CLAUDE_CODE_OAUTH_TOKEN`, `OPENAI_API_KEY`, `CODEX_API_KEY`, …) unless the user's Launch setting sets them | Test (env overlay tests in `supervisor.rs` + adapter tests) |
| MUST record ledger intent before spawn and MUST verify identity (pid, pgid, start time, argv hash) before any signal; MUST NOT signal a process not in the ledger (ADR-0010) | Test (`collateral`, `ledger` suites, zero-false-kill gate) |
| MUST pass values to AppleScript as `on run argv` arguments, never by string splicing | Review |
| MUST NOT add `unsafe` (`pre_exec` etc.); the workspace forbids it — new FFI goes in an isolated, audited crate with a decision-log entry (D-17) | CI (`unsafe_code = "forbid"`) |

### C3. Files and paths (imports, backups, worktrees, data dir)

| Rule | Enforced by |
|---|---|
| MUST write only inside AI Juggler's data dir or a destination the user chose in a native dialog; MUST NOT write into a vendor folder used in place (ADR-0018) | Test (guardrails "writes confinement") |
| MUST create sensitive files owner-only (`0600`, dirs `0700`) atomically (`create_new` + rename), as `identity.rs::write_owner_only` does | Test (mode assertions) |
| MUST reject archive entries that are not regular files, contain `..`/absolute/`.` components, or are not on the layout allowlist; MUST cap total bytes, entry count and per-entry size before writing | Test (`backup_restore` hostile-archive cases) + Fuzz (archive target) |
| MUST NOT follow a symlink out of the tree being copied/restored; MUST use `symlink_metadata` and refuse or list outside links; SHOULD use descriptor-relative opens (`openat`/`O_NOFOLLOW`, e.g. `cap-std`) where a race matters | Test (symlink-escape fixtures) + Review |
| MUST exclude credentials by construction (allowlist of what is copied) and run the D-14 redaction scan on anything copied or exported | Test |
| MUST treat SQLite files from outside (restore, import) as untrusted: `trusted_schema=OFF`, defensive mode, `integrity_check`, schema comparison before use | Test |
| MUST canonicalise before comparing paths, and compare the canonical form you will actually open (no check-then-use on different strings) | Review |
| A file read from a user's project for display (AJ-E1532's project icons) MUST be read-only, a regular file (never a link), size-capped before and while reading, and accepted only by its first bytes (an SVG with scripts or foreign content refused); the UI MUST name it by an id the backend issued, never by a path | Test (`features::projects::icon`, `tests/project_marks.rs`) |

### C4. Secrets and credentials

| Rule | Enforced by |
|---|---|
| MUST NOT read, copy, parse, log, transmit or automate vendor sign-in material (Keychain items, `.credentials.json`, `auth.json`, OAuth tokens); login state comes from metadata only; Keychain probes MUST NOT pass `-w` | Test (guardrails) + CI grep gate (`find-generic-password.*-w`, `get_generic_password` on vendor services) |
| MUST store AI Juggler's own secrets (device token, vault key, Fleet identity key) in the Keychain, never in files, env vars, SQLite, settings, backups or crash reports | Review + backup allowlist test |
| MUST wrap secrets in a type whose `Debug`/`Display` are redacted (`VaultKey` pattern) and that zeroizes on drop | Review + unit test |
| MUST NOT put secrets in URLs, query strings, IPC event payloads, or the webview at all (exception: the recovery code shown once, never stored by the UI) | Review |
| MUST keep secrets out of fixtures and the repo; CI secret scanning and push protection stay on | CI (GitHub secret scanning, push protection) |

### C5. Crypto

| Rule | Enforced by |
|---|---|
| MUST use vetted crates only: RustCrypto AEADs (`chacha20poly1305`), `ring`/`rustls`, `ed25519-dalek`, `x25519-dalek`, `spake2`, `sha2`, `hmac`, `subtle`, `getrandom`; MUST NOT implement primitives, modes, KDFs or protocols | Review + `deny.toml` `[bans]` for known-bad crates |
| MUST use AEAD with a fresh random nonce from `getrandom` per message (XChaCha20 for random nonces); MUST NOT expose an API that takes a caller nonce outside tests | Review + `#[cfg(test)]` gating |
| MUST bind format headers/version/context into the AAD; MUST use domain-separated hashes for derived ids (`"figurehead-vault-key-id"` pattern) | Review |
| MUST compare secrets/MACs in constant time (`subtle`) | Review |
| MUST change a container or wire format only with a version bump and an ADR amendment, keeping old versions readable or refused with a reason (ADR-0016/0017) | Design |
| MUST NOT allow TLS < 1.3 or non-Ed25519 signatures on the Fleet link; MUST NOT add `dangerous_*` TLS options anywhere | Test (`verify.rs`) + CI grep gate for `dangerous` |

### C6. Network and Fleet

| Rule | Enforced by |
|---|---|
| MUST authenticate both ends of every Fleet connection with pinned Ed25519 keys; the only unpinned path is pairing, and it MUST grant nothing until SPAKE2 confirmation | Test (link suites) |
| MUST authorise every RPC against the peer registry (`NoAccess/View/Control`) and default new peers to `View` | Test |
| MUST carry ids (recipe, project, session), never paths, programs, argv, env or credentials, in remote commands; MUST apply sequence/replay/expiry checks | Test + proto lint (breaking-change and field-type review) |
| MUST keep the listener opt-in, size-limited, timeout-bounded and rate/concurrency-limited; SHOULD bind only to the interfaces the user chose | Test |
| MUST keep mDNS advertisements to non-sensitive data (no user name, no account labels) | Review |
| MUST make no network call to figurehead.ai or the stats/crash endpoints until the user opted in, and MUST use HTTPS with system roots | Test (guardrails "network allowlist") |

### C7. MCP server, Unix socket and agent bridge (pre-implementation rules)

| Rule | Enforced by |
|---|---|
| MUST land an ADR with a STRIDE section before any code | Design |
| MUST use stdio or a Unix socket; MUST NOT open a TCP/HTTP listener. If HTTP is ever needed: bind 127.0.0.1, validate `Origin` (403), require a bearer token [P2] | Design + Test |
| MUST place the socket in a `0700` dir owned by the user, create it `0600`, verify the peer uid with `getpeereid`/`peer_cred()` equals ours, and require a per-run random token handed only to processes AI Juggler spawned [L1][L2][L3] | Test |
| MUST NOT accept or forward tokens not issued to this server (no token passthrough); MUST NOT expose vendor credentials or the device token to agents [P1] | Review + Test |
| MUST label every cross-agent message with its origin and trust level, and deliver it as data, not instructions (spotlighting, delimiting) [P5] | Test (fixtures with injected instructions) |
| MUST NOT let a message from one agent trigger a state-changing action (launch, switch, kill, file write, network) without the user's click (ADR-0003); privileged functions live in code, not in model output [O9][P4] | Test (guardrails) |
| MUST NOT give one agent configuration all three of private data, untrusted content and an outbound channel; document which leg each tool removes [P3] | Design |
| MUST show the exact command, args and env names before writing or copying an MCP server config into any tool (AJ-E1507), and drop secret-looking env values [P1] | Test + Review |
| MUST rate-limit, size-limit and audit-log bridge traffic (who, what, when — not content) | Test |

### C8. Updater and release

| Rule | Enforced by |
|---|---|
| MUST keep updater signatures mandatory with `requireSignedVersion: true` and no custom `version_comparator` that allows downgrades | Test (updater fixtures in `juggler-app` tests) |
| MUST sign and notarize in a job that runs no third-party build code: build unsigned (no secrets) → upload artifact → a `release` environment job restricted to `main` signs, notarizes, staples, verifies and publishes | CI (workflow structure) + Review |
| MUST keep signing secrets environment-scoped (not repo-level), prefer an App Store Connect API key over Apple ID + app password, and rotate on any suspected exposure | Review (settings checklist in RELEASING.md) |
| MUST pin every tool installed in a release job to an exact version with a checksum (tauri-cli, pnpm), every action to a SHA, and every container image to a digest | CI (zizmor `unpinned-*`) |
| MUST publish SHA256SUMS, a CycloneDX SBOM and GitHub build-provenance attestations for each release asset; SHOULD build with `cargo auditable` | CI |
| MUST keep a written key-compromise and rotation runbook (updater key, Developer ID) | Review |
| MUST NOT add hardened-runtime exception entitlements or `get-task-allow` to release builds | CI (`codesign -d --entitlements -` check in the verify step) |
| MUST NOT ship a test seam. The only seams are listed below; each is compiled into debug builds only (`cfg(debug_assertions)`, or a feature a release build refuses) | CI (`scripts/dev_bridge_check.sh`; `scripts/release_seams_check.sh` on the release binary in build-macos.yml) |

**The test seams** (debug builds only):

- **The dev bridge** (`dev-bridge` feature, AIJ_DEV_BRIDGE): read-only routes for QA in a browser (docs/qa/DEV-BRIDGE.md). A release build with the feature doesn't compile.
- **`AIJ_NO_DIALOGS`**: launch dialogs go to stderr and answer their last button, for headless runs and the tests that start the real binary (`launch_alert::seams`).
- **`AIJ_DIALOG_ANSWER=<label>`**: launch dialogs answer with that button, for the tests that press Try again or Quit on the real binary (`launch_alert::seams`). An environment variable that answers a dialog is a lever someone else could pull, so a release binary must not contain it.

### C9. Dependencies

| Rule | Enforced by |
|---|---|
| MUST pass `cargo deny check` (advisories, bans, licenses, sources) with every `ignore` carrying a reason, a date and a removal condition | CI |
| MUST add new direct crates only with a recorded review (cargo-vet audit or import) and SHOULD prefer crates with no build script/proc-macro | CI (cargo-vet) + Review |
| MUST keep `[bans.build]` `executables = "deny"` and an `allow-build-scripts` list | CI |
| MUST install the UI with `pnpm install --frozen-lockfile`, a pinned pnpm (`packageManager`), lifecycle scripts off except an `allowBuilds` list, `minimumReleaseAge` ≥ 1 day, `trustPolicy: no-downgrade`, `blockExoticSubdeps: true` | CI |
| MUST run `pnpm audit --prod --audit-level high` and `pnpm audit signatures` on PRs that touch the lockfile | CI |
| SHOULD use Dependabot `cooldown` for version updates (never for security updates) | Config |
| MUST NOT add git or path dependencies from unknown sources (`[sources]` deny) | CI |

### C10. Logging, telemetry and crash reports

| Rule | Enforced by |
|---|---|
| MUST NOT log secrets, tokens, env values, transcript content, prompts, full command lines containing env-looking values, or Keychain query results [O6] | Test (redaction test) + Review |
| MUST scrub home dir, user name, host name, URLs and query strings from anything that may leave the Mac (crash reports, stats) — `crash.rs::scrub` is the one implementation | Test |
| MUST strip CR/LF and control characters from untrusted strings before logging or showing them in plain-text logs | Review |
| MUST keep usage statistics and crash reports opt-in, aggregated, identifier-free, and generated from the schema (D-19) | CI (schema generators `--check`) |
| SHOULD audit-log security-relevant events (kills, pairing attempts, permission changes, restores, bridge messages) as metadata | Test |

### C11. UI and webview

| Rule | Enforced by |
|---|---|
| MUST NOT use `dangerouslySetInnerHTML`, `innerHTML`, `eval`, `new Function`, or render Markdown/HTML from transcripts, terminal output, peers or agents without a sanitiser and an ADR | CI (ESLint `react/no-danger`, `no-eval`, `no-implied-eval`; grep gate) |
| MUST keep the CSP free of remote origins, `unsafe-eval` and inline scripts; SHOULD add `object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'` | CI (CSP lint on `tauri.conf.json`) |
| MUST open external links only through a Rust command that resolves the URL from state or checks an allowlist of `https` hosts; MUST NOT use `target="_blank"`/`window.open` for data-derived URLs | Review + ESLint rule |
| MUST install `on_navigation` (allow only the app origin) and an `on_new_window` handler that denies everything | Test |
| MUST set an explicit xterm `linkHandler` that routes through the same allowlisted opener, and MUST NOT load clipboard/OSC 52 addons | Review |
| SHOULD adopt the isolation pattern, validating IPC argument shapes in the hook | Design |
| MUST load no remote content (CDN fonts, scripts, images) | CI (CSP) |

---

## Threat model (STRIDE)

New attack surface (an IPC command, a socket, a server, file handling or spawning) adds or updates its row here, and gets its tests, in the same PR.

**Assets:**

- vendor sign-ins (Keychain items and vendor folders), which AI Juggler must never touch;
- transcripts and repositories;
- AI Juggler's database and settings;
- the Fleet identity key and peer pins;
- the vault key and recovery code;
- the device token;
- the update channel and its signing keys;
- the user's processes.

**Actors:**

- a network attacker on the LAN or Wi-Fi;
- a malicious or compromised paired peer;
- a malicious local process running as the same user, including a prompt-injected agent;
- a malicious dependency;
- a malicious backup file;
- a compromised Figurehead server or GitHub release;
- a malicious web page, through DNS rebinding or links.

**Trust boundaries:**

- TB1: webview ↔ Rust core (IPC)
- TB2: core ↔ child processes and PTYs
- TB3: core ↔ the filesystem outside the data dir (vendor folders, transcripts, user-chosen files)
- TB4: core ↔ the LAN (Fleet)
- TB5: core ↔ the internet (Figurehead, updater, stats)
- TB6: core ↔ other local processes (the future MCP socket)
- TB7: CI ↔ the release artifacts

| Component | S | T | R | I | D | E |
|---|---|---|---|---|---|---|
| **Webview UI** (TB1) | Script injected via rendered transcript/peer/agent text impersonates the user to IPC | UI state tampered by injected script | Actions from injected script indistinguishable from clicks | IPC returns transcripts/paths to injected script | Event flood / huge renders | **XSS → `terminal_write` → code exec**. Mitigate: CSP, no HTML sinks, command allowlist, isolation, nav guards |
| **IPC / Tauri core** (TB1) | Other windows/origins calling commands | Path/URL/program args from UI | No audit of destructive commands | Errors echo secrets/paths | Expensive commands spammed | Command trusts UI-supplied paths or programs; mitigate by id-resolution, scopes enforced in command |
| **Supervisor & PTY** (TB2) | Wrong process identified as ours (pid reuse) → ledger + identity verify | Env injection into child (inherited env) | Kill audit log (exists) | Child inherits secrets in env; PTY scrollback holds secrets | Fork bombs / runaway helpers (Cleanup Centre) | Shell interpolation (none today); PATH hijack of `security`/`git` |
| **Adapters, import, transcripts** (TB3) | A crafted folder posing as a vendor setup | Symlink swap during copy | Import result lists what came in (exists) | Credentials copied via symlink/rename; transcripts read into index | Huge/malformed transcripts (fuzz target exists) | Copy writes through absolute link into the original |
| **Storage (SQLite, data dir)** | — | Other same-user processes modify DB/overlays (`--settings` overlay files are executable config for Claude) | — | DB 0600 (good); data dir default perms | DB corruption | Overlay/statusline wrapper tampered → code runs inside Claude (keep 0700 dirs, owner checks) |
| **Backup / restore** (TB3) | Backup from another Mac flagged (exists) | Malicious `.ajbackup`; manifest hashes are not authenticity | Restore keeps `*.before-restore-*` (exists) | Export scrubbed + redaction-scanned (good) | Gzip/zstd bomb | Crafted SQLite triggers/views on open |
| **Cloud backup (FHB1)** (TB5) | Server swaps in another account's blob → key id mismatch refuses | AEAD detects; header not in AAD | — | Server holds ciphertext only; recovery code shown once | Server withholds/rolls back backups (accept; show dates) | — |
| **Fleet link** (TB4) | Impersonation with stolen identity file; pairing MITM prevented by SPAKE2 binding | mTLS integrity | Pairing attempt log (exists); command audit | mDNS leaks host name; telemetry to `View` peers | Pre-auth connection floods on `0.0.0.0` | `Control` peer launches/stops — bounded to id-only commands |
| **Figurehead account / stats / crash** (TB5) | Device token theft (Keychain, good) | TLS (rustls, native roots) | — | Opt-in, scrubbed, no identifiers | Endpoint outage degrades gracefully | — |
| **Updater & release** (TB5, TB7) | Fake feed → signature required | Swapped asset → minisign + `requireSignedVersion`; compromised build job | Release notes + tags; add provenance attestations | Signing secrets exposed to build or dependency code | GitHub outage → no update | Signed malicious update = code exec on every install |
| **macOS integration** (osascript, ScreenCaptureKit, TCC) | Window-title spoofing to focus wrong window | AppleScript splice | — | Screen capture read-only, not recorded | — | TCC grants (Screen Recording, Accessibility, Automation) are inherited by anything that can drive the app → keep IPC tight |
| **Launch hand-over socket** (TB6) | A local process answering for the app → per-user 0700 dir, 0600 socket, peer-uid and owner checks before trusting an answer | None: the ask carries no data | n/a | The answer is one fixed string | A same-user process can hold a connection for about a second; other users can't reach it | None: the only action is "bring the window forward" |
| **MCP server / socket / agent bridge** (TB6, future) | Any local process connects (need uid + token) | Injected instructions in messages | Need message audit log | Transcripts/repo data exfiltrated via a chained agent (lethal trifecta) | Message storms between agents | Agent A drives agent B to actions A couldn't take (confused deputy) → explicit user click, capability-tagged data (CaMeL) |

---

## References (all accessed 2026-10-21)

**Tauri**
- [T1] Tauri, Security overview — https://v2.tauri.app/security/
- [T2] Tauri, Capabilities — https://v2.tauri.app/security/capabilities/
- [T3] tauri-build `AppManifest` — https://docs.rs/tauri-build/latest/tauri_build/struct.AppManifest.html
- [T4] Tauri, Runtime Authority — https://v2.tauri.app/security/runtime-authority/
- [T5] Tauri, Permissions — https://v2.tauri.app/security/permissions/
- [T6] Tauri, Command Scopes — https://v2.tauri.app/security/scope/
- [T7] Tauri, Content Security Policy — https://v2.tauri.app/security/csp/
- [T8] Tauri, Isolation Pattern — https://v2.tauri.app/concept/inter-process-communication/isolation/
- [T9] Tauri, Application Lifecycle Threats — https://v2.tauri.app/security/lifecycle/
- [T10] Tauri, Updater plugin — https://v2.tauri.app/plugin/updater/ (plus `tauri-plugin-updater` 2.14.0 source, `src/config.rs`, read from the local cargo registry)
- [T11] Tauri, Shell plugin — https://v2.tauri.app/plugin/shell/
- [T12] Tauri, Opener plugin — https://v2.tauri.app/plugin/opener/
- [T13] `tauri::webview::WebviewWindowBuilder` (`on_navigation`, `on_new_window`) — https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindowBuilder.html

**OWASP and Electron**
- [O1] OWASP ASVS project page (5.0.0 stable) — https://owasp.org/www-project-application-security-verification-standard/
- [O2] OWASP ASVS 5.0 chapter list — https://github.com/OWASP/ASVS/tree/master/5.0/en
- [O3] Electron, Security checklist — https://www.electronjs.org/docs/latest/tutorial/security
- [O4] OWASP Desktop App Security Top 10 — https://owasp.org/www-project-desktop-app-security-top-10/
- [O5] OWASP OS Command Injection Defense Cheat Sheet — https://cheatsheetseries.owasp.org/cheatsheets/OS_Command_Injection_Defense_Cheat_Sheet.html
- [O6] OWASP Logging Cheat Sheet — https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html
- [O7] OWASP Cryptographic Storage Cheat Sheet — https://cheatsheetseries.owasp.org/cheatsheets/Cryptographic_Storage_Cheat_Sheet.html
- [O8] OWASP Threat Modeling Cheat Sheet — https://cheatsheetseries.owasp.org/cheatsheets/Threat_Modeling_Cheat_Sheet.html
- [O9] OWASP GenAI, LLM01:2025 Prompt Injection — https://genai.owasp.org/llmrisk/llm01-prompt-injection/

**Rust and dependency tooling**
- [R1] ANSSI, Secure Rust Guidelines — https://anssi-fr.github.io/rust-guide/ (rule ids read from the guide's source, https://github.com/ANSSI-FR/rust-guide, files `src/en/devenv.md`, `libraries.md`, `unsafe/generalities.md`, `integer.md`, `errors.md`, `unsafe/memory.md`, `standard.md`)
- [R2] RustSec — https://rustsec.org/
- [R3] cargo-auditable — https://github.com/rust-secure-code/cargo-auditable
- [R4] cargo-deny — https://embarkstudios.github.io/cargo-deny/
- [R5] cargo-deny, bans config — https://embarkstudios.github.io/cargo-deny/checks/bans/cfg.html
- [R6] cargo-vet — https://mozilla.github.io/cargo-vet/
- [R7] Rust std, `std::process::Command` — https://doc.rust-lang.org/std/process/struct.Command.html
- [R8] Rust blog, CVE-2024-24576 — https://blog.rust-lang.org/2024/04/09/cve-2024-24576/
- [R9] Rust blog, CVE-2022-21658 — https://blog.rust-lang.org/2022/01/20/cve-2022-21658/
- [R10] open(2) — https://man7.org/linux/man-pages/man2/open.2.html
- [R11] CWE-59 — https://cwe.mitre.org/data/definitions/59.html
- [R12] zeroize — https://docs.rs/zeroize/latest/zeroize/

**Supply chain and CI**
- [S1] SLSA v1.2 Build track — https://slsa.dev/spec/v1.2/build-track-basics (v1.1 levels: https://slsa.dev/spec/v1.1/levels)
- [S2] GitHub, Security hardening for GitHub Actions — https://docs.github.com/en/actions/security-for-github-actions/security-guides/security-hardening-for-github-actions
- [S3] GitHub, Artifact attestations — https://docs.github.com/en/actions/security-for-github-actions/using-artifact-attestations/using-artifact-attestations-to-establish-provenance-for-builds
- [S4] Sigstore, Cosign signing overview — https://docs.sigstore.dev/cosign/signing/overview/
- [S5] CycloneDX specification overview — https://cyclonedx.org/specification/overview/
- [S6] OpenSSF Scorecard — https://github.com/ossf/scorecard
- [S7] zizmor audits — https://docs.zizmor.sh/audits/
- [S8] GitHub, Dependabot options reference — https://docs.github.com/en/code-security/dependabot/working-with-dependabot/dependabot-options-reference
- [S9] Renovate configuration options — https://docs.renovatebot.com/configuration-options/
- [S10] pnpm, Supply chain security — https://pnpm.io/supply-chain-security
- [S11] pnpm audit — https://pnpm.io/cli/audit
- [S12] Socket for GitHub — https://docs.socket.dev/docs/socket-for-github
- [S13] GitHub, About code scanning with CodeQL — https://docs.github.com/en/code-security/code-scanning/introduction-to-code-scanning/about-code-scanning-with-codeql
- [S14] Semgrep, Supported languages — https://docs.semgrep.dev/supported-languages

**macOS**
- [M1] Apple, Hardened Runtime — https://developer.apple.com/documentation/security/hardened-runtime
- [M2] Apple, Notarizing macOS software before distribution — https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution
- [M3] Tauri, macOS code signing — https://v2.tauri.app/distribute/sign/macos/
- [M4] Apple, Keychain services — https://developer.apple.com/documentation/security/keychain-services
- [M5] Apple, `kSecUseDataProtectionKeychain` — https://developer.apple.com/documentation/security/ksecusedataprotectionkeychain
- [M6] Apple Platform Security, Controlling app access to files in macOS — https://support.apple.com/guide/security/controlling-app-access-to-files-secddd1d86a6/web

**Local IPC**
- [L1] unix(7) — https://man7.org/linux/man-pages/man7/unix.7.html
- [L2] getpeereid(3) — https://man.freebsd.org/cgi/man.cgi?query=getpeereid&sektion=3
- [L3] tokio `UnixStream::peer_cred` — https://docs.rs/tokio/latest/tokio/net/struct.UnixStream.html

**MCP and prompt injection**
- [P1] MCP, Security Best Practices (spec revision 2025-11-25) — https://modelcontextprotocol.io/specification/2025-06-18/basic/security_best_practices (the URL served the 2025-11-25 text)
- [P2] MCP, Transports (2025-11-25) — https://modelcontextprotocol.io/specification/2025-11-25/basic/transports
- [P3] S. Willison, "The lethal trifecta for AI agents" — https://simonwillison.net/2025/Jun/16/the-lethal-trifecta/
- [P4] Debenedetti et al., "Defeating Prompt Injections by Design" (CaMeL) — https://arxiv.org/abs/2503.18813
- [P5] Hines et al., "Defending Against Indirect Prompt Injection Attacks With Spotlighting" — https://arxiv.org/abs/2403.14720

**Secure by design and disclosure**
- [D1] CISA, Secure by Design — https://www.cisa.gov/securebydesign
- [D2] CISA, Secure by Design principles — https://www.cisa.gov/resources-tools/resources/secure-by-design
- [D3] CISA, Vulnerability Disclosure Policy Template — https://www.cisa.gov/vulnerability-disclosure-policy-template
- [D4] GitHub, Adding a security policy — https://docs.github.com/en/code-security/getting-started/adding-a-security-policy-to-your-repository
- [D5] GitHub, Private vulnerability reporting — https://docs.github.com/en/code-security/security-advisories/working-with-repository-security-advisories/configuring-private-vulnerability-reporting-for-a-repository
- [D6] RFC 9116, security.txt — https://www.rfc-editor.org/rfc/rfc9116

**Fuzzing**
- [F1] Rust Fuzz Book, cargo-fuzz — https://rust-fuzz.github.io/book/cargo-fuzz.html
- STRIDE definitions: Microsoft Threat Modeling Tool threats — https://learn.microsoft.com/en-us/azure/security/develop/threat-modeling-tool-threats

**Notes on the sources**

- **Apple pages** [M1][M2][M4][M5] were read through Apple's documentation JSON endpoints, because the HTML pages don't render without JavaScript.
- **Not fully read:** Renovate's option bodies [S9] and CISA's pledge goal list [D1].
- **Not confirmed from source:**
  - the Tauri default for `on_new_window` with no handler set;
  - whether CodeQL's Rust support is GA;
  - Apple's TCC treatment of Screen Recording, which the files page doesn't cover.
