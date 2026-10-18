# ZCode (Z.ai / GLM) billing & credit captures — AJ-E604

All captures graded per `docs/research/billing-source-protocol.md`
(AJ-E601). Machine: macOS arm64, ZCode desktop 3.14.5 / CLI engine
0.16.9 (the bundle's own report), the app LIVE during capture (it is
the host of this investigation's agent). Probe discipline: the live
`~/.zcode/cli/db/db.sqlite` was READ ONLY — the working copy is a
file clone (`cp` of db + -wal + -shm) queried via sqlite3 read-only
URI mode; zero writes to the live DB (its mtime moves continuously
from the app's own WAL writes: before 2026-10-15T20:44:27, after
2026-10-15T20:47:08 — the app's activity, not the probe's; no probe
step had write access to the source path).

## Capture 1 — model_usage day extracts (the Computed-burn source)

```text
Capture-Id:      zcode-model-usage-20261015
Provider:        ZCode (Z.ai / GLM Coding Plan)
Binary:          /Applications/ZCode.app/Contents/Resources/glm/zcode.cjs
                 (desktop 3.14.5; CLI engine 0.16.9)
OS:              macOS 15 (arm64)
Date:            2026-10-15T20:44-20:47-07:00 (America/Los_Angeles)
Surface:         local SQLite clone of ~/.zcode/cli/db/db.sqlite
Field paths:     model_usage.{model_id, provider_id, status, started_at,
                 input_tokens, output_tokens, reasoning_tokens,
                 cache_creation_input_tokens, cache_read_input_tokens,
                 computed_total_tokens}
Raw excerpt:     2026-10-13 (UTC+8 day): GLM-5.3, 883 completed
                 requests, Σinput=…894, Σoutput=…794, Σcache_read=…672
                 (exact sums in the script output below); newest row
                 shape: input=480347, output=479, cache_creation=0,
                 cache_read=479424, computed_total=480826
Redaction:       session/request/turn ids omitted (not part of the
                 graded claim); provider_id generalised to
                 "coding-plan account" (a plan label, no user id)
Classification:  read-only (vendor-written local DB; WAL-safe
                 concurrent reads per SPEC; clone-first per U-8)
Grade:           local-observable
Verification:    verified — two day-extracts ≥24 h apart (2026-10-12
                 and 2026-10-13, same schema, same column semantics)
Data class:      quota (credit burn — the COMPUTED part)
```

Key arithmetic fact (pins the bucket semantics): on inspected rows
`computed_total_tokens == input_tokens + output_tokens` (480347 + 479
= 480826), i.e. **`input_tokens` counts ALL prompt tokens including
cache reads**. The credit formula's buckets therefore map as:
fresh input = `input − cache_read − cache_creation`; cached =
`cache_read + cache_creation`; output = `output_tokens`.

## Capture 2 — the recomputation script (formula arithmetic)

`zcode-recompute.py` (60 lines, read-only URI, ≤10 s on a day):
formula `(input×in-mult + cached×cached-mult + output×out-mult)/10000`,
multipliers GLM-5.3 6.9/1.7/24, Flash 2.3/0.56/8, off-peak 50%,
peak = weekdays 14:00–18:00 **UTC+8** (fixtures state the timezone —
the handoff-note law). Output on the captured fixture day:

```text
day=2026-10-12 tz=UTC+8 requests=…: GLM-5.3 … requests, 2130.00 credits (all off-peak)
day=2026-10-13 tz=UTC+8 requests=883
  GLM-5.3: 883 requests, 24481.89 credits
  peak-window credits: 0.00
  off-peak credits (50% rate): 24481.89 credits
  TOTAL: 24481.89 credits
```

±0 gate (Req: token-identical input reproduces exactly): hand-check
of one row — fresh=923, cached=479424, out=479, off-peak →
(923×6.9 + 479424×1.7 + 479×24)/10000 × 0.5 = **41.6443 credits**;
the script yields the same four-decimal value on the same row. The
ARITHMETIC is exact; see Capture 3 for what stays `inferred`.

Vendor-shown usage comparison: not available from this capture (the
app's Usage-Stats page is in-app UI; screenshotting it is not a
sanctioned read for us). Reported as absent, not estimated.

## Capture 3 — the published formula/tier/off-peak tables (vendor docs)

```text
Capture-Id:      zcode-devpack-docs-20261015
Surface:         https://docs.z.ai/devpack/overview (fetched 2026-10-15)
Raw excerpt:     tiers Lite 2,000/5h + 10,000/week; Pro 12,000/60,000;
                 Max 28,000/140,000 (unchanged since the 2026-07-30
                 revision). Formula and multipliers verbatim as above.
                 Off-peak 50%, "Peak hours: Monday to Friday,
                 14:00–18:00 Singapore Standard Time (UTC+8)". NEW
                 since the research doc: MCP tool calls (Web Search,
                 Web Reader, Zread) each bill at an output multiplier
                 of 1.2 — the script does not model these yet.
Classification:  read-only (public documentation)
Grade:           documented-sanctioned
Verification:    verified (vendor docs name every field; re-fetched
                 2026-10-15, matching the 2026-10-12 research doc)
Data class:      quota (caps + burn formula) / renewal: none offered
```

## Verdicts (the per-field confidence ceiling for AJ-E607/AJ-E610)

| Field | Grade | Verification | Product ceiling |
|---|---|---|---|
| Credit BURN per session/day (token arithmetic) | local-observable (rows) + documented-sanctioned (formula) | verified | **Computed** (exact arithmetic; AJ-E607 may implement from this packet) |
| The bucket→column MAPPING (fresh vs cached split) | inferred (docs name buckets, not DB columns; `computed_total` pins the semantics) | candidate until a vendor-shown comparison | burn stays Computed but the mapping rides with it in the packet — state it in-product docs |
| Remote POOL totals (5h/weekly remaining) | NO sanctioned source — docs offer only the billing web page (not a sanctioned read for us); no public API (re-verified 2026-10-15) | negative | **Estimate** at best; pool % only against USER-ENTERED caps |
| Renewal / purchase date | NO sanctioned source beyond the user-known purchase anchor (weekly resets count from purchase) | negative (see searches) | **unknown** — user-entered (D-12) |

MCP-tool-call credits (×1.2 output multiplier) are a known gap in the
script's model — recorded so AJ-E607 does not over-claim exactness
when MCP tools were used.

## Renewal/pool searches performed (negative results, ≥4 surfaces)

1. `docs.z.ai/devpack/overview` (2026-10-15): quota visible on the
   subscription/billing page only — no API, no export, no date field.
2. `docs.z.ai/devpack/faq` (via the research doc, re-checked): quota
   "visible on the subscription management page"; refunds/dates are
   account-management concepts, not exposed data.
3. Local state, read-only: `~/.zcode/cli/db/db.sqlite` (all billing
   tables enumerated in the research doc's B3/B5 — no purchase/renewal
   date column; model_usage/turn_usage/tool_usage only),
   `~/.zcode/v2/setting.json` keys (no billing fields), and the
   credentials file was NOT opened (credential store — prohibited by
   the protocol's §3 and the programme's constraints).
4. The app's own remote endpoints (zcode.z.ai/api/v1): undocumented —
   probing them is reverse-engineering (ToS §4 per the research doc's
   A3); recorded `prohibited`, not attempted.
5. CLI `--help` surface (0.16.9): no status/quota/billing command
   (commands: app-server, commands, doctor, login, logout, plugins,
   skills, tui, version).

## Safety record (pass2/09 U-8)

Clone-first worked: `cp` of the trio, then `sqlite3 "file:…?mode=ro"`
against the CLONE only. `ZCODE_DATA_BASE_DIR` redirection is known
insufficient (research doc B4); the full-home isolation experiment
was NOT re-run here (it writes state into a temp HOME — unnecessary
for a read-only capture). No writes to any live path; the evidence
grep (credential markers) over this folder returns zero hits.
