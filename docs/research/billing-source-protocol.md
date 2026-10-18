# Billing & renewal source protocol (AJ-E601)

Status: **Active protocol — every per-Provider investigation (AJ-E602–E605)
must pass through this document before any discovered source may feed
product code.**

Implements: decision-log D-12 ("sanctioned, read-only, verified live");
deep/09 §Q1 sanctioned-only rule; pass2/09 U-4/U-5 (unverified parity
claims stay inferred). The bar is TESTABLE here, not asserted.

## 1. Evidence grades

Exactly four grades exist. Every candidate source gets exactly one,
recorded in the Provider's research doc with the capture that justifies
it. No grade may be skipped, blended, or "upgraded by confidence".

| Grade | Definition | May feed product? | UI state it can produce |
|---|---|---|---|
| `documented-sanctioned` | A vendor doc/SDK reference NAMES the field and the surface that carries it | Yes — after §3 verification | `Measured` |
| `local-observable` | The field is present in a local file/feed THE VENDOR ITSELF writes (statusline stdin, transcript JSONL, stats cache) | Yes — after §3 verification | `Measured` (local scope honestly noted) |
| `inferred` | Derived by us (arithmetic over tokens/pricing tables, heuristics, parity assumptions) — NEVER presentable as fact | No — display-only as an estimate, clearly labelled | `Estimate` |
| `prohibited` | Scraping, undocumented endpoints, credential stores, ToS-ambiguous surfaces | Never | — (the source is not used; state stays `unknown`) |

Rules that bite:

- A vendor marketing page announcing "billing API coming soon" is
  `inferred` at best — nothing is observable until the binary ships it.
- `ccusage`-style THIRD-PARTY arithmetic over transcripts is
  `inferred` (our own or a tool's — the grade follows the METHOD, not
  the author).
- A field that exists in the vendor's WEB UI but not in any
  sanctioned/local surface is NOT `documented-sanctioned` for us — we
  have no sanctioned way to read it. It is `unknown` in product.

## 2. Capture template (Requirement 2)

Every candidate source is verified live against a REAL installed
Provider binary. One capture per source, archived (secrets redacted)
under `docs/research/evidence/billing/`:

```text
Capture-Id:      <provider>-<surface>-<YYYYMMDD>
Provider:        <name>
Binary:          <path> --version → <exact version string>
OS:              <os + version>
Date:            <ISO 8601, local tz>
Surface:         <statusline-stdin | headless-stream-json | stats-cache | sdk | ...>
Field path:      <exact JSON/file path, e.g. rate_limits.five_hour.used_percentage>
Raw excerpt:     <minimal redacted snippet showing the field>
Redaction:       <what was removed/why — tokens, ids, paths>
Classification:  read-only | read-write | ToS-ambiguous (→ prohibited, clause cited)
Grade:           <§1 grade>
Verification:    candidate | verified (per §3, with the second capture's
                 id or the changelog entry URL)
Data class:      quota | renewal   (graded SEPARATELY per §4)
```

A capture without every field above is not evidence. `Redaction: none`
must be justified, not omitted.

## 3. Read-only classification (Requirement 3)

- `read-only`: the surface is one the vendor's own client writes and
  our process only reads a file/pipe it was handed (statusline stdin,
  transcript JSONL under the config dir, a stats cache).
- `read-write` or ambiguous (endpoints that accept state changes,
  credential-adjacent stores, anything a ToS reserves to the vendor):
  record as `prohibited` WITH the doc clause quoted. When the ToS is
  silent, silence is ambiguity — prohibited. We never "try it and
  see".

## 4. Verification (Requirement 4) — the D-12 gate

A source is `verified` only if:

- **two independent captures ≥24 h apart** show a stable field shape
  (same path, same type, sane value drift), OR
- **one capture + a vendor changelog entry** that documents the field
  (the vendor's own release notes name it — the shape is then pinned
  by the vendor, not by our sampling).

