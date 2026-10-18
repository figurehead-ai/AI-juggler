# E6 — Quota, Cost & Billing Research

> Epic owner: quota/cost domain. Status: TODO (Stage-1 docs only per D-15; > no code ships until a separate implementation approval).
> Settled law, do not re-litigate: D-01 (quota is a trigger/input, not the > headline — Switch → See → Clean), D-06 (inspector fields only when
> measurable, else unavailable-with-reason), D-07 (measurable signals only,
> no composite score), D-12 (subscription expiry is user-entered, optional;
> automated population is research-gated), D-15 (this file is tasks, not code).
> Canonical design: `docs/review/deep/09-handover-and-quota-engine-design.md`
> Part Q (§Q1–Q9); display home: pass2/04 §§3–6 (I-05/I-06, Usage tab); > ranking home: deep/09 §Q6 + pass2/02 §7; critic constraints:
> pass2/09 C-4/C-5, U-4/U-5/U-8. Honesty rule: SPEC.md:259. > Two clocks, never conflated in data or copy: (1) the **window clock** —
> rolling 5h/weekly quota pools that reset; (2) the **subscription clock** —
> the paid period that renews/expires (D-12). A full window is not an > expiring subscription and an expiring subscription is not a full window.

## Conventions for every task in this file

- Confidence labels are exactly `Measured / Computed / Estimate` (deep/09 §Q2 Q-4; `Confidence` at
crates/juggler-core/src/model.rs:288-297). No stars, scores, or hidden footnotes. USD is never Measured (§Q2 Q-4).
- Sanctioned sources only: local files + vendor-fed local JSON. No dashboard scraping, no undocumented endpoints, no
credential-byte reads (deep/09 §Q1; docs/research/codex.md:70-72). Unproven source ⇒ value is `unknown`.
- Suggest, never switch: ranking/alerts prepare; every state change is one explicit user click (SPEC.md:263, ADR-0003 lineage in deep/09 §Q4/Q6).
- Research tasks (AJ-E601–E605) produce evidence documents, not code. Their exit gate is the sanctioned-source proof protocol in AJ-E601.

---

### AJ-E601 Sanctioned-source proof protocol for Provider billing & renewal data | Status: DONE-EVIDENCED (evidence: docs/qa/evidence/AJ-E601/manifest.md) | Priority P0 | Effort M | Depends: none
**Goal**: One reusable protocol that every per-Provider billing/renewal investigation (AJ-E602–E605)
must pass before any discovered source may feed product code, so D-12's "sanctioned, read-only,
verified live" bar is testable rather than asserted.
**User story**: As Sara (P5, enterprise-locked, deep/10 §0), I need Juggler to never populate my
renewal date from an unproven source, because a wrong expiry date in a procurement audit is worse than `unknown`.
**Requirements**:
1. The protocol SHALL define four evidence grades: `documented-sanctioned` (vendor docs/SDK
   reference naming the field), `local-observable` (field present in a local file/feed the vendor
   itself writes), `inferred` (derived, never presentable as fact), and `prohibited` (scraping,
   undocumented endpoints, credential stores); only the first two may feed product, and only after Requirement 2.
2. Each candidate source SHALL be verified live on a real installed Provider binary: capture the raw
   payload/file excerpt (secrets redacted), binary `--version`, OS, date, and the exact field path, stored per AJ-E601 Evidence below.
3. Each candidate SHALL be classified read-only or read-write; read-write or ToS-ambiguous sources
   SHALL be recorded as `prohibited` with the doc clause cited.
4. A source SHALL be marked `verified` only if two independent captures ≥24h apart (or one capture
   plus a vendor changelog entry) show a stable field shape; a single capture is `candidate`.
5. Renewal/billing dates SHALL be graded separately from quota-window data; a Provider may pass for
   quota and fail for renewal (the expected default per D-12).
6. The protocol SHALL define the demotion rule: if a `verified` source changes shape in a later
   Provider release, it demotes to `unknown` in product within one release cycle, never silently coerced.
**UI/UX reasoning**: Nielsen/NN-g #1 Visibility of system status + deep/09 §Q2 honesty contract —
the protocol's grades map 1:1 onto UI states (`Measured/Computed/Estimate/unknown`); no UI state may
exist that the protocol cannot produce. No user-facing surface in this task.
**Performance budget**: Protocol document ≤400 lines; one live capture ≤15 min per source; verification adds 0 ms runtime (research-only).
**Quality gates**: Protocol reviewed against D-12 text (decision-log.md:113-124) line-by-line; every
grade has one worked example (real or explicitly hypothetical-labelled); zero product code changed (`git diff --stat crates/ ui/` empty).
**Evidence/validation proof**: Deliver `docs/research/billing-source-protocol.md` containing the
grade table, the capture template (binary version, field path, redacted excerpt, timestamp), and one
fully worked Claude statusline example cross-checked against docs/research/claude-code.md:54. Pass
threshold: a second agent, given only the protocol + a raw capture, classifies it to the same grade.
Store evidence in `docs/research/evidence/billing/` (redacted captures only; pre-commit grep for
`sk-`, `Bearer`, cookie headers must return 0 hits).
**Visual testing**: None (research protocol). Verify the evidence folder renders in Markdown preview
with no broken tables at 100-col terminal width.
**Files to touch**: `docs/research/billing-source-protocol.md` (new),
`docs/research/evidence/billing/README.md` (new); read-only: `docs/review/decision-log.md`,
`docs/research/claude-code.md`, `docs/research/codex.md`, `docs/research/zcode.md`.
**Research basis**: decision-log.md:113-124 (D-12 mandatory research item); deep/09 §Q1
(sanctioned-only rule, lines 327-332); pass2/09 U-4/U-5 (unverified parity claims must stay inferred).
**Handover note**: Next agent: do not soften Requirement 4 to ship faster — D-12's gate is the
two-capture rule. Trap: vendor marketing pages announcing "billing API coming soon" are `inferred`,
not `documented-sanctioned`. Gemini has no `docs/research/gemini.md` yet; AJ-E605 creates it under this protocol.

### AJ-E602 Claude (Anthropic) billing, quota & renewal-date investigation | Status: DONE-EVIDENCED (evidence: docs/qa/evidence/AJ-E602/manifest.md) | Priority P0 | Effort M | Depends: AJ-E601
**Goal**: Determine, with live proof, exactly which Claude Code surfaces expose (a) 5h/weekly quota,
(b) token/cost totals, and (c) any subscription renewal/billing date — and grade each under AJ-E601.
**User story**: As Maya (P1, 2×Max, deep/10 §0), I want my renewal warning to come from a real
Anthropic source if one exists, and to stay `unknown` if it does not, so I never trust a guessed date with a $200/month decision.
**Requirements**:
1. The investigation SHALL test, on a live Claude Code install (record version): statusline stdin
   JSON `rate_limits.five_hour/seven_day.{used_percentage,resets_at}` (SPEC.md:192), transcript
   `usage` + `cost.total_cost_usd` fields, `stats-cache.json`, and `/usage` output, grading each per AJ-E601.
2. It SHALL explicitly test for a renewal/billing-date source: Anthropic Console API, Claude Code
   CLI flags/env, local config/state files, and any documented SDK billing endpoint; absence SHALL
   be recorded as a negative result with the searches performed, not omitted.
3. It SHALL verify the statusline chaining constraint (SPEC.md:192, deep/09 §Q1 lines 341-345): a
   pre-existing user statusline survives capture, proven with a fixture script.
4. It SHALL record which fields exist on Pro vs Max vs API-key auth, since `rate_limits` is
   Pro/Max-only (deep/09 §Q1 line 336); API-key accounts SHALL be shown to fall back to transcript sums.
