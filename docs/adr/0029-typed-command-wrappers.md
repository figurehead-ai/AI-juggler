# ADR-0029: Typed command wrappers, generated in-repo from the command registry

**Status:** Accepted
**Date:** 2026-11-06
**Deciders:** the 0.4 completion session (AJ-E2019, an engineering ruling)

## Context

The UI calls the backend by name: `invoke<T>("places_find", { tool, conversationId, account, app })`.
Neither side's compiler checks that call:

- A renamed command or argument breaks at run time.
- `scripts/ipc_audit.py` catches a name that no longer exists, but not an argument that changed.
- A wrong result type passes silently.

0.4.1 hit this. `places_find` gained two arguments, and the read-only test bridge's route table was only caught because the lab-bridge build happened to be compiled.

The types themselves are already shared: ts-rs writes every `…Info` struct to `ui/src/types/generated/`, and `scripts/ts_types_fresh.sh` keeps them committed. What isn't shared is the command list with each command's arguments and result. That list exists once already: the `app_commands!` registry in `crates/juggler-app/src/lib.rs`, which `build.rs` reads for the webview's permission files (AJ-E2003).

AJ-E2019 offered two ways:

- **tauri-specta**, if 2.0 is stable. It isn't: on 2026-11-06 the latest is `v2.0.0-rc.25` (2026-05-08), still a release candidate after two years of them. It would also replace ts-rs for the types it exports, and change how commands are registered (`collect_commands!`), touching all of the app's commands at once.
- **An in-repo generator**, from what the repository already has.

## Decision

An in-repo generator.

1. **One source.** The `app_commands!` registry stays the list of commands. Each entry already names the function and whether it reads or writes; the generator reads the function's signature from its source. The arguments are the parameters after the Tauri-injected ones (`State`, `AppHandle`, `Window`, `Channel` are left out); the result is `AppResult<T>`'s `T`.
2. **What it writes.** It writes `ui/src/types/generated/commands.ts`, a `Commands` map from each command's name to `{ args: {…}; result: … }`. Arguments use Tauri's camelCase, and types come from ts-rs where they're structs, so a type exists once. `lib/ipc/transport.ts` gets a typed `call<K extends keyof Commands>(name: K, args: Commands[K]["args"]): Promise<Commands[K]["result"]>`. `lib/ipc/<feature>.ts` calls through it. A raw `invoke` is then refused by the UI lint's existing rule that only `lib/ipc` talks to the backend, tightened to `call`.
3. **Where it runs.** It is a Rust test in `juggler-app` (like ts-rs's `export_bindings`), so `cargo test` writes it. `scripts/ts_types_fresh.sh`, which CI already runs, fails when the committed file differs. No new tool joins the build.
4. **Parsing.** It uses `syn` (already in the dependency tree through Tauri's macros). A signature it can't map, such as an `impl Trait` argument, fails the generator by name, instead of emitting `unknown`.
5. **Revisit** when tauri-specta 2.0 is released stable. Moving then means replacing one generator; the `Commands` map's shape is close to what tauri-specta emits.

## Consequences

- A renamed command or argument, or a changed result, is a TypeScript error in the UI the moment the Rust side changes and the test runs.
- The dev bridge's route table (`dev_bridge/routes.rs`) can be checked against the same signatures (a follow-up), which closes the gap 0.4.1 hit.
- The generator is ours to keep: about the size of `build.rs`'s registry reader. It is tested by its own snapshot test.
- Implementation follows AJ-E2018's split of `lib/bridge.ts` into `lib/ipc/<feature>.ts`, so the wrappers land in their final modules (AJ-E2019 depends on AJ-E2018).

## Implementation notes (2026-11-06, AJ-E2019)

- A Tauri `Channel<T>` is an argument the page sends (`lib/ipc/transport.ts`'s `channel`), so the map lists it, typed as `@tauri-apps/api`'s `Channel<T>`. On the read-only bridge, which carries no stream, a streaming command goes through `withoutStream` and the bridge refuses it, as before.
- A command that answers nothing (`()`, sent as `null`) has the result `void`, as a TypeScript function that returns nothing does; `()` elsewhere is `null`.
- An `Option` argument is optional (`name?: T | null`): Tauri reads a missing one as `None`.
- Wire types without a ts-rs export got one (65 types), including a few of `juggler-core`'s, behind its `ts` feature, which only the app turns on.
- The dev bridge check landed with the generator (`bridge_routes_match_the_commands`).
