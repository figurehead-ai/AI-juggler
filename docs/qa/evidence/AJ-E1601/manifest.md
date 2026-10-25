# Evidence manifest — AJ-E1601

Task: `docs/tasks/E16-more-tools.md` AJ-E1601 (accounts on a compatible
endpoint: OpenCode Go and Kimi on Claude Code and Codex), as D-34 ruled.
Status: **DONE-EVIDENCED** on fixtures, an in-memory key store and stand-in
binaries; the live run on spare accounts is the maintainer's step
(`docs/internal/MAINTAINER-STEPS.md` step 16, "Through another service").
2026-10-25, reviewer claude-opus.

## How it works

- **A service is data** (ADR-0019): `crates/juggler-adapters/src/services.json`
  holds each service's published addresses (one per protocol), the
  variable each tool reads its key from, the models its guides name, its
  plans, how honest its usage can be, and the guides' addresses. A third
  service is a new entry and no code. The contract types are
  `juggler_adapter::endpoint` (`ServicePreset`, `ServiceEndpoint`,
  `WireApi`, `UsageHonesty`).
- **How a tool is pointed at an address is its adapter's** business
  (`HarnessAdapter::endpoint_wire` / `configure_endpoint`):
  - Claude Code (`claude_endpoint.rs`): `ANTHROPIC_BASE_URL` and every
    model variable in the profile's own `settings.json` `env`, and
    `hasCompletedOnboarding` in its own `.claude.json`;
  - Codex (`codex_endpoint.rs`): `model`, `model_provider` and a
    `[model_providers.<id>]` block (`base_url`, `env_key`, `wire_api`) in
    its own `CODEX_HOME/config.toml`.
  Both refuse a folder without AI Juggler's marker, and never replace a
  setting that's there.
