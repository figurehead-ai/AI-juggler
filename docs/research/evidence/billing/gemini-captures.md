# Google Gemini CLI billing & quota captures — AJ-E605

All grades per `docs/research/billing-source-protocol.md` (AJ-E601).
Machine: macOS arm64. **No Gemini install exists** (`which gemini`
empty; no `~/.gemini`; no npm-global/brew entry — probed
2026-10-16). Per the AJ-E605 performance-budget clause this packet is
**docs-only with live items marked `unverified — no install`** — a
passing outcome, not a failure. No credential store was touched
(there is none to touch). Zero scraping prototypes; every claim
below cites an official doc page or first-party announcement with
its date. Research base for `docs/research/gemini.md`.

## Capture 0 — no-install statement (protocol §4 doc-grade path)

```text
Capture-Id:      gemini-no-install-20261016
Provider:        Google Gemini CLI (legacy) / Antigravity CLI (successor)
Probe date:      2026-10-16
Probes:          `which gemini` → empty
                 `ls ~/.gemini` → absent
                 `npm ls -g | grep -i gemini` → empty
                 `brew list | grep -i gemini` → empty
Outcome:         docs-only packet; every live item below is
                 `unverified — no install` (AJ-E605 budget clause)
```

## Capture 1 — sunset banner + tier table (documented-sanctioned)

```text
Capture-Id:      gemini-quota-doc-20261016
Surface:         https://geminicli.com/docs/resources/quota-and-pricing
                 (checked 2026-10-16; banner: individual users moved
                 to Antigravity CLI on 2026-06-18)
Supporting:      announcement 2026-05-19
                 https://developers.googleblog.com/an-important-update-transitioning-gemini-cli-to-antigravity-cli
                 maintainer discussion
                 https://github.com/google-gemini/gemini-cli/discussions/28017
Graded facts:    individual (free/Pro/Ultra) login paths DEAD;
                 remaining: API key free 250 req/day (Flash-only),
                 API key pay-as-you-go (per token), Vertex Express /
                 pay-as-you-go, Workspace Standard 1500 / Enterprise
                 2000; NO 5h/weekly window on the legacy CLI (daily
                 caps only); /stats = session tokens + limit info;
                 usage summary on session exit
Grade:           documented-sanctioned (official doc, dated)
Live shape:      unverified — no install
```

## Capture 2 — API rate-limit/reset rules (documented-sanctioned)

```text
Capture-Id:      gemini-rate-limits-doc-20261016
Surface:         https://ai.google.dev/gemini-api/docs/rate-limits
                 (page updated 2026-10-15)
Graded facts:    RPM/input-TPM/RPD per PROJECT (not per key);
                 RPD resets midnight Pacific; spend-based rolling
                 10-min caps (Tier 1 $10 / 2 $50 / 3 $200) → 429
                 RESOURCE_EXHAUSTED; usage tiers auto-upgrade by
                 spend history; rate-limit view = AI Studio web
                 dashboard only — no API/header/local-file feed for
                 remaining quota or reset times is documented
Grade:           documented-sanctioned (reset rule) /
                 prohibited-by-absence stays N/A (nothing undocumented
                 probed); quota-feed absence recorded as negative
```

## Capture 3 — renewal surfaces negative search (D-12)

```text
Capture-Id:      gemini-renewal-negative-20261016
Searched (2026-10-16, all negative for a queryable renewal date):
  1. developers.google.com API index — no consumer-subscription
     API; Gemini API billing = prepaid credits expiring 12 months
     after purchase (ai.google.dev/gemini-api/docs/billing,
     2026-09-28)
  2. Google Play Developer API purchases.subscriptionsv2 —
     expiryTime only for YOUR OWN app's subscriptions
  3. myaccount.google.com/subscriptions + Play Store UI —
     human-read web only
  4. one.google.com / gemini.google/subscriptions — marketing
     tables, no date feed
  5. documented ~/.gemini config/state keys — no billing field
Verdict:         renewal = UNKNOWN source; D-12 → user-entered date
                 or Unknown; NEVER inferred from resets/first-seen/tier
```

## Capture 4 — Antigravity CLI successor surfaces (documented, ungraded live)

```text
Capture-Id:      antigravity-docs-20261016
Surfaces:        https://antigravity.google/docs/cli/commands/usage
                 (/usage alias /quota — refreshes model quota
                 display); https://antigravity.google/docs/plans
                 (5-hour refresh, weekly caps, Ultra 5X token rate);
                 blog /changes-to-antigravity-plans (2026-05)
Community [U]:   skainguyen1412/antigravity-usage reads quota from
                 local IDE state offline — local-observable CANDIDATE,
                 path/shape undocumented → prohibited until captured
                 & graded per protocol §6
Live grade:      unverified — no install (successor out of scope
                 for AJ-E605; investigate under a new task ID)
```

## Verdict table (what E607/E610/E611 may rely on)

| Field | Grade | Basis |
|---|---|---|
| Session token usage (legacy CLI) | documented-sanctioned shape; live `unverified — no install` | /stats + exit summary (official docs) |
| Account quota % | none | no feed exists at any grade |
| Absolute burn | unknown | no verified local event source |
| Daily reset rule (API path) | documented-sanctioned | midnight Pacific (rate-limits doc) |
| Renewal date | unknown → user-entered (D-12) | §3 negative search |
| Antigravity quota display | documented-sanctioned candidate | /usage (official), ungraded live |

## Verdict

Gemini enters the E6 engine as the honest-zero Provider: **no feed
verifies, so quota renders unavailable-with-reason (D-06), renewal is
user-entered or Unknown (D-12), and ranking headroom is unknown.**
The one durable machine-fact is the API path's midnight-Pacific
daily reset — a window-clock rule only, never a renewal inference.
The successor Antigravity CLI has a documented quota readout
(`/usage`) and 5h/weekly shape — the first surface to capture when a
Gemini-family Provider actually lands.
