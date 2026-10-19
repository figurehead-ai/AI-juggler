# Billing and renewal claims register (AJ-E810, first application)

The E6 subscription-clock claims in `docs/research/billing-apis.md`, one
row per claim, under `docs/research/EVIDENCE-STANDARD.md`. Re-opened on
2026-10-18 where the source allows automated reading.

## Claude (Anthropic)

| Claim | Source · accessed | Label | Evidence |
|---|---|---|---|
| Plan credits and their expiry are shown in a UI, not an API | platform.claude.com/docs/en/about-claude/api-credits-for-subscribers · 2026-10-18 | Verified-primary | Credits and their expiry date appear in the Console's Billing page; claiming happens in claude.ai Settings › Billing |
| The Organization (Admin) API has no renewal, invoice or billing-period endpoint | platform.claude.com/docs/en/api/beta/organization · 2026-10-18 | Negative-evidence | The reference covers organisation info, API keys, encryption keys and identity federation; no billing resource was found in the part read (the page is very long) |
| No sanctioned public endpoint returns a consumer subscription's renewal date | `billing-apis.md` search log · 2026-10-13 | Negative-evidence | Not found by that search; renewal lives in claude.ai Settings (UI only) |
| Usage windows are observable locally from the CLI's own output | `docs/research/claude-code.md` · 2026-10-13 | Verified-primary | Statusline and transcript events, read by the usage pipeline (Measured) |

## Codex (OpenAI)

| Claim | Source · accessed | Label | Evidence |
|---|---|---|---|
| Usage and billing periods are reviewed in the Usage Dashboard (UI) | help.openai.com/en/articles/10478918 · 2026-10-13 | Verified-secondary | The help article describes the dashboard; on 2026-10-18 it refused automated reading (403), so the earlier access stands |
| No public API returns a ChatGPT-plan renewal date | `billing-apis.md` search log · 2026-10-13 | Negative-evidence | Not found by that search |
| Codex can bill API credits instead of the plan | community.openai.com/t/1358277, /t/1389087 · 2026-10-13 | Verified-secondary | Users report API billing despite a ChatGPT login; the reason for the identity assertion (AJ-E1001) |

## ZCode

| Claim | Source · accessed | Label | Evidence |
|---|---|---|---|
| No billing API is documented | `docs/research/zcode.md` · 2026-10-13 | Negative-evidence | Not found in the research corpus; credit pools are Computed |

## Gemini (no adapter yet)

| Claim | Source · accessed | Label | Evidence |
|---|---|---|---|
| Renewal source unknown | — | Inferred | Not researched in depth; user-entered on arrival |

## What follows for the product

Every Provider's renewal date stays **user-entered** (D-12). No claim
above is a Verified-primary *renewal API*, so none may feed product
code. The UI says `Unknown — no renewal date entered` until the user
types one. No live API test was run: no candidate passed the
"sanctioned, read-only, renewal-bearing" bar, and live tests never use
real accounts.
