# Model prices for the cost ledger (seed v4)

The bundled price table `crates/juggler-core/src/cost_ledger_seed.json` (v4, 0.3.7), and where each number comes from. Evidence standard: AJ-E810 (source URL, access date, evidence, confidence label).

Why: the 0.0.12 QA walk (P15) found the seed priced only `claude-opus-4` and `claude-sonnet-4`. Every model in real use was unpriced, so Usage showed no dollar figure for any row.

## Sources

| Vendor | Source (official) | Accessed | Label |
|---|---|---|---|
| Anthropic | https://platform.claude.com/docs/en/about-claude/pricing ("Model pricing", "Prompt caching", "Long context pricing", "Fast mode pricing", "Data residency pricing") | 2026-11-03 (v4; first read 2026-10-21) | Verified-primary |
| Anthropic (model IDs) | https://platform.claude.com/docs/en/about-claude/models/overview | 2026-10-21 | Verified-primary |
| OpenAI | https://developers.openai.com/api/docs/pricing (Standard tier, short and long context; "Specialized models" for Codex) | 2026-11-03 (v4; first read 2026-10-21, gpt-6.1-terra 2026-10-26) | Verified-primary |

`docs.claude.com/…/pricing` redirects to the Anthropic page above. `platform.openai.com/docs/pricing` redirects to the OpenAI page above.

## What the pages say (paraphrased; prices in USD per 1M tokens)

**Anthropic.** Columns: base input, 5-minute cache writes, 1-hour cache writes, cache hits and refreshes, output. Every row re-read 2026-11-03.

| Model | Input | Output | Cache hit | 5m write | 1h write |
|---|---|---|---|---|---|
| Claude Fable 5.1 | 10 | 50 | 0.25 | 12.50 | 20 |
| Claude Fable 5 | 10 | 50 | 1 | 12.50 | 20 |
| Claude Opus 5.5 | 4 | 20 | 0.20 | 5 | 8 |
| Claude Opus 5, 4.8, 4.7, 4.6, 4.5 | 5 | 25 | 0.50 | 6.25 | 10 |
| Claude Opus 4.1, Opus 4 | 15 | 75 | 1.50 | 18.75 | 30 |
| Claude Sonnet 5.5 | 2 | 10 | 0.10 | 2.50 | 4 |
| Claude Sonnet 5 | 2 | 10 | 0.20 | 2.50 | 4 |
| Claude Sonnet 4.6, 4.5, 4 | 3 | 15 | 0.30 | 3.75 | 6 |
| Claude Haiku 5.5, prompts up to 100,000 tokens | 0.10 | 0.50 | 0.01 | 0.125 | 0.20 |
| Claude Haiku 5.5, prompts over 100,000 tokens | 0.50 | 2.50 | 0.05 | 0.625 | 1 |
| Claude Haiku 4.5 | 1 | 5 | 0.10 | 1.25 | 2 |

