# Evidence manifest — AJ-E1525

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1525 (ToolSpec: tools as data;
ADR-0019 migration step 1). Status: **DONE-EVIDENCED**. 2026-10-21,
reviewer claude-opus.

## What exists now

- **`ToolSpec`** (`crates/juggler-adapter/src/spec.rs`): the id, display
  name, vendor, mark and vendor mark, install page and command, sign-in
  words, "the same as" words, the desktop app's name, kinds, rules files,
  first run's scan words, setup groups and default label, flow flags and
  terms notes. It's domain data with no serde and no TS.
  `HarnessAdapter::spec()` defaults to `ToolSpec::generic(id)`: the id as
  the name, the letter mark, one group, nothing claimed. Claude Code and
  Codex supply their own (`claude_code_spec()`, `codex_spec()`). The fake
  adapters name themselves "Fake harness (test)".
- **`tools_catalog`** (`crates/juggler-app/src/features/tools/`, the first
  feature module): one async command serving `ToolSpecInfo` rows, the
  primary tool first. Each row composes the spec with the adapter's
  capabilities (in place, limits) and isolation keys, so neither is
  written twice. `provider_display()` in `commands.rs` reads the specs;
  its per-tool table is gone.
- **The UI reads the catalog** (`ui/src/lib/tools.ts`, `lib/ipc/tools.ts`).
  It is loaded once before the first render, and the preview and the
  tests use `src/preview/tools.ts`. `providers.ts` (names, sign-in words,
  "the same as") reads specs. App's name table and its fallbacks are
  gone, as are PackSecretsCard's fallback and Home's sign-in branch.
  First run's groups, default labels, "Open in the Claude app", install
  commands and its tool list come from the specs, with no Codex special
  cases. The UI screens hold no tool-id literal.
- **The lint** (`scripts/tool_literal_lint.py`, in `just verify` and CI):
  it counts tool-id literals per file in shared code against
  `scripts/tool_literals.allow` (150 sites in 15 files today). The list
  only shrinks: a new literal fails, and so does a site that went
  without its line being lowered. Adapters, tests, generated code,
  fixtures and preview data are exempt.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 a spec from each adapter | cargo `tools_catalog::the_catalog_serves_every_tool_as_data_the_primary_first`: every tool has a name other than its id, a vendor, an https install page, groups, isolation keys, rules files and sign-in words, with one primary | PASS |
| Req.2 one `tools_catalog` command | registered in `lib.rs`; async via `ipc::blocking`; `ToolSpecInfo` exported to TS | PASS |
| Req.3 names, links, sign-in words, groups and the first-run list moved | the lint shows 0 literals in `App.tsx`, `FirstRun.tsx`, `providers.ts`, `Home.tsx`, `PackSecretsCard.tsx`; vitest `first-run` (groups from specs, with titles naming Claude only for Claude Code; Codex under its own heading) | PASS |
| Req.4 the lint with a shrinking allowlist | `tool_literal_lint.py --selftest` (shared sites flagged; adapters, tests and inline tests exempt; exactly allowed passes; a stale higher count fails; an unlisted file fails); in `just verify` and CI | PASS |
| Req.5 first run and Add account from a catalog with an unknown tool | vitest `tools-catalog`: "Acme Coder" (no screen knows it) gets its own group with its words and version line in first run, is offered by name in Add account, and its sign-in and "same as" words come from its spec; an id the catalog lacks renders as itself with the letter mark; cargo `a_tool_only_its_id_describes_gets_the_neutral_spec` | PASS |

## Gates

GATES_PLACEHOLDER

## Not covered here

- Still allowlisted, for the next steps of ADR-0019: the handover pair
  data (`fidelity.rs` 49, `validation.rs` 25), the usage pipeline's
  per-tool sources (7), usage stats (6) and the bridge's preview mocks
  (44, which move to `src/preview/` with ENGINEERING-GUIDE U3).
- Marks: the `mark` ids are in the specs; AJ-E1526 draws them.
