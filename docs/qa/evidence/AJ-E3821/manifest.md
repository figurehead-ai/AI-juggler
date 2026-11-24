# Evidence manifest: AJ-E3821

**Task:** `docs/tasks/E38-chat.md`, AJ-E3821: Remove lets the engines go of the model before deleting it, so the disk is free when Remove says so (found in `docs/qa/evidence/AJ-E3712/qwen-download-check.md`, step 4).

**Status:** DONE (0.5.1)

**Branch:** `release/0.5.1`. Fix: 5296623e2 (`ai_model_remove` calls `service::release_engines()` before the store's remove; each engine's `release` drops its running helper). Closed out 2026-11-23 (Opus); the tests below added for 0.5.1 F16.

## By requirement

R1: PASS — Remove releases the engines before the store deletes the file: cargo `features::ai::models::commands::tests::remove_releases_the_engines_before_it_deletes_the_file` (in `ai_model_remove`, `release_engines()` comes before `.remove(&model_id)`). The release lets the file go at once: cargo `features::ai::engines::local::tests::release_kills_and_reaps_the_loaded_helper_before_it_returns` (a fake helper loaded by a job; when `release` returns its process is gone, reaped, and the file is deleted). Live re-check: PL1 step H6 (`docs/qa/0.5-walks/0.5.1-PL1.md`), run when the lab holds a model.
R2: PASS — cargo `features::ai::engines::local::tests::a_job_running_at_release_ends_as_usual_then_the_helper_goes`: a job generating when `release` is called ends with its usual answer (release waits for it), the helper goes after, and the engine stays Ready (no crash counted). Live: PL1 step H6's second run (Remove with a Bot answer running).

## Redaction checklist

- [x] No credentials, tokens, or keys here
- [x] No captures here
- [x] No real names or home paths