Other facts from the Anthropic pages:
- **Cache writes:** a 5-minute write costs 1.25× base input and a 1-hour write 2×, on every model ("Prompt caching").
- **Long context:** 4.6 and later models, except Claude Haiku 5.5, bill the full 1M context at the standard rate. Claude Haiku 5.5 is priced by prompt length: a request whose prompt (all its input tokens, cache reads and writes included) is over 100,000 tokens pays the higher row, for the whole request.
- **Fast mode** (Opus 5.5, Opus 5, Opus 4.8): Opus 5.5 $8 input / $40 output; Opus 5 and 4.8 $10 / $50, across the whole context window. The caching multipliers apply on top (so Opus 5.5 fast: 5m write $10, 1h write $16, cache hit $0.40; Opus 5 and 4.8 fast: $12.50, $20, $1).
- **US-only inference** (`inference_geo: "us"`, Claude 4.6 and later, Haiku 5.5's higher prices included): 1.1× every token price.
- **Model IDs:** IDs are dateless from the 4.6 generation on, for example `claude-opus-4-6`. Earlier models have dated IDs, for example `claude-haiku-4-5-20251001`, plus a dateless alias.
- **Dated-ID lookup:** the ledger prices a dated ID through its alias row (`PricingCatalog::row_for`).

**OpenAI.** Standard tier. "Short context: ≤272K input tokens. Long context: >272K input tokens." The long-context columns are listed for the GPT-6, GPT-5.6, GPT-5.5 and GPT-5.4 rows only. Columns: input, output, cached input; then the long-context three. Every row re-read 2026-11-03.

| Model | Input | Output | Cached input | Long input | Long output | Long cached |
|---|---|---|---|---|---|---|
| gpt-6-astra | 10.00 | 50.00 | 1.00 | 20.00 | 75.00 | 2.00 |
| gpt-6.1-terra (added 2026-10-26) | 1.00 | 5.00 | 0.05 | 2.00 | 7.50 | 0.10 |
| gpt-6.1-sol | 2.00 | 10.00 | 0.10 | 4.00 | 15.00 | 0.20 |
| gpt-6-sol | 2.00 | 10.00 | 0.20 | 4.00 | 15.00 | 0.40 |
| gpt-6-luna | 0.10 | 0.50 | 0.01 | 0.20 | 0.75 | 0.02 |
| gpt-5.6-sol | 4.00 | 20.00 | 0.40 | 8.00 | 30.00 | 0.80 |
| gpt-5.6-terra | 2.00 | 12.00 | 0.20 | 4.00 | 18.00 | 0.40 |
| gpt-5.6-luna | 0.20 | 1.20 | 0.02 | 0.40 | 1.80 | 0.04 |
| gpt-5.5 | 5.00 | 30.00 | 0.50 | 10.00 | 45.00 | 1.00 |
| gpt-5.4 | 2.50 | 15.00 | 0.25 | 5.00 | 22.50 | 0.50 |
| gpt-5.4-mini | 0.75 | 4.50 | 0.075 |
| gpt-5.4-nano | 0.20 | 1.25 | 0.02 |
| gpt-5.3-codex (Specialized models, Codex) | 1.75 | 14.00 | 0.175 |
| gpt-5.2 | 1.75 | 14.00 | 0.175 |
| gpt-5.1, gpt-5 | 1.25 | 10.00 | 0.125 |
| gpt-5-mini | 0.25 | 2.00 | 0.025 |
| gpt-5-nano | 0.05 | 0.40 | 0.005 |

## What the ledger does with them, and the known gaps

Every dollar figure is an estimate at API list prices (the ledger's own label), not what a subscription costs.

**Priced per call (0.3.7, the usage audit).** A vendor's price depends on each call: its prompt's size (long-context tiers), its 1-hour cache writes, its speed and where it ran. A sum of tokens keeps none of that, so the store prices each call when it is read (`PricingCatalog::price_event`) and keeps the cost beside it in `usage_detail`, with the prices' version (`v4`, plus a fingerprint of the user's overrides). When the bundled table or an override changes, every stored call is priced again in batches (`UsageRepo::reprice`); the token rows never change. Summaries add the stored costs; a call with none (an unpriced model, a call whose file was gone before 0.3.7) is priced from its sum at the standard rate.

- **The tier a call pays:** fast mode's rates when the call ran fast and the model has them; else the long-context rates when its prompt is over the model's threshold (OpenAI: input including cached input over 272,000; Claude Haiku 5.5: input, cache reads and cache writes over 100,000); else the standard rates. Then 1.1× for US-only inference where the model offers it.
- **What a call's prompt is:** Claude: `input_tokens + cache_read_input_tokens + cache_creation_input_tokens`. Codex: its `input_tokens`, which include the cached part.
- **What is read for it:** Claude's `cache_creation.ephemeral_1h_input_tokens`, `speed` and `inference_geo` (the defaults `standard` and `global` aren't stored); Codex's thread kind and conversation from its `session_meta` (that's for counting sessions, not price).
- **Reasoning tokens:** Codex's reasoning tokens are stored apart from output (P14) and priced at the output rate.

The gaps below are labelled **Inferred** where they rest on our reading rather than a vendor statement.

- **A user override is one set of rates:** it has no long-context, fast-mode or 1-hour rate, so a call priced by an override pays its rates whatever its tier (1-hour writes at its cache-write rate).
- **OpenAI cache writes:** their 30-minute and 12-hour prices are not modelled (cache-write rate 0). Codex's `cache_write_input_tokens` is stored apart from input, and whether it is also inside `input_tokens` is not stated on the protocol page we read (Inferred).
- **OpenAI's Fast tier** (the former Priority processing, for example gpt-5.3-codex $3.50 / $0.35 / $28.00) is not modelled: Codex's rollouts don't say which tier a call ran on (Inferred; not seen in the audit's data).
- **Batch discounts** are not modelled: the coding tools don't use the Batch API.
- **Codex models with no row on the page** stay unpriced. This covers older `gpt-5-codex`, `gpt-5.1-codex*`, `gpt-5.2-codex` and `codex-mini-latest`, and `codex-auto-review`, the model of Codex's automatic reviews (3.95 B tokens on the audit's Mac). The page has no row by that name (checked 2026-10-21, 2026-10-26 and 2026-11-03). Third-party price lists quote a rate for it, but no OpenAI page does, so it stays unpriced: never a guess (D-06). Usage's Prices and details names it as having no list price, and its total leaves it out in words.
- **Left out of the seed:** retired models (Claude Haiku 3.5), limited-availability models (Claude Mythos) and "pro" models.

## Follow-up: the pricing-update path (P15 d)

New models drop out silently today: a model with no row shows "No price for <model> yet" until someone edits the seed. Proposed follow-up task:

1. List the models in stored usage that have no price, and show them in Settings › Pricing with "Add a price".
2. Add a release checklist step: re-read both pages above and refresh the seed. A row older than 90 days is stale under AJ-E810.
3. Retire the inspector's separate `quality/pricing.v1.json`, which still keys `opus`/`sonnet` and never matches a real model ID, in favour of this ledger.
4. ~~Store 5-minute and 1-hour cache writes apart, and price each at its own rate.~~ Done in 0.3.7 (seed v4, per-call pricing).
5. A price change re-prices stored history at once today; the usage audit's design asks for it on the user's OK, with the change shown ("Re-priced with the prices of 3 Nov: +$412").
