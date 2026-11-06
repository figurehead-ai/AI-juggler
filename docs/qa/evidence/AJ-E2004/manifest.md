# Evidence manifest — AJ-E2004

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2004, review for
sensitive paths. Branch `release/0.4.1` (D-83).
Status: **DONE-EVIDENCED** for R1; R2 is the maintainer's setting.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | `.github/CODEOWNERS`: `.github/`, `crates/juggler-app/capabilities/` and `permissions/`, `tauri.conf.json` and `tauri.*.conf.json`, every crate's `build.rs`, `deny.toml`, `crates/juggler-link/`, `crates/juggler-account/`, `SECURITY.md`, all owned by the maintainer. |
| R2 | PASS | The branch-protection setting and what it changes are recorded in `docs/RELEASING.md` › "Review for sensitive paths (AJ-E2004)". Turning it on is the maintainer's step (the task says so); the record is this requirement's evidence. |
