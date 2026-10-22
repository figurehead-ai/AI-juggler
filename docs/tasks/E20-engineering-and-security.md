# E20 — Engineering and security foundations

> Source: the maintainer, 2026-10-21: "research best practices for building
> extensive apps like this in this language. Then create patterns, rules and
> guidelines for code written. We don't want bunch of spaghetti code, remember
> this is a public repo… Also research how to create best secure, and hack
> proof systems and guidelines." Research:
> `docs/research/engineering-standards.md` (43 sources) and a security review
> (60+ sources, 2026-10-21). The rules are in
> `docs/engineering/ENGINEERING-GUIDE.md` and
> `docs/engineering/SECURITY-GUIDELINES.md`. The architecture contract is
> ADR-0019.
>
> **Law for every task here:**
> - **No behaviour change** unless the task says otherwise. Refactors are
>   mechanical and tested before and after.
> - **Checks before cleanups.** A check lands with an allowlist of today's
>   sites that may only shrink, so new code can't add to the debt while the
>   old debt is paid down.
> - **Repository settings, secrets, environments and branch protection are
>   the maintainer's to change.** Agents prepare the change and the exact
>   steps, and the maintainer applies them.

## Security

### AJ-E2001 Sign and publish separately from the build | Status: TODO | Priority P0 | Effort L | Depends: —
**Goal**: No build or dependency code ever runs where the signing keys are.
**User story**: As a user, I can trust that an update is what the maintainers built, even if a dependency were compromised.
**Requirements**:
1. A **build** job with no secrets: `permissions: contents: read`, `persist-credentials: false`, exact-pinned tauri-cli (`--locked`) and pnpm. It produces the unsigned `.app` and the updater tarball as an artifact.
2. A **sign** job in a `release` environment (the maintainer moves the secrets there and sets the branch policy to `main`). It imports the certificate into a temporary keychain, then runs codesign, notarization (with an App Store Connect API key), staple, `tauri signer sign` and verify. It runs no third-party build code.
3. A **publish** job alone holds `contents: write`.
4. Nightly, Promote and the release notes keep working end to end. The first nightly after the change is verified like a release.
**UI/UX reasoning**: n/a.
**Performance budget**: the release takes at most +5 minutes.
**Quality gates**: a dry run on a branch with signing skipped; then the real nightly.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2001/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: `.github/workflows/build-macos.yml`, `nightly.yml`, `promote.yml`, `docs/RELEASING.md`.
**Research basis**: the security guidelines, Updater and release; Tauri's signing docs; GitHub environments.
**Handover note**: The maintainer moves the secrets into the environment and creates the API key; give them the exact steps.

### AJ-E2002 Agent bridge security design | Status: TODO | Priority P0 | Effort M | Depends: —
**Goal**: The local socket and MCP bridge are designed secure before any code exists.
**User story**: As a user, no other app on my Mac can talk to AI Juggler's agents, and no agent can make another act without my click.
**Requirements**:
1. Fold the guidelines' agent-bridge rules into ADR-0020 (AJ-E1701):
   - a 0700 directory and a 0600 socket;
   - a peer-uid check plus a per-run token;
   - no token passthrough;
   - messages are data, with provenance;
   - no state change without the user's click;
   - capability-tagged data, a defence against confused deputies.
2. Add its threat-model row and the test list.
**UI/UX reasoning**: n/a.
**Performance budget**: n/a.
**Quality gates**: AJ-E1701 can't merge without it.
**Evidence/validation proof**: the ADR.
**Visual testing**: n/a.
**Files to touch**: `docs/adr/0020-agent-bridge.md`, `docs/engineering/SECURITY-GUIDELINES.md`.
**Research basis**: the MCP spec's security practices; the security guidelines (agent bridge).
**Handover note**: —

