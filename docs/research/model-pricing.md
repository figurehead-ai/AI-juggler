# Model prices for the cost ledger (seed v3)

The bundled price table `crates/juggler-core/src/cost_ledger_seed.json` (v3), and where each number comes from. Evidence standard: AJ-E810 (source URL, access date, evidence, confidence label).

Why: the 0.0.12 QA walk (P15) found the seed priced only `claude-opus-4` and `claude-sonnet-4`. Every model in real use was unpriced, so Usage showed no dollar figure for any row.

## Sources

| Vendor | Source (official) | Accessed | Label |
|---|---|---|---|
| Anthropic | https://platform.claude.com/docs/en/about-claude/pricing (the "Model pricing" table) | 2026-10-21 | Verified-primary |
| Anthropic (model IDs) | https://platform.claude.com/docs/en/about-claude/models/overview | 2026-10-21 | Verified-primary |
| OpenAI | https://developers.openai.com/api/docs/pricing (Standard tier; "Specialized models" for Codex) | 2026-10-21; gpt-6.1-terra 2026-10-26 | Verified-primary |

`docs.claude.com/…/pricing` redirects to the Anthropic page above. `platform.openai.com/docs/pricing` redirects to the OpenAI page above.

## What the pages say (paraphrased; prices in USD per 1M tokens)

**Anthropic.** Columns: base input, 5-minute cache writes, 1-hour cache writes, cache hits and refreshes, output.

| Model | Input | Output | Cache hit | 5m write |
|---|---|---|---|---|
| Claude Fable 5.1 | 10 | 50 | 0.25 | 12.50 |
| Claude Fable 5 | 10 | 50 | 1 | 12.50 |
| Claude Opus 5.5 | 4 | 20 | 0.20 | 5 |
| Claude Opus 5, 4.8, 4.7, 4.6, 4.5 | 5 | 25 | 0.50 | 6.25 |
| Claude Opus 4.1, Opus 4 | 15 | 75 | 1.50 | 18.75 |
| Claude Sonnet 5.5 | 2 | 10 | 0.10 | 2.50 |
| Claude Sonnet 5 | 2 | 10 | 0.20 | 2.50 |
| Claude Sonnet 4.6, 4.5, 4 | 3 | 15 | 0.30 | 3.75 |
| Claude Haiku 4.5 | 1 | 5 | 0.10 | 1.25 |

Other facts from the Anthropic pages:
- **Long context:** 4.6 and later models bill the full 1M context at the standard rate.
- **Model IDs:** IDs are dateless from the 4.6 generation on, for example `claude-opus-4-6`. Earlier models have dated IDs, for example `claude-haiku-4-5-20251001`, plus a dateless alias.
- **Dated-ID lookup:** the ledger prices a dated ID through its alias row (`PricingCatalog::row_for`).

**OpenAI.** Standard tier, short context (≤272K input tokens). Columns: input, output, cached input.

| Model | Input | Output | Cached input |
|---|---|---|---|
| gpt-6-astra | 10.00 | 50.00 | 1.00 |
| gpt-6.1-terra (added 2026-10-26) | 1.00 | 5.00 | 0.05 |
| gpt-6.1-sol | 2.00 | 10.00 | 0.10 |
| gpt-6-sol | 2.00 | 10.00 | 0.20 |
| gpt-6-luna | 0.10 | 0.50 | 0.01 |
| gpt-5.6-sol | 4.00 | 20.00 | 0.40 |
| gpt-5.6-terra | 2.00 | 12.00 | 0.20 |
| gpt-5.6-luna | 0.20 | 1.20 | 0.02 |
| gpt-5.5 | 5.00 | 30.00 | 0.50 |
| gpt-5.4 | 2.50 | 15.00 | 0.25 |
| gpt-5.4-mini | 0.75 | 4.50 | 0.075 |
| gpt-5.4-nano | 0.20 | 1.25 | 0.02 |
| gpt-5.3-codex (Specialized models, Codex) | 1.75 | 14.00 | 0.175 |
| gpt-5.2 | 1.75 | 14.00 | 0.175 |
| gpt-5.1, gpt-5 | 1.25 | 10.00 | 0.125 |
| gpt-5-mini | 0.25 | 2.00 | 0.025 |
| gpt-5-nano | 0.05 | 0.40 | 0.005 |

## What the ledger does with them, and the known gaps

Every dollar figure is an estimate at API list prices (the ledger's own label), not what a subscription costs.

The gaps below are labelled **Inferred** where they rest on our reading rather than a vendor statement.

- **Claude cache writes** are priced at the 5-minute rate.
  - Claude Code's transcripts split `cache_creation` into 5-minute and 1-hour writes, but the store keeps one cache-write class.
  - So 1-hour writes, at 2× base input, are under-priced.
- **Claude Haiku 5.5** is not in the seed. It is priced by prompt length (more above 100,000 tokens), and a per-row table can't represent that. It shows "No price for claude-haiku-5-5 yet".
- **Pricing modifiers:** Fast mode, US-only inference (1.1×) and Batch discounts are not modelled.
- **OpenAI long context:** requests over 272K input tokens cost more. The table has short-context rates only, so such requests are under-priced.
- **OpenAI cache writes:** their 30-minute and 12-hour prices are not modelled (cache-write rate 0). Codex's `cache_write_input_tokens` is stored apart from input, and whether it is also inside `input_tokens` is not stated on the protocol page we read (Inferred).
- **Codex models with no row on the page** stay unpriced. This covers older `gpt-5-codex`, `gpt-5.1-codex*`, `gpt-5.2-codex` and `codex-mini-latest`, and `codex-auto-review`, which the QA oracle saw in real use (252 rows). The page has no row by that name (checked 2026-10-21, and again 2026-10-26). Third-party price lists quote a rate for it, but no OpenAI page does, so it stays unpriced: never a guess (D-06). Usage's Prices and details names it as having no list price, and its total leaves it out in words.
- **Left out of the seed:** retired models (Claude Haiku 3.5), limited-availability models (Claude Mythos) and "pro" models.
- **Reasoning tokens:** Codex's reasoning tokens are stored apart from output (P14) and priced at the output rate.

## Follow-up: the pricing-update path (P15 d)

New models drop out silently today: a model with no row shows "No price for <model> yet" until someone edits the seed. Proposed follow-up task:

1. List the models in stored usage that have no price, and show them in Settings › Pricing with "Add a price".
2. Add a release checklist step: re-read both pages above and refresh the seed. A row older than 90 days is stale under AJ-E810.
3. Retire the inspector's separate `quality/pricing.v1.json`, which still keys `opus`/`sonnet` and never matches a real model ID, in favour of this ledger.
4. Store 5-minute and 1-hour cache writes apart, and price each at its own rate.
