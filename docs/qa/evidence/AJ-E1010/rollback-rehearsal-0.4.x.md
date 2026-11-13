# Rollback rehearsal on real signed builds, 0.4.3 and 0.4.4 (attempt, 2026-11-13)

AJ-E1010 R2: "A rollback path to the previous version SHALL be documented
and rehearsed once per release channel". The maintainer approved
downloading the two releases' DMGs for this ("yes, download the 0.4.3 and
0.4.4 DMGs for the rollback rehearsal", 2026-11-13). Run by claude-opus.

## The downloads: PASS

| Check | 0.4.3 | 0.4.4 |
|---|---|---|
| `SHA256SUMS.txt` of the release | OK | OK |
| Build provenance (`gh attestation verify … -R figurehead-ai/AI-juggler`) | `nightly.yml`, `refs/heads/main` | `nightly.yml`, `refs/heads/main` |
| The app copied from the read-only mounted DMG: `codesign --verify --deep --strict` | valid | valid |
| Gatekeeper (`spctl --assess`) | Notarized Developer ID | Notarized Developer ID |
| Version in the bundle | 0.4.3 | 0.4.4 |

Database schema: no change between 0.4.3 and 0.4.4 (no migration in
`crates/juggler-core/src/storage/` between the tags), so the documented way
back is "install the previous `.dmg` over the newer one".

## The run: not completed, and why (expected behaviour)

Each build was opened from the scratch copy (`open -n --env AIJ_NO_RESTORE=1
--env HOME=<throwaway>`), never from `/Applications`, while the maintainer's
installed AI Juggler was running. Each launch ran about half a second and
exited; its crash watchdog started it again three times; the direct run
exited 0 at once. No crash report.

**Cause, confirmed in the code:** `tauri-plugin-single-instance` (2.5.5,
`crates/juggler-app/src/lib.rs`, `single_instance::init(|app, …|
main_window::bring_forward(app))`) is keyed by the bundle id: its socket is
a fixed path under `/tmp` named from `ai.figurehead.juggler`
(`platform_impl/macos.rs`, `socket_path`). A second instance with the same
bundle id hands over to the running one and exits, whatever its `HOME`; the
running one brings its window forward. So these launches were handed to the
maintainer's own AI Juggler, whose window came forward each time. Expected
behaviour, not a bug. The maintainer's data was not opened (its `app.log`
has no "test run" line; `last-run` unchanged).

**Rule since:** no production-identity build is launched while the installed
app runs. No supported override gives a release build another single-instance
identity (the socket follows the signed bundle id; the edition comes from the
same id), so the rehearsal runs on Dev-identity builds of the same commits.

Cleanup: no process left, nothing mounted, the scratch folder deleted; the
two provenance records kept.

## Next

The same rehearsal on CI's release-profile Dev builds (`rc-dev-app`) of the
two commits: the same code under the Dev identity (its own bundle id, data
folder, lock, keychain items, port and single-instance socket). 0.4.3's,
`rc-b281d3ee6fdf` (`b281d3ee6`, the same tree as `v0.4.3`), is already on
this Mac; 0.4.4's is `rc-dev-app-0d2f9268b…` (26 MB, run 43391684003).
