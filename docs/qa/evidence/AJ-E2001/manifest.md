# Evidence manifest — AJ-E2001

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2001, sign and
publish separately from the build. Status: **DONE-EVIDENCED** (agent
check of the workflows as they ran 0.4.0, 2026-11-06). The 0.1 manifest's
"parked" note is superseded by this one.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | `.github/workflows/build-macos.yml`: the `compile` and `build` jobs hold `permissions: contents: read` and no secrets, check out with `persist-credentials: false`, fetch with `cargo fetch --locked`, install tauri-cli from its release zip by checksum and exact version (`scripts/install_tauri_cli.sh`), pnpm by version and sha512 (`PNPM:`), and upload the unsigned `.app` and updater tarball as an artifact. |
| R2 | PASS | The `sign` job runs in the `release` environment (secrets there, branch policy `main`): a temporary keychain, `codesign`, notarization with the App Store Connect API key, staple, `tauri signer sign`, verify (`scripts/sign_macos.sh`); it builds nothing and runs no dependency code. `sign-rehearsal` proves the same steps without secrets. |
| R3 | PASS | In `nightly.yml` only the two publishing jobs hold `contents: write`: `release` (named "publish": downloads the signed artifacts, writes the feeds and checksums with the repo's own Python, `gh release create`) and `publish` ("nightly feed": `gh release upload`). Neither builds or runs third-party code; every other job is read-only or `permissions: {}`. |
| R4 | PASS | 0.4.0 went end to end on 2026-11-06: nightly run 42169341655 (built, signed, notarized, published v0.4.0 with its notes), promote run 42171949364 (beta/v0.4.0, the beta-channel feed checked to serve 0.4.0). `release-rehearsal.yml` dry-runs the chain on any branch. |
