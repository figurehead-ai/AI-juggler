# Evidence manifest — AJ-E2007

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2007, supply chain
and release evidence. Branch `release/0.4.1` (D-83). Status: **DONE**
(agent self-check, 2026-11-06). Local runs are below. The release-only
parts (R5) and the new workflow (R6) first run in CI on the release PR
and the next nightly; confirm them there.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | `supply-chain/` from `cargo vet init`, with audits imported from Mozilla, Google, the Bytecode Alliance, ISRG, Embark and Zcash, then pruned. 137 crates are fully audited, 4 partially, 604 exempted. `cargo vet --locked`: "Vetting Succeeded". `scripts/vet_exemptions.py` (selftest passes) fails a change that adds exemptions or exempts a crate the base didn't. CI's static job and `just supply-chain` run both, with cargo-vet 0.10.3 prebuilt and digest-checked (`scripts/install_release_tool.sh`). |
| R2 | PASS | `deny.toml` `[bans.build]`: executables and interpreted scripts denied, dependencies and archives included, builtin globs on. It first flagged 2,934 files in 54 crates: Windows import libraries, the JS Tauri embeds, ring's perlasm and maintainers' scripts. Each crate is named in `bypass` by exact path (by glob for winapi's import libraries and ring's perlasm), grouped with why, so a new file fails. `cargo deny check`: exit 0. |
| R3 | PASS | `ui/pnpm-workspace.yaml`: `minimumReleaseAge: 1440`, `trustPolicy: no-downgrade`, `blockExoticSubdeps: true`; pnpm reads all three (`pnpm config get`). `packageManager: pnpm@10.34.7+sha512…` in `ui/package.json`. A fresh resolution under the policy passes, and so does `pnpm install --frozen-lockfile`. `pnpm audit --prod --audit-level moderate`: "No known vulnerabilities found", now a step in CI's ui job. pnpm has no separate signature audit: the trust policy is its provenance check. |
| R4 | PASS | `.github/dependabot.yml`: a cooldown on every ecosystem (7 days; majors 14 for cargo and npm). The Playwright image is already digest-pinned (`@sha256:47b510dd…`) in every workflow and script. |
| R5 | PASS (local; confirm on the next nightly) | `build-macos.yml` compiles the app through `cargo auditable` (`--runner scripts/cargo_auditable_runner.sh`) and the agent bridge with `cargo auditable build`; cargo-auditable 0.7.7 is prebuilt and digest-checked. Locally the runner put the `__DATA,.dep-v0` section in `juggler-agent`. SBOMs (CycloneDX 1.5) come from the main arm64 compile: cargo-cyclonedx 0.5.9 for juggler-app (520 components locally) and juggler-agentbridge, and `scripts/js_sbom.py` for the UI (10 packages, matching `package.json`; selftest passes). `nightly.yml` attests build provenance for the DMG, the `.app.tar.gz` and the SBOMs (`actions/attest-build-provenance`, SHA-pinned; `id-token`/`attestations: write` on that job only). `gh attestation verify` is in `docs/RELEASING.md`. |
| R6 | PASS (local; confirm on the release PR) | `.github/workflows/supply-chain.yml`: zizmor 1.32.0 (prebuilt, digest-checked) on the workflows, and the OpenSSF Scorecard (`ossf/scorecard-action`, SHA-pinned), on pull requests that touch workflows, on main and weekly. Scorecard publishes from main only. `ci.yml` keeps a read-only top-level `permissions` (`contents: read`, `pull-requests: read`). `persist-credentials: false` is now on every checkout: visual-sweep's five were the ones missing. actionlint passes on every changed workflow. zizmor wasn't run locally (the session may not download and run binaries), so its first findings will show on the release PR. |

Performance budget (CI at most +3 minutes): cargo-vet is a download plus a
run of a few seconds, `pnpm audit` a few seconds, and the SBOMs about half
a minute in one compile job.
