# ADR-0019: Extension architecture — tools as data, features as modules

- Status: Accepted (maintainer direction, 2026-10-21)
- Date: 2026-10-21
- Deciders: maintainer + coordinator
- Relates to: ADR-0002 (adapter trait), ADR-0018 (setups where they are),
  E15–E19

## Context

ADR-0002 promised that "a new harness = one crate/module implementing the
trait; core, supervisor and UI unchanged". By 0.0.11 the code has drifted
from that:

- About 114 tool-id string checks (`"claude-code"`, `"codex"`) sit in
  backend code outside the adapters, in `juggler-app`, `juggler-core` and
  `juggler-handover`. About 59 sit in the UI.
- Display names, install links, sign-in words, tool groups and the list of
  first-run tools are hard-coded in `commands.rs` and `FirstRun.tsx`.
- `commands.rs` has grown to 14,386 lines, holding every feature's
  commands in one file.

E16 adds eight or more tools, and E17–E19 add pools, messaging and tasks.
The maintainer (2026-10-21): "everything needs to be well architected for
scalability and expansions, meaning no hardcoded code duplications, no
quick hacks."

The repo already shows the right pattern in one place: Claude Code and
Codex share one layout-driven environment scanner (`Layout`,
`installs_from`), each supplying only its data.

## Decision

### 1. Tools are data, described by their adapter

Each adapter supplies a **`ToolSpec`**, static and versioned, holding
everything any layer needs to know about the tool:

- id, display name, vendor, and its mark (an icon asset id);
- the install link, and the words for the tool's own sign-in;
- kinds: CLI, desktop, endpoint account type;
- home isolation: an env var, XDG folders or HOME, with the keys pinned
  after overrides;
- the layout: config evidence, history and private files (the existing
  `Layout`, made public through the spec);
- rules files (`CLAUDE.md`, `AGENTS.md`, …) and the MCP config format,
  through a `ConfigFormat` handler that the adapter implements;
- quota sources and their honesty labels;
- capability flags: in place, handover pairs, desktop, the agent bridge;
- terms notes, such as "interactive use only".

The backend serves the specs to the UI through one `tools_catalog`
command. **The UI and the commands never branch on a tool id.** They read
the spec. Pair data, such as the fidelity matrices and the validation
register, is keyed by spec capabilities, or kept in per-tool data files
owned by the adapter crate. It is never a literal in shared code.

### 2. Adapters are composed from shared building blocks

The common mechanics are library pieces, configured by the spec, never
copied:

- home isolation strategies;
- the layout-driven scanner, importer and adopter;
- a version probe and binary locator that run under the user's resolved
  PATH;
- login-shape probes;
- the transcript-reader trait, with JSONL and SQLite helpers;
- the quota-source trait.

A new tool is its spec, plus the small trait impls for what is truly its
own, plus fixtures. The **conformance kit** (AJ-E1521) is the contract,
and every adapter runs it.

### 3. Features are modules

Each feature area (sync, notifications, pools, messaging, tasks, the agent
bridge's tools, the menu bar, machine health) is a module with:

- a **service trait** as its public API, with its own state behind it;
- a **repository** for storage, with its migrations named for the module,
  in the one schema sequence;
- **commands** in the module's own file, registered through a module
  registry, not appended to `commands.rs`;
- **events** on one typed event bus, never ad-hoc emits;
- **UI contributions** declared by the module: screens, inspector tabs,
  settings sections, Home cards and rail marks, rendered by the shell from
  a registry.

Modules depend on core types and on other modules' traits, never on their
internals. The agent bridge (ADR-0020, AJ-E1701) exposes a tool registry that
modules contribute MCP tools to.

### 4. Plugins later, on the same seams

The declarative tool manifest (SPEC §4, post-v1) is a `ToolSpec` loaded
from TOML, plus the generic building blocks. Out-of-process plugins come
after 1.0, if ever, through the same traits.

### 5. Enforcement

- **A tool-literal lint** in `just verify` and CI fails on a tool id string
  outside `juggler-adapters`, adapter data files and tests. It starts with
  an allowlist of today's sites, which only shrinks.
- **The conformance kit** runs for every registered adapter.
- **Layering rules** (QUALITY.md §4) gain the module rule: no import of
  another module's internals.
- **Review:** a per-tool branch, duplicated logic or a shortcut is a review
  finding even when the tests pass.

## Migration

1. Before E16's first adapter, in release 0.1:
   - `ToolSpec` and `tools_catalog`;
   - move display names, install links, sign-in words, groups and the
     first-run tool list into the specs;
   - make `FirstRun.tsx`, `providers.ts` and the commands branch-free;
   - the lint with its allowlist.
2. With rules and MCP sync (AJ-E1507): `ConfigFormat` per adapter.
3. Between 0.1 and E16: split `commands.rs` into per-module command files
   behind the registry, mechanically, with no behaviour change, in one
   release branch while no other feature work is in flight.
4. E17–E19 are built as modules from day one.
5. The allowlist reaches zero by 0.2.

## Alternatives considered

- **Keep per-tool branches and add each new tool's cases.** That's quickest
  for tool three, and costs N× for every later tool and feature.
- **A plugin runtime now (WASM or out-of-process).** It is premature: the
  seams aren't stable yet, and it would add a big security surface.
- **One mega-trait for everything.** It blurs ownership. A spec plus
  focused traits keeps each piece testable.

## Consequences

- Adding a tool touches only its adapter, its spec and its fixtures. The UI
  needs no change, and that is checked by the lint and by a test that
  renders first run from a catalog with an unknown fake tool.
- Feature work lands in module folders. `commands.rs` shrinks.
- Some churn now (migration steps 1 and 3) prevents rework across E16–E19.
