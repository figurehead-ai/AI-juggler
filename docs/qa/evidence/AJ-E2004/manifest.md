# Evidence manifest — AJ-E2004

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2004, review for
sensitive paths. Branch `release/0.4.1` (D-83).
Status: **DONE-EVIDENCED** for R1; R2 is the maintainer's setting.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | `.github/CODEOWNERS`: `.github/`, `crates/juggler-app/capabilities/` and `permissions/`, `tauri.conf.json` and `tauri.*.conf.json`, every crate's `build.rs`, `deny.toml`, `crates/juggler-link/`, `crates/juggler-account/`, `SECURITY.md`, all owned by the maintainer. |
| R2 | DEFERRED (D-88) | The setting that would make code-owner review required stays off (D-88, 2026-11-06): every pull request is authored by the maintainer's account, and GitHub never counts an author's own approval, so it would block every release pull request. CODEOWNERS requests the review; `docs/RELEASING.md` › "Review for sensitive paths (AJ-E2004)" records why and what would let it be turned on (an agent identity of its own). |
