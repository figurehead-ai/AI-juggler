# Evidence manifest — AJ-E2008

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2008, fuzzing on a
schedule. Branch `release/0.4.1` (D-83).
Status: **DONE-EVIDENCED** (agent self-check, 2026-11-06); the first
nightly run's result is recorded here once it has run on `main`.

The task has one requirement paragraph; its parts are numbered here.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | A nightly, time-boxed workflow with a cached corpus: `.github/workflows/fuzz.yml` (03:17 UTC, six targets in parallel, 4 minutes of fuzzing each under a 15-minute job cap, so a night takes at most 15 minutes, within the 30-minute budget; the corpus restored and saved per target through the Actions cache; a crash's input uploaded as `fuzz-crash-<target>`). |
| R2 | PASS | New targets (`fuzz/fuzz_targets/`): `backup_staging` (a `.ajbackup` staged as a restore stages it, `juggler_app::backup::stage_archive`; nothing outside staging), `cloud_container` (the cloud backup container and the recovery-code parser), `pairing_messages` (the typed code, the other side's SPAKE2 message and confirmation), `shell_quoting` (a quoted value reads back exactly). |
| R3 | PASS | The existing targets join it: `transcript_parse`, `link_frame_decode` are in the same matrix. The targets build in CI on any pull request touching `fuzz/`. |
