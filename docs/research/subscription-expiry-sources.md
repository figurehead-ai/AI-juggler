# Subscription-expiry sources — the D-12 gate (AJ-E411 research stub)

Owner: AJ-E411 (zcode-primary, 2026-10-14). Status: **no sanctioned
source exists today** — the renewal date stays user-entered only.

## The gate (decision-log D-12, law)

1. Renewal dates are **user-entered only**. Inference or guessing from
   plan tier, usage patterns, or billing cadence is PROHIBITED.
2. Any future Provider billing/renewal source must be **sanctioned**
   (named in this document with a decision-log amendment), **read-only**,
   and **verified live** against a real account before it may populate
   the field.
3. Unproven sources keep rendering `Expiry: unknown` — never a guess.
4. Scraping Provider dashboards is explicitly out (fragile, ToS-hostile,
   and unverifiable).
5. A sanctioned source is a **NEW task** (adapter work + verification
   fixtures + D-12 amendment) — it is never a silent change to the
   `RenewalSource` seam.

## The seam today

`crates/juggler-core/src/renewal.rs` ships the seam:

- `trait RenewalSource { fn lookup(provider, account_label) -> Result<Option<String>, String> }`
- the only implementation: `UnknownRenewalSource` — always `Ok(None)`.

## Candidate audit (why nothing is sanctioned)

| Candidate | Verdict | Reason |
|---|---|---|
| Claude Code / Anthropic billing API | none public | No documented billing/renewal endpoint; the CLI's OAuth scope covers session use, not billing. |
| Codex / OpenAI billing API | unsanctioned | A billing API exists for org dashboards, but renewal *dates* for consumer subscriptions are not exposed; org-scoped keys are out of scope (credentials never leave the account owner's login). |
| ZCode / Z.ai | none public | No documented billing API. |
| Receipt emails / calendar scraping | PROHIBITED | Mail access is out of scope (privacy invariant); scraping is unverifiable. |
| Plan-tier heuristics ("Max renews monthly") | PROHIBITED | D-12 forbids inference from plan tier. |

## Revisit trigger

A provider publishing a documented, read-only billing/renewal endpoint
with a verification path against a real account reopens this note as a
new task (adapter + fixtures + live verification + D-12 amendment).
