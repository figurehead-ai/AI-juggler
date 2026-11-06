# Evidence manifest — AJ-E2005

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2005, the Fleet
identity key in the Keychain. Branch `release/0.4.1` (D-83).
Status: **DONE-EVIDENCED** (agent self-check, 2026-11-06).

The task has one requirement paragraph; its three parts are numbered here.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | Migrate the key from its file to the Keychain, then remove the file: `juggler_link::MachineIdentity::load_kept` with a `KeyKeeper` (`crates/juggler-link/src/identity.rs::kept_secret`): a key only in its old file is put in the keeper, read back, and only then is the file deleted; a keeper that refuses leaves the file in place. The app keeps the identity in the edition's keychain (`crates/juggler-app/src/fleet.rs::keep_in_keychain`, items `fleet/identity.key` and `fleet/exchange.key` under the edition's service), set at start before anything can load it (`lib.rs::run`). The same public identity comes back, so no peer re-pairs. |
| R2 | PASS | The Fleet folder holds no private key afterwards: `identity::tests::keys_from_before_move_into_the_keeper_and_the_folder_keeps_none` (the folder is empty). |
| R3 | PASS | The migration both ways: file → keeper, then keeper alone on later loads, and Reset forgetting the kept keys and keeping new ones the same way (same test); refusal leaves the file (`a_keeper_that_refuses_leaves_the_file_where_it_was`). Going back to a build before 0.4.1 uses the data copy taken before the update, which still holds the files. |

Not on macOS (0.6, 0.7): the keys stay in owner-only files until that
system's credential seam exists (`keep_in_keychain` sets no keeper).
