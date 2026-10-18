# 10 — Product Proposal: Missing Features (Mini-PRDs)

> Parent: [full-competitive-ux-review.md](../full-competitive-ux-review.md#L1) ·
> Inputs: [PROPOSAL.md](../../PROPOSAL.md#L1), [SPEC.md](../../SPEC.md#L1), [EPICS.md](../../plan/EPICS.md#L1), [07-personas-and-journeys.md](07-personas-and-journeys.md#L1)
> Method: repo claims cite `file:line`; competitor claims link out. Invariant per feature: prepare/passive, execute on click ([SPEC.md:26](../../SPEC.md#L26), [PROPOSAL.md:62](../../PROPOSAL.md#L62)).

## 0. Personas (frozen for all PRDs)

| ID | Persona | One-liner | Sessions/day | Wallet |
|---|---|---|---|---|
| P1 | Maya, Solo Multi-Maxer | 2×Max+Codex+GLM, bootstrapped SaaS | 4–8 | Self-funded |
| P2 | Devon, Staff Eng | 10–20 parallel, 4 services | 10–20 | Employer |
| P3 | Priya, Agency | 4 clients, per-client billing/audit | 6–14 | Client pass-thru |
| P4 | Jonas, OSS Maintainer | Pro+Lite, 16 GB, bursty | 3–10 | Free; contributes |
| P5 | Sara, Enterprise-Locked | SSO-approved accounts only | 2–6 | Procurement |
| P6 | Alex, Student Optimiser | 1×Pro/Lite, 8 GB, deadline-driven | 1–4 | Free only |

Source: [07-personas:56](07-personas-and-journeys.md#L56) splits [PROPOSAL.md:51](../../PROPOSAL.md#L51) A–D; solo, salaried, and billable users optimise for opposite things, so one UI must not conflate them. Truths: quota-blindness is pain #1 for 5/6 yet sits at M3 after adapters ([EPICS.md:26](../../plan/EPICS.md#L26)); `Stuck/WaitingUser/QuotaBlocked` modelled ([model.rs:188](../../../crates/juggler-core/src/model.rs#L188)) but never emitted ([commands.rs:782](../../../crates/juggler-app/src/commands.rs#L782)).

**PRD shape:** Problem → Persona → Story → UX flow → Controls/states → Backend/data → Acceptance → Effort (S≤3d, M≈1–2w, L≈3–4w) → Priority → Metric. **Non-negotiables:** no auto-rotation (suggest + click only); every number carries Measured/Computed/Estimated ([SPEC.md:259](../../SPEC.md#L259)); no proxy/credential reads beyond the F4.1 decision; local-first, fleet carries telemetry never credentials ([SPEC.md:377](../../SPEC.md#L377)); honest empty states, never fabricate (`percent={0}`, [full review:193](../full-competitive-ux-review.md#L193)).

---

## 1. Quota Engine + Forecast — P0 · L

**Problem.** Walls kill in-flight work ([PROPOSAL.md:23](../../PROPOSAL.md#L23)); engine is SPEC-only ([SPEC.md:254](../../SPEC.md#L254)), rings hardcoded `percent={0}`. This is G2, the noun in "juggler". **Persona:** P1 (primary), P4, P6 — small tanks first.
**Story.** As Maya launching a 3h refactor, I see "A exhausts in ~40m; B has 80% headroom" *before* Start and route correctly once.
**UX flow.** (1) NewSession rows: 5h/weekly ring + `%` text (never colour-only), burn arrow, reset countdown, forecast pill. (2) Admission=No-quota → "exhausts ~13:40; route to B?" [Use B] [Launch anyway]. (3) Handover wizard reuses ranked rows. (4) Overview rings → Usage detail. (5) 80%/90% → quiet notification (§7), never modal.
**Controls/states.** `no-data` ("start one session to measure"), `measured`, `estimated-stale` (>15m), `blocked` (+reset CTA); per-account threshold sliders; honesty-tier badge on every figure.
**Backend/data.** Statusline `rate_limits` feed ([SPEC.md:192](../../SPEC.md#L192)); append-only `usage_events` + rollups ([SPEC.md:258](../../SPEC.md#L258)); burn = EWMA tokens/min (30m); forecast = remaining ÷ burn with widening confidence band. Pull M3-1/M3-2 forward ([full review:917](../full-competitive-ux-review.md#L917)).
**Acceptance.** Claude ±5% vs `/usage` over a full window; forecast MAPE ≤30% after 20m burn; Pro weekly renders (not Max-only). **Metric:** walls with <10m warning → <10%. **P0 · L.**

## 2. Needs-Attention Inbox — P0 · M

**Problem.** "Which of 15 needs me?" is unanswerable — statuses never emitted ([full review:201](../full-competitive-ux-review.md#L201)); P2's 01:14 waiter sat 7h ([07-personas:252](07-personas-and-journeys.md#L252)). **Persona:** P2, P1.
**Story.** As Devon at 08:00, I clear three overnight blockers from one queue in two minutes.
**UX flow.** (1) Rail first item `Inbox (3)` + tray count. (2) Rows: session · project · account · why-chip (WaitingUser/Stuck?/QuotaBlocked/Crashed/Review-ready) · ticking wait · primary action (Reply / Hand over / Resume-at-reset / View diff). (3) Row → session panel on that action. (4) Snooze 1h/1d, mute per session. (5) `g i`, `j/k`, `Enter`.
**Controls/states.** Row `fresh / snoozed` (excluded from badge) / `escalated` (>30m); global `all-clear` ("All clear — N running") as designed success state; `degraded` banner, never silent (cf. F2.1).
**Backend/data.** Emit statuses at choke point `apply_lifecycle_event` ([commands.rs:782](../../../crates/juggler-app/src/commands.rs#L782)); heuristics per [SPEC.md:220](../../SPEC.md#L220); inbox = `WHERE status IN (...) ORDER BY waiting_since`; `machine_id` column from day one (fleet-ready).
**Acceptance.** Fake-harness corpus ≥90% precision/recall ([EPICS.md:79](../../plan/EPICS.md#L79)); badge = unsnoozed rows (UI test). **Metric:** median waiting-time; overnight waiters. **P0 · M** (WOW-a; ship with §1).

## 3. Smart Routing Suggestion — P1 · M (needs §1)

**Problem.** cc-switch switches without restart ([full review:665](../full-competitive-ux-review.md#L665)); claude-swap auto-rotates at ~90%. Answer with a guardrail-compliant suggestion, never rotation. **Persona:** P1, P6 ("GLM for grunt work").
**Story.** As Maya, B ranks first with reasons ("82% headroom · light footprint · usual for this repo"); one click accepts; live sessions untouched.
**UX flow.** (1) NewSession pre-sorted; top row badged "Suggested". (2) "Why?" expands factors. (3) [Launch suggested] / [Choose another]. (4) At 90% on a *running* account: inbox row "New sessions → B?" affects only the next-launch default. (5) "Don't suggest GLM here" persists per project.
**Controls/states.** Score = 0.5 headroom + 0.3 footprint-fit + 0.2 project history; `suggested / dismissed-project / no-headroom` (all >85% → show reset countdowns).
**Backend/data.** §1 headroom + `footprint_stats` ([SPEC.md:235](../../SPEC.md#L235)) + per-project history; pure SQLite function, no ML.
**Acceptance.** Never fires for a live-session switch (guardrail asserts no such API, [SPEC.md:136](../../SPEC.md#L136)); explanation matches stored factors. **Metric:** acceptance >40%; wrong-account launches → ~0. **P1 · M.**

## 4. Worktree Isolation — P0/P1 · M

**Problem.** Session = only `cwd` ([SPEC.md:113](../../SPEC.md#L113)) despite parallel-on-one-repo promise ([PROPOSAL.md:75](../../PROPOSAL.md#L75)). Claude Squad (1 task = 1 worktree, [full review:579](../full-competitive-ux-review.md#L579)), Conductor ([workflow docs](https://www.conductor.build/docs/concepts/workflow)), Vibe all isolate. **Persona:** P2, P1.
**Story.** As Devon with 6 parallels on one service, each session owns a branch; no clobbering.
**UX flow.** (1) "Isolate in worktree" default ON when repo has a live session. (2) Branch preview `juggler/<profile>/<short-id>` editable. (3) Row shows branch + dirty dot; shared branch → amber warn (never block). (4) Finish → [Open diff] [Clean up] (refuses listing uncommitted files). (5) Non-git cwd: hidden + tooltip.
**Controls/states.** `shared-cwd / worktree-clean / worktree-dirty / worktree-orphaned`; base-ref picker (HEAD default).
**Backend/data.** `git worktree add` under `<managed>/worktrees/<id>`; `worktree_path, branch, base_ref` on SessionRecord; `git status --porcelain` throttled 15s; managed-dir-only writes.
**Acceptance.** Concurrent-edit fixture: no conflicts; shared-branch warning fires; dirty cleanup blocked with file list. **Metric:** collision warnings; worktree adoption %. **P0 (P2) / P1 · M.**

## 5. Session Search / Replay / Archive — P1 · L

**Problem.** No archive/delete/search/retention ([full review:86](../full-competitive-ux-review.md#L86)); ~4 MB/day growth ([SPEC.md:233](../../SPEC.md#L233)); Codex users demand delete ([research/codex.md:80](../../research/codex.md#L80)). **Persona:** P4 (200 stale rows), P2, P6 at 60 sessions.
**Story.** As Jonas, I bulk-archive 40 repro sessions, then find "the Stripe session" by content.
**UX flow.** (1) Filter chips Active / Archived / All. (2) Bulk → Archive / Export pack / Delete (typed confirm; managed files only). (3) Search: title/project/account/transcript (FTS) with snippets. (4) Replay: read-only scrubber (messages + burn over time), [Fork from here] primes a new session — Opcode checkpoints ([opcode](https://github.com/winfunc/opcode)) adapted to lineage. (5) Retention "archive after N days" (default 30, never auto-delete) + storage readout.
**Controls/states.** Lifecycle `active → archived → deleted` (tombstone kept for lineage); replay `readonly`; fork adds `forked_from` edge.
**Backend/data.** SQLite FTS5 over Transcript IR (message text; tool payloads excluded); `archived_at` migration; replay reads stored JSONL, never re-invokes harness; GC touches managed dirs only, logs counts.
**Acceptance.** 1k-session search <300 ms; archive keeps `transcript_ref` resolvable; fork primes with exact prefix. **Metric:** sessions >90d still Active ↓80%. **P1 · L** (search M + replay M + lifecycle S, in that order).

## 6. Cost Ledger — P1 · M (needs §1)

**Problem.** Tokens only, no USD ([full review:76](../full-competitive-ux-review.md#L76)); ccusage proves JSONL→cost ([ccusage](https://github.com/ryoppippi/ccusage)). P3 cannot invoice; P1 cannot judge Max #2. **Persona:** P3, P6 ($5 top-ups), P1.
**Story.** As Priya, I export per-client CSV (account × project × USD, tier-labelled) in one click, not 45 min of spreadsheets.
**UX flow.** (1) Usage matrix: tokens → USD with badge "Estimated — list pricing, not your subscription fee" (API-equivalent value ≠ cash paid, permanently labelled). (2) Daily budget bar per account (P6 $5 cap alert). (3) [Export CSV] [Per-client pack] via project→client tags (§15). (4) Pricing editor + "last synced". (5) Manual subscription fees per account → "did this sub earn its $200?"
**Controls/states.** `priced / unpriced-model` (USD `—`, never `$0.00`) / `stale-pricing` (>30d); budget ok/warn/over.
**Backend/data.** Versioned pricing (model → $/MTok), models.dev sync + offline edit; USD joined at query time (never baked into `usage_events`, so repricing is retroactive); streaming CSV.
**Acceptance.** Fixture totals match ccusage on same corpus; export reconciles to screen to the cent. **Metric:** Priya invoice prep <5 min. **P1 · M.** Label discipline *is* the feature.

## 7. Notifications Centre — P1 · M

**Problem.** No quiet middle: silence today, spam if naive. Threshold alerts promised ([SPEC.md:257](../../SPEC.md#L257)) with no centre/digest/mute; P2 mutes-all within a week ([full review:751](../full-competitive-ux-review.md#L751)). **Persona:** P2, P4.
**Story.** As Devon, one hourly digest with three items; interrupt only if a session waits >15 min.
**UX flow.** (1) Header bell (unsnoozed count) → panel: Today / Earlier by project. (2) Items deep-link to the clearing inbox row. (3) Per-event toggles (WaitingUser, Stuck?, Quota 80/90/100%, Crash, Review-ready, Reset-arrived) × Interrupt / Digest / Off. (4) Quiet hours 22:00–08:00 digest-only. (5) Per-project mute, per-session snooze (shared store with §2).
**Controls/states.** Item `unread / read / snoozed / muted`; `empty` = "Nothing needs you" (designed success); OS-permission-denied → in-app-only banner.
**Backend/data.** `notifications(event, session_id, created_at, read_at, snoozed_until, channel)`; OS notify on Interrupt only; digest assembled on schedule.
**Acceptance.** Muted project never interrupts (fixtures); digest = exact unsnoozed actionable set; 21:59/22:01 boundary test. **Metric:** <8 interrupts/day; mute-all <5%. **P1 · M.**

## 8. Account Health — P1 · S–M

**Problem.** Presence-only login probe ([claude_code.rs:332](../../../crates/juggler-adapters/src/claude_code.rs#L332)); no expiry/tier/wrong-account detection despite wrong-billing reports ([research/claude-code.md:36](../../research/claude-code.md#L36)). **Persona:** P3, P5, P1.
**Story.** As Priya, "Client A usually uses Work Max — this is Personal Max. Launch anyway?" stops a mis-billing.
**UX flow.** (1) Health card per account: tier read-back where exposed, login state, expiry countdown, last-verified. (2) Degraded → cause + vendor-login CTA in embedded terminal ([claude_code.rs:354](../../../crates/juggler-adapters/src/claude_code.rs#L354)). (3) Affinity warning when profile ≠ usual (plurality of last 20). (4) ReadyHome skip sticky per *healthy* profile only (fixes [full review:227](../full-competitive-ux-review.md#L227)).
**Controls/states.** `healthy / expiring-soon / expired / tier-mismatch / unknown` (probe failure never shown as healthy); affinity `usual / unusual-confirmed` (remembered per project+profile).
**Backend/data.** Extend metadata-only `login_state` (length + keychain existence, never contents); hourly + on-launch refresh; affinity = derived query. F4.1 decision gates any "import keeps you logged in" claim — print it on the card.
**Acceptance.** Expired fixture cannot pass ReadyHome silently; warning fires exactly on plurality mismatch; no credential bytes in logs. **Metric:** wrong-account launches → 0. **P1 · S–M.**

## 9. Reset Queue / Scheduled Continue — P1 · M (needs §1+§2)

**Problem.** Reset "return" is a suggestion only ([SPEC.md:288](../../SPEC.md#L288)); Codex users demand auto-wait parity ([#21073](https://github.com/openai/codex/issues/21073)); Claude already auto-waits ([research/claude-code.md:53](../../research/claude-code.md#L53)). **Persona:** P1 (17:00 return), P4.
**Story.** As Maya blocked at 13:00, I queue "resume at reset ≈17:00"; one notification at reset, one click resumes — no clock-babysitting, no unapproved action.
**UX flow.** (1) QuotaBlocked row → [Queue at reset] showing reset time + target (return-to-source vs continue-here). (2) Queue section in inbox with countdowns, [Cancel] [Resume now]. (3) At reset: interrupt + row flips to `ready`; resume still one click ([SPEC.md:26](../../SPEC.md#L26)). (4) Optional "Block launch when Admission=No" (default off).
**Controls/states.** `queued / ready / resumed / cancelled / stale` (lineage broken → explained); FIFO per account, one click each.
**Backend/data.** `reset_queue(session_id, account_id, resume_at, kind)`; `resume_at` from feed `resets_at` ([SPEC.md:192](../../SPEC.md#L192)), Estimated-labelled when inferred; local timer only; relaunch reconciles past-due to `ready`, never executes.
**Acceptance.** Fake-clock guardrail: never launches unclicked; countdown error ≤60s on fixtures. **Metric:** resume latency after reset <15m. **P1 · M.**

## 10. Command Palette (⌘K) — P1 · M

**Problem.** Rail caps 15/8 without search ([WorkflowRail.tsx:184](../../../ui/src/components/WorkflowRail.tsx#L184)); Recognition/Flexibility fail ([full review:759](../full-competitive-ux-review.md#L759)). **Persona:** P2; everyone past 15 sessions.
**Story.** As Devon: ⌘K → "stripe" → Enter → session with [Resume] focused, <3s, mouse untouched.
**UX flow.** (1) ⌘K anywhere (terminal yields the chord, §18). (2) Fuzzy over projects/sessions (FTS snippets via §5)/accounts/*actions*. (3) Recents when empty; `↑↓` Enter; jump/resume inline, handover opens wizard. (4) Esc closes, focus returns. (5) `?` opens shortcuts.
**Controls/states.** `open / querying / no-results` (path-query alone may offer path creation); full `role=dialog` + trap (fixes [full review:872](../full-competitive-ux-review.md#L872)).
**Backend/data.** Thin client over §5 FTS + static action registry; cap 50, recents outrank fuzzy score.
**Acceptance.** All daily actions ≤3 keystrokes (keyboard walk); opens <100 ms at 1k sessions. **Metric:** palette uses/day. **P1 · M.**

## 11. Review / Diff Queue — P2 · L (dogfood-gated)

**Problem.** Finished sessions pile up (P2 pain #2) yet demand is `[inferred]` ([full review:503](../full-competitive-ux-review.md#L503)). Vibe's inline-comment loop ([full review:639](../full-competitive-ux-review.md#L639)) and Conductor's diff+PR set expectations. **Persona:** P2, P4.
**Story.** As Devon, five finished sessions wait with diff stats; I approve three, return one with a comment, archive one — no five terminals.
**UX flow.** (1) Inbox "Ready for review": project, branch (§4), diff stat, summary line. (2) Split view: file list + read-only diff, PACK.md tab. (3) [Approve → archive] [Continue] [Comment to agent…] (sent only on [Send], never auto). (4) [Create PR] via `gh` when a worktree branch exists. (5) Empty: "No finished work waiting."
**Controls/states.** `awaiting-review / approved / changes-requested / pr-created`; diff `clean / dirty-uncommitted` (shown) / `no-changes` (say so).
**Backend/data.** `git diff <base>...HEAD` + `status` on the session worktree/cwd; comment = resume primed with the comment; decisions logged (§17).
**Acceptance.** Diff byte-matches `git diff` on fixtures; no comment reaches harness without Send. **Gate:** read-only v1 only after 5 dogfood diaries confirm demand, else defer. **Metric:** unreviewed age. **P2 · L** — most over-buildable item here; stage it.

## 12. Backup / Restore / Export — P1 · S–M

**Problem.** One WAL sqlite holds lineage + usage ([SPEC.md:399](../../SPEC.md#L399)), atomic writes only ([SPEC.md:22](../../SPEC.md#L22)) — no backup or corrupt-DB UX. Blocker for P5. **Persona:** P5, P3.
**Story.** As Sara after a disk error: "Database damaged — Restore from backup (yesterday 18:02)?" not a blank start.
**UX flow.** (1) Rolling backups (last 7) on clean exit + daily; Settings lists sizes/dates. (2) [Export backup…]: sqlite snapshot + packs + config (no credentials, no harness state). (3) [Restore…] previews, then destructive-confirmed [Restore and relaunch]. (4) Corrupt-boot screen: newest-good restore or "Start fresh" (renames, never deletes). (5) Transcript retention picker per §5.
**Controls/states.** `backed-up / backup-failed` (banner, never silent) / `restore-preview / restored`; fresh start → `juggler.sqlite.corrupt-<ts>`.
**Backend/data.** `VACUUM INTO` snapshots; manifest (schema + app version); restore migrates forward, refuses newer schema explicitly; atomic writes ([core/fs.rs:18](../../../crates/juggler-core/src/fs.rs#L18)).
**Acceptance.** Kill -9 mid-write → restore or fresh, never half-migrated; export→wipe→import preserves lineage; bundle has zero credential-path files. **Metric:** unrecoverable-DB incidents → 0. **P1 · S–M.**

## 13. Updates: App + Harness Tracking — P1 · M

**Problem.** Updater + harness tracking specified ([SPEC.md:355](../../SPEC.md#L355)) but unsigned builds block P5 and weekly harness churn ([PROPOSAL.md:132](../../PROPOSAL.md#L132)) breaks adapters silently. **Persona:** P5, P4.
**Story.** As Jonas: "Claude 2.1.301 available (you run 2.1.296) — adapter verified to 2.1.298" so I update deliberately.
**UX flow.** (1) Settings → Updates: channel picker (Beta until 1.0, [SPEC.md:359](../../SPEC.md#L359)), [Check now], Install-on-quit / Relaunch now. (2) Harness cards: installed vs latest vs verified range; [Open vendor updater] — never auto-updates ([EPICS.md:119](../../plan/EPICS.md#L119)). (3) Verify-fail → hard block, no bypass. (4) "What changed" → CHANGELOG.
**Controls/states.** App `up-to-date / available / downloading / verified-ready / verify-failed`; harness `verified / beyond-verified-range` (probe results shown) / `check-failed-offline` (neutral).
**Backend/data.** `tauri-plugin-updater` + Releases `latest.json` ([SPEC.md:357](../../SPEC.md#L357)), signature non-disableable; daily harness checks are the only sanctioned background network ([SPEC.md:23](../../SPEC.md#L23)); drift from `discover()` ranges.
**Acceptance.** Bad-signature artifact rejected (negative test); channel switch resolves correct feed; never claims verified beyond declared range. **Metric:** % installs on verified range. **P1 · M** (signing CI is M5; this covers UX/tracking).

## 14. MCP / Settings Sync — P2 · M (needs §8)

**Problem.** Only project sync exists (M4-6, [EPICS.md:108](../../plan/EPICS.md#L108)); no MCP/rules portability ([full review:148](../full-competitive-ux-review.md#L148)). Opcode's MCP registry ([opcode](https://github.com/winfunc/opcode)) makes it table stakes. **Persona:** P3, P2.
**Story.** As Priya onboarding Client D, I clone A's profile, tick 5 of 6 MCP servers, untick the secret-bearing one — nothing moves silently.
**UX flow.** (1) [Sync from…] → checklist: MCP servers (env masked), rules files, model defaults, permission mode. (2) Per-item diff (add / overwrite / identical-greyed). (3) [Apply selected]; summary of exactly what changed. (4) Rules conversion AGENTS.md ↔ CLAUDE.md ↔ `.zcode/AGENTS.md` as a sync item type ([SPEC.md:289](../../SPEC.md#L289)).
**Controls/states.** Item `new / changed / identical / conflict` (default Keep target); secrets-adjacent items default **unticked** + warning icon.
**Backend/data.** Adapter `settings_overlay` read/write on managed dirs only ([model.rs:101](../../../crates/juggler-core/src/model.rs#L101)); vendor formats parsed inside adapters only; applies via `atomic_write`, logged (§17), env values never logged (key names only).
**Acceptance.** Unchecked items byte-identical after sync (hash test); conflict never overwrites by default. **Metric:** new-account time-to-productive <5 min. **P2 · M.**

## 15. Multi-Repo & Multi-Client Grouping — P2 · M

**Problem.** Flat rail ([WorkflowRail.tsx](../../../ui/src/components/WorkflowRail.tsx#L1)); P3's clients and P2's services (one product, many repos) lack grouping and screen-share safety. Vibe does per-repo worktrees ([full review:641](../full-competitive-ux-review.md#L641)). **Persona:** P3, P2.
**Story.** As Priya on a Client B call, "Present mode: Client B" hides all other clients' names, sessions, notifications.
**UX flow.** (1) Optional Group tag (client/service) + collapse + filter chips (also relieves the 15-row cap; §10 backs it). (2) Group repo-set launch: primary repo = cwd, siblings listed in the priming pack; §4 isolation per-repo. (3) Present mode (`⌘⇧P`): only that group visible, emails masked, persistent banner while active. (4) Per-group budget (§6) + quota attribution.
**Controls/states.** Group `expanded / collapsed / active-filter`; Present `off / on(group)`. Group delete untags, never deletes projects.
**Backend/data.** `groups(id, name, colour)` + nullable `projects.group_id` (ungrouped is first-class); Present is a pure UI filter (no data movement).
**Acceptance.** Present-mode DOM contains zero other-group strings (screenshot-diff test); launch pack lists siblings exactly. **Metric:** Present-mode adoption. **P2 · M** (Present alone is S, may pull forward).

## 16. Templates & Prompts Library — P2 · S–M

**Problem.** Every session starts blank; P4 re-types repro scaffolds, P6 lacks exemplars. No template concept in SPEC §9. Precedent: Opcode agent library ([opcode](https://github.com/winfunc/opcode)). **Persona:** P4, P6.
**Story.** As Jonas, Template "Issue repro" pre-fills the opener with repo slots resolved; I edit, then launch.
**UX flow.** (1) NewSession Template picker (None default; built-ins: Issue repro, PR review, Refactor-with-tests, Cheap grunt pass). (2) Slots `{{project}} {{branch}} {{issue}}` become inline fields when unresolved — never sent literally. (3) [Save prompt as template]; templates are local markdown, CRUD in Settings. (4) In-terminal `/slug` snippets deferred — v1 is launch-time only and says so (terminal passthrough).
**Controls/states.** `builtin` (read-only, duplicable) / `custom / invalid` (bad slot blocks launch until fixed or removed). No sharing/sync in v1 (§22).
**Backend/data.** Markdown under `<app-data>/templates/` (editable outside the app — deliberate); slots resolved from session context; use logged by id only (§17).
**Acceptance.** Unresolved `{{slot}}` never reaches a harness (launch-path test); built-ins render on two fixture projects. **Metric:** template launch share; P6 time-to-first-prompt. **P2 · S–M.**

## 17. Audit Log — P1 (P5/P3) · S–M

**Problem.** Lineage stops at `HandedOver` ([full review:282](../full-competitive-ux-review.md#L282)); no record of who clicked what — the artefact Sara's review and Priya's Q&A need. Fleet commands already require attribution ([EPICS.md:140](../../plan/EPICS.md#L140)); build locally now. **Persona:** P5, P3.
**Story.** As Priya asked "what did AI do Tuesday?", I export an audit CSV and answer from record, not memory.
**UX flow.** (1) Settings → Audit: reverse-chron, filter by project/account/type/date. (2) Session panel shows its slice (lineage + timestamps + actor). (3) [Export CSV/JSON] with redaction (paths shortened, emails masked). (4) Append-only UI, no edit/delete; retention follows §5. (5) Confirm steps preview their audit line ("Records: Handover sess-42 A→B, T2, you clicked Confirm").
**Controls/states.** Events: `launch, stop, handover, return, resume, review-decision, sync-apply, pack-export, settings-change, backup, restore`; actor `local-user` → `console:<machine>` (column ships now).
**Backend/data.** `audit_log(id, ts, actor, event, subject_type, subject_id, detail_json)` written in the action's transaction; `detail_json` allowlisted only (never prompts, env values, credential paths).
**Acceptance.** Every state-changing command emits exactly one row (coverage test); redacted export has no absolute home paths (regex). **Metric:** exports/month. **P1 · S–M** — cheap now, costly to retrofit.

## 18. Keyboard System — P1 · M (pairs §10)

**Problem.** Only `⌘B/⌘N` exist ([App.tsx:205](../../../ui/src/App.tsx#L205)); modals lack trap/return-focus, tabs lack arrows, splitter not keyboard-operable ([full review:869](../full-competitive-ux-review.md#L869)). Product for P2, procurement gate for P5. **Persona:** P2, P5.
**Story.** As Devon, a full morning — triage, jump, resume, handover — without the mouse, focus never lost.
**System.** L1 global: `⌘K ⌘N ⌘B ⌘⇧P`, `g i/p/s`, `?` sheet. L2 list: `j/k Enter x s m`. L3 session: `r h ⌥⌘T`. Terminal focused → single-key layer suspends (Esc exits; ⌘-chords live). Cheat sheet generated from the registry (cannot drift). All modals `role=dialog` + trap + Esc + return-focus; `prefers-reduced-motion` + AA subtext (≥4.5:1) ship together — sales features for P5.
**Controls/states.** Status-bar indicator `⌨ App / ⌨ Terminal — Esc to exit`; OS-reserved chords win; remapping UI is P2.
**Backend/data.** Single frontend keymap registry feeding menus, tooltips, sheet, palette. No storage.
**Acceptance.** 12-action mouse-disabled walk passes; trap test per modal; sheet diff-matches registry in CI. **Metric:** keyboard task-completion rate. **P1 · M.**

## 19. Tray + Global Hotkey — P1 · S–M

**Problem.** Tray quota + hotkey promised ([SPEC.md:352](../../SPEC.md#L352)), absent in `ui/src`. cc-switch's tray row is the incumbent's home screen ([full review:665](../full-competitive-ux-review.md#L665)). **Persona:** P1 (08:30 glance), P6.
**Story.** As Maya, the menu bar reads "2 need you · A 78% · reset 17:00"; one click lists accounts + quick-launch.
**UX flow.** (1) Tray title = worst signal (inbox > quota > all-clear). (2) Menu: inbox lines (→ session), account rows % + reset (→ Accounts), top-5 recent quick-launch (suggested profile, still a click), [Open] [Quit]. (3) Hotkey `⌥⌘J` (remappable) → floating instances list (palette-lite, keyboard-first). (4) Badge dot only when unsnoozed inbox >0. (5) Quit with live sessions warns with count.
**Controls/states.** `all-clear / needs-attention(n) / quota-low (≥90%) / blocked`; colour always paired with text.
**Backend/data.** One pre-aggregated core query (inbox + quota + recents) pushed on change, never webview-polled; Tauri global-shortcut plugin; tray is a view, never a state owner.
**Acceptance.** Summary updates <1s after fixture change; quick-launch logs the click (§17); hotkey list keyboard-operable. **Metric:** tray-open → action rate. **P1 · S–M.**

## 20. Fleet Teaser — P2 · S now, L later

**Problem.** Fleet (M6/M7, [SPEC.md:455](../../SPEC.md#L455); gRPC/mTLS [SPEC.md:373](../../SPEC.md#L373)) is correctly late but must not be invisible or data-unready. **Persona:** P2, P1.
**Story.** As Devon, Settings previews v0.2 fleet and my inbox already has a machine column — upgrade feels like a filter opening.
**UX flow (v0.1 only).** (1) Settings → Fleet card: honest status, pairing explainer (code/QR, LAN/VPN, no relay, credentials never leave, [SPEC.md:383](../../SPEC.md#L383)), [Notify me] (local flag, no telemetry). (2) Local machine renders as disabled "This Mac" filter chip wherever machines will appear — IA rehearsed, zero fake data. (3) No ComingSoon nav item; the card is the whole surface.
**Controls/states.** Only `local-only` in v0.1; schema ships now: `sessions.machine_id`, `audit.actor`, inbox grouping.
**Backend/data.** Now: three columns + M5 transport spike only ([EPICS.md:121](../../plan/EPICS.md#L121)). Later: read-only remote board → remote control → cross-machine handover last (T2 needs same binary + checkout path, [SPEC.md:388](../../SPEC.md#L388)).
**Acceptance.** No fleet dead-end (IA test); local sessions round-trip with `machine_id`; copy claims nothing the spike hasn't proven. **Metric:** [Notify me] flags (M6 demand signal). **P2 · S.**

## 21. Cut / Defer List

| Item | Verdict | Why |
|---|---|---|
| ZCode adapter before Claude loop | Defer Phase 3 | Bundle-only, undocumented ([SPEC.md:282](../../SPEC.md#L282)); fragility before §1/§2 prove value |
| Codex quota polish | Ship labelled, defer tuning | No API ([SPEC.md:203](../../SPEC.md#L203)); honest Estimated beats fiction |
| T3/T4 marketing | Cut until proven | Crate doc-only ([full review:243](../full-competitive-ux-review.md#L243)); advertise RESUME-only (F3.1) |
| Kanban clone | Cut | Sunset category; import users, not the metaphor ([full review:512](../full-competitive-ux-review.md#L512)) |
| Full comment-to-agent | Behind §11 gate | `[inferred]` demand; read-only diff first |
| Public adapter SDK | Post-1.0 | Publish in-tree conformance suite in M2 (F1.8) instead |
| Overview L3 analytics | Defer | Fix L1→L2 inversion first ([full review:861](../full-competitive-ux-review.md#L861)) |
| Replay fork (full) | Stretch | Ship read-only scrub; fork only if replay is used |
| Win/Linux packaging | Post-1.0 | macOS-first decided ([SPEC.md:18](../../SPEC.md#L18)); CI cross-builds keep core honest |
| Fleet remote control | v0.3 | Read-only board first; commands widen audit surface |

## 22. Anti-Features (never build)

1. **Auto-rotation / auto-switch at threshold** (claude-swap pattern) — violates guardrail 2 ([PROPOSAL.md:62](../../PROPOSAL.md#L62)). Offer §3 suggestion + click instead.
2. **Traffic proxy / gateway / request routing** — prohibition zone ([PROPOSAL.md:42](../../PROPOSAL.md#L42), [PROPOSAL.md:125](../../PROPOSAL.md#L125)).
3. **Login automation, captcha/OTP handling, credential storage** ([PROPOSAL.md:43](../../PROPOSAL.md#L43)).
4. **Account pooling / sharing between people** and team RBAC/SSO admin in v1 ([PROPOSAL.md:45](../../PROPOSAL.md#L45)); Fleet = *my* machines only.
5. **A replacement chat UI** ([PROPOSAL.md:46](../../PROPOSAL.md#L46)) — embed terminals; harnesses own UX.
6. **Autonomous task assignment** ("which account deserves this") ([PROPOSAL.md:44](../../PROPOSAL.md#L44)) — inbox + suggestions, human assigns.
7. **Auto-accept permissions** (Claude Squad `-y` daemon pattern) — crosses the click-bound invariant; do not copy.
8. **Fabricated metrics** (hardcoded rings, "Machine OK") — honest empty states only; a supervision tool that lies about state is worse than none.
9. **Cloud relay / mandatory account / default telemetry** — local-first is the trust claim ([SPEC.md:23](../../SPEC.md#L23)).
10. **Gamification** (streaks, burn leaderboards) — wrong incentive at the quota wall; P4 explicitly rejects it.
Test for any proposal: *does it require sharing, proxying, auto-rotating, or auto-assigning?* Yes → decline citing the guardrail.

## 23. Naming & Positioning Proposal

**Keep the name; fix the sentence.** "AI Juggler" already encodes the wedge — the risk is sounding like another orchestrator. Current README undersells state ("spec phase", contradicted by CHANGELOG per [full review:25](../full-competitive-ux-review.md#L25)); positioning must lead with juggling, not sessions.
- **Tagline (primary):** "Run every AI coding account you pay for — quota-aware, without the terminal sprawl."
- **Category line:** "Orchestrators run your agents. Switchers swap your account. Juggler does both, quota-aware." ([full review:483](../full-competitive-ux-review.md#L483) — make it true via §1/§2 before printing it.)
- **Hero screenshot (only one matters):** NewSession with forecast — "A exhausts in ~40m; B has 80% headroom" — plus a Pro/Lite variant so P6 doesn't self-exclude. Never lead with a board screenshot; boards are commodities.
- **Vocabulary:** retire "transplant" from user copy (keep in SPEC); user words: *Hand over*, *Continue on…*, *Return when reset*. Persona gloss at first use for P6; no "instance" in user copy until §2's merge decision (Instances/Sessions conflation, F2.2) is resolved — say *Sessions* and *Accounts*.
- **Trust page (one page, linkable):** guardrails §5 of PROPOSAL + decided F4.1 + "we never proxy, never rotate, never read credentials" with `guardrails.rs` as the evidence link. This page *is* Sara's aha and Priya's sales artefact.
- **Sunset window:** one honest comparison/migration page for Vibe Kanban + Crystal users ([full review:507](../full-competitive-ux-review.md#L507)) + config import (§21) — import users, not the kanban metaphor.

## 24. OSS Monetisation Options (core stays free)

Constraint: quota/forecast/inbox are the core promise — paywalling them destroys the wedge and P6 trust. Monetise *around* supervision, never *inside* the juggle. MIT licence decided ([SPEC.md:466](../../SPEC.md#L466)); all options below respect it (no relicensing retroactively).

| Option | What sells | Persona fit | Risk |
|---|---|---|---|
| GitHub Sponsors + Open Collective | Goodwill | P4, P1 ($5–15/mo) | Voluntary; not a business |
| Pro Pack (one-off $49) | §6 per-client PDF, §11 PR polish, §20 fleet-early | P1, P3 | Must not gate §1/§2/§3 |
| Agency seats ($30–60/seat/mo) | §15 Present mode, §17 audit export, support | P3 | Support load; keep optional |
| Enterprise support contract | Signed builds, security reviews, MDM help | P5 (procurement) | Never promise SSO/RBAC (non-goal) |
| Paid adapter maintenance | Sponsor-a-harness (e.g. ZCode upkeep) | P4 ecosystem | Governance: no pay-to-prioritise core |
| Hosted update/signing (none) | — | — | Do not cloud-ify; relay = trust + ToS risk |

**Recommendation:** Sponsors now; Pro Pack at v0.1-beta *only if* §6/§15/§17 ship polished; defer any seat pricing until Priya-class dogfood proves invoice value. Publish a "Core free forever" pledge — it is a distribution asset for P6's campus funnel, not charity.

## 25. MoSCoW + RICE

RICE = Reach (dogfood users, 10= all 6 personas) × Impact (0.25–3) × Confidence % ÷ Effort (person-weeks). Scores order *within* MoSCoW bands; guardrail/blocker items (†) jump bands regardless of score.

| # | Feature | MoSCoW | R | I | C | E(wk) | RICE | Phase |
|---|---|---|---|---|---|---|---|---|
| 1 | Quota + forecast | **Must** | 10 | 3 | 80 | 4 | 600 | 1 |
| 2 | Needs-attention inbox | **Must** | 9 | 3 | 90 | 2 | 1215 | 1 |
| 8 | Account health | **Must**† | 7 | 2 | 80 | 1 | 1120 | 1 |
| 12 | Backup/restore | **Must**† | 6 | 2 | 90 | 1 | 1080 | 1 |
| 17 | Audit log | **Must**† | 5 | 1 | 90 | 1 | 450 | 1 |
| 3 | Smart routing | Should | 8 | 2 | 70 | 2 | 560 | 1–2 |
| 4 | Worktree isolation | Should | 8 | 2 | 80 | 2 | 640 | 1 |
| 5 | Search/replay/archive | Should | 8 | 2 | 80 | 4 | 320 | 1–2 |
| 6 | Cost ledger | Should | 7 | 2 | 80 | 2 | 560 | 2 |
| 7 | Notifications centre | Should | 8 | 1 | 80 | 2 | 320 | 1–2 |
| 9 | Reset queue | Should | 7 | 2 | 70 | 2 | 490 | 2 |
| 10 | Command palette | Should | 8 | 2 | 90 | 2 | 720 | 1 |
| 13 | Updates (UX/tracking) | Should | 9 | 1 | 80 | 2 | 360 | 1–2 |
| 18 | Keyboard system | Should | 6 | 2 | 90 | 2 | 540 | 1 |
| 19 | Tray + hotkey | Should | 8 | 1 | 90 | 1 | 720 | 1 |
| 11 | Review/diff queue | Could | 6 | 2 | 40‡ | 4 | 120 | 2–3 gated |
| 14 | MCP/settings sync | Could | 6 | 1 | 70 | 2 | 210 | 2–3 |
| 15 | Multi-repo grouping | Could | 5 | 2 | 60 | 2 | 300 | 2 |
| 16 | Templates/prompts | Could | 6 | 1 | 70 | 1.5 | 280 | 2 |
| 20 | Fleet teaser (schema) | Could | 5 | 1 | 90 | 1 | 450 | 1 (S only) |
| — | Fleet control, SDK, kanban | **Won't (v1)** | — | — | — | — | — | post-1.0 |

† Blocker-class: gates P5/P3 trust. ‡ `[inferred]` demand — dogfood-gates §11.

**Sequencing (one line each).** Phase 0 (days): truth pass, F4.1 decision, `guardrails.rs`, honest empty states. Phase 1 (v0.1-beta): §1 + §2 + handover-honesty + §4 + §8/§12/§17/§10/§18/§19 + §20-schema. Phase 2: Codex + §3/§6/§9/§14 + scale fixes (incremental tailing, shared sysinfo, joined queries — F5.1). Phase 3: ZCode + §11-gated/§15/§16 + signed 1.0; Fleet M6/M7 after 1.0 on the Phase-1 schema.

## 26. Risks & Falsification

| Risk | Kill-test (do first) |
|---|---|
| Forecast wrong on Pro/weekly windows | Dogfood P4/P6 cohort 1 week; MAPE per window type, not blended |
| Inbox becomes second spam channel | Count interrupts in dogfood; >8/day → tighten defaults before beta |
| Worktree confuses non-git users | First-run test with P6: can they say where files went? |
| USD misread as cash owed | Show ledger to 3 users; ask "what did you pay?" — any confusion = copy bug, fix before ship |
| Priya Present mode is invented | 3 agency interviews before §15 beyond the S slice (§20 validation in 07-file) |
| Review queue demand is Devon's alone | §11 gate: 5 diaries or it stays read-only |

**What would change this proposal:** a vendor shipping sanctioned multi-account + quota APIs (absorbs §1/§3 into adapters), or dogfood showing handover — not quota — is the daily pain (re-promote handover UX above §6/§9). Until then: **forecast + inbox + honest handover is the product; everything else is retention.**

## 27. Dependency Graph & Critical Path

Nothing here is independent. The graph below is the build order;
violating it re-creates the parent review's inverted roadmap.

```
§8 Account health ──► §3 Routing ──► §9 Reset queue
§1 Quota/forecast ──► §3, §6, §9, §19 (tray numbers)
§2 Status emission ─► §7 Notifications ─► §19 badge
§2 ─► §11 Review-ready ─► §11 Diff queue
§5 FTS index ───────► §10 Palette (snippets)
§4 Worktree ────────► §11 Diff base_ref
§17 Audit log ──────► §14 Sync, §11 Decisions, Fleet actors
§15 Groups ─────────► §6 Per-client export
§18 Keymap registry ► §10 Palette, §15 Present, §19 Hotkey list
§20 Schema (machine_id) ► Fleet M6 (no migration later)
```

**Critical path to v0.1-beta:** §8 (S) → §1 (L) → §2 (M) →
handover-honesty (outside this file, F3.3–F3.5) → §10+§18 (M).
Everything else parallelises around that spine. If the spine
slips, cut §5-replay and §11 first — they are the only Must-adjacent
items with a read-only fallback (search-only, no fork).

**Data migrations in order:**
1. `sessions.machine_id, waiting_since` (§2/§20) — additive, safe.
2. `usage_events` burn-rate materialisation (§1) — backfill from JSONL.
3. `sessions.archived_at, worktree_path, branch, base_ref` (§4/§5).
4. `audit_log` + `notifications` + `reset_queue` (§17/§7/§9).
5. `groups` + `projects.group_id` (§15) — last; pure metadata.

Each migration ships with a down-test from an empty schema, per
[EPICS.md:44](../../plan/EPICS.md#L44) round-trip discipline.
No migration may read harness state dirs; all read `juggler.sqlite`
plus managed-dir JSONL only.

## 28. Expanded UX Flows (three make-or-break journeys)

### 28.1 Maya: quota wall at 13:00 (validates §1+§9+handover)

| Step | Surface | System does | Maya does |
|---|---|---|---|
| 11:20 | Tray | Account A crosses 80%; digest item queued | Nothing (no interrupt) |
| 12:40 | Session header | Forecast pill: "A exhausts ~13:10" | Glances, keeps working |
| 13:02 | Inbox | QuotaBlocked row appears, badge 0→1 | Clicks row |
| 13:03 | Handover wizard | Targets ranked; B 82% headroom; fidelity listed | [Hand over to B] |
| 13:04 | Session B | Primed with transcript + siblings; lineage recorded | Continues task |
| 16:58 | Notification | Interrupt: "A reset — return?" | [Resume on A] |
| 17:00 | Session A | Resumed with "while you were away" delta | Reviews, ships |

Failure branch: if B also <10% headroom, wizard shows reset
countdowns instead of a ranking and offers [Queue at reset] (§9)
as primary. The user is never offered a dead-end handover.

### 28.2 Devon: Monday 08:00 triage (validates §2+§18+§11)

| Step | Keys/action | Result |
|---|---|---|
| Open app | `g i` | Inbox: 2 waiting, 1 stuck?, 1 quota, 3 review-ready |
| Oldest waiter | `j j Enter` | Session panel, permission prompt focused |
| Answer | Type + Enter in terminal | Row clears; badge decrements live |
| Stuck? row | `Enter` → [View output] | Last-output pane shows model-wait vs hang evidence |
| Review batch | `x x x` select → [Approve → archive] | Three archived; §17 rows written |
| Jump to spike | `⌘K` "spike" Enter | Fourth repo session focused |

Target: 7 inbox items cleared in <4 minutes, mouse untouched.
Measure this exact script in dogfood; >6 minutes fails §2.

### 28.3 Priya: Friday invoice (validates §6+§15+§17)

| Step | Surface | Outcome |
|---|---|---|
| Filter | Group chip "Client B" | Rail + Usage scoped; banner confirms filter |
| Verify | Usage matrix | "Estimated" badges visible; unpriced model shows `—` |
| Cross-check | Session slice (§17) | 14 sessions, 2 handovers, all timestamped |
| Export | [Per-client pack] | CSV + audit JSON in one bundle |
| Share | Present mode for call | No other client string in DOM |

Failure branch: any unpriced model in range → export still runs,
USD column shows `n/a (model unpriced)` and the cover note says so.
Never silently sum a partial total as if complete.

## 29. Persona × Feature Coverage Check

P0/P1 cells must have a Phase-1 home; "—" means deliberately unserved.

| Feature | P1 | P2 | P3 | P4 | P5 | P6 | Owner phase |
|---|---|---|---|---|---|---|---|
| §1 Quota/forecast | P0 | P0 | P1 | P0 | P1 | P0 | 1 |
| §2 Inbox | P1 | P0 | P1 | P1 | P1 | P2 | 1 |
| §3 Routing | P1 | P1 | P1 | P1 | P1 | P0 | 1–2 |
| §4 Worktree | P1 | P0 | P1 | P1 | P2 | P2 | 1 |
| §5 Lifecycle | P1 | P1 | P1 | P1 | P1 | P1 | 1–2 |
| §6 Cost ledger | P1 | P1 | P0 | P1 | P2 | P0 | 2 |
| §7 Notifications | P1 | P1 | P2 | P1 | P2 | P2 | 1–2 |
| §8 Health | P1 | P1 | P0 | P2 | P0 | P2 | 1 |
| §9 Reset queue | P1 | P1 | P2 | P1 | P2 | P1 | 2 |
| §10 Palette | P1 | P0 | P1 | P1 | P2 | P1 | 1 |
| §11 Review queue | P2 | P1 | P1 | P1 | P2 | P2 | gated |
| §12 Backup | P2 | P2 | P1 | P2 | P0 | P2 | 1 |
| §13 Updates | P2 | P2 | P1 | P1 | P0 | P2 | 1–2 |
| §14 MCP sync | P2 | P1 | P1 | P2 | P2 | P2 | 2–3 |
| §15 Grouping | — | P1 | P0 | — | P1 | — | 2 |
| §16 Templates | P2 | P2 | P2 | P1 | P2 | P1 | 2 |
| §17 Audit | P2 | P2 | P0 | P2 | P0 | — | 1 |
| §18 Keyboard | P1 | P0 | P2 | P1 | P1 | P2 | 1 |
| §19 Tray/hotkey | P1 | P1 | P1 | P2 | P2 | P1 | 1 |
| §20 Fleet teaser | P2 | P1 | P2 | P2 | P2 | P2 | 1 (S) |

Reading: no persona is P0-orphaned in Phase 1. P3's audit/health
pair and P5's backup/signed-build pair are the trust gates;
P6 is served by §1+§3+§6-translation, not by any P6-only feature —
deliberate: small-tank correctness beats student-specific UI.

## 30. Validation Plan (before any Phase-1 code freezes)

1. **Five dogfood diaries** (P1 + P2 archetypes, 1 week): does the
   01:14-waiter and 13:00-wall recur as modelled? Re-score RICE
   Impact from diaries, not opinion.
2. **Fidelity-label A/B:** handover with vs without carry-list;
   measure click-through and post-handover "lost work?" reports.
3. **Forecast calibration:** log predicted vs actual exhaustion
   per window type from day one (even before UI ships) so Phase 1
   launches with error bars, not hopes.
4. **Present-mode falsification:** 3 agency owners react to a
   clickable mock; if leakage fear doesn't move, cut §15 to tags-only.
5. **Sara gate paper:** F4.1 options + §12/§17/§13 artefacts to a
   security reviewer *before* building; a "no" removes enterprise
   language from positioning (§23) rather than blocking the roadmap.
6. **Small-tank check:** verify statusline covers Pro weekly and
   GLM credit windows; if the feed is Max-5h-only, §1 copy must say
   so per-account — never imply uniform coverage.

## 31. Settings Inventory (single home for every new knob)

New features die when their settings scatter. Every control above
lives in exactly one place; the table below is the contract.

| Setting | Home | Default | PRD |
|---|---|---|---|
| Quota warn thresholds (80/90%) | Account card | 80 / 90 | §1 |
| Forecast low-confidence hide | Usage → Display | Show with band | §1 |
| Inbox snooze durations | Inbox header menu | 1h / 1d | §2 |
| Routing suggestion per project | Project panel | On | §3 |
| Worktree default per project | Project panel | Auto (live session) | §4 |
| Retention: archive after days | Settings → Storage | 30, never auto-delete | §5 |
| Budget cap per account ($) | Account card | Off | §6 |
| Notification modes + quiet hours | Settings → Notifications | Digest hourly | §7 |
| Affinity warning | Settings → Launch | On | §8 |
| Block launch at Admission=No | Settings → Launch | Off | §9 |
| Palette recents count | Settings → Keyboard | 8 | §10 |
| Review auto-surface | Inbox menu | On (read-only) | §11 |
| Backup count + schedule | Settings → Backup | 7, daily + on exit | §12 |
| Update channel + auto-check | Settings → Updates | Beta, on | §13 |
| Sync conflict default | Sync wizard | Keep target | §14 |
| Present mode group | Rail group menu | Off | §15 |
| Template directory | Settings → Templates | app-data | §16 |
| Audit retention | Settings → Storage | Follow §5 | §17 |
| Global hotkey chord | Settings → Keyboard | `⌥⌘J` | §19 |
| Fleet notify-me flag | Settings → Fleet | Off | §20 |

Rules: no setting defaults to a state-changing automation;
every default above is passive. Import/export of settings
travels in the §12 bundle (minus machine identity and fleet keys).

## 32. Empty, Loading & Error States (no silent surfaces)

The parent review's F2.1/F2.4 failures generalise: every new
surface ships all five states or it does not ship.

| Surface | Empty (designed) | Loading | Error (recoverable) |
|---|---|---|---|
| Quota rings | "Start a session to measure" | Skeleton ring | "Feed stale 22m — Retry" banner |
| Inbox | "All clear — N running" | Row skeletons | "Watcher paused — Resume" |
| Search | "No matches — try project name" | Progress >50ms | "Index rebuilding (n%)" |
| Diff queue | "No finished work waiting" | Diff skeleton | "Not a git repo" explained |
| Backup list | "First backup at exit" | — | "Backup failed — Open logs" |
| Fleet card | "This Mac only (v0.1)" | — | Spike status, never fake peers |

`window.alert` is banned for all of the above; one banner/toast
system owns errors app-wide, with [Retry] where a retry exists
and a log link where it does not. First render never shows an
empty state before the first fetch resolves.

## 33. Open Questions for the Maintainer

1. F4.1 credential-copy: amend SPEC S8 (option b + guardrail test)
   or change import behaviour? Blocks §8 copy and §23 trust page.
2. Instances vs Sessions: merge now (this file assumes Sessions +
   Accounts in user copy) or fund a real Instances entity?
3. Pricing source: models.dev sync vs bundled table + manual edit —
   who owns freshness when offline for a month?
4. Worktree root: inside managed dir (clean invariant) vs sibling
   of the repo (friendlier to external tooling)? Pick one; §4 and
   §11 both depend on the answer.
5. Pro Pack boundary: which §6/§15/§17 artefacts are paid without
   gating the core juggle? Decide before v0.1-beta, publish it.

*End. Companion deep-dives: [07-personas](07-personas-and-journeys.md#L1) for journey evidence; parent [full review Roadmap](../full-competitive-ux-review.md#L901) for milestone mapping.*