5. Any renewal source found SHALL NOT be wired to product in this task; it is handed to AJ-E610 as a graded evidence packet.
**UI/UX reasoning**: No UI built. Copy consequence recorded for AJ-E610: per Nielsen/NN-g #9 (help
users recognise errors) + D-12, the renewal field's empty state is exactly `Renewal: unknown — add date` and never a pre-filled guess.
**Performance budget**: Live capture session ≤2h; statusline fixture rerun ≤5 min; 0 runtime cost.
**Quality gates**: Every claim cites a capture file or a vendor doc URL with access date; negative
results list ≥4 searched surfaces for renewal; no credential bytes in evidence (grep gate as AJ-E601).
**Evidence/validation proof**: `docs/research/evidence/billing/claude-captures.md` with redacted
statusline JSON, transcript excerpt, binary version, and per-field AJ-E601 grades; pass = quota
fields reach `documented-sanctioned` or `local-observable`, renewal field reaches a stated grade
(including `prohibited`/absent). Store alongside AJ-E601 evidence folder; link from
`docs/research/claude-code.md` §5 addendum (append only, do not rewrite).
**Visual testing**: None. If `/usage` screenshots are captured, redact account email, capture at
default terminal size, and store as evidence only (not product visual tests).
**Files to touch**: `docs/research/evidence/billing/claude-captures.md` (new),
`docs/research/claude-code.md` (append §5 addendum); read-only: `docs/SPEC.md:191-195`,
`crates/juggler-adapters/src/claude_transcript.rs:140-169`.
**Research basis**: deep/09 §Q1 lines 334-352 (Claude row, chaining rule); SPEC.md:192;
docs/research/claude-code.md:50-55; decision-log.md:118-122 (D-12 research mandate names Claude first).
**Handover note**: Trap: `stats-cache.json`/`/usage` are local-machine only (deep/09 lines 345-347)
— grade them reconciliation, never multi-device truth. Do not re-litigate D-12: even a found renewal
source enters product only via AJ-E610's gate.


*Implementation mapping (recorded in this task's PR per the claiming protocol)*: evidence packet `docs/research/evidence/billing/claude-captures.md` — transcripts (`message.usage`) + `cost-state` rollups verified local-observable from the imported managed copies (tokens Measured, session cost Computed); statusline rate_limits stays candidate (the unauth login screen never invokes the statusline — live leg pending the maintainer's re-login, consistent with the protocol's worked example); stats-cache//usage candidate (no live instance; reconciliation-grade only, local-machine by the vendor's own docs); renewal negative across ≥4 surfaces (Console/Admin API is org-API scope; claude.ai/Stripe is the human surface); the ANTHROPIC_API_KEY-vs-subscription billing gotcha recorded for AJ-E607; addendum appended to claude-code.md §5.
### AJ-E603 OpenAI / Codex billing, quota & renewal-date investigation | Status: DONE-EVIDENCED (evidence: docs/qa/evidence/AJ-E603/manifest.md) | Priority P0 | Effort M | Depends: AJ-E601
**Goal**: Prove what Codex exposes locally for quota windows, token usage, and any ChatGPT-plan
renewal/billing date, and fix the confidence ceiling product may claim (expected: Estimated, deep/09 §Q1).
**User story**: As Maya (P1) with a ChatGPT-plan Codex account, I want Juggler to label my Codex
headroom `Estimate` and my renewal `unknown` unless OpenAI documents otherwise, because the
"63%-but-unusable" report (pass2/09 U-5) shows vendor percentages can mislead.
**Requirements**:
1. The investigation SHALL capture, on a live Codex install (record version): rollout
   `token_usage_record` lines, `codex exec --json` `turn.completed.usage`, and `/statusline` widgets
   `five-hour-limit`/`weekly-limit` (docs/research/codex.md:43,51,59), grading each per AJ-E601.
2. It SHALL test the OpenAI API-side billing surfaces (usage/costs endpoints, API-key scoped) and
   state precisely why they do or do not describe a ChatGPT-subscription Codex account; API-key
   billing SHALL NOT be presented as subscription renewal evidence.
3. It SHALL search for any sanctioned ChatGPT plan renewal/billing-date source (documented API/SDK,
   local CODEX_HOME state, CLI output); the scraping line in docs/research/codex.md:70-72 SHALL be applied and cited for every rejected source.
4. It SHALL record plan-dependence: Pro has no 5h limit (deep/09 §Q3 line 376) — captures SHALL name
   the plan tier tested and mark untested tiers `unverified`, per pass2/09 U-5 (n=1 community evidence is not proof).
5. Output SHALL be an evidence packet for AJ-E607 (ingest) and AJ-E610 (expiry UI) with a per-field confidence ceiling table.
**UI/UX reasoning**: No UI built. Recorded copy consequence: Codex quota always renders with the
`Estimate` badge and `≈` reset times (deep/09 §Q3 line 394); Nielsen/NN-g #1 — a styled-as-exact
Codex percentage is a protocol violation, flagged here so AJ-E610 cannot ship it.
**Performance budget**: Live capture ≤2h; `exec --json` fixture run ≤10 min; 0 runtime cost.
**Quality gates**: Each field graded under AJ-E601 with capture or doc citation; renewal negative
result lists ≥4 searched surfaces; zero scraping prototypes retained in the repo (search scripts deleted or stored as text descriptions only).
**Evidence/validation proof**: `docs/research/evidence/billing/codex-captures.md` (version, plan
tier, redacted rollout/`exec --json`/statusline excerpts, ceiling table). Pass threshold: a reviewer
can reproduce the `five-hour-limit` widget value from the stored capture, and the renewal section
reaches an explicit grade. Append summary to `docs/research/codex.md` (append only).
**Visual testing**: None. Optional `/status` terminal capture as evidence only, email/IDs redacted, default terminal size.
**Files to touch**: `docs/research/evidence/billing/codex-captures.md` (new),
`docs/research/codex.md` (append addendum); read-only: `docs/SPEC.md:198-204`, `docs/research/codex.md:43-72`.
**Research basis**: deep/09 §Q1 line 338 (Codex row, Estimated ceiling), §Q3 lines 374-377;
SPEC.md:203; docs/research/codex.md:51-52,70-72; pass2/09 U-5.
**Handover note**: Trap: OpenAI API costs endpoints describe API keys, not ChatGPT seats —
conflating them is the classic false-`Measured` bug. If a future OpenAI subscription API appears, it
still needs AJ-E601's two-capture rule before AJ-E610 may use it.