### AJ-E2003 Lock down the webview: command allowlist, capabilities, navigation | Status: TODO | Priority P1 | Effort M | Depends: —
**Goal**: Injected script in the webview can do as little as possible.
**User story**: As a user, text from a transcript or another agent can never run commands on my Mac.
**Requirements**:
1. An `AppManifest::commands` allowlist in `build.rs`; commands are granted by capability files, per window.
2. A capability lint that forbids `shell:*`, `fs:*`, `http:*`, `opener:*` and remote URLs.
3. `on_navigation` allows only the app origin; new windows are denied; outside links go through a Rust opener with an https host allowlist; xterm gets an explicit link handler.
4. A stricter CSP: `script-src 'self'`, `object-src 'none'`, `base-uri 'none'`, `frame-ancestors 'none'`, `form-action 'none'`.
5. `core:default` is replaced by the subset actually used.
6. Commands take ids, never paths, URLs or programs, unless the user picked one in a native dialog in the same flow.
**UI/UX reasoning**: n/a.
**Performance budget**: n/a.
**Quality gates**: the lint; navigation tests; a grep check on command parameter types.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2003/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: `crates/juggler-app/build.rs`, `capabilities/`, `tauri.conf.json`, `lib.rs`.
**Research basis**: Tauri's capabilities, runtime authority and CSP docs.
**Handover note**: —

### AJ-E2004 Review for sensitive paths | Status: TODO | Priority P1 | Effort S | Depends: —
**Goal**: A change to release, capability, crypto or network code always gets a human look.
**User story**: As the maintainer, nothing touching signing or permissions merges unseen.
**Requirements**:
1. A `CODEOWNERS` file for `.github/**`, capabilities, `tauri.conf*.json`, `build.rs`, `deny.toml`, `crates/juggler-link/**`, `crates/juggler-account/**` and `SECURITY.md`.
2. One required approval on those paths. That's a branch-protection setting, the maintainer's step.
**UI/UX reasoning**: n/a.
**Performance budget**: n/a.
**Quality gates**: n/a.
**Evidence/validation proof**: the settings, recorded in `docs/RELEASING.md`.
**Visual testing**: n/a.
**Files to touch**: `.github/CODEOWNERS`.
**Research basis**: OpenSSF Scorecard (code review, branch protection).
**Handover note**: Agents keep merging the rest after review; only these paths wait for the maintainer.

### AJ-E2005 The Fleet identity key in the Keychain | Status: TODO | Priority P1 | Effort M | Depends: —
**Goal**: The key that proves this Mac to its peers can't be read as a plain file.
**User story**: As a user, a tool running on my Mac can't copy my Fleet identity.
**Requirements**: Migrate the key from its file to the Keychain, then remove the file. Add a test that the Fleet folder holds no private key, and test the migration both ways.
**UI/UX reasoning**: invisible.
**Performance budget**: n/a.
**Quality gates**: migration tests.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2005/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: `crates/juggler-link/src/identity.rs`.
**Research basis**: Apple Keychain services.
**Handover note**: —

### AJ-E2006 Hostile files: restore and import | Status: TODO | Priority P1 | Effort M | Depends: —
**Goal**: A crafted backup or folder can't harm the Mac or pull in what it shouldn't.
**User story**: As a user restoring a backup someone sent me, AI Juggler refuses anything unsafe.
**Requirements**:
1. Restore:
   - caps on total bytes, entry count and entry size;
   - `create_new` staging;
   - the staged database opened with `trusted_schema=OFF` and in defensive mode, then an integrity check and a schema comparison before it's used;
   - a decompression output cap in cloud restore.
2. Import copies only links that resolve inside the source, and lists outside links for the user. Copied files get the redaction scan.
3. Fixtures for hostile archives and symlink escapes.
**UI/UX reasoning**: refusals in plain words.
**Performance budget**: within the restore budget.
**Quality gates**: hostile fixtures.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2006/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: `crates/juggler-app/src/backup.rs`, `crates/juggler-adapters/src/*environment*.rs`.
**Research basis**: the security guidelines (files and paths); SQLite's defensive mode.
**Handover note**: —

### AJ-E2007 Supply chain and release evidence | Status: TODO | Priority P2 | Effort M | Depends: AJ-E2001
**Goal**: Every dependency is reviewed, and every release can be verified.
**User story**: As a user or auditor, I can check where a build came from and what went into it.
**Requirements**:
1. `cargo vet` with imported audits and an exemptions list that only shrinks.
2. `[bans.build]` in `deny.toml`.
3. `pnpm audit --prod` and signature checks, with pnpm's `minimumReleaseAge`, `trustPolicy` and `blockExoticSubdeps`, and a `packageManager` pin.
4. A Dependabot cooldown, and a digest-pinned Playwright image.
5. A CycloneDX SBOM for Rust and JS, build provenance attestations for the DMG, tarball and SBOM, `cargo auditable`, and `gh attestation verify` documented.
6. zizmor and OpenSSF Scorecard on PRs and weekly; a top-level read-only `permissions` in `ci.yml`; `persist-credentials: false` on every checkout.
**UI/UX reasoning**: n/a.
**Performance budget**: CI takes at most +3 minutes.
**Quality gates**: the checks themselves.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2007/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: `.github/workflows/`, `deny.toml`, `supply-chain/`, `ui/.npmrc`, `docs/RELEASING.md`.
**Research basis**: SLSA, Sigstore, OpenSSF Scorecard, cargo-vet.
**Handover note**: —

