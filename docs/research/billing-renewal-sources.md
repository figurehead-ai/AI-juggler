# Billing & renewal-date sources — the D-12 research note (AJ-E309 Req.4)

> D-12 law: renewal dates are **user-entered, optional, never
> inferred**. No code may auto-populate a renewal date until a source
> is sanctioned, read-only, and verified live (with a recorded
> maintainer confirmation and a D-12 amendment). "No source found" is
> an honest, acceptable entry — and it is the entry for every Provider
> today.
>
> This note is the per-Provider index the first-run Confirm row
> relies on. The full evidence packets live in
> `docs/research/evidence/billing/` (AJ-E602–E605, graded under the
> protocol in `docs/research/billing-source-protocol.md`); the
> Stage-1 log is `docs/research/billing-apis.md` (AJ-E909) and the
> seam rationale is `docs/research/subscription-expiry-sources.md`
> (AJ-E411). Confidence labels follow AJ-E810; a negative result is
> "not found by these searches", never a claim that the vendor denies
> one exists.

## Per-Provider verdict

| Provider | Sanctioned renewal-date source? | Evidence (URL · access date) | Confidence | Verified live? |
|---|---|---|---|---|
| Claude Code (Anthropic) | **No source found.** Pro/Max are billed through claude.ai; the Console/Admin API covers org API keys, usage and spend — not consumer subscription renewal. The CLI (`claude --help`, 2.1.300) and the statusline payload carry no renewal field. Monthly API-credit grants for Max/Team expire per billing cycle, but no documented endpoint exposes that cycle's date. | [Support: subscription vs API billing](https://support.claude.com/en/articles/9876003-i-have-a-paid-claude-subscription-pro-max-team-or-enterprise-plans-why-is-claude-api-usage-billed-separately-from-my-paid-claude-plan) · 2026-10-15; [Admin API — organization](https://platform.claude.com/docs/en/api/beta/organization) · 2026-10-15; [API credits for subscribers](https://platform.claude.com/docs/en/about-claude/api-credits-for-subscribers) · 2026-10-13; re-searched 2026-10-18 (no official endpoint surfaced; secondary coverage of the 2026-10-07 credit grants only) | Negative-evidence | n/a — nothing to verify |
| Codex (OpenAI) | **No source found.** ChatGPT plans and API billing are separate systems; the Costs/Usage Admin API reports API-key spend only. `codex --help` / `login status` (0.163.0) expose no plan dates; Codex's `/status` shows usage-window resets, which are NOT renewal dates. The `backend-api` usage endpoint some tools read is undocumented → prohibited (protocol §1/§3). | [Costs API reference](https://developers.openai.com/api/reference/resources/admin/subresources/organization/subresources/usage/methods/costs) · 2026-10-15; [Usage & costs guide](https://help.openai.com/en/articles/10478918-reviewing-api-usage-and-costs) · 2026-10-13; [About ChatGPT Pro tiers](https://help.openai.com/en/articles/9793128-about-chatgpt-pro-tiers) · 2026-10-18; [CodexBar #1119 — no confirmed renewal field](https://github.com/steipete/CodexBar/issues/1119) · 2026-10-18 (Verified-secondary) | Negative-evidence | n/a |
| ZCode (Z.ai GLM Coding Plan) | **No source found.** Renewal is automatic per billing cycle and visible only on the subscription/billing web pages; the docs offer no API, export, or date field. Local `~/.zcode` state has no purchase/renewal column (credential store never opened). | [DevPack overview](https://docs.z.ai/devpack/overview) · 2026-10-15, re-checked 2026-10-18; [DevPack usage policy](https://docs.z.ai/devpack/usage-policy.md) · 2026-10-18 (auto-renewal + cancel-3-days rule, dashboard-only) | Negative-evidence | n/a |
| Gemini CLI (Google) | **No source found.** No consumer-subscription API; Gemini API billing is prepaid credits; Google One / Play subscription dates are human-read web pages (the Play Developer API only covers your OWN app's subscriptions). Successor Antigravity CLI documents a quota readout (`/usage`), not a renewal date. | [Gemini API billing](https://ai.google.dev/gemini-api/docs/billing) · 2026-10-16; [Gemini CLI quota & pricing](https://geminicli.com/docs/resources/quota-and-pricing) · 2026-10-16; [Antigravity CLI usage](https://antigravity.google/docs/cli/commands/usage) · 2026-10-16 | Negative-evidence | n/a |

Rejected candidate classes (for every Provider): scraping a vendor
dashboard (ToS-hostile, unverifiable), reading receipt e-mail or
calendars (outside the privacy invariant), reading credential stores
(prohibited outright), and inferring from plan tier, first-seen date,
or usage-window resets (D-12 forbids inference). Window resets are a
quota clock (AJ-E606), never a renewal clock.

## What the product does until a row flips

- The first-run Confirm row (AJ-E309) and the Accounts editor are the
  ONLY writers of `renewal_date`, each an explicit user action;
  empty persists as unknown and renders **"expiry unknown"**.
- `crates/juggler-core/src/renewal.rs` keeps `UnknownRenewalSource`
  as the only `RenewalSource` implementation — no auto-population
  code exists (pinned by AJ-E303's inference fields, which carry no
  renewal or plan inference).

## Revisit trigger

A Provider publishing a documented, read-only renewal-date endpoint
opens a NEW task: adapter work + fixtures + a sacrificial-account live
verification recorded here with the maintainer's dated confirmation
+ a D-12 amendment. Re-run these searches quarterly or on a Provider
release (AJ-E810 staleness rule); next due 2027-01-18.
