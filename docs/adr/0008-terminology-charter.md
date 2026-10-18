# ADR-0008: Terminology charter — Providers / Accounts / Instances

**Status:** Accepted
**Date:** 2026-10-13
**Deciders:** Vuxtra engineering + maintainer (decision-log D-02, D-10)
**Implements:** D-02, D-10

## Context

The shipped product used "harness" (technical), "Profile" (ambiguous —
external tools like cc-switch and Codex use *profile* to mean *account*),
and collapsed Instances into Sessions (ADR-0007 flagged the drift).
The maintainer's canonical workflow names the desired nouns outright:
"we should call them providers, accounts, and instances of the tool"
(docs/expected-worfklow.md:11).

## Decision

1. **Provider** = the tool family you subscribe to (Claude Code, Codex,
   ZCode, later Gemini/OpenCode). Domain: `HarnessId` with additive
   alias `ProviderId`; wire values (`claude-code` | `codex` | `zcode`)
   never change.
2. **Account** = one subscription/login under a Provider (unchanged
   domain entity).
3. **Instance** = one running occurrence of a tool under one Account on
   one project; `kind: Terminal | Desktop`. Domain: `SessionRecord`
   gains additive alias `InstanceRecord` (AJ-E104 splits the
   conversation id out as `ConversationId`).
4. **Session** = the conversation *inside* an Instance (the native
   conversation id), never a running process.
5. **"Profile" is demoted to Launch settings** (domain alias
   `LaunchRecipe`, AJ-E105): a per-Account overlay used at launch. It
   is never a rail noun and never means Account in user copy.
6. Rename is **phased, never big-bang**: UI copy first (Phase A), then
   domain aliases (Phase B), then docs; every step
   serialisation-compatible; a terminology lint lands with Phase C.

## Alternatives considered

- **Copy-only rename (UI strings only)** — rejected: the domain/copy
  split is what caused the Instance/Session collapse; both layers must
  converge or drift returns.
- **Big-bang rename in one PR** — rejected: unreviewable, breaks
  generated types and DB round-trips; D-02 mandates phases.
- **Keep "harness"/"profile"** — rejected: maintainer named the nouns;
  homonym with external tools is the highest-risk confusion.

## Amendment A-1 (AJ-E104, 2026-10-13): identifier semantics for the split

Requirement 1 required picking one convention for the conversation
identifier. Settled:

- **`SessionId` remains the *Instance* identifier** (juggler-assigned;
  unchanged type, wire, supervisor/journal keys — zero migration).
- **`ConversationId` is a new newtype for the *native* conversation
  identifier** (Claude uuid / Codex thread / ZCode sess_*). It replaces
  the untyped `native_id: Option<String>` field; the DB column
  `native_id` and its values are unchanged (the type wraps the same
  string). `conversation: Option<ConversationId>` is honest-None while
  the conversation is undiscovered (the "Starting…" UI state covers it)
  rather than fabricating an id.
- `pub type InstanceStatus = SessionStatus;` — wire strings unchanged.

## Consequences

E1 executes in three phases with schema v5 as the migration anchor.
Docs, wireframes, and marketing must pass a term lint. External tools'
vocabulary is explicitly not imported (D-10).
