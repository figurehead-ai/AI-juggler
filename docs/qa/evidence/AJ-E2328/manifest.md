# Evidence manifest: AJ-E2328

**Task:** `docs/tasks/E23-session-runtime.md`, AJ-E2328: A move carries the worktree's ownership (the Claude app's worktree entry, its cleanup's sighting, the side-session notes and the permission paths) with the conversation; a health check finds and repairs sessions already split.

**Branch:** `release/0.5.1`. Fixtures only: temp profile folders, a fixture git folder per worktree, a scripted system in place of every quit and start. No real profile was written; the one look at the real profiles was a read-only dry detection (below).

## What changed

- The Claude adapter keeps the formats it relies on in one manifest (`claude_move/known_formats.json`), explained in `docs/design/claude-known-formats.md`; `scripts/claude_compat_check.py` compares this Mac with it.
- A move carries the worktree's ownership: the target profile's worktree file owns it on the branch it is really on, the source forgets it and its cleanup's sighting, the notes folder and the record's paths move, both files backed up and undone byte for byte; refused in words on a format or app version the manifest doesn't name.
- The move preview says what goes with the conversation (its worktree, which App owns it after, its notes).
- Settings › Desktop apps › Moving conversations: each app's version and whether moves were checked on it; a conversation an older move left split has Repair…, whose sheet names the Apps it quits and says the worktree's files are never touched; Repair takes the move's own path. A worktree another App's cleanup has seen is an info line.

## Requirements

| Requirement | Verdict | Proof | Commit | Date |
|---|---|---|---|---|
| R1 formats researched behind the adapter; a version gate refuses with the reason | PASS | `known_formats.json` + `docs/design/claude-known-formats.md`; `claude_move_matrix::e2328_the_format_gate_refuses_an_unknown_schema_and_version` (unknown schema and unchecked app version refuse in words, Copy still works) | cd31399aa, d8e1a47a7, 1b137d3ed | 2026-11-23 |
| R2 the lease is carried as a reversible step with a backup; Undo restores both files byte for byte | PASS | `claude_move_matrix::e2328_a_worktree_moves_with_its_session_and_the_old_sweep_removes_nothing`, `e2328_undo_restores_both_leases` | 5c1541d49, d8e1a47a7 | 2026-11-23 |
| R3 the real branch is recorded (never a checkout); an unreadable HEAD refuses in words | PASS | `claude_move_matrix::e2328_a_diverged_head_is_recorded_never_checked_out` | d8e1a47a7 | 2026-11-23 |
| R4 the source's sighting is dropped; a root another profile also sweeps is named in the notes | PASS | `claude_move_matrix::e2328_a_worktree_moves_with_its_session_and_the_old_sweep_removes_nothing` | d8e1a47a7 | 2026-11-23 |
| R5 the notes folder moves as a host-side area; source-profile paths in grants and shell folders point at the target | PASS | `claude_move_matrix::e2328_the_notes_folder_and_its_grants_move` | 5c1541d49, d8e1a47a7 | 2026-11-23 |
| R6 the preview lists the worktree(s) and notes in plain words; Undo restores both profiles' files and the notes | PASS | the move preview's worktree/notes lines (UI) and `e2328_undo_restores_both_leases` | 20e11bd93 | 2026-11-23 |
| R7 the health check finds split sessions and Repair names and quits only the involved profiles (consent first), moves the ownership, clears the stale sighting, restarts, verifies, never touches the worktree's files; refused cleanly | PASS | `handover::moving::repair::tests::the_check_names_a_split_conversation_the_profiles_repair_quits_and_the_app_version`, `repair_quits_only_the_two_profiles_moves_the_ownership_and_never_touches_the_worktree` (quits exactly the two by process, both started again, B owns it, A's sighting gone, worktree bytes equal, re-detection finds nothing), `a_busy_profile_refuses_the_repair_before_anything_is_quit`, `an_unchecked_app_version_refuses_the_repair_in_words`, `nothing_to_repair_shows_no_finding_and_an_unchecked_version_says_so`; the maintainer's ruling: `a_conversation_with_two_worktrees_keeps_both`, `claude_move_matrix::e2328_the_repair_keeps_a_conversations_two_worktrees`, `a_worktree_another_profile_tidies_is_an_info_line`, `claude_move_matrix::e2328_a_worktree_another_profile_tidies_is_told`; UI `moving-section.test.tsx` (sheet names the Apps it quits, says the files are never touched, acts only on the confirm, Cancel repairs nothing, no internal words on screen) | e1636cb5b, 83aea6be4, f64a1d766 | 2026-11-23 |
| R8 the matrix cases | PASS (two caveats) | `claude_move_matrix` e2328 cases (8, all green): moved then old sweep, diverged HEAD, notes folder, Undo restoring both, format gate, the repair on a 0.4.x-split fixture, two worktrees, another profile tidying. Caveats: "the new profile resuming in the same tree" checks only the worktree entry's folder and branch (no app is run); the old profile's cleanup is checked against a stand-in of the app's rule as its logs show it (`sweep_would_remove`), not the app's own sweep | d8e1a47a7, e1636cb5b | 2026-11-23 |
| R9 a known-format manifest per Claude app version; `claude_compat_check.py` (read-only, no network, no environment reads; ok / drift / unknown_version) | PASS | `claude_compat_check.py --selftest`: ok (0 failures) | cd31399aa | 2026-11-23 |
| R9 c the gate reads the same manifest; Settings › Apps shows "Claude app <version> · moves checked" / "· not yet checked" | PASS | the move and Repair refuse an unchecked version (R1, R7 tests); the gate is asked through the adapter (`InPlaceMove::moves_checked`); `moving-section.test.tsx` "says the Claude app's version and whether moves were checked on it"; `repair::tests::the_check_names_…_and_the_app_version` (version read from the app bundle as the move reads it) | e1636cb5b, 83aea6be4 | 2026-11-23 |
| R9 d documented in `docs/design/` | PASS | `docs/design/claude-known-formats.md` (formats, the gate, updating the manifest with the script) | 1b137d3ed | 2026-11-23 |

## Checks run

- `cargo test -p juggler-adapters --test claude_move_matrix e2328`: 8 passed.
- `cargo test -p juggler-app --lib repair::tests`: 7 passed.
- `cargo clippy -p juggler-adapter -p juggler-adapters -p juggler-app --all-targets -- -D warnings`: clean for this item's code (other items' in-progress code tripped `collapsible_if`, `doc_markdown` and `panic`, allowed for this run only).
- UI: `vitest run moving-section.test.tsx desktop-apps.test.tsx` 14 passed; `tsc -b --noEmit` clean; `ui_lint.py` nothing on these files.
- `private_data_lint.py`: clean. `claude_compat_check.py --selftest`: ok.

## Dry detection on the real profiles (read-only)

The adapter's check run once against this Mac's Claude app profiles, printing counts and worktree names only (a temporary example, deleted after): 8 profiles with a worktree file; the installed Claude app 5.11492.5 is checked; 3 split conversations (`feature-videos-977838`: owned by another profile; `qa-real-walk-be289a`: owned by another profile, branch differs, two worktrees; `quizzical-lamport-f9e1a1`: branch differs only); 2 worktrees another profile tidies (`brave-margulis-815701`, `quizzical-lamport-f9e1a1`). Nothing was repaired.

## Left

- The live walk in AJ-E2323's PL1 (a refused then successful move of a session in its own worktree, the lease following it) belongs to that walk.