- **The account** (`features::service_accounts`): its key is kept in AI
  Juggler's own Keychain item (`KeychainStore::for_item`, service
  `<edition id>.service-keys`, one item per account), then each profile is
  made in its own folder, configured, given launch settings with the
  model, and linked (`service_profile:<id>` in AI Juggler's settings:
  account, service, model; no key). A failed step takes back everything.
  The Accounts page groups the linked profiles into one account
  (`SubscriptionInfo.service`), its sign-in state saying whether the key
  is in the Keychain (asked of the item's attributes only).
- **The key reaches only that account's sessions**: every session start
  goes through `features::sessions::spawn::spawn_account_session`, which
  reads the key into memory for that one launch and puts it in the
  variable the service's guide names. The launch spec the adapter builds,
  and the spawn ledger's command line, never hold it.
- **Removed with the account**: removing its last profile deletes the
  Keychain item.
- **Switching**: a profile through another service is its own side of a
  pair (`<tool>+service`); its six cells are in the matrix and the
  register, never CI-green, so never "Proven" and never the one-click door.

## Requirements

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 an account kind "Through another service" for Claude Code and Codex, presets for OpenCode Go and Kimi as their guides publish; a third service is data | `cargo test -p juggler-adapters --lib -- services claude_endpoint codex_endpoint` (9); `cargo test -p juggler-adapter --lib endpoint` (3); `service_accounts::each_service_offers_the_tools_that_speak_its_protocol` | pass | pass | PASS | branch head | 2026-10-25 | claude-opus |
| Req.1 only AI Juggler's own profile folders are written, never `~/.claude` or `~/.codex` | `claude_endpoint::tests::a_folder_ai_juggler_did_not_make_is_never_written`; `codex_endpoint::tests::a_folder_ai_juggler_did_not_make_is_never_written`; `trust::a_provider_address_is_only_ever_one_a_service_publishes` | pass | pass | PASS | branch head | 2026-10-25 | claude-opus |
| Req.2 the key is pasted once and kept in the Keychain; never under the data folder or a profile's folder, the database or a log | `cargo test -p juggler-app --test service_accounts` (`an_account_through_another_service_keeps_its_key_only_in_the_keychain`) | key in 0 files | 0 files | PASS | branch head | 2026-10-25 | claude-opus |
| Req.2 handed only to that account's sessions, at launch, in the variable the tool expects; never in argv (the LaunchSpec) — D-37's test that AI Juggler's own key reaches only the launch it starts | `service_accounts::the_key_reaches_only_that_accounts_launch_and_never_its_arguments` (also a real start through `launch_profile`, then the data folder searched) | pass | pass | PASS | branch head | 2026-10-25 | claude-opus |
| Req.2 every session start goes through the one spawn path that adds it (a precise grep) | `service_accounts::every_session_starts_through_the_one_spawn_path` | 1 path + the sign-in terminal | 1 + 1 | PASS | branch head | 2026-10-25 | claude-opus |
| Req.2 never in a backup | `cargo test -p juggler-app --test backup_restore` (`a_service_accounts_key_never_reaches_a_backup`) | key in 0 entries | 0 | PASS | branch head | 2026-10-25 | claude-opus |
| Req.2 never shown again; refused input never said back | vitest `service-accounts.test.tsx` (masked field, the request recorded without the key); `service_accounts::a_service_key_or_tool_that_wont_do_is_refused_and_nothing_is_made` | pass | pass | PASS | branch head | 2026-10-25 | claude-opus |
| Req.2 removed with the account (its last profile) | `service_accounts::removing_the_last_profile_removes_the_key` | pass | pass | PASS | branch head | 2026-10-25 | claude-opus |
| Req.2 a Mac without the key starts nothing, says so, and takes it again | `service_accounts::without_its_key_here_nothing_starts_until_it_is_pasted_again`; vitest "without its key here, asks for the key, never a sign-in" | pass | pass | PASS | branch head | 2026-10-25 | claude-opus |
| Req.2 the Keychain items are AI Juggler's own, per edition | `juggler-app` lib `edition::tests::editions_share_nothing` | pass | pass | PASS | branch head | 2026-10-25 | claude-opus |
| Req.3 usage: OpenCode Go an estimate (dollar shares per 5 hours and per week), Kimi unknown; no Claude runway or "measure" offer on such an account | `service_accounts::the_accounts_page_shows_one_account_with_both_profiles`; `subscriptions::tests::profiles_on_one_service_key_are_one_account_whatever_their_tools`; vitest "the account card" (3) | pass | pass | PASS | branch head | 2026-10-25 | claude-opus |
| Privacy page: the no-proxy line reworded as D-34 says, the Keychain key named, and the trust test allowing exactly the presets' published hosts (from the data, each found word for word in the research) | `cargo test -p juggler-app --test trust` (14, incl. `ai_juggler_never_points_a_tool_at_another_address`, `a_provider_address_is_only_ever_one_a_service_publishes`, `every_trust_claim_names_tests_that_exist`) | pass | pass | PASS | branch head | 2026-10-25 | claude-opus |
| Switching matrix: "Claude Code → Claude Code on another service" (and Codex's, both ways and service to service) is never a proven one-click switch | `juggler-handover` lib `validation::tests::the_same_tool_through_another_service_is_never_a_proven_switch`; `cargo test -p juggler-handover --test validation_matrix`; `juggler-app --test validation_gate`; vitest "the switching matrix" | never Proven, capped ≤ F1 | as required | PASS | branch head | 2026-10-25 | claude-opus |
| UI: Add account › More › Through another service; the sheet (plan, masked key, tools, model, name); the account card with its profiles beneath, each with its tool's mark; lazy-loaded | vitest `service-accounts.test.tsx` (6); `ui/tests/visual/service-accounts-gallery.spec.ts` (8 states, both themes, axe 0 critical/serious; run locally, captures checked by eye) | pass | pass | PASS | branch head | 2026-10-25 | claude-opus |
| Conformance kit and the existing launch, switch and restart paths unchanged | `cargo test -p juggler-app --test profiles_launch --test manage_accounts --test switch_back_path --test codex_switch --test restart_crashed --test conversations --test switch_graph_targets --test juggling` | pass | pass | PASS | branch head | 2026-10-25 | claude-opus |
| Live: each service answers each tool, no prompt or sign-in appears, the key is in no file the tools write, and the switch says beta | `docs/internal/MAINTAINER-STEPS.md` step 16, "Through another service" | — | — | UNPROVEN | branch head | 2026-10-25 | claude-opus |

Captures: `docs/qa/evidence/_gallery/94-service-accounts/` (add-account-more,
service-sheet, accounts-service, accounts-key-missing; dark and light).

## Unverified until the live check (AJ-E1612)

- Claude Code with `ANTHROPIC_API_KEY` (Kimi's guide) may ask once to use
  the key, and then keeps its last 20 characters in the profile's
  `.claude.json` (`customApiKeyResponses`): that would be Claude Code's
  own write, which no test here can see. OpenCode Go uses
  `ANTHROPIC_AUTH_TOKEN`, which has no such prompt.
- Whether a fresh profile with `hasCompletedOnboarding` skips Claude
  Code's sign-in screen; Kimi's guide also sets `penguinModeOrgEnabled`
  and context-window variables whose values it doesn't publish (not set).
- Codex: no `models.json` catalog is written (Kimi's guide recommends
  one), so Codex may warn about an unknown model. OpenCode Go's Codex
  guide names no model: the user types one.
- Only Kimi's global addresses (`api.kimi.ai`) are presets; mainland China
  (`api.kimi.com`) would be one more entry.
- Usage: the OpenCode Go "estimate" shows the tokens its sessions used
  here; dollars aren't priced (Go's per-model allowances aren't
  published per model).
- The Keychain store was exercised through an in-memory store; the real
  item (created by the app, so no access prompt) is checked live.

## For the maintainer

- The six through-another-service pairs join beta-exit's open cells (the
  register now lists 102 open, from 54).
- Visual gate: the new family `94-service-accounts` has one gated state
  (`add-account-more`, both themes) and needs its baselines made in the
  pinned container.

## Redaction checklist

- [x] No credentials, tokens or keys (fixture keys only)
- [x] Paths generalised (temporary folders only)
- [x] No transcript content