### AJ-E2008 Fuzzing on a schedule | Status: TODO | Priority P2 | Effort M | Depends: —
**Goal**: Parsers of untrusted input are fuzzed continuously.
**User story**: As a maintainer, crashes in parsers are found before users hit them.
**Requirements**: A nightly, time-boxed fuzz workflow with a cached corpus. New targets: backup archive staging, the cloud backup container, the recovery-code parser, pairing messages and shell quoting. The existing transcript targets join it.
**UI/UX reasoning**: n/a.
**Performance budget**: ≤ 30 minutes a night.
**Quality gates**: the targets build in CI.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2008/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: `fuzz/`, `.github/workflows/fuzz.yml`.
**Research basis**: cargo-fuzz; QUALITY.md's promised fuzz job.
**Handover note**: —

### AJ-E2009 Incident readiness: key-compromise runbook, SECURITY.md, security.txt | Status: TODO | Priority P2 | Effort S | Depends: AJ-E2001
**Goal**: If a key leaks, we know exactly what to do; researchers know how to reach us.
**User story**: As a researcher, I find the policy and a contact, and as a maintainer I have a tested plan.
**Requirements**:
1. A runbook covering: updater key rotation (a runtime list of public keys), Developer ID revocation, a secret rotation calendar, and incident communications.
2. SECURITY.md stating the supported channels and versions, safe harbour, a 90-day disclosure target, CVEs through GitHub advisories, and a fallback email.
3. `security.txt` on figurehead.ai.
**UI/UX reasoning**: n/a.
**Performance budget**: n/a.
**Quality gates**: n/a.
**Evidence/validation proof**: the docs.
**Visual testing**: n/a.
**Files to touch**: `docs/RELEASING.md`, `SECURITY.md`, the website repo.
**Research basis**: CISA secure by design; disclosure norms.
**Handover note**: The website change follows the website repo's rules.

### AJ-E2010 Process, crypto and listener hygiene | Status: TODO | Priority P3 | Effort M | Depends: —
**Goal**: The small hardening rules hold everywhere.
**User story**: As a user, what AI Juggler starts gets only what it needs.
**Requirements**:
1. An environment allowlist for spawned processes (clear the environment, then add only the allowed variables and the overlay), with tests.
2. Absolute tool paths (`/usr/bin/security`, `/usr/bin/open`, a resolved git), and `--` before path arguments.
3. AppleScript gets its values through `on run argv`, and an Apple Events usage string is added.
4. Fleet listener: choose the interface, limit the accept rate and concurrency, set keep-alive timeouts.
5. Cloud backup container v2: the header is bound into the AAD; keys and secret strings are `Zeroizing`; a `Redacted<T>` newtype, with a test that `AppError` never prints a secret.
**UI/UX reasoning**: n/a.
**Performance budget**: n/a.
**Quality gates**: tests per item.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2010/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: `crates/juggler-supervisor/`, `crates/juggler-link/`, `crates/juggler-account/`, `crates/juggler-app/src/`.
**Research basis**: the security guidelines (process spawning, crypto, network).
**Handover note**: The PATH resolution from the maintainer's 0.0.11 report (resolved once at startup) lives inside this allowlist.

## Engineering

### AJ-E2011 The cheap checks: stop the debt growing | Status: TODO | Priority P0 | Effort M | Depends: —
**Goal**: Each rule that can be checked is checked, so new code can't add to the debt.
**User story**: As a contributor, CI tells me when I break a rule, before review does.
**Requirements**:
1. **Tool-literal lint** (with AJ-E1525).
2. **A layering check** of `cargo metadata` against `docs/plan/layering.toml`, plus the rule that no adapter imports a sibling adapter.
3. **An IPC audit:**
   - UI command names match the registered commands;
   - uncalled commands are listed;
   - non-async commands need an allowlist or a marker;
   - command bodies are at most 40 lines;
   - command names follow `<feature>_<verb>`.
