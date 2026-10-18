# OpenAI / Codex billing & quota captures — AJ-E603

All captures graded per `docs/research/billing-source-protocol.md`
(AJ-E601). Machine: macOS arm64. Two live installs probed: the
maintainer's **Codex Desktop** (rollout-verified `cli_version
0.162.0-alpha.17.2`, sessions on disk) and a fresh **npm CLI
0.163.0** (`@openai/codex`, installed 2026-10-15 into the global npm
prefix; probes run against a throwaway `CODEX_HOME=/tmp/e603-home`).
`~/.codex/auth.json` was NEVER opened (credential store — prohibited,
protocol §3). Zero scraping prototypes retained (all probes are
first-party CLI invocations or public docs).

## Capture 1 — rollout `token_usage_record` (the Measured-tokens source)

```text
Capture-Id:      codex-rollout-usage-20261015
Provider:        OpenAI Codex (Codex Desktop, ChatGPT-plan auth)
Binary:          Codex Desktop bundle; rollout session_meta records
                 cli_version 0.162.0-alpha.17.2 (2026-10-14 session)
OS:              macOS 15 (arm64)
Date:            2026-10-15T21:00-07:00 (America/Los_Angeles)
Surface:         ~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl (read)
Field paths:     payload.usage.{input_tokens, cached_input_tokens,
                 cache_write_input_tokens, output_tokens,
                 reasoning_output_tokens, total_tokens} +
                 turn_token_usage / thread_token_usage rollups
Raw excerpt (ids redacted): input=52381, cached=5888,
                 cache_write=0, output=200, reasoning=136,
                 total=52581; one record per response, per turn, and
                 per-thread rollups on the same line
Redaction:       thread/turn/session/response ids reduced to <id>;
                 cwd and creator ids omitted (not part of the claim)
Classification:  read-only (the vendor's own client writes the JSONL)
Grade:           local-observable
Verification:    verified — two captures ≥24 h apart (2026-10-09 and
                 2026-10-14 sessions, identical record shape) plus the
                 npm-CLI 0.163.0 unauth run writing the same rollout
                 envelope (Capture 2)
Data class:      quota (model TOKENS — not plan credits)
```

## Capture 2 — `codex exec --json` event shape (CLI 0.163.0, unauth)

```text
Capture-Id:      codex-exec-json-20261015
Binary:          codex-cli 0.163.0 (npm @openai/codex)
Surface:         `codex exec --json --skip-git-repo-check` against an
                 empty throwaway CODEX_HOME (no credentials present)
Raw excerpt:     {"type":"thread.started","thread_id":"<id>"} →
                 {"type":"turn.started"} → typed {"type":"error", …}
                 events (401 Unauthorized here — the SCHEMA is the
                 capture; an authenticated run's turn.completed
                 carries the same usage object as Capture 1 per
                 learn.chatgpt.com/docs/non-interactive-mode)
Redaction:       ids reduced; cf-ray values omitted
Classification:  read-only
Grade:           local-observable (event envelope verified live; the
                 usage payload's authenticated shape is
                 documented-sanctioned, see docs URL above)
Verification:    verified for the envelope (live, twice: trusted and
                 untrusted-dir variants); the authenticated usage
                 payload leg rides Capture 1's two live captures
Data class:      quota (tokens)
```

## Capture 3 — the API-side billing surface (and why it is NOT seat evidence)

`GET /v1/organization/costs` (+`/usage/completions`): **Admin API
key** auth, organization-scoped, reports **pay-as-you-go API-key
usage only** — "ChatGPT subscription plans … are not tied to an API
key or platform organization — the API billing and ChatGPT plans are
entirely separate accounts/billing systems" (checked 2026-10-15;
[Costs API reference](https://developers.openai.com/api/reference/resources/admin/subresources/organization/subresources/usage/methods/costs),
[OpenAI usage-API announcement](https://community.openai.com/t/introducing-the-usage-api-track-api-usage-and-costs-programmatically/1043058)).
Grade: `documented-sanctioned` for **API-key cost** — and explicitly
NOT subscription renewal evidence. API-key billing SHALL NOT be
presented as ChatGPT-seat usage (the classic false-`Measured` bug).

## The /statusline widgets (why the ceiling stays Estimate)

`tui.status_line` widgets `five-hour-limit` / `weekly-limit` exist
(maintainer-confirmed, research doc §5). The widget RENDERS limits
the client fetches from an **undocumented** endpoint
(`account/rateLimits/read`, per third-party `aistat`) — undocumented
endpoints are `prohibited` under the protocol §1/§3, and dashboard
scraping is the ToS line ("Automatically or programmatically extract
data or Output … circumvent … rate limits", OpenAI ToS (a)/(b),
retrieved 2026-10-12 in the research doc §7). Widgets are therefore
UI-only evidence for HUMANS; product reads nothing from them.

## Renewal/billing-date searches performed (negative, ≥4 surfaces)

1. **CLI surface** (`codex --help`, `login status`, subcommand help,
   0.163.0): no billing/renewal/plan command; `login status` reports
   only "Not logged in" style state.
2. **Documented config keys** (config-reference, research doc §2):
   no billing/plan/renewal key family exists.
3. **API docs** (Capture 3): costs/usage endpoints carry
   `line_item`/`time` data for API usage — no subscription renewal
   date field, and wrong scope for seats regardless.
4. **Local CODEX_HOME state**: rollout lines enumerated (session_meta,
   turn_context, response_item, token_usage_record, event_msg,
   world_state — none carry plan dates); `auth.json` NOT opened
   (credential store, prohibited); goals/logs sqlite files are
   client-local telemetry.
5. **ChatGPT plan docs** (learn.chatgpt.com/docs/pricing, 2026-10):
   usage/billing lives behind ChatGPT sign-in web UI; `/usage` in-CLI
   opens those settings — a human surface, not a data source.

## Plan-dependence record (pass2/09 U-5 discipline)

The maintainer's plan tier is NOT recorded in any captured artifact
(rollouts carry no plan field) — tier: **unverified**. The captured
usage shape is therefore attributed to "a ChatGPT-plan Codex install,
tier unknown". Pro-has-no-5h-limit (research doc §5) stays
community/doc-sourced, not live-verified; n=1 evidence is not proof —
per-tier ceilings stay `unverified` until an authenticated capture
names its tier.

## Verdicts (the confidence-ceiling table for AJ-E607/AJ-E610)

| Field | Grade | Verification | Product ceiling |
|---|---|---|---|
| Per-turn/session TOKEN counts (rollouts + exec --json) | local-observable | verified (two live captures ≥24 h + envelope) | **Measured** tokens |
| Plan-credit/quota PERCENTAGE (5h/weekly) | no sanctioned source (widgets render an undocumented feed; scraping prohibited) | negative | **Estimate** — badge + `≈` reset times mandatory (a styled-as-exact Codex percentage is a protocol violation) |
| API-key COST (org usage API) | documented-sanctioned (Admin key, org scope) | verified docs 2026-10-15 | Measured for API-key usage ONLY — never seat evidence |
| Renewal / billing date | no sanctioned source (≥4 surfaces, negatives above) | negative | **unknown** — user-entered (D-12) |
