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

## The run: not completed

Each build was opened with `open -n --env AIJ_NO_RESTORE=1 --env
HOME=<throwaway>` from the scratch copy, never from `/Applications`, beside
the maintainer's running AI Juggler (left untouched). The keychain is off
for a run whose `HOME` isn't the user's (`juggler_platform::keychain::off`),
the hand-over socket is per data folder, and no login item is registered on
a first launch: checked in the code before running.

- 0.4.3 on a fresh throwaway home opened its data (schema 52, `juggler.lock`,
  `last-run`, the app log) but exited about half a second after each start;
  its crash watchdog started it again three times (`crash-relaunches`), then
  stopped. No macOS crash report was written. The main window never showed.
- The next launches (0.4.4 over that data, then 0.4.3 again) logged nothing
  in the throwaway data folder; the binary run directly with the throwaway
  `HOME` exited 0 at once and created no file in it.
- The maintainer's own data was not opened: its `app.log` (last written
  2026-11-12 17:35) has no "test run" line, and its `last-run` is unchanged.

So the production edition doesn't stay up when launched from a scratch copy
with a throwaway `HOME` beside the running AI Juggler, and the rehearsal
can't be done this way. Cleanup: no process left, nothing mounted, the
scratch folder deleted; the two provenance records were kept.

## Next

Rehearse on CI's release-profile Dev builds of the two commits (`rc-dev-app`,
`scripts/qa/fetch_rc.sh`): the same code, its own bundle id, data folder,
lock, keychain items and port, as the release-candidate checks already run.
The short-lived production run above is a lead for the 0.4.x line (why a
production build exits at once from a scratch copy with another `HOME`).