Anything less is `candidate`: usable in research docs, NEVER in
product. Do not soften this to ship faster — a single capture proves
the field existed once.

## 5. Quota vs renewal graded separately (Requirement 5)

Quota-window data (used %, resets_at) and renewal/billing DATES are
different claims with different evidence. A Provider may be `verified`
for quota and `unknown` for renewal — and per D-12 the EXPECTED
default is exactly that: renewal dates ship on user entry; a wrong
expiry in a procurement audit is worse than `unknown`. Each capture
row carries its `Data class`; a Provider's research doc reports a
per-class verdict, never a blanket one.

## 6. Demotion rule (Requirement 6)

If a `verified` source changes shape in a later Provider release
(path moved, type changed, field absent), product demotes that field
to `unknown` WITHIN ONE RELEASE CYCLE of us observing the change —
never silently coerced (no `?? fallback`, no shape-guessing parse).
The research doc records: capture of the old shape, capture of the
new, the demotion date, and the product state that flipped.

## 7. UI mapping (the honesty contract)

No UI state may exist that this protocol cannot produce:
`Measured` ← verified `documented-sanctioned`/`local-observable`;
`Computed` ← derived from a verified source by sanctioned arithmetic
(still not a vendor fact — the derivation is shown); `Estimate` ←
`inferred`; `unknown` ← everything else, including `prohibited` and
`candidate`. D-12's expiry default is `unknown` until a per-class
`verified` verdict exists.

## 8. Worked example — Claude statusline rate limits (quota class)

Capture (see `evidence/billing/claude-statusline-20261013.md`):

```text
Capture-Id:      claude-statusline-20261013
Binary:          claude 2.1.300 (macOS 15, arm64)
Surface:         statusline-stdin
Field paths:     rate_limits.five_hour.used_percentage,
                 rate_limits.five_hour.resets_at,
                 rate_limits.seven_day.used_percentage
Classification:  read-only (the vendor's client pipes its own JSON to
                 our script; we write nothing)
Data class:      quota
```

Classification walk-through (the reasoning every capture gets):

1. **Grade.** The vendor's statusline docs name `rate_limits` with
   `used_percentage` and `resets_at`, "present only for claude.ai Pro
   and Max subscribers" (code.claude.com/docs/en/statusline, cross-
   checked in `docs/research/claude-code.md` §5). The field is BOTH
   documented by the vendor AND present in a feed the vendor's client
   writes locally → `documented-sanctioned` + `local-observable`
   (record the stronger: `documented-sanctioned`).
2. **Read-only?** Statusline stdin is handed to our script by the
   client; the docs describe the script as output-only. No endpoint,
   no credential store → `read-only`.
3. **Verification.** Path two needs ONE LIVE observation plus the
   vendor doc/changelog. The capture machine's binary is
   unauthenticated and emits no `rate_limits` — zero live
   observations — so the honest verdict is `candidate`, NOT
   `verified`. (This example initially claimed `verified` and was
   downgraded by the blind classification check — the check IS the
   protocol working.)
4. **Data class.** `quota` only — the payload carries NO renewal or
   billing date. Claude's renewal verdict therefore stays `unknown`
   per D-12 (user-entered dates remain the product path).

Result: `Claude / quota / documented-sanctioned / candidate
(may-feed-product: no)`; `Claude / renewal / unknown`. The quota
verdict flips to `verified` only when AJ-E602's authenticated capture
shows the live field.

## 9. Per-Provider investigation checklist (AJ-E602–E605)

1. Enumerate candidate surfaces (statusline, headless stream-json,
   hooks, stats caches, SDKs, official APIs) from the vendor's DOCS
   first — never from endpoint probing.
2. Capture each per §2 (≤15 min per source budget).
3. Grade, classify, verify per §3–§5; record per-class verdicts.
4. Grep the evidence folder for `sk-`, `Bearer`, `Cookie` — must be
   ZERO hits before anything merges.
5. Update the Provider's research doc with the verdict table; product
   surfaces only ever read grades this protocol produced.