4. **Freshness of the generated TS types.**
5. **`clippy.toml` and the workspace lint additions:** `expect` with a reason, print and `dbg` lints, `disallowed-methods`.
6. **A size budget** for Rust and TS files.
7. **cargo-machete, knip and jscpd** (`--fail-on-new-clones`).
8. **`[workspace.dependencies]`.**
9. **A truth pass on QUALITY.md and ADR-0002:** add the gates they claim (`cargo-semver-checks`, blocking coverage) or reword them.
Every check starts with an allowlist that may only shrink.
**UI/UX reasoning**: n/a.
**Performance budget**: `just verify` takes at most +2 minutes.
**Quality gates**: each script has a self-test.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2011/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: `scripts/`, `justfile`, `.github/workflows/ci.yml`, `Cargo.toml`, `clippy.toml`, `docs/plan/`.
**Research basis**: the engineering guide's enforcement steps 1–9 and 12.
**Handover note**: —

### AJ-E2012 A UI linter and stricter TypeScript | Status: TODO | Priority P1 | Effort M | Depends: —
**Goal**: The UI gets the same rigour as the Rust code.
**User story**: As a contributor, lint catches hook mistakes, `any` and boundary breaks.
**Requirements**:
1. An ESLint flat config: typescript-eslint `strict-type-checked` (pinned), react-hooks, jsx-a11y, `max-lines`, complexity, and import boundaries (only `lib/ipc` calls the backend). Run it as `pnpm lint` in CI.
2. tsconfig: `noUncheckedIndexedAccess`, `noImplicitReturns`, `noImplicitOverride`, `verbatimModuleSyntax`.
3. Overrides for today's big files, which may only shrink.
**UI/UX reasoning**: n/a.
**Performance budget**: lint < 60 s.
**Quality gates**: lint clean in CI.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2012/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: `ui/eslint.config.js`, `ui/tsconfig*.json`, `ui/package.json`.
**Research basis**: the engineering guide, TypeScript and React.
**Handover note**: —

### AJ-E2013 Tracing with redaction | Status: TODO | Priority P1 | Effort M | Depends: —
**Goal**: One structured, redacted log instead of scattered prints.
**User story**: As a maintainer reading a bug report, I see what happened, and never a secret.
**Requirements**: Library crates use `tracing` only. The app installs one subscriber (a rolling file in the data folder, plus a redaction layer). `eprintln!` is replaced, spans are added around commands and long jobs, and the print lints are on.
**UI/UX reasoning**: n/a.
**Performance budget**: n/a.
**Quality gates**: a redaction test over the log output.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2013/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: every crate's `Cargo.toml`; `crates/juggler-app/src/`.
**Research basis**: the engineering guide, logging.
**Handover note**: Export diagnostics (AJ-E1517) reads this log.

### AJ-E2014 Slow commands off the main thread | Status: TODO | Priority P1 | Effort M | Depends: AJ-E2011
**Goal**: The window never freezes while AI Juggler scans, cleans up or switches.
**User story**: As a user, the app stays responsive while it works.
**Requirements**: Every command that touches the filesystem, SQLite, the network or processes is `async`, with its blocking work in `spawn_blocking`, starting with the scans, the cleanup inventory, switch execution and the kill ladder. The IPC audit's allowlist shrinks to pure commands only.
**UI/UX reasoning**: n/a.
**Performance budget**: no main-thread command over 16 ms.
**Quality gates**: the IPC audit.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2014/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: `crates/juggler-app/src/`.
**Research basis**: the engineering guide (threading); Tauri's command docs.
**Handover note**: Do it as part of AJ-E2017's split where possible.

### AJ-E2015 ARCHITECTURE.md | Status: TODO | Priority P1 | Effort S | Depends: —
**Goal**: A newcomer finds their way, and the invariants are written down.
**User story**: As a new contributor, I know where things live and what must never happen.
**Requirements**:
1. A root `ARCHITECTURE.md` in matklad's style: a bird's-eye view, a codemap, invariants stated as absences, boundaries, and cross-cutting concerns.
2. Each crate's `//!` doc gains a "must not know" line.
3. It is revisited at each milestone gate.
**UI/UX reasoning**: n/a.
**Performance budget**: n/a.
**Quality gates**: a staleness date check.
**Evidence/validation proof**: the doc.
**Visual testing**: n/a.
**Files to touch**: `ARCHITECTURE.md`, `crates/*/src/lib.rs`.
**Research basis**: matklad, "ARCHITECTURE.md".
**Handover note**: —

