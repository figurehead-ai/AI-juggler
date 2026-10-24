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
(docs/internal/expected-workflow.md:11).

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

## Amendment A-2 (D-35, 2026-10-24): profiles are not accounts

Amends Decision 2 and Decision 5. The maintainer, 2026-10-24: "We are
clumping accounts with profiles, which is not the same."

- **Profile** = the setup an app runs on: a Claude Code folder
  (`CLAUDE_CONFIG_DIR`), a Claude app profile (its data folder), a
  `CODEX_HOME`. Sessions run in a profile.
- **Account** = the subscription a profile is signed in to. Limits and
  money live on the account.
- One account can have several profiles. The same login in two profiles
  is normal, never a warning (this retires E3's "same login twice"
  warning).
- **Launch settings sit under a profile.** Decision 5's "demoted to
  Launch settings" no longer holds: "profile" is a user-facing noun again,
  with the meaning above, and never means account. `LaunchRecipe` keeps
  its name.
- **Where each word shows:** profiles where sessions run (rail rows,
  places, badges, New session); accounts where limits and money live
  (runway, the Accounts page, Usage). A place is an app + profile pair
  (D-36).
- **The stored entity stays one per folder in 0.1.x** (today's
  `Account`, keyed by `state_dir`, is a profile). Accounts are worked out
  when read, from each profile's signed-in identity (account and
  organisation), so a personal plan and a team seat on one email stay two
  accounts. A separate stored account entity waits for 0.2.
- The terminology lint learns the new meaning: "profile" is allowed where
  it names the setup an app runs on (D-35 citations, "profile badge",
  "app + profile"), and still flagged where it stands in for account.

## Consequences

E1 executes in three phases with schema v5 as the migration anchor.
Docs, wireframes, and marketing must pass a term lint. External tools'
vocabulary is explicitly not imported (D-10).
