# Evidence manifest — AJ-E1207

Task: `docs/tasks/E12-optional-account.md` AJ-E1207 (guardrail tests and
visual captures): one test for every promise the cloud path makes (the
two-path law, ADR-0014). Every guardrail runs against a recording fake
of the service; none needs the network. 2026-10-18, reviewer
claude-opus.

| # | Promise | Test | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| 1 | Signed out is silent: zero connections | `cloud_guardrails :: g1_signed_out_is_silent` tries every cloud-facing call (backup now, schedule tick, list, restore, discovery list and pin, status) with no account and sees **0** requests. Also `figurehead :: signed_out_is_silent` (AJ-E1202) | PASS | 2026-10-18 | claude-opus |
| 2 | Toggles mean it: a feature that's off never calls its endpoint (backup, discovery, usage sync) | `g2_a_feature_that_is_off_never_calls_its_endpoint`: linked, all three off, everything tried → no `/backups`, no `/devices`, no usage call, no request at all. Also `cloud_backup :: off_or_unsaved_code…` and `fleet_discovery :: off_means_no_devices_call` | PASS | 2026-10-18 | claude-opus |
| 3 | No plaintext leaves the Mac: seeded strings never appear in uploaded bytes, which start with the format magic | `g3_g4_no_plaintext_and_no_credentials_leave_the_mac`: an Account label, a project path under a fake `$HOME`, the `$HOME` name itself and seeded sign-in material are all absent from the upload, which starts with `FHB1`. Also `vault_container` and `cloud_backup :: round_trip…` | PASS | 2026-10-18 | claude-opus |
| 4 | No credentials: a filesystem-access audit during backup shows no reads of Provider credential files | Same test: the export's read record (every file it opens comes from its allowlist plus the live database) holds nothing under the Account folders or `fleet/`, and no credential or `.claude` file. Also `backup_restore :: rehearsal…`: seeded credentials and the Fleet key appear in no archive entry | PASS | 2026-10-18 | claude-opus |
| 5 | The token exists only in the Keychain (the `fhd_` prefix appears nowhere else) | `g5_the_device_token_exists_only_in_the_keychain`: after linking, backing up and discovery, the token is only in the secret store; `fhd_` is in no byte under the data folder and no uploaded body. Also `figurehead :: a_browser_link_stores_the_token_only_in_the_secret_store` | PASS | 2026-10-18 | claude-opus |
| 6 | A `401` removes the token and turns every toggle off; local data unchanged | `g6_a_401_unlinks_locally_and_keeps_local_data`: a 401 on upload → token gone, every toggle off, projects unchanged, and silent afterwards. Also `figurehead :: a_401_unlinks_locally_and_keeps_local_data` | PASS | 2026-10-18 | claude-opus |
| 7 | Entitlements drive the UI: a paid-and-disabled feature shows Upgrade and makes no calls; `402` stops uploads | `g7_entitlements_drive_behaviour`: a paid, disabled Cloud backup → `access: upgrade`, not stored as on, and no `/backups` or `/devices` call. Included but answered 402 → the upload stops with "plan doesn't include". The Upgrade button is in gallery `44-settings` (paid-locked) | PASS | 2026-10-18 | claude-opus |
| 8 | The desktop path is unchanged: the full existing suite passes with the account feature compiled in and never used | `just verify` (every Rust suite), the full vitest and the full Playwright suite on this branch, with the account code compiled in and no account linked | PASS (full gate on the PR) | 2026-10-18 | claude-opus |

Budget: the guardrail suite runs in **0.18 s** (`cloud_guardrails`) and
3.7 s (`figurehead`), against the 30 s budget. Log: `guardrails.log`.
The visual half is AJ-E1203's gallery (`44-settings`) together with
`59-vault`, `60-cloud-backup` and `61-account-macs`.
