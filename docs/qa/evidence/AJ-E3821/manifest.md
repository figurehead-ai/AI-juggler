# Evidence manifest: AJ-E3821

**Task:** `docs/tasks/E38-chat.md`, AJ-E3821: Remove lets the engines go of the model before deleting it, so the disk is free when Remove says so (found in `docs/qa/evidence/AJ-E3712/qwen-download-check.md`, step 4).

**Branch:** `release/0.5.1`. Fix: 5296623e2 (`ai_model_remove` calls `service::release_engines()` before the store's remove; each engine's `release` drops its running helper). Closed out 2026-11-23 (Opus).

## By requirement

R1: OPEN — no test covers it and no PL1 step re-checks it yet. Only part of it is tested: an engine lets go of its helper on `release` (`engines::local::tests::a_job_loads_the_model_once_then_generates_with_its_frame_shape_and_sampling` ends with `release()` and the card no longer running). No test covers Remove calling it before the delete, or the disk being free when Remove returns. The proof the task names is a live re-check of Remove's free space (AJ-E3712's re-check row, "the E2323 PL1 walk"), but `docs/qa/0.5-walks/0.5.1-PL1.md` has no step for it yet: Part H needs one.
R2: OPEN — no test: a job running on the model while Remove releases its engine should end with the engine's usual stop, never a crash report. This needs a fake-helper test in `engines/local/tests.rs`, or the same PL1 step with a Bot answer running.

## Redaction checklist

- [x] No credentials, tokens, or keys here
- [x] No captures here
- [x] No real names or home paths
