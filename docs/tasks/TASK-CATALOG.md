# Task Catalog — All Tasks (generated from epic files, verify with grep)

| ID | Title | Epic file | Pri | Effort | Depends |
|---|---|---|---|---|---|
| AJ-E101 | ADR-0008 terminology charter and rename freeze | E1-terminology-and-naming.md | P0 | S | — |
| AJ-E102 | Phase A — UI copy sweep to Provider/Account/Instance | E1-terminology-and-naming.md | P0 | M | AJ-E101 |
| AJ-E103 | Phase B1 — Domain aliases ProviderId and ProviderAdapter | E1-terminology-and-naming.md | P0 | M | AJ-E101 |
| AJ-E104 | Phase B2 — Split Instance from Session (InstanceRecord + ConversationId) | E1-terminology-and-naming.md | P0 | L | AJ-E103 |
| AJ-E105 | Profile demotion — Launch settings (UI) and LaunchRecipe (domain) | E1-terminology-and-naming.md | P0 | M | AJ-E104 |
| AJ-E106 | DB schema v5 — provider_id / instances / launch_recipes migration | E1-terminology-and-naming.md | P0 | M | AJ-E105 |
| AJ-E107 | Bridge and generated-types rename (commands + TS) | E1-terminology-and-naming.md | P1 | M | AJ-E106 |
| AJ-E108 | Docs sweep and terminology lint gate | E1-terminology-and-naming.md | P1 | S | AJ-E107 |
| AJ-E201 | Switch Graph domain types and append-only storage | E2-switching-engine.md | P0 | M | none |
| AJ-E202 | Portable context pack v1 — schema, deterministic generation, manifest | E2-switching-engine.md | P0 | M | AJ-E201 |
| AJ-E203 | Per-pair fidelity matrix and capped scoring engine | E2-switching-engine.md | P0 | M | AJ-E202 |
| AJ-E204 | HandoverSupport trait and honest Claude transplant rebuild | E2-switching-engine.md | P0 | L | AJ-E203 |
| AJ-E205 | Switch execution pipeline — freeze, stage, verify, launch, rollback | E2-switching-engine.md | P0 | L | AJ-E204 |
| AJ-E206 | Hybrid door — 1-click for proven switches, wizard for lossy ones (D-03) | E2-switching-engine.md | P0 | M | AJ-E205 |
| AJ-E207 | Switch-back resume+delta and both-progressed divergence (D-05) | E2-switching-engine.md | P0 | L | AJ-E206 |
| AJ-E208 | Pack redaction policy — global default plus per-Provider override (D-14) | E2-switching-engine.md | P0 | M | AJ-E202 |
| AJ-E209 | Beta fidelity badges, explainable ranking, and Switch Graph UI | E2-switching-engine.md | P0 | L | AJ-E203, AJ-E206, AJ-E207 |
| AJ-E210 | Fixture + live validation matrix per Provider pair and beta-exit gate (D-04) | E2-switching-engine.md | P0 | XL | AJ-E204, AJ-E205, AJ-E207, AJ-E209 |
| AJ-E301 | Parameterise the wizard by Provider and add the Step-0 install gate | E3-smart-onboarding.md | P0 | M | none |
| AJ-E302 | Widen the scan engine: aliases, cc-switch, installed CLIs, projects, desktop | E3-smart-onboarding.md | P0 | L | AJ-E301 |
| AJ-E303 | Found / Likely / Guessed inference with inline confidence display | E3-smart-onboarding.md | P0 | M | AJ-E302 |
| AJ-E304 | Recommendation engine: one accept-click juggling set | E3-smart-onboarding.md | P0 | M | AJ-E303 |
| AJ-E305 | Flows (a) first install and (b) new project within click budgets | E3-smart-onboarding.md | P0 | L | AJ-E304 |
| AJ-E306 | Flows (c) add Provider, (d) add Account, (e) add Instance | E3-smart-onboarding.md | P0 | L | AJ-E301, AJ-E303 |
| AJ-E307 | Flow (f) import from competitor/manual dirs, provenance and degraded import | E3-smart-onboarding.md | P0 | L | AJ-E302, AJ-E303 |
| AJ-E308 | Login-state taxonomy, auto-detected sign-in queue, and isolation verification | E3-smart-onboarding.md | P0 | L | AJ-E301 |
| AJ-E309 | First-run policy choice (D-14/D-12 hook), expert hatches, and onboarding metrics | E3-smart-onboarding.md | P1 | M | AJ-E305, AJ-E308 |
| AJ-E401 | Enrich the Instance read model (I-01..I-11 data contract) | E4-indicators-and-inspector.md | P0 | L | none |
| AJ-E402 | Render the 11-field indicator row and card variant | E4-indicators-and-inspector.md | P0 | L | AJ-E401 |
| AJ-E403 | Density modes, legend, and the stale-data rule | E4-indicators-and-inspector.md | P1 | M | AJ-E402 |
| AJ-E404 | Needs-me-first sorting, filtering, search, and snooze | E4-indicators-and-inspector.md | P0 | M | AJ-E402,AJ-E403 |
| AJ-E405 | Emit Stuck, WaitingUser, and QuotaBlocked (needs-attention upstream) | E4-indicators-and-inspector.md | P0 | L | AJ-E401 |
| AJ-E406 | Right Inspector shell: 6 tabs x 5 selection types | E4-indicators-and-inspector.md | P0 | L | AJ-E401 |
| AJ-E407 | Inspector content for Instance and Session selections | E4-indicators-and-inspector.md | P0 | L | AJ-E406,AJ-E401 |
| AJ-E408 | Inspector content for Provider, Account, and Project selections | E4-indicators-and-inspector.md | P0 | L | AJ-E406,AJ-E401 |
| AJ-E409 | Quality signals without a score: tokens/min, errors, context, stuck evidence | E4-indicators-and-inspector.md | P0 | L | AJ-E406,AJ-E405 |
| AJ-E410 | Processes tab, resource collector, and orphan (left-behind) contract | E4-indicators-and-inspector.md | P0 | L | AJ-E406,AJ-E401 |
| AJ-E411 | Subscription-expiry warning hook (manual renewal date, D-12) | E4-indicators-and-inspector.md | P1 | M | AJ-E401,AJ-E408 |
| AJ-E412 | Running-model telemetry (what the Instance is actually running) | E4-indicators-and-inspector.md | P1 | M | AJ-E401,AJ-E407 |
| AJ-E413 | Desktop adapter contract — launch and track a windowed harness | E4-indicators-and-inspector.md | P0 | L | AJ-E406 |
| AJ-E414 | Focus Window service — go to that exact instance's window | E4-indicators-and-inspector.md | P0 | M | AJ-E413 |
| AJ-E415 | Desktop live preview — read-only window mirror in the panel | E4-indicators-and-inspector.md | P1 | L | AJ-E413, AJ-E414 |
| AJ-E501 | Persistent spawn ledger with intent-before-spawn | E5-process-ownership-and-cleanup.md | P0 | L | none |
| AJ-E502 | Process-group ownership at spawn (Unix PGID, Windows seam) | E5-process-ownership-and-cleanup.md | P0 | M | AJ-E501 |
| AJ-E503 | ProcessIdentity and fail-closed verify-before-kill | E5-process-ownership-and-cleanup.md | P0 | M | AJ-E501 |
| AJ-E504 | Reconciler loop, orphan detection, and children-map eviction | E5-process-ownership-and-cleanup.md | P0 | L | AJ-E501, AJ-E502, AJ-E503 |
| AJ-E505 | Port occupancy map and leftover inventory data | E5-process-ownership-and-cleanup.md | P0 | M | AJ-E504 |
| AJ-E506 | Cleanup Centre (read-only) and per-Instance Processes view | E5-process-ownership-and-cleanup.md | P0 | L | AJ-E504, AJ-E505 |
| AJ-E507 | Verify-before-kill sheet and manual TERM→KILL ladder | E5-process-ownership-and-cleanup.md | P0 | L | AJ-E503, AJ-E506 |
| AJ-E508 | Cleanup tiers Tier 0-3: notify-only default, per-Account opt-in | E5-process-ownership-and-cleanup.md | P0 | M | AJ-E507, AJ-E509 |
| AJ-E509 | Append-only cleanup audit log (intent before signal) | E5-process-ownership-and-cleanup.md | P0 | M | AJ-E501 |
| AJ-E510 | Juggler-crash recovery: re-adoption with stop control | E5-process-ownership-and-cleanup.md | P0 | L | AJ-E501, AJ-E503, AJ-E504 |
| AJ-E511 | Fleet-shaped ownership: machine identity on every process record | E5-process-ownership-and-cleanup.md | P1 | M | AJ-E501 |
| AJ-E512 | False-kill collateral fixtures, hog alerts, and gate evidence | E5-process-ownership-and-cleanup.md | P0 | M | AJ-E507, AJ-E508, AJ-E510 |
| AJ-E601 | Sanctioned-source proof protocol for Provider billing & renewal data | E6-quota-cost-and-billing-research.md | P0 | M | none |
| AJ-E602 | Claude (Anthropic) billing, quota & renewal-date investigation | E6-quota-cost-and-billing-research.md | P0 | M | AJ-E601 |
| AJ-E603 | OpenAI / Codex billing, quota & renewal-date investigation | E6-quota-cost-and-billing-research.md | P0 | M | AJ-E601 |
| AJ-E604 | ZCode (Z.ai / GLM) billing, credit & renewal-date investigation | E6-quota-cost-and-billing-research.md | P0 | M | AJ-E601 |
| AJ-E605 | Gemini CLI billing, quota & renewal-date investigation | E6-quota-cost-and-billing-research.md | P1 | M | AJ-E601 |
| AJ-E606 | Two-clock quota engine: window maths + subscription clock domain model | E6-quota-cost-and-billing-research.md | P0 | L | AJ-E602, AJ-E603, AJ-E604 |
| AJ-E607 | Ingest pipeline: statusline spool, transcript, rollout & SQLite sources | E6-quota-cost-and-billing-research.md | P0 | L | AJ-E606, AJ-E602, AJ-E603, AJ-E604, AJ-E605 |
| AJ-E608 | Burn-rate forecast: EWMA, time-to-exhaust & confidence inheritance | E6-quota-cost-and-billing-research.md | P0 | M | AJ-E606, AJ-E607 |
| AJ-E609 | Cost ledger: USD pricing, per-model attribution & subscription value | E6-quota-cost-and-billing-research.md | P1 | M | AJ-E606, AJ-E607 |
| AJ-E610 | Quota & expiry display: confidence labels, two-clock indicators, manual renewal entry (D-12) | E6-quota-cost-and-billing-research.md | P0 | L | AJ-E606, AJ-E607, AJ-E608, AJ-E609, AJ-E602, AJ-E603, AJ-E604, AJ-E605 |
| AJ-E611 | Smart destination ranking inputs & quota alerts / reset-queue integration | E6-quota-cost-and-billing-research.md | P1 | M | AJ-E608, AJ-E609, AJ-E610 |
| AJ-E701 | Fleet architecture ADR and threat model | E7-fleet-at-launch.md | P0 | M | none |
| AJ-E702 | Machine identity and pairing trust store | E7-fleet-at-launch.md | P0 | M | AJ-E701 |
| AJ-E703 | Transport and security model (gRPC/mTLS, versioned protobuf) | E7-fleet-at-launch.md | P0 | L | AJ-E701, AJ-E702 |
| AJ-E704 | Discovery and pairing wizard (mDNS + code/QR) | E7-fleet-at-launch.md | P0 | M | AJ-E702, AJ-E703 |
| AJ-E705 | Remote inspector (fleet-shaped right inspector) | E7-fleet-at-launch.md | P0 | L | AJ-E703 |
| AJ-E706 | Cross-machine switching implications | E7-fleet-at-launch.md | P0 | L | AJ-E703, AJ-E705 |
| AJ-E707 | Remote process ownership and cleanup | E7-fleet-at-launch.md | P0 | L | AJ-E703, AJ-E705 |
| AJ-E708 | Offline and conflict behaviour | E7-fleet-at-launch.md | P0 | L | AJ-E703 |
| AJ-E709 | Fleet indicators, permissions, and alert routing | E7-fleet-at-launch.md | P1 | M | AJ-E704, AJ-E705 |
| AJ-E710 | Fleet beta gate, dogfood, and security evidence pack | E7-fleet-at-launch.md | P0 | M | AJ-E702, AJ-E703, AJ-E704, AJ-E705, AJ-E706, AJ-E707, AJ-E708, AJ-E709 |
| AJ-E711 | Remote Focus Window — go to an instance's window on another machine | E7-fleet-at-launch.md | P1 | M | AJ-E705, AJ-E414 |
| AJ-E801 | Evidence store layout and done-proof contract | E8-validation-visual-performance-quality.md | P0 | M | none |
| AJ-E802 | Screenshot harness and states gallery | E8-validation-visual-performance-quality.md | P0 | L | AJ-E801 |
| AJ-E803 | Visual regression thresholds and review workflow | E8-validation-visual-performance-quality.md | P0 | M | AJ-E802 |
| AJ-E804 | Accessibility verification programme | E8-validation-visual-performance-quality.md | P0 | M | AJ-E802 |
| AJ-E805 | Performance budgets and load proof (10 Accounts / 50 Instances) | E8-validation-visual-performance-quality.md | P0 | L | AJ-E801 |
| AJ-E806 | Quality gates and CI enforcement | E8-validation-visual-performance-quality.md | P0 | M | AJ-E801 |
| AJ-E807 | Milestone gate verification and sign-off | E8-validation-visual-performance-quality.md | P0 | M | AJ-E803, AJ-E804, AJ-E805, AJ-E806 |
| AJ-E808 | Provider-pair fidelity validation matrix (operations) | E8-validation-visual-performance-quality.md | P0 | L | AJ-E801 |
| AJ-E809 | Beta dogfood programme | E8-validation-visual-performance-quality.md | P1 | M | AJ-E807, AJ-E808 |
| AJ-E810 | External research evidence standard | E8-validation-visual-performance-quality.md | P1 | S | AJ-E801 |
| AJ-E901 | SPEC §3 domain-model rewrite (Provider/Instance/Switch Graph/ledger) | E9-docs-adrs-and-plan-rewrite.md | P0 | M | — |
| AJ-E902 | SPEC §5 supervisor/ledger + §6 two-clock usage rewrite | E9-docs-adrs-and-plan-rewrite.md | P0 | M | AJ-E901 |
| AJ-E903 | SPEC §7 Switching rewrite (Switch Graph, pack v2, divergence) | E9-docs-adrs-and-plan-rewrite.md | P0 | M | AJ-E901 |
| AJ-E904 | SPEC §9 UX rewrite (Inspector primary, indicators, Cleanup Centre) | E9-docs-adrs-and-plan-rewrite.md | P0 | M | AJ-E901 |
| AJ-E905 | SPEC §11 Fleet rewrite (Fleet at launch) + §12.1 onboarding generalisation | E9-docs-adrs-and-plan-rewrite.md | P0 | M | AJ-E901, AJ-E902 |
| AJ-E906 | PROPOSAL thesis + EPICS resequence rewrite | E9-docs-adrs-and-plan-rewrite.md | P0 | M | AJ-E901, AJ-E902, AJ-E903, AJ-E904, AJ-E905 |
| AJ-E907 | ADR bundle 0008-0013 drafting and acceptance | E9-docs-adrs-and-plan-rewrite.md | P0 | M | AJ-E901 |
| AJ-E908 | README truth fix + beta-exit criteria document | E9-docs-adrs-and-plan-rewrite.md | P1 | S | AJ-E906 |
| AJ-E909 | Provider billing/renewal API research log (Stage-1 record) | E9-docs-adrs-and-plan-rewrite.md | P1 | M | — |
| AJ-E910 | Cross-doc consistency proof and Stage-1 acceptance packet | E9-docs-adrs-and-plan-rewrite.md | P0 | S | AJ-E901, AJ-E902, AJ-E903, AJ-E904, AJ-E905, AJ-E906, AJ-E907, AJ-E908, AJ-E909 |
| AJ-E1001 | Per-Instance identity assertion (wrong-account billing guard) | E10-derived-workflows-and-beta.md | P0 | M | — |
| AJ-E1002 | Git worktree isolation for parallel Instances | E10-derived-workflows-and-beta.md | P0 | L | AJ-E1001 |
| AJ-E1003 | Quota reset queue and parked continuation | E10-derived-workflows-and-beta.md | P1 | M | AJ-E1001 |
| AJ-E1004 | Overnight and unattended run supervision | E10-derived-workflows-and-beta.md | P1 | M | AJ-E1003 |
| AJ-E1005 | Morning triage and multi-project day board | E10-derived-workflows-and-beta.md | P1 | M | AJ-E1004 |
| AJ-E1006 | Pair review and model arbitrage workflows [inferred — falsify first] | E10-derived-workflows-and-beta.md | P2 | M | AJ-E1005 |
| AJ-E1007 | Session lifecycle: search, archive, replay, retention | E10-derived-workflows-and-beta.md | P1 | M | AJ-E1005 |
| AJ-E1008 | Provider outage failover day [inferred — falsify first] | E10-derived-workflows-and-beta.md | P2 | M | AJ-E1006 |
| AJ-E1009 | Backup, restore and machine migration of Juggler's own data | E10-derived-workflows-and-beta.md | P1 | M | AJ-E1007 |
| AJ-E1010 | Updates and release integrity for beta | E10-derived-workflows-and-beta.md | P1 | M | AJ-E1009 |
| AJ-E1011 | Team pack export and colleague handoff | E10-derived-workflows-and-beta.md | P2 | M | AJ-E1007, AJ-E1009 |

Total: 107 tasks. Regenerate/verify: `grep -h '^### AJ-' docs/tasks/E*.md | wc -l` must equal this count.
Status of every task at generation: TODO. The epic files are canonical; this catalog is an index, never edited to claim progress.