*Implementation mapping (recorded in this task's PR per the claiming protocol)*: evidence packet `docs/research/evidence/billing/codex-captures.md` — rollout token_usage_record verified (Desktop 0.162.0-alpha.17.2, two sessions ≥24 h) + the npm CLI 0.163.0 exec --json envelope (throwaway CODEX_HOME); the percentage feed is the undocumented rateLimits endpoint → prohibited (widgets are human surfaces); /v1/organization/costs is Admin-key org-scope API usage — never seat evidence; tier unverified recorded per U-5; ceiling: tokens Measured, plan % Estimate with ≈ resets, renewal unknown; addendum appended to codex.md §9.
### AJ-E604 ZCode (Z.ai / GLM) billing, credit & renewal-date investigation | Status: DONE-EVIDENCED (evidence: docs/qa/evidence/AJ-E604/manifest.md) | Priority P0 | Effort M | Depends: AJ-E601
**Goal**: Prove ZCode's locally-computable credit burn and test exhaustively for any sanctioned
pool-total or renewal/billing-date source, fixing what AJ-E607 may compute vs what must stay user-entered.
**User story**: As Alex (P6, Pro/Lite, deep/10 §0) on a tight budget, I need my GLM credit burn
exact to the published formula and my plan expiry honest, because a silently-wrong pool % makes me ration the wrong week.
**Requirements**:
1. The investigation SHALL verify, against a live ZCode install (record app version; binary at
   SPEC.md:208 path), that SQLite `model_usage` rows (read-only, WAL-safe per SPEC.md:209) reproduce
   per-session token counts, and SHALL document the exact query.
2. It SHALL recompute credits from the published formula — per-model multipliers, off-peak 50%
   windows (docs/research/zcode.md:14-16) — on a captured day and compare with any vendor-shown
   usage; discrepancy SHALL be reported numerically, not rounded away.
3. It SHALL test for a sanctioned remote pool total (5h/weekly) and any renewal/purchase-date source
   beyond the user-known purchase anchor (deep/09 §Q3 line 386); docs/research/zcode.md:77 ("no
   public quota API") SHALL be re-verified or refuted with dated evidence.
4. Plan-cap table (Lite 2k/10k, Pro 12k/60k, Max 28k/140k credits; deep/09 §Q2 line 368) SHALL be
   re-checked against current vendor docs with access dates; changed tiers update the packet, never silently.
5. SQLite-clone/HOME-isolation safety (pass2/09 U-8) SHALL be probed read-only; any probe needing writes to the live DB is `prohibited`.
**UI/UX reasoning**: No UI built. Copy consequence for AJ-E610: ZCode burn is `Computed`, pool %
stays `Estimate` until the user enters plan caps (deep/09 Q-3) — the investigation SHALL supply the
exact cap-entry wording inputs (tier names as vendor spells them).
**Performance budget**: Read-only SQLite capture ≤1h; formula recomputation script ≤60 lines, runs ≤10s on a 30-day fixture; 0 runtime cost.
**Quality gates**: Credit recomputation matches captured rows to ±0 credits on token-identical input
(formula arithmetic) and reports vendor-display delta separately; every source AJ-E601-graded; no
writes to a live ZCode DB during research (mtime before/after recorded).
**Evidence/validation proof**: `docs/research/evidence/billing/zcode-captures.md` with schema
excerpt, the recomputation script output on a redacted fixture day, tier-table verification, and
renewal-source grade. Pass = AJ-E607 can implement Computed burn from the packet alone. Append summary to `docs/research/zcode.md` (append only).
**Visual testing**: None. Vendor usage-screen capture, if used as comparison evidence, is stored redacted and labelled with app version + date.
**Files to touch**: `docs/research/evidence/billing/zcode-captures.md` (new),
`docs/research/zcode.md` (append addendum); read-only: `docs/SPEC.md:206-212`, `docs/research/zcode.md:14-16,72-77`.
**Research basis**: deep/09 §Q1 line 339, §Q2 Q-3, §Q3 lines 377-378,386; SPEC.md:210; pass2/09 U-8 (ZCode parity unproven until dogfood).
**Handover note**: Trap: "Computed exactly" (SPEC.md:210) covers burn, not the remote pool — keep
those grades separate in the packet or AJ-E610 will overstate. Peak window is weekdays 14:00–18:00
UTC+8 (zcode.md:16); fixtures must state timezone or credit maths is unverifiable.


*Implementation mapping (recorded in this task's PR per the claiming protocol)*: evidence packet `docs/research/evidence/billing/zcode-captures.md` + `zcode-recompute.py` (60-line read-only script; ±0 hand-check) — model_usage verified across two days; the published formula/tiers/off-peak re-verified dated 2026-10-15 (unchanged; NEW: MCP tool calls bill at output ×1.2 — script gap recorded); the bucket→column mapping stated as inferred (computed_total = input + output pins input as all-inclusive); clone-first probe with zero writes (the running app's own WAL explains mtime movement); pool totals Estimate-at-best vs user-entered caps; renewal negative; addendum appended to zcode.md §B8.
### AJ-E605 Gemini CLI billing, quota & renewal-date investigation | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E601
**Goal**: Create the missing Gemini research baseline: what Gemini CLI / Google AI plans expose
locally or via documented APIs for quota, usage, cost, and renewal dates, graded under AJ-E601
(expected starting state: unverified, pass2/09 U-4).
**User story**: As Maya (P1) considering a Gemini account as overflow, I want to know before I rely
on it whether Juggler can see its quota at all, so a switch target never shows invented headroom.
**Requirements**:
1. The investigation SHALL create `docs/research/gemini.md` in the house format of
   `docs/research/codex.md` (surfaces table, [V]/[U] markers, dated URLs), covering: installed CLI
   identity/version, local state/transcript locations, any usage/quota readouts, and plan tiers (AI Pro/Ultra/API-key).
2. It SHALL test documented Google surfaces only (Gemini CLI local output/files, documented Cloud/AI
   Studio usage APIs with their auth scope stated) and grade each under AJ-E601; undocumented endpoints are `prohibited` with citation.
3. It SHALL explicitly answer the D-12 question for Gemini: is there a sanctioned subscription
   renewal/billing-date source? Negative results SHALL list searched surfaces as in AJ-E602 Requirement 2.
4. It SHALL state the confidence ceiling per field and the fallback (transcript-sum Estimated, or
   `unknown`) that AJ-E607 must implement if no feed verifies.
5. It SHALL NOT assume parity with Claude/Codex (pass2/09 U-4): every parity-shaped claim without a capture is marked `[U]`/inferred.
**UI/UX reasoning**: No UI built. Consequence recorded for AJ-E610/E611: until a Gemini source
verifies, Gemini accounts render quota as unavailable-with-reason (`No verified quota source for
this Provider yet`) per D-06, and ranking (AJ-E611) treats headroom as unknown, never zero or full.
**Performance budget**: Research + one live CLI capture ≤3h total; 0 runtime cost. If no Gemini
install is available, the task completes as a docs-only packet with every live item marked
`unverified — no install`, which is a passing outcome, not a failure.
**Quality gates**: `docs/research/gemini.md` passes the same [V]/[U] discipline as sibling files
(spot-check: every [V] has a URL or capture); renewal answer is explicit (found-graded or absent-with-search-log); zero product code changed.
**Evidence/validation proof**: `docs/research/gemini.md` +
`docs/research/evidence/billing/gemini-captures.md` (or an explicit no-install statement with the
doc-based grades). Pass threshold: AJ-E607's Gemini row can be written from the packet without new
assumptions. Reviewer reproduces one graded claim from the cited doc/capture.
**Visual testing**: None. Any CLI readout capture is evidence-only, redacted, version-labelled.
**Files to touch**: `docs/research/gemini.md` (new), `docs/research/evidence/billing/gemini-captures.md` (new); read-only:
`docs/research/landscape-and-stack.md:11-21` (Gemini switcher context), `docs/review/pass2/09-critic-crosschecks.md:167-174`.
**Research basis**: decision-log.md:119-121 (D-12 names "Gemini and successors"); pass2/09 U-4
(Gemini quota parity unverified); deep/09 §Q1 table (Gemini absent — the gap this task closes).
**Handover note**: Do not import cc-switch's Gemini support (landscape-and-stack.md:12) as evidence
of a quota API — account switching ≠ usage telemetry. Successor Providers reuse this file's
structure via AJ-E601; add a section, don't fork the protocol.


**Status report (2026-10-16)**: DONE-EVIDENCED — evidence
`docs/qa/evidence/AJ-E605/manifest.md`; packet
`docs/research/evidence/billing/gemini-captures.md`; baseline
`docs/research/gemini.md`. Headline verified fact: Gemini CLI was
SUNSET for individual users 2026-06-18 (moved to closed-source
Antigravity CLI) — the open-source CLI serves API-key/Vertex paths
only. No install existed locally: docs-only packet with live items
`unverified — no install` (the budget's documented passing outcome).
D-12 renewal answer: ABSENT (5-surface negative log) → user-entered.
Gemini quota renders unavailable-with-reason (D-06); headroom unknown.

**Implementation-mapping**: `docs/research/gemini.md` (new, 8
sections incl. §7 Antigravity successor),
`docs/research/evidence/billing/gemini-captures.md` (new),
`docs/research/evidence/billing/README.md` (row). Zero product code.

### AJ-E606 Two-clock quota engine: window maths + subscription clock domain model | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E602, AJ-E603, AJ-E604
**Goal**: Implement the pure domain core that keeps the window clock (rolling 5h/weekly pools) and
the subscription clock (renewal/expiry per D-12) as separate, testable models over one append-only
event store, replacing today's empty `UsageSnapshot::default()` inserts (crates/juggler-app/src/commands.rs:706,1182).
**User story**: As Maya (P1) at 13:00, I see "5h window resets 17:00 · subscription renews in 6
days" as two unrelated facts, because conflating them would make me switch accounts when I actually
need to renew, or renew when I only need to wait.
**Requirements**:
1. The engine SHALL store append-only `usage_events` and `quota_samples` exactly as deep/09 §Q8
   (lines 494-510), including `UNIQUE(account, msg_uuid)` dedupe, `source ∈
   statusline|transcript|rollout|sqlite|api_retry`, and per-sample `confidence`; migration SHALL be
   additive (next V-number after current storage migrations) and SHALL NOT alter existing session rows' semantics.
2. Window maths SHALL follow deep/09 §Q3 verbatim: windows are event sets `(t−W, t]`, never buckets;
   Measured % trusts vendor `used_percentage` verbatim with clamped [0,100] slope interpolation;
   Computed/Estimated % is `100 × window_sum / cap` only when a user-confirmed cap exists, otherwise
   the engine SHALL expose absolute burn and no percentage.
3. The subscription clock SHALL be a separate type (e.g. `SubscriptionClock { renewal_date:
   Option<Date>, entered_by_user: bool, source_grade }`) that no window computation reads or writes;
   with no user-entered date it SHALL evaluate to `Unknown` (D-12), and no code path SHALL infer it
   from window resets, first-seen dates, or plan tier.
4. Account quota state SHALL implement the §Q8 machine `Unknown → Learning → Tracking ⇄ Stale(>10m,
   Measured only) → Exhausted → (reset) → Tracking`; `Unknown`/`Learning` SHALL carry no numeric percentage (never 0%).
5. All maths SHALL be pure functions over `usage_events` (deep/09 line 517-518) with injected clock,
   so property tests need no harness, DB, or wall time.
6. `UsageSnapshot` (model.rs:301-309) SHALL be populated from the engine at its two current
   default-insert sites, or those sites SHALL be migrated to an engine query — no third representation of quota state may be introduced.
**UI/UX reasoning**: Nielsen/NN-g #1 + D-06 honest fields: the two clocks get visually distinct
treatments specified in AJ-E610 (countdown chip vs calendar date), and this task's API SHALL expose
them as separate fields so the UI cannot merge them into one "time left" number. Empty states are
typed (`Unknown`/`Learning`), not `0%` (deep/09 line 516-517, prior F2.4).
**Performance budget**: Window fold ≤5 ms for 100k events/account (release build, M-class laptop);
engine tick (all accounts) ≤20 ms; storage growth ≤5 MB/day/account at ccusage-scale local
aggregation (deep/09 lines 347-348); no polling loop in this task (ingest is AJ-E607).
**Quality gates**: `cargo test -p juggler-core` green including new property tests; `cargo clippy
--workspace -- -D warnings` clean (workspace lints, Cargo.toml); boundary rule `(t−W, t]` proven
identical on 5h and weekly windows; clock-skew ±5 min never yields negative headroom (deep/09 lines 396-401).
**Evidence/validation proof**: Property-test suite (proptest, matching
`crates/juggler-core/tests/storage_roundtrip.rs` conventions) covering: monotone window sum under
insertion, boundary inclusion, dedupe-by-uuid (resumed-file double-count fixture from
claude_transcript.rs:167-169 contract), unknown-cap ⇒ no percentage, subscription-clock isolation
(fuzz window events, assert renewal value unchanged). Pass = all properties hold at ≥256 cases each;
store test output + `wc -l` of new modules in `docs/qa/gate-M3.md` evidence section (create if absent, per EPICS.md:95 M3-6).
**Visual testing**: None (domain core). AJ-E610 owns all screenshots; this task's review SHALL
include a terminal dump of engine state for the fixture timeline (Unknown→Learning→Tracking→Exhausted→Tracking) as text evidence.
**Files to touch**: `crates/juggler-core/src/model.rs` (extend near 277-309), new
`crates/juggler-core/src/quota.rs` (window/forecast-free maths + SubscriptionClock),
`crates/juggler-core/src/storage/mod.rs` + new `crates/juggler-core/src/storage/usage.rs` (migration
+ event store), `crates/juggler-app/src/commands.rs:706,1182` (populate snapshot), `crates/juggler-core/tests/quota_windows.rs` (new).
**Research basis**: deep/09 §Q3 lines 372-401, §Q8 lines 490-518; SPEC.md:254-259 (§6 usage engine);
model.rs:261-309 (TokenCounts/QuotaWindow/Confidence/UsageSnapshot); EPICS.md:91 (M3-2).
**Handover note**: Trap: bucketed hourly aggregation looks simpler and silently breaks rolling
windows at boundaries — the property tests exist to catch exactly that. The subscription clock is
deliberately boring (a date + provenance); resist "smart" inference — D-12 settled that. Forecast lives in AJ-E608, not here.


**Status report (2026-10-16)**: DONE-EVIDENCED — evidence
`docs/qa/evidence/AJ-E606/manifest.md`; gate section
`docs/qa/gate-M3.md` (test output + module sizes + §Q8 timeline
dump). Schema V18 additive; §Q3 boundary/monotone/cap/skew properties
×256 green; D-12 isolation property green; dedupe contract pinned on
the real store; the three UsageSnapshot insert sites migrated to
`engine_usage_snapshot` (single representation).

**Implementation-mapping**: `crates/juggler-core/src/quota.rs` (new),
`crates/juggler-core/src/storage/usage.rs` (new) + storage/mod.rs
(V18, UsageRepo, settings keys), `crates/juggler-core/tests/quota_windows.rs`
(new), `crates/juggler-app/src/commands.rs` (engine_usage_snapshot at
launch_session / desktop_launch_arc / session_handover_impl),
`docs/qa/gate-M3.md` (new).

### AJ-E607 Ingest pipeline: statusline spool, transcript, rollout & SQLite sources | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E606, AJ-E602, AJ-E603, AJ-E604, AJ-E605
**Goal**: Feed AJ-E606 from every sanctioned local source — Claude statusline spool + transcript
usage, Codex rollout/`exec --json`, ZCode SQLite `model_usage`, Gemini per AJ-E605's packet — with
dedupe and per-source confidence, and no undocumented endpoint ever contacted.
**User story**: As Devon (P2, 10–20 parallel sessions), I need quota numbers that update while I
work without me running `/usage` in five terminals, and I need my existing custom statusline to keep working exactly as before.
**Requirements**:
1. Claude statusline capture SHALL install a managed pass-through script per profile dir
   (SPEC.md:192) that appends stdin JSON to an append-only spool and chains any pre-existing user
   statusline byte-for-byte in behaviour; uninstall SHALL restore the prior statusline exactly (fixture-proven, deep/09 §Q9 line 523-524).
2. Each source adapter SHALL emit `UsageEvent`s (AJ-E606 schema) tagged with its AJ-E601 grade:
   Claude statusline ⇒ Measured (fresh <10 min only, §Q2 Q-1), Claude transcript ⇒ Measured tokens,
   Codex rollout ⇒ Measured tokens / Estimated windows, ZCode SQLite ⇒ Computed credits, Gemini ⇒
   per AJ-E605 ceiling (or no emitter if unverified).
3. Ingest SHALL dedupe by message `uuid` before summing (claude_transcript.rs:167-169 contract) and
   SHALL be idempotent: re-tailing the same spool/transcript segment inserts zero new events.
4. The pipeline SHALL be single-writer per account (deep/09 line 512) and passive: it SHALL NOT
   launch, signal, or write into any Provider process or DB (ZCode SQLite opened read-only; WAL-safe per SPEC.md:209).
5. Source health SHALL be observable: each source exposes `last_event_at` + parse-error count; a
   source silent >10 min demotes its Measured samples to Stale in AJ-E606 terms, never deletes them.
6. Spool files SHALL rotate at 10 MB and live only under the managed dir; no quota data leaves the
   machine (fleet carries telemetry, never credentials — deep/10 line 20 invariants).
**UI/UX reasoning**: Nielsen/NN-g #1/#9: ingest failures surface as the AJ-E610 `Feed stale 22m —
Retry` banner pattern (deep/10 line 475), never as frozen green rings; this task SHALL emit the
typed stale/error state the banner binds to. Statusline chaining follows NN-g #3 User control — the
user's own statusline is never silently replaced.
**Performance budget**: Spool tail latency event→engine ≤2s; ingest CPU ≤1% sustained with 20 active
sessions; transcript re-scan incremental (offset-tracked), full-file reparse only on truncation/rotation; spool replay of 24h data ≤3s.
**Quality gates**: Fixture corpus per source (captured shapes from AJ-E602–E605 packets,
secrets-redacted) parses with 0 dropped valid lines; chaining fixture (custom statusline printing
marker text) passes; `tests/guardrails.rs`-style assertion: no ingest code path contains an HTTP
call to a Provider host (grep + code review checklist).
**Evidence/validation proof**: (a) Unit/integration tests per source fixture, (b) live-dogfood: one
real Claude session ≥20 min where spool-derived 5h % matches `/usage` within ±5% (SPEC.md:453 M3
gate), logged with timestamps in `docs/qa/gate-M3.md`; (c) idempotency proof: run ingest twice over
the same fixture dir, `SELECT COUNT(*)` identical. Pass thresholds as stated; evidence stored in
`docs/qa/gate-M3.md` + fixture dir `crates/juggler-adapters/tests/fixtures/quota/`.
**Visual testing**: No product UI. Capture one terminal screenshot of the chained statusline (user
marker + Juggler capture active simultaneously) at default size as dogfood evidence in
`docs/qa/gate-M3.md`; pass = both outputs visible, no duplication.
**Files to touch**: new `crates/juggler-adapters/src/quota_ingest.rs` (spool tail + source
adapters), `crates/juggler-adapters/src/claude_transcript.rs:140-169` (reuse
`extract_usage`/dedupe), `crates/juggler-adapters/src/claude_code.rs` (statusline install/uninstall
near login-state code ~332-354), `crates/juggler-core/src/storage/usage.rs` (from AJ-E606),
`crates/juggler-app/src/state.rs` (pipeline task ownership), fixtures under `crates/juggler-adapters/tests/fixtures/quota/`.
**Research basis**: deep/09 §Q1 lines 334-352, §Q8 lines 512-518; SPEC.md:190-194,202-210;
docs/research/claude-code.md:54-55 (ccusage local-JSONL proof); EPICS.md:90-93 (M3-1/M3-3/M3-4).
**Handover note**: Build order is law (deep/09 line 530-532): spool+ingest first, each Provider
emitter ships value alone. Trap: ZCode pool % is not in the SQLite DB — credits are; do not let a
"helpful" join invent the pool. Gemini emitter stays absent until AJ-E605 verifies a source; an
absent emitter with honest `unknown` beats a speculative parser.


**Status report (2026-10-16)**: DONE-EVIDENCED — evidence
`docs/qa/evidence/AJ-E607/manifest.md`; gate section `docs/qa/gate-M3.md`
§AJ-E607. Fixture corpus (5 sources' captured shapes) 13/13 +
pipeline integration 1/1 green; chaining byte-for-byte; idempotency
proven at store and pipeline level; no-HTTP grep gate test-pinned.
The live ±5% dogfood + chained-statusline screenshot are PENDING the
maintainer's authenticated Claude login (the gate-E5 open item) —
recorded in gate-M3, rides the next authenticated session.

**Implementation-mapping**: `quota_ingest.rs` (adapters, new),
`usage_pipeline.rs` (app, new) + `usage_source_health` command,
fixtures `tests/fixtures/quota/` + `quota_ingest.rs` (13) +
`usage_pipeline.rs` (1) tests, `gate-M3.md` §AJ-E607.

### AJ-E608 Burn-rate forecast: EWMA, time-to-exhaust & confidence inheritance | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E606, AJ-E607
**Goal**: Turn AJ-E606 window state into honest burn forecasts — EWMA burn-rate, time-to-exhaust
(TTE), and launch-gate projections — that widen or disappear when the data cannot support them, per deep/09 §Q4.
**User story**: As Maya (P1) starting a 3-hour refactor, I want "A exhausts in ~40 min" before I
launch (deep/10 §1 story), with a range when it's an estimate, so I route to B once instead of hitting the wall mid-task.
**Requirements**:
1. Burn-rate SHALL be EWMA over 5/15/60-min token (or credit) rates per account; the headline rate
   is 15-min and the cone is the 5↔60 spread (deep/09 §Q4 lines 405-407). A single-rate implementation is non-compliant.
2. Cold start SHALL be enforced: with <10 min of data the forecast API returns `Learning` and no number, at any confidence (deep/09 line 407).
3. TTE SHALL be `headroom / rate15`, rendered only when TTE < window remaining, else the state is
   `wont-exhaust` ("won't exhaust this window at current pace", lines 408-410) — the engine SHALL NOT emit a TTE beyond the reset.
4. Forecast confidence SHALL inherit the lowest confidence of its inputs (§Q2 Q-2); Estimated
   forecasts SHALL be ranges (e.g. "40–70 min"), Measured forecasts point + range-on-hover data (lines 410-412).
5. The launch/handover gate SHALL project the intended session's cost as the median historical
   session burn for this project vs headroom, returning exactly `OK / Tight / No` in the admission
   advisor's verdict language (SPEC.md:237); `No` warns and SHALL never block launch (deep/09 lines 413-416).
6. Calibration SHALL be recorded silently: when realised exhaustion deviates >2× from forecast, a
   calibration note is logged (never an interruption, §Q5 line 435) and exposed to AJ-E611 as a trust input.
**UI/UX reasoning**: Nielsen/NN-g #1 + deep/10 §1 controls: forecast pill states are `learning`
(copy exactly "learning — no forecast yet"), `tracking`, `wont-exhaust`, `exhausts ~HH:MM`;
Estimated pills always show the range, never a bare point. Apple HIG (clarity): the pill never uses
colour alone — arrow + text (`↑ fast`, `→ steady`) paired with the number.
**Performance budget**: Forecast recompute ≤2 ms/account on each ingest batch; no timer shorter than
30s for background recompute; memory ≤1 MB of rate state per account; fixture-timeline evaluation of 30 days of events ≤500 ms.
**Quality gates**: Fixture timelines (synthetic constant/accelerating/stalled burn) reproduce known
TTE within ±10%; cold-start test asserts no numeric output before 10 min; confidence-inheritance
test: one Estimated input ⇒ range output, asserted in types or tests, not just UI.
**Evidence/validation proof**: (a) Fixture-timeline test suite in
`crates/juggler-core/tests/quota_forecast.rs` with MAPE ≤30% after 20 min of burn on the
accelerating fixture (deep/10 §1 acceptance, line 31); (b) live-dogfood: ≥5 real sessions where
forecast-at-20-min vs actual exhaustion is logged in `docs/qa/gate-M3.md`, pass = median error ≤30%,
every miss >2× has a calibration note. Store fixture generator + expected outputs alongside tests so
the oracle is independent of the implementation.
**Visual testing**: Screenshots owned by AJ-E610; this task SHALL supply the four pill states as
fixture data and review AJ-E610 captures against them (checklist in Evidence: each state present, ranges on Estimated, no number in `learning`).
**Files to touch**: new `crates/juggler-core/src/forecast.rs`, `crates/juggler-core/src/quota.rs`
(AJ-E606, consume window API only), `crates/juggler-core/tests/quota_forecast.rs` (new),
`crates/juggler-app/src/commands.rs` (expose forecast query for UI), read-only: `docs/SPEC.md:237,256-258`.
**Research basis**: deep/09 §Q4 lines 403-421, §Q2 Q-2; deep/10 §1 lines 28-31 (UX flow + MAPE
acceptance); SPEC.md:257 (burn-down projections); EPICS.md:91 (M3-2 projections within stated error bounds).
**Handover note**: Trap: a forecast that always shows a number trains users to ignore it by the
third miss — `Learning` and `wont-exhaust` are features, not missing polish. ZCode forecasts run on
credits, Codex on Estimated windows; the cone math is shared, the units and badges are not interchangeable.


**Status report (2026-10-16)**: DONE-EVIDENCED — evidence
`docs/qa/evidence/AJ-E608/manifest.md`; gate `docs/qa/gate-M3.md`
§AJ-E608. 13/13 fixture-timeline tests (constant ±10%, accelerating
MAPE ≤30%, cold-start no-number, reset-bounded TTE, confidence
inheritance, credits units). Numeric trap found+fixed: empty float
`.sum()` yields −0.0 on this toolchain — signed-zero normalisation +
finite guard, documented in-code.

**Implementation-mapping**: `quota_forecast.rs` (new) +
`tests/quota_forecast.rs` (new).

### AJ-E609 Cost ledger: USD pricing, per-model attribution & subscription value | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E606, AJ-E607
**Goal**: Price every usage event per model into an auditable USD ledger — versioned pricing,
per-session/account/project/day rollups, CSV export, and the dual subscription view ("≈$X at API
prices · you pay flat") — without ever labelling USD as Measured (deep/09 §Q7, §Q2 Q-4).
**User story**: As Priya (P3, agency, deep/10 §0/§6), I export a per-client CSV (account × project ×
USD, tier-labelled) in one click for invoicing, and as Maya (P1) I can answer "did this Max subscription earn its $200?" from the same ledger.
**Requirements**:
1. Pricing SHALL be a versioned table `pricing(model, effective_from, in, out, cache_r, cache_w,
   source_url)` (deep/09 lines 473-476), bundled with the app, user-overridable per model, with
   `last synced` metadata; an unpriced model yields USD `—`, never `$0.00` (deep/10 line 74).
2. Ledger entries SHALL price the AJ-E606 `UsageEvent` at event-time rates; USD SHALL be joined at
   query time, never baked into `usage_events`, so repricing is retroactive (deep/10 line 75). ZCode
   credits SHALL display alongside USD and SHALL NOT be converted at an invented rate (deep/09 lines 487-488).
3. Rollups SHALL exist per session, account, project, and day, each carrying: tokens by class
   (input/output/cache_r/cache_w/reasoning, model.rs:263-275), USD estimate, and the token
   confidence label; USD itself is always labelled `Estimate` except vendor `cost.total_cost_usd`
   passthrough, labelled `vendor-reported` (lines 480-482).
4. Subscription accounts SHALL show both views — API-equivalent value and the user-entered flat fee
   (fee entry is part of AJ-E610's account form) — computing "API-equivalent ÷ fee" only when a fee exists; no fee ⇒ no ratio, no guessed fee.
5. Per-model attribution SHALL survive model switches mid-session: rollups group by `(session,
   model)`, and a session total is the sum of its model rows, reconciling to the cent with the CSV export (deep/10 line 76).
6. CSV export SHALL stream (no full-table in-memory materialisation) and reuse the exact rollup query the UI displays.
**UI/UX reasoning**: deep/10 §6 controls: badge copy exactly "Estimated — list pricing, not your
subscription fee" (permanent, not a tooltip); states `priced / unpriced-model / stale-pricing (>30d)
/ budget ok|warn|over`. Nielsen/NN-g #5 Error prevention: the pricing editor shows `source_url` +
effective date before save, and stale pricing warns inline rather than silently repricing history.
Per-client export is labelled with project→client tags; untagged projects export under their project name, never a fabricated client.
**Performance budget**: Rollup query ≤50 ms over 1M events (indexed per §Q8 `idx_usage_acct_ts`);
CSV export of 1M rows streams at ≤50 MB peak memory; pricing-table sync/edit applies in ≤100 ms and
reprices on next query (no background rewrite job).
**Quality gates**: Fixture corpus totals match ccusage on the same Claude JSONL corpus to the cent
(deep/10 line 76; ccusage pattern per docs/research/claude-code.md:55); export-vs-screen
reconciliation test; grep gate: no UI string in this feature contains "you paid $" applied to API-equivalent value.
**Evidence/validation proof**: (a) ccusage cross-check log on a shared fixture corpus stored in
`docs/qa/gate-M3.md` (corpus hash, both totals, delta = $0.00); (b) repricing test: change one
model's rate, assert historical rollup changes and `usage_events` rows are byte-identical; (c)
dogfood: Priya-flow export for one real week reconciles to screen totals. Evidence in
`docs/qa/gate-M3.md` + `crates/juggler-core/tests/cost_ledger.rs`.
**Visual testing**: Capture Usage-tab cost matrix in `priced`, `unpriced-model`, and `stale-pricing`
states at 1440×900 and 1280×800 (light+dark if themes exist); pass = badge text visible without
hover, `—` (not `$0.00`) for unpriced, dual subscription view shows both numbers with distinct
labels. A11y: table headers associated (`th scope`), totals row announced, contrast ≥4.5:1 (WCAG 2.2 AA 1.4.3).
**Files to touch**: new `crates/juggler-core/src/pricing.rs` + `crates/juggler-core/src/storage/usage.rs` (rollup queries, AJ-E606 file),
`crates/juggler-app/src/commands.rs` (rollup + CSV export commands), `ui/src/screens/WorkPanels.tsx`
(Usage cost matrix; confirm current panel home at build time),
`crates/juggler-core/tests/cost_ledger.rs` (new), bundled pricing seed (new file beside
`pricing.rs`, name per repo convention at implementation).
**Research basis**: deep/09 §Q7 lines 471-488, §Q2 Q-4; deep/10 §6 lines 69-76 (PRD, "label
discipline is the feature"); docs/research/claude-code.md:55 (ccusage); pass2/09 line 215 (pricing
table missing = F1.2, cost stays Est/hidden until this lands).
**Handover note**: Deep/10 line 75 and deep/09 §Q7 differ slightly (query-time join vs event-time
priced entry) — this task follows query-time join with event-time rate selection (`effective_from ≤
ts`), which satisfies both; record the reading in the PR. Trap: never sum ZCode credits into USD totals in CSV either — separate column, always.


**Status report (2026-10-16)**: DONE-EVIDENCED — evidence
`docs/qa/evidence/AJ-E609/manifest.md`; gate `docs/qa/gate-M3.md`
§AJ-E609. ccusage-pattern oracle cross-check delta $0.00 (corpus
sha256 recorded); repricing retroactive with byte-identical stored
rows; CSV↔rollup reconciliation to the cent; 24 visual captures
(25-costs) + 5 vitest incl. the "you paid $" grep gate. The live-week
export dogfood is PENDING the authenticated session (recorded).
Query-time join with event-time rate selection — the deep/10-75 vs
deep/09-§Q7 reading — recorded here and in the PR.

**Implementation-mapping**: `cost_ledger.rs` + seed (new), usage.rs
repo + V19, 4 commands, `CostMatrix.tsx` + App mount + bridge seams,
`tests/cost_ledger.rs` (12) + `ccusage-oracle.py`, gallery 25-costs.

### AJ-E610 Quota & expiry display: confidence labels, two-clock indicators, manual renewal entry (D-12) | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E606, AJ-E607, AJ-E608, AJ-E609, AJ-E602, AJ-E603, AJ-E604, AJ-E605
**Goal**: Render the two clocks honestly everywhere quota appears — rail/account indicators
(I-05/I-06), Right Inspector Usage tab (D-06), NewSession rows, and tray — with confidence badges,
typed empty/stale states, and the D-12 user-entered renewal date with approaching-expiry warning.
**User story**: As Maya (P1) glancing at the rail at 08:30, I read "A 82% · resets 17:00 · Measured"
and "renews in 6 days" in one look, and as Alex (P6) with no date entered I see "Renewal: unknown — add date", never a fabricated countdown.
**Requirements**:
1. Every rendered quota/cost number SHALL carry its badge text exactly `Measured` / `Computed` /
   `Estimate` (deep/09 Q-4), adjacent to the number (not hover-only); stale Measured (>10 min) SHALL
   render "last measured Nh ago" and demote visually (Q-1). USD additionally carries AJ-E609's fee disclaimer.
2. The window clock SHALL render as ring + `%` text (never colour-only, deep/10 line 28) + reset
   countdown; the subscription clock SHALL render as a calendar-style date line (`Renews 3 Oct` /
   `Expires 3 Oct` / `Renewal: unknown — add date`) — the two SHALL NOT share a merged "time left" element (AJ-E606 Requirement 3).
3. Renewal entry SHALL be an optional date field per Account (account card + Accounts screen,
   `ui/src/screens/Profiles.tsx` until the D-02 rename lands) with states: `none` (unknown copy
   above), `set`, `approaching` (≤7 days, user-tunable 1–30), `overdue`; saving/clearing requires
   explicit user action and the stored provenance is `entered_by_user` (AJ-E606 SubscriptionClock).
   A graded source from AJ-E602–E605 MAY pre-fill only as an unconfirmed suggestion the user accepts
   or edits — auto-population without confirmation is prohibited until a separate decision supersedes D-12.
4. Typed states SHALL ship for every surface: `no-data` ("Start a session to measure"), `learning`,
   `measured`, `estimated-stale`, `blocked` (+ reset CTA), `unavailable-with-reason` (e.g. Gemini
   pre-verification, AJ-E605) — matching deep/10 lines 29,475. Rendering `percent={0}` for unknown (prior F2.4 bug pattern) is a failing defect.
5. Pro SHALL render the weekly window, not Max-only assumptions (deep/10 line 31); plan tiers
   without a 5h window (Codex Pro, AJ-E603) SHALL omit that ring with reason text, not an empty ring.
6. Keyboard/screen-reader access: rings are `role=img` with text equivalent ("82% of 5-hour window
   used, resets 17:00, Measured"); all actions reachable by keyboard; shortcuts follow the host
   app's existing scheme (no new global chords in this task).
**UI/UX reasoning**: Nielsen/NN-g #1 Visibility of system status + #4 Consistency; WCAG 2.2 AA 1.4.1
Use of Color (ring always paired with % text), 1.4.3 Contrast ≥4.5:1 for badge text; Apple HIG
clarity/deference — glanceable rail, detail in Inspector Usage tab (pass2/04 §§5–6 canonical;
pass2/09 C-6: import 04 §6 refusals, do not re-promise quality scores). Expiry warning is quiet
(row/banner, deep/09 §Q5 digest discipline), never a modal; copy for approaching: "Renews in 6 days — check your plan" only when a date exists.
**Performance budget**: Indicator re-render ≤16 ms on engine update; surfaces update from pushed
aggregates, poll interval ≥30s fallback (never webview-polled per deep/10 line 192 pattern); 20
accounts × 2 windows render at 60fps scroll; tray aggregate query ≤10 ms.
**Quality gates**: UI test matrix: every state in Requirement 4 rendered by fixture data with
snapshot assertions on copy; grep gate over `ui/src` for quota percentages rendered without an
adjacent confidence badge (allowlist only the test fixtures); D-12 test: fresh account with no date
shows unknown copy and zero inferred dates in storage.
**Evidence/validation proof**: (a) Fixture-driven UI tests (state matrix) green in the repo's UI
test command; (b) live-dogfood checklist in `docs/qa/gate-M3.md`: Claude ±5% vs `/usage` over a full
window on 2 live accounts (EPICS.md:95 gate), statusline chaining preserved, Pro weekly visible; (c)
manual-entry dogfood: enter/clear a renewal date, restart app, value + provenance persist. Pass
thresholds as stated; failures block M3-6 gate sign-off.
**Visual testing**: Capture at 1440×900 and 1280×800, light+dark: rail indicators (Measured fresh,
Estimated, stale, no-data), Inspector Usage tab (two clocks separated), account card renewal states
(`none/set/approaching/overdue`), NewSession row with forecast pill, tray menu. Comparison:
side-by-side against fixture-state checklist (pixel-diff optional; state/copy conformance
mandatory). A11y checks: axe or equivalent 0 critical violations, colour-only audit (grayscale
screenshot still fully readable), focus order walkthrough recorded.
**Files to touch**: `ui/src/components/WorkflowRail.tsx` (indicators),
`ui/src/screens/WorkPanels.tsx` (Inspector Usage tab; confirm at build time),
`ui/src/screens/Profiles.tsx` (renewal/fee entry; D-02 rename may move this),
`ui/src/screens/NewSession.tsx` (quota rows), `ui/src/types/generated/` (regenerate from core types
via existing codegen — never hand-edit), `crates/juggler-app/src/commands.rs` (quota/expiry
queries), `crates/juggler-core/src/storage/accounts.rs` (renewal fields on Account).
**Research basis**: pass2/04 §§3–6 (I-05/I-06, Usage tab; via pass2/09 line 215 canonical-home
table); deep/09 §Q2 Q-1–Q-4, §Q5; deep/10 §1 lines 28-31, §19 tray lines 188-192; decision-log.md:113-124 (D-12); SPEC.md:345-352.
**Handover note**: D-02 rename (Provider/Account/Instance) may land before this task — re-check
screen file names at pickup; the copy strings above are settled even if files move. Trap: the
approaching-expiry threshold belongs to the subscription clock only; never fire renewal warnings
from window-reset data, and never fire window alerts from the renewal date (AJ-E611 consumes each separately).


**Status report (2026-10-16)**: DONE-EVIDENCED — evidence
`docs/qa/evidence/AJ-E610/manifest.md`; gate `docs/qa/gate-M3.md`
§AJ-E610. Backend 5/5 + UI matrix 16/16 + visual 262/262 (16 new
26-quota captures). Tray recorded N/A (no tray surface exists yet);
live dogfoods PENDING the authenticated session (gate-E5 blocker).

**Implementation-mapping**: quota_display(+_all)/account_set_fee
commands, `QuotaRings.tsx` (+test), RightInspector Usage-tab/account
card/FeeEditor, NewSession QuotaChip, `tests/quota_display.rs`,
gallery 26-quota.

### AJ-E611 Smart destination ranking inputs & quota alerts / reset-queue integration | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E608, AJ-E609, AJ-E610
**Goal**: Wire AJ-E606–E609 outputs into (a) the explainable destination ranking used at
NewSession/handover/reset-return and (b) the anti-fatigue alert + reset-queue policy, so quota
becomes the ranking/alert input D-01 intends — suggest-only, never auto-switching.
**User story**: As Maya (P1) at the 13:00 wall (deep/10 §28.1), I get one quiet notification — "A
exhausted · B has 80% headroom, usual for this repo [Hand over] [Queue at reset ≈17:00]" — and I
click; nothing ever switches, launches, or resumes without my click.
**Requirements**:
1. Ranking SHALL apply hard filters before scoring (deep/09 §Q6 lines 449-452): wrong Provider for a
   restricted request, logged-out (claude_code.rs login-state), quota-exhausted with no reset before
   need-by — filtered accounts are shown greyed with the filter reason, not silently dropped.
2. Survivors SHALL be scored `0.45·headroom + 0.20·affinity + 0.15·fidelity + 0.10·costFit +
   0.10·calm` (line 454) with inputs defined at lines 456-464: headroom confidence-discounted
   (Estimated ×0.85), affinity from project history (model.rs ProjectHistory accounts), fidelity
   from the switch fidelity matrix, costFit from AJ-E609 projected USD, calm from active-session
   load. Weights SHALL be constants in one core function, unit-tested, not tuned per call site.
3. Every ranked row SHALL show its top-2 contributing factors in words + confidence badge + TTE
   (lines 466-469), with a "Why this order?" expander listing all five inputs; a row without an
   explanation SHALL NOT render. Unknown headroom (AJ-E605 unverified Providers) ranks by remaining
   inputs and says so ("headroom unknown — ranked without quota").
4. Alerts SHALL implement the §Q5 table verbatim (lines 429-435): TTE<45m tray/ring once per window;
   TTE<15m or ≥90% actionable notification once (snooze 30m); exhausted once per exhaustion;
   reset-reached once; >2× forecast error silent-log only. The per-(account,window) FSM `Armed →
   Warned → Critical → Exhausted → (reset) → Armed` (lines 440-443) SHALL guarantee at-most-once
   semantics; non-actionable crossings batch into an hourly digest with per-project mute + quiet hours.
5. Subscription-clock alerts are separate: renewal `approaching` (AJ-E610 state) MAY join the hourly
   digest; it SHALL NOT trigger the window-alert FSM, a handover suggestion, or any "switch now" copy.
6. Reset-queue SHALL implement deep/09 §Q4 lines 417-421 + deep/10 §9: `reset_queue` (§Q8 schema),
   passive timer + notification at reset, resume only from the notification/row click (fake-clock
   guardrail test: no launch ever fires unclicked), states `queued/ready/resumed/dismissed` (+`stale` when lineage breaks, explained).
7. No code path in this task SHALL execute a switch, launch, or resume without an explicit click;
   this SHALL be asserted by a guardrail test in the style of SPEC.md:263's invariant.
**UI/UX reasoning**: Nielsen/NN-g #3 User control + #6 Recognition over recall: ranked rows explain
themselves in place; the alert actions are verbs on the notification itself ([Hand over] [Queue at
reset] [Snooze 30m]). Copy discipline: Estimated alerts read differently from Measured ("≈90%
(estimate)") per §Q5 line 438-440. NN-g notification guidance (batch, digest, quiet hours) is
implemented as Requirement 4, not as settings-page promises. Ranking surfaces: NewSession pre-sorted
with top row badged "Suggested" (deep/10 §3), handover wizard reuses the same rows — one component, two hosts.
**Performance budget**: Ranking compute ≤10 ms for 20 accounts on demand (never on a timer); alert
FSM evaluation ≤1 ms per engine tick; digest assembly ≤50 ms hourly; reset-queue timer one wakeup per queued item (no polling loop <60s).
**Quality gates**: Ranking unit tests: hard-filter cases, Estimated-discount tie-break (Measured
wins ties), explanation completeness (every row ≥2 factors); alert FSM property test: random trigger
sequences never emit two notifications for one crossing; guardrail test (Requirement 7) green;
ranking never mutates session state (assert via storage diff in test).
**Evidence/validation proof**: (a) Test suite in `crates/juggler-core/tests/ranking_alerts.rs` +
guardrail test, all green; (b) fixture-day simulation: scripted timeline (burn → 90% → exhausted →
reset) produces exactly the §Q5 notification sequence, logged and diffed against expected in
`docs/qa/gate-M3.md`; (c) live-dogfood: one real quota-wall event handled end-to-end (alert → click
→ handover or queue → resume-after-reset click), timed and recorded; pass = zero auto-actions,
resume latency after reset <15 min (deep/10 §9 metric). Wrong-account affinity check: fixture where
project history favours account B asserts B's affinity factor appears in its explanation (wrong-billing guard, deep/09 lines 458-461).
**Visual testing**: Capture: NewSession ranked list (Suggested + explanations + greyed filtered
row), "Why this order?" expanded, actionable 90% notification, inbox/queue rows (`queued` countdown,
`ready`), digest panel with a renewal item visually distinct from window items. Viewports 1440×900 +
1280×800, light+dark. Pass = every row shows badge + ≥2 factors without hover; notification actions
keyboard-focusable; grayscale-readable (WCAG 1.4.1). A11y: notifications use polite live-region
semantics, ranking list is a labelled listbox/list with arrow-key navigation.
**Files to touch**: new `crates/juggler-core/src/ranking.rs`, new `crates/juggler-core/src/alerts.rs` (FSM + reset-queue logic),
`crates/juggler-core/src/storage/usage.rs` (`reset_queue` table, AJ-E606),
`crates/juggler-app/src/commands.rs` (ranking query, alert/queue commands),
`ui/src/screens/NewSession.tsx` (ranked rows — shared with handover wizard host, coordinate with
switch epic), notification/tray integration in `crates/juggler-app/src/state.rs` (confirm at build
time), `crates/juggler-core/tests/ranking_alerts.rs` (new).
**Research basis**: deep/09 §Q6 lines 445-469, §Q5 lines 423-443, §Q4 lines 417-421; deep/10 §3
lines 42-49, §9 lines 96-103, §28.1 lines 341-347; pass2/09 line 218 (ranking canonical home:
pass2/02 §7 — reconcile weights there if §7 differs, and record the reconciliation); decision-log.md:12-18 (D-01: quota as trigger/input).
**Handover note**: Canonical-home conflict is real: pass2/09 assigns ranking display to 04 and
ranking logic to 02 §7, while deep/09 §Q6 carries the only concrete formula — implement §Q6, cite 02
§7, and flag any weight divergence to the epic owner instead of averaging them. Trap: `calm` and
affinity can conspire to recommend the wrong-billing account; the affinity explanation (Requirement
3) is the user's defence, so never truncate it. This task closes the epic: when it lands, update
`docs/qa/gate-M3.md` (M3-6, EPICS.md:95) and the D-12 research status in
`docs/review/decision-log.md` only via maintainer decision — agents do not edit settled decisions.


**Status report (2026-10-16)**: DONE-EVIDENCED — evidence
`docs/qa/evidence/AJ-E611/manifest.md`; gate `docs/qa/gate-M3.md`
§AJ-E611. Core 11/11 (weights-in-one-function, greyed filters,
Estimated ×0.85 tie-break, exact §Q5 fixture-day sequence, 256-case
at-most-once proptest, digest mute/quiet/cadence, purity guardrail);
app guardrails 4/4 (no-spawn, digest-once-not-FSM, suggest-only
storage diff, queue stale/no-launch). The guardrail suite exposed and
fixed a REAL E606 bug (samples_for_account ignored the window column
— a five-hour sample polluted the weekly reading). Visual 280/280
incl. 16 new 27-ranking captures + 2 axe passes; two real a11y
defects fixed (nested-interactive listbox; modal text-faint contrast
— token cannot reach 4.5:1 on dark surfaces; rail's faint uses are
flagged for a maintainer token decision). Canonical-home
reconciliation: pass2/02 §7 adopts §Q6 wholesale — identical
weights, nothing to flag; §7's three extension terms (switch cost,
return likelihood, cumulative hop cost) are switch-epic scope
(replay packs/hop graphs), documented as stubs in ranking.rs. Live
quota-wall dogfood PENDING the authenticated session (recorded).
This closes the E6 epic: E601–E611 all DONE-EVIDENCED.

**Implementation-mapping**: `ranking.rs` + `alerts.rs` (new core),
window-scoped `samples_for_account` + reset-queue in usage.rs,
per-window `AccountEvaluation`, 4 app commands (ranking/alerts/queue
— no resume command by design), `RankedRows.tsx` +
`NotificationRegion.tsx` (new), NewSession mount, tests
`ranking_alerts.rs` (11) + `ranking_alerts_guardrails.rs` (4),
gallery 27-ranking + 2 axe passes.