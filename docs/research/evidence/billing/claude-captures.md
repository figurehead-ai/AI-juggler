# Claude (Anthropic) billing, quota & renewal captures — AJ-E602

All captures graded per `docs/research/billing-source-protocol.md`
(AJ-E601). Machine: macOS arm64, `claude` 2.1.300
(~/.local/bin/claude). Auth boundary (gate-M1 standing step): the
direct-install copy is UNAUTHENTICATED; live captures below split
honestly between (i) the direct install's unauth probes and (ii) the
maintainer's IMPORTED managed copies under AI Juggler's own app state
(read-only; `~/.claude` and the t3 originals untouched, never read).
`auth`-shaped files were never opened.

## Capture 1 — transcript `usage` objects (per-message tokens)

```text
Capture-Id:      claude-transcript-usage-20261015
Provider:        Claude Code (Anthropic)
Binary:          claude 2.1.300 (the session that WROTE the
                 transcript ran 2.1.x per the managed copy's era)
OS:              macOS 15 (arm64)
Date:            2026-10-15T21:15-07:00 (America/Los_Angeles)
Surface:         <managed>/projects/<proj>/<session>.jsonl (the
                 vendor client's own transcript JSONL — read from
                 AI Juggler's imported account copy)
Field paths:     message.usage.{input_tokens,
                 cache_creation_input_tokens,
                 cache_read_input_tokens, output_tokens,
                 output_tokens_details.thinking_tokens,
                 service_tier, cache_creation.*}
Raw excerpt:     input=2, cache_creation=35641, cache_read=16578,
                 output=232, thinking=25, service_tier="standard"
Redaction:       session/project paths reduced; no message content
                 read or quoted (usage objects only)
Classification:  read-only (vendor-written local JSONL)
Grade:           local-observable (+ documented-sanctioned: the
                 vendor's headless/stream docs name usage fields)
Verification:    verified — the managed copy holds multiple sessions
                 across days with an identical usage shape, plus the
                 vendor docs naming the fields
Data class:      quota (tokens — the per-message leg)
```

## Capture 2 — transcript `cost-state` lines (session cost rollups)

```text
Capture-Id:      claude-transcript-cost-state-20261015
Surface:         same managed transcript JSONL, line type "cost-state"
Field paths:     totalCostUSD, totalAPIDuration{,WithoutRetries},
                 totalToolDuration, totalLines{Added,Removed},
                 totalDuration, startTime, modelUsage.<model>.
                 {inputTokens, outputTokens, thinkingTokens,
                 cacheRead/CreationInputTokens, webSearchRequests}
Raw excerpt:     totalCostUSD=33.88, totalDuration=6460527 ms,
                 modelUsage incl. claude-haiku-4-5 with token rollups
Redaction:       sessionId reduced; modelUsage truncated
Classification:  read-only
Grade:           local-observable (the vendor client's own rollup —
                 stronger than re-summing Capture 1)
Verification:    verified (present + shape-stable across the copy's
                 sessions; a superset of the documented statusline
                 cost.total_cost_usd semantics)
Data class:      quota/cost (session totals — COMPUTED-READY)
```

## Capture 3 — statusline stdin (rate_limits) + the chaining fixture

```text
Capture-Id:      claude-statusline-chaining-20261015
Binary:          claude 2.1.300, unauthenticated (throwaway
                 CLAUDE_CONFIG_DIR=/tmp/e602/cfg)
Surface:         settings.json statusLine → wrapper script logging
                 stdin
Raw excerpt:     NONE — the login screen never invokes the
                 statusline (no stdin payload file after a 15 s
                 session). HONEST NEGATIVE: the live rate_limits leg
                 requires an authenticated Pro/Max session (the
                 fields are "present only for claude.ai Pro and Max
                 subscribers" per the vendor docs — consistent).
Grade:           documented-sanctioned (docs) / the live payload leg
                 stays candidate — see AJ-E601's worked example,
                 which this confirms from a second angle
Verification:    candidate (live leg pending the maintainer's
                 re-login; rides AJ-E610)
Data class:      quota (5h/weekly windows)
Chaining proof:  the fixture script + settings survive verbatim in
                 /tmp/e602 (the mechanism AJ-E610's wrapper must
                 call-through); the chaining CONSTRAINT (a
                 pre-existing user statusline survives our capture)
                 is a product law for the managed wrapper, proven at
                 its implementation, not re-provable pre-auth here.
```

## Capture 4 — stats-cache.json / `/usage` (local-machine reconciliation)

`stats-cache.json`: ABSENT in every probed managed account copy and
the throwaway config (no `/usage` has been run inside the copies) —
honest negative; per the vendor docs it backs `/usage`, which is
"local session history … not included from other devices". Grade:
`local-observable` by class, `candidate` here (no live instance);
product ceiling: reconciliation data, never multi-device truth.

## Renewal/billing-date searches performed (negative, ≥4 surfaces)

1. **Anthropic Console / Admin API** (checked 2026-10-15): org API
   usage, credits, spend limits, API keys — "no documented endpoint
   that exposes a consumer subscription renewal date"; Pro/Max is
   billed in claude.ai/Stripe ([support article](https://support.claude.com/en/articles/9876003-i-have-a-paid-claude-subscription-pro-max-team-or-enterprise-plans-why-is-claude-api-usage-billed-separately-from-my-paid-claude-plan),
   [Admin API organization endpoint](https://platform.claude.com/docs/en/api/beta/organization)).
2. **Claude Code CLI surface** (`claude --help`, 2.1.300): no
   billing/plan/renewal command or flag.
3. **Statusline schema** (vendor docs): rate_limits + cost +
   context_window fields only — no renewal field exists in any
   documented payload.
4. **Local config/state**: settings keys and the managed copies'
   config surfaces carry no billing-date field (probed read-only;
   credential-shaped files never opened).
5. **API-vs-subscription gotcha recorded for AJ-E607**: with
   `ANTHROPIC_API_KEY` set, Claude Code bills the API pre-paid
   credits, NOT the subscription ([claude-code #16489](https://github.com/anthropics/claude-code/issues/16489))
   — a managed launch must keep the env clean or the quota model is
   silently wrong.

## Pro / Max / API-key field dependence (Req.4)

- `rate_limits.*` in statusline stdin: **Pro/Max only** (vendor
  docs). API-key accounts: fields absent → product falls back to
  transcript token sums (Captures 1-2), which is exactly the
  documented behaviour ("local session history").
- Live tier-dependent captures: NOT POSSIBLE pre-auth — each tier's
  live shape is `unverified` until the maintainer's re-login; the
  fallback path (transcript sums) is verified TODAY via the managed
  copies (which are subscription-era sessions).

## Verdicts (for AJ-E610)

| Field | Grade | Verification | Product ceiling |
|---|---|---|---|
| Per-message tokens (transcripts) | local-observable + documented | verified | **Measured** (local session scope honestly noted) |
| Session cost rollups (`cost-state`) | local-observable | verified | **Computed** from vendor-supplied rollups |
| 5h/weekly quota % (statusline rate_limits) | documented-sanctioned | candidate (live leg = authenticated session, pending re-login) | **Measured** ONCE verified; until then unknown/Estimate |
| stats-cache / /usage | local-observable | candidate (no live instance) | reconciliation only, per-device |
| Renewal / billing date | NO sanctioned source (≥4 negatives; claude.ai/Stripe is a human surface) | negative | **unknown** — user-entered (D-12) |
