# Evidence standard for external claims (AJ-E810)

What counts as proven when a task depends on something outside this
repository: a Provider's API, SDK, docs, file format, billing behaviour,
or a competitor's feature. It extends the honest-fields rule (D-06) to
research: what we haven't proven stays unknown, in every document and
every UI string.

## Every claim carries four things

| Field | What it holds |
|---|---|
| Source | The primary URL (vendor docs, SDK reference, release notes, the binary's own output), or a named capture under `docs/research/evidence/` |
| Accessed | The date the source was read for this claim (`YYYY-MM-DD`) |
| Evidence | A short paraphrase, or a quote of at most a sentence, of what the source says |
| Label | One of the four below |

| Label | Meaning |
|---|---|
| **Verified-primary** | The vendor's own docs, SDK or binary say it, read on the date given |
| **Verified-secondary** | A reliable second-hand source (vendor community, support article, reputable write-up) says it; not yet confirmed first-hand |
| **Negative-evidence** | We searched and didn't find it. The search log (queries, sources checked, date) is recorded. Always worded as "not found by this search", never as "the vendor doesn't offer it" |
| **Inferred** | Our own reasoning or arithmetic. Never shown as fact; in the product it is an estimate or `unknown` |

## Rules

1. **No label, no use.** A gate review (AJ-E807) rejects a claim without
   one, and product code may rely only on Verified-primary claims (or
   local observation under the billing protocol, AJ-E601).
2. **"No API found" needs its search log** — queries, sources checked,
   date — stored with the research document.
3. **Live where possible, never with secrets.** Sanctioned APIs, SDKs and
   docs are tested live when a candidate exists. Tests use sacrificial
   accounts only, with the maintainer's approval recorded; never a real
   user's account, credentials or keys. A step that needs a paid account
   action stops and becomes a maintainer action.
4. **Staleness.** A claim whose access date is more than 90 days old is
   stale. It is re-verified before it is used in a gate or in public
   copy. `scripts/research_staleness.sh` lists stale research documents.
5. **Re-verification sweep:** once per release candidate (≤ 1 hour), the
   research documents' sources are re-opened, and a source that has
   moved or changed is relabelled.

## Where things live

- Research logs: `docs/research/*.md`, each with a `Research date:` line.
- Captures (secrets redacted): `docs/research/evidence/`.
- The first application, the E6 billing and renewal claims:
  `docs/qa/evidence/AJ-E810/billing-claims-register.md`.
