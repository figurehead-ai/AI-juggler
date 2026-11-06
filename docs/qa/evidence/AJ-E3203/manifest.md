# AJ-E3203 — Claude app profiles found when they aren't running

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | Asked once on the App page, names only: walk A6 (`docs/qa/0.4-walks/A6.md`); `crates/juggler-adapter/src/siblings.rs::no_file_inside_a_profile_folder_is_opened`; `find-profiles.test.tsx` |
| R2 | PASS | The Privacy page's line ("…offered too; you add the ones you use. Only the names…"), checked in A6; `crates/juggler-app/tests/trust.rs::every_trust_claim_names_tests_that_exist`, `privacy-page.test.tsx` |
| R3 | PASS | D-84: Find them lists offers with Add / Not now; Add makes "Claude app · Work Pro" an App (walk A6, captures `walks-0.4/A6/2-find-them-*`, `3-three-profiles-*`); `find-profiles.test.tsx` |
| L2 | GAP | The L2 harness on the replica isn't run yet |