### AJ-E2016 Adapter building blocks and focused traits | Status: TODO | Priority P1 | Effort M | Depends: AJ-E1525
**Goal**: A new tool is data plus a few small impls.
**User story**: As a contributor adding a tool, I compose existing pieces and the conformance kit tells me what's missing.
**Requirements**:
1. Neutral `common/` modules in `juggler-adapters` (layout, probe, home, jsonl, sqlite). Move `Layout` and `installs_from` out of `claude_environment.rs`, and the ZCode credit logic out of `quota_ingest.rs`.
2. Split `HarnessAdapter` into capability traits (discover, launch, transcripts, quota, handover), each with 12 methods or fewer.
**UI/UX reasoning**: n/a.
**Performance budget**: n/a.
**Quality gates**: the conformance kit; `cargo-semver-checks` on `juggler-adapter`.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2016/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: `crates/juggler-adapter/`, `crates/juggler-adapters/`.
**Research basis**: ADR-0019 §2; the engineering guide (architecture).
**Handover note**: Before E16's first adapter.

### AJ-E2017 Features as modules: split commands.rs and AppState | Status: TODO | Priority P1 | Effort L | Depends: AJ-E2011
**Goal**: Each feature lives in its own folder, behind its own service.
**User story**: As a contributor, I change one feature without reading 14,000 lines.
**Requirements**:
1. `features/<x>/{service,repo,commands,dto,events}` with per-feature managed state, moved mechanically with no behaviour change.
2. The typed event enum.
3. A storage-concurrency ADR: per-repository connections or a pool instead of one `Mutex<Storage>`.
**UI/UX reasoning**: n/a.
**Performance budget**: unchanged.
**Quality gates**: the full suite before and after; the layering and IPC checks.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2017/manifest.md`.
**Visual testing**: unchanged.
**Files to touch**: `crates/juggler-app/src/`.
**Research basis**: ADR-0019 §3 and migration step 3.
**Handover note**: One release branch with no other feature work in flight (between 0.1 and E16).

### AJ-E2018 UI: split bridge.ts, keep preview out of production, a shell registry | Status: TODO | Priority P1 | Effort L | Depends: AJ-E2012
**Goal**: The UI grows by adding modules, not by growing App.tsx.
**User story**: As a contributor, a new screen is one registry entry and one folder.
**Requirements**:
1. `lib/ipc/<feature>.ts` holds the only `invoke` calls.
2. `src/preview/` is loaded lazily, outside the production bundle, with typed `PreviewHooks`.
3. A shell registry renders screens, inspector tabs, settings sections and Home cards, so App.tsx drops below 400 lines and its state moves into hooks.
4. A `useResource` hook is the one way screens read backend state.
**UI/UX reasoning**: no visible change.
**Performance budget**: the bundle doesn't grow; preview code is out of production.
**Quality gates**: vitest, Playwright and the visual gate are unchanged.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E2018/manifest.md`.
**Visual testing**: unchanged baselines.
**Files to touch**: `ui/src/`.
**Research basis**: the engineering guide (TypeScript and React).
**Handover note**: The same release branch as AJ-E2017.

### AJ-E2019 Typed command wrappers | Status: TODO | Priority P2 | Effort M | Depends: AJ-E2018
**Goal**: Command names and arguments are checked by the compiler on both sides.
**User story**: As a contributor, a renamed command can't break the UI silently.
**Requirements**: An ADR choosing tauri-specta (when 2.0 is stable) or an in-repo generator, then generate the wrappers.
**UI/UX reasoning**: n/a.
**Performance budget**: n/a.
**Quality gates**: the freshness check.
**Evidence/validation proof**: the ADR, and `docs/qa/evidence/AJ-E2019/manifest.md`.
**Visual testing**: n/a.
**Files to touch**: `crates/juggler-app/`, `ui/src/lib/ipc/`.
**Research basis**: the engineering guide (IPC).
**Handover note**: —
