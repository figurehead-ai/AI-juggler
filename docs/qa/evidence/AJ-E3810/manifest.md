# Evidence manifest — AJ-E3810

Task: AJ-E3810 (the `models` section of the remote config, D-131).
Status: **DONE for the design, the schema and the typed section.** The
amendment is [ADR-0035 §11](../../../adr/0035-remote-config.md),
accepted 2026-11-18 by the coordinating session's review. Open question
3 (where our own model files are hosted) waits for the maintainer.
Nothing was downloaded to build this.

Tests: cargo `juggler-app` `features::ai::models` (`tests.rs`, `ledger.rs`)
and `features::remote_config::service::tests::the_published_schema_is_the_apps_own`;
cargo `juggler-remote-config` `tests/lists.rs`.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Amendment first | ADR-0035 §11 committed before the schema and the code | written, reviewed, accepted | accepted with two changes (11.5 basis wording, 11.9 verdicts) | PASS | c11dba0be; the acceptance commit | 2026-11-18 | Claude Opus; the coordinating session's review |
| R1 the entry fields (§11.1): id, name, engine, pinned URL, mirror, SHA-256, size, licence, minimum chip, memory and free disk, speed, sampling, per-job verdict and line | `models::tests::a_valid_entry_of_each_engine_is_read_typed`; `a_missing_or_malformed_file_field_drops_the_whole_entry`; `unknown_fields_are_ignored_and_unknown_verdicts_read_as_not_tested`; `the_list_is_capped` | typed as the ADR's table; a missing hash, size, URL or licence drops the entry; 16 entries at most; text caps 40 and 120 | pass | PASS | the AJ-E3810 section commit | 2026-11-18 | Claude Opus (self) |
| R1 the two new list field kinds: a bounded whole number and a SHA-256 | `juggler-remote-config` `tests/lists.rs` `counts_and_hashes_are_closed`, `the_registry_refuses_bad_list_declarations` | out of bounds, negative, fractional, string, upper-case, short or long hashes refused | pass | PASS | the list kinds commit | 2026-11-18 | Claude Opus (self) |
| R2 the hosts (§11.2): huggingface.co at a commit, official orgs only; Hugging Face's file hosts as the only redirect targets; models.figurehead.ai by hash | `models::tests::a_url_off_the_rules_drops_the_whole_entry` (host off the list, unofficial or wrong-case org, a branch name in place of a commit, upper-case commit, `..`, http, userinfo, port, percent-encoding, a query, a sub-folder, our path with another hash); `redirects_go_only_to_the_sources_file_hosts` | each refused; our own path with the entry's hash accepted | pass | PASS | the AJ-E3810 section commit | 2026-11-18 | Claude Opus (self) |
| R2 the schema in `config/remote/schema.json` | `remote_config::service::tests::the_published_schema_is_the_apps_own` (the registry's export, every channel source checked against it) | equal to the export | pass | PASS | the AJ-E3810 section commit | 2026-11-18 | Claude Opus (self) |
| R2 the trust rules: an unsigned or forged section is never read | `models::tests::an_unsigned_or_forged_models_section_is_never_read` (a genuine file is read; the same payload under another signature and a file with no signed part are refused; the app then offers the built-in list) | refused; built-in list only | pass | PASS | the AJ-E3810 section commit | 2026-11-18 | Claude Opus (self) |
| R2 the kill switch and an absent section (§11.4) | `models::tests::the_listing_tells_unread_from_empty` (no file, no section and a newer schema read as unread; an empty section as read; the section's Apple entry replaces the built-in one); `juggler-remote-config` `tests/lists.rs` `a_newer_or_broken_list_gives_nothing` | unread is not empty | pass | PASS | the AJ-E3810 section commit | 2026-11-18 | Claude Opus (self) |
| R2 `local_models_on`, the engine-wide stop | declared with the local engine (AJ-E3802, AJ-E3813) | — | not built here: no engine runs a downloaded model yet | PENDING | — | — | — |
| R3 the config can't add an engine | `models::tests::an_unknown_engine_or_format_drops_the_whole_entry`; `apples_entry_can_never_fetch_and_only_one_is_read` (Apple with a URL or a hash dropped; a second Apple entry dropped) | dropped whole | pass | PASS | the AJ-E3810 section commit | 2026-11-18 | Claude Opus (self) |
| R4 a withdrawn model stays usable with a note | `models::tests::withdrawn_models_stay_usable_with_a_note`; `a_downgraded_entry_never_replaces_the_installed_file` | withdrawn or absent: Withdrawn (works, note); unread: Unknown (works, no note); another file under the same id: ignored | pass | PASS | the AJ-E3810 section commit | 2026-11-18 | Claude Opus (self) |
| R5 listed only after passing D-130 for a job other than attention; the verdict recorded in `config/remote/models-ledger.json`, confirmed by Opus | `models::ledger::every_channel_source_passes_the_ledger`; `the_ledger_states_round_3s_basis_honestly`; `the_publisher_refuses_what_the_ledger_does_not_back` (a raised verdict, a changed file under an old id, only attention passing, a link in a line, an id not in the ledger; a narrowed verdict accepted); `the_ledger_must_back_each_pass` (missing evidence, not confirmed by Opus, round 3's basis on another id, no file recorded yet) | refused as listed | pass | PASS | the AJ-E3810 section commit | 2026-11-18 | Claude Opus (self) |
| R5 round 3's six models: the basis is D-131's acceptance of `docs/qa/evidence/AJ-E3709/v5-final.md` (c8b4ec881), the critical bar met on the after-fix floor under the shared rules, not on a fresh blind run; any later model needs a full harness run | the ledger's `bases.round3_after_fix_floor.statement`; `the_ledger_states_round_3s_basis_honestly` | stated in plain words, six ids only | pass | PASS | the AJ-E3810 section commit | 2026-11-18 | Claude Opus (self) |
| R6 the built-in list: Apple's model only, read by the same reader | `models::tests::the_built_in_list_is_apple_only_through_the_same_reader` | one Apple entry with round 3's verdicts; no targeting; offered when nothing is read | pass | PASS | the AJ-E3810 section commit | 2026-11-18 | Claude Opus (self) |

**The nightly source** now lists Apple's model, Qwen3 1.7B, Nemotron 3
Nano 4B, Granite 4.1 3B and Gemma 4 E2B, with the verdicts and lines of
round 3's recommendation table. The source changes nothing live until
Publish config runs with the maintainer's approval. Qwen3 4B Instruct
waits for our own conversion (open question 3) and its verification
run. Beta and stable list nothing yet.
