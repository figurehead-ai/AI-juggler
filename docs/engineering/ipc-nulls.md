# IPC rule I6: an absent value is `null`, and is typed so

This rule goes into the engineering guide's IPC section as I6, next to I1–I5, when the guide lands (PR #101). It is kept here until then so it applies now.

**I6 MUST: A Rust `Option<T>` on the wire is `null`, never a missing key. TypeScript types it `T | null` and checks it with `!= null` (or a typed guard), never `?:` or `!== undefined`.**

## The convention

- **Rust:** IPC structs derive `serde::Serialize` with no `skip_serializing_if`, so every `Option` field is always sent: `null` or a value. This is serde's default, and what ts-rs generates (`T | null`).
- **TypeScript:** a field for an `Option` is `field: T | null`: present, maybe null. It is never `field?: T`, which claims the key can be missing and says nothing about `null`.
- **Checks:** `value != null` (true for both `null` and `undefined`), `typeof value === "number"`, or a typed formatter. `value !== undefined` lets `null` through, so it is wrong for any wire field.
- **Mocks and fixtures** send what serde sends: every optional field present, as `null`. Build them from the typed builders in `ui/src/lib/wire.ts` (or annotate the object with the wire type), so a missing field is a type error.
- **New IPC types** derive `ts_rs::TS` (rule I1). A hand-written bridge interface must mirror its Rust struct field for field, with `T | null` for every `Option`.

## Why

The 0.0.12 report: clicking Usage blanked the whole window.
- `CostRollupsInfo.total_usd`, `SubscriptionView.value_ratio` and `CostRollup.credits` arrived as `null`.
- The hand-written types said `?: number`, and `CostMatrix` checked `!== undefined`, so `null.toFixed` threw.
- The preview mock sent `subscription: {}` (missing keys, not nulls), so no test saw it.

The same lie hid more bugs in hand-written types (`WindowDisplay`, `ForecastDisplay`, `RankedRowData`, `SwitchTargetData`):
- every ranked account read as filtered;
- the switch picker said "exhausts ≈0 min";
- a ring said "resets 00:00".

Those types now mirror Rust, and the compiler found each site.

## Enforcement

- **Today:** review, and `tsc` once the types are truthful (a `T | null` field can't reach `.toFixed` unguarded).
- **AJ-E2012 (the UI linter):**
  - enable `@typescript-eslint/no-unnecessary-condition` and `@typescript-eslint/strict-boolean-expressions` for wire types;
  - add a lint that bans `!== undefined` / `=== undefined` on a property of a type imported from `ui/src/types/generated/` or `ui/src/lib/bridge.ts`.
