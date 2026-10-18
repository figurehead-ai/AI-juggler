# Provider billing / renewal API research log (AJ-E909, D-12)

> Purpose: decide, per Provider, whether a **sanctioned, read-only,
> live-verifiable** source exists for subscription renewal/expiry (and
> quota-window fields) that a local supervisor app may use. Until a
> source passes live verification, the product rule is **user-entered
> only** (D-12). Confidence labels: Verified-primary / 
> Verified-secondary / Negative-evidence / Inferred. Negative evidence
> is "not found by this search", never a vendor denial.
>
> Research date: 2026-10-13. Re-verify quarterly or on Provider
> releases (AJ-E810 staleness rule).

## Summary table (the product rule)

| Provider | Renewal/expiry source found? | Quota-window source | Product rule (now) | Confidence |
|---|---|---|---|---|
| Claude (Anthropic) | Console shows billing/expiry in **Settings > Billing** UI; Admin API covers org API-keys/members; **no public billing/invoice or consumer renewal API** found | `claude` CLI `/usage`-style local output (Measured); Compliance API is org-scoped usage, not renewal | **User-entered only** | Negative-evidence (searches below) |
| Codex (OpenAI) | Usage Dashboard shows billing-period activity; **no public subscription-renewal API** for ChatGPT-plan billing found; known trap: Codex can bill via API credits instead of the plan (community-verified) | Local usage signals (Estimated per SPEC §6) | **User-entered only** | Negative-evidence (searches below) |
| ZCode | No public billing API found in research corpus (`docs/research/zcode.md`) | Computed credit pools (research) | **User-entered only** | Negative-evidence |
| Gemini (future) | Not researched in depth yet (no adapter shipped) | — | **User-entered only on arrival**; research slot in E6 | Inferred |

## Search record

### Claude / Anthropic — 2026-10-13

- Query: "Anthropic Claude API billing subscription renewal date API
  endpoint organization invoices".
- Sources checked:
  - API credits for Max/Team plans — billing/expiry visible in Console
    Settings > Billing (UI, not an API):
    https://platform.claude.com/docs/en/about-claude/api-credits-for-subscribers
    (Verified-secondary)
  - Organization/Admin API (org API keys/members; requires an Admin API
    key; not consumer subscription renewal):
    https://platform.claude.com/docs/en/api/beta/organization
    (Verified-secondary)
  - Compliance API (org-scoped usage/content access, Aug 2025): Anthropic
    news, Claude Code on Team/Enterprise (Verified-secondary)
  - Community results consistently indicate renewal/tier live in
    claude.ai Settings > Subscription (UI only) (Verified-secondary).
- Verdict: **no sanctioned public endpoint for consumer subscription
  renewal dates found**. The Console Billing UI is human-only; scraping
  it is not sanctioned. → User-entered only.

### Codex / OpenAI — 2026-10-13

- Query: "OpenAI ChatGPT Codex subscription billing renewal date API
  dashboard usage endpoint developer".
- Sources checked:
  - Usage & costs guide (Usage Dashboard, billing periods):
    https://help.openai.com/en/articles/10478918-reviewing-api-usage-and-costs
    (Verified-secondary)
  - Community: Codex CLI billed via API despite ChatGPT login
    (https://community.openai.com/t/1358277) and subscription-limit
    fallback billing (https://community.openai.com/t/1389087) — both
    Verified-secondary; directly relevant to our wrong-account-billing
    identity assertion (AJ-E1001).
  - No public API for ChatGPT-plan renewal dates surfaced.
- Verdict: **no sanctioned public renewal endpoint found**; the
  API-vs-plan billing ambiguity is itself a reason to keep expiry
  user-entered. → User-entered only.

### ZCode — research corpus

- `docs/research/zcode.md` documents credit pools and Computed usage;
  no billing API documented there. (Negative-evidence within corpus.)

## Live verification (blocked, recorded)

No live API test was run: no candidate endpoint passed the
"sanctioned + read-only + renewal-bearing" bar to justify one, and
live tests must not use real user accounts (task rule). Blocking
action for any future candidate: define a sacrificial-account
verification step and record its result here before changing the
product rule.

## Consequences for E6

- Subscription clock ships on manual entry for all Providers (D-12
  default). Expiry UI copy: `Unknown — no renewal date entered`.
- Window clocks proceed from local transcript/quota events as specified
  (Measured/Computed/Estimated), unaffected by this log.
- The wrong-account-billing trap (Codex API-vs-plan) strengthens the
  P0 identity assertion before headless use (AJ-E1001).
