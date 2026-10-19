# Evidence manifest — AJ-E1103

Task: `docs/tasks/E11-usage-statistics.md` AJ-E1103 (wire the events into
the app). 2026-10-18, reviewer claude-opus. Test log:
[`cargo-test-usage-stats.txt`](cargo-test-usage-stats.txt).

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Every schema metric has an emitting site or a documented "not emitted yet" reason; Provider ids map to the schema (`other` otherwise), models to `model_family` | Req.1 | `usage_stats::SITES` lists all 77; `every_metric_is_accounted_for_and_its_site_exists` checks the list equals the schema, finds a real `count`/`observe` call (backend), `statsEvent`/`statsGauge` call (interface) or `StatGauge::…` (gauge) for each claimed site, and **no** call for the 3 marked not yet (`auto_restart.enabled`, `instance.auto_restarted`, `context_sync.applied`). `mappings_keep_only_schema_words` (an unknown CLI → `other`; a private model name → `other`) | PASS | 2026-10-18 | claude-opus |
| Gauges snapshotted when a week closes, from the read model, through the bucket helpers | Req.2 | `enabled::gauges` runs inside the tick's week close; counts are `CountBucket` buckets, CLI versions `major.minor` or `other`. The live upload test sees them in the report (`projects.count` = `0`) | PASS | 2026-10-18 | claude-opus |
| Token and minute totals only where Measured, rounded at assembly | Req.3 | `session.tokens`: only rows the store newly accepted (`insert_events_marking`, so a re-read never double counts) and `Confidence::Measured`; minutes are the Instance's own measured run time; both rounded to two significant figures in assembly (AJ-E1104) | PASS | 2026-10-18 | claude-opus |
| No site passes anything derived from a path, label, name, a setting value outside an enum, or error text | Req.4 | Every site passes `&'static str` schema words or a mapped Provider; `StatEvent::parse` drops anything else (debug builds assert). `error_classes_come_from_the_variant_never_the_text` (a secret-shaped message and a home path don't change the class); `CoreError::stats_code` reads SQLite's typed result code. The interface can count only `UI_METRICS` and only schema values: `the_interface_cannot_widen_what_is_counted` (a schema metric the interface doesn't own, a path as a value, an extra dimension and an unknown theme: none stored) | PASS | 2026-10-18 | claude-opus |
| Zero measurable cost in off; ≤ 1 µs per event otherwise | perf budget | Every site's first check is `recording()` (a lock read); look-ups (a recipe's Provider, memory before a kill, the hourly self-sample) run only while recording. `record()` 37–50 ns (AJ-E1102) plus `parse` | PASS | 2026-10-18 | claude-opus |
| Interface events | quality gate | `ui/src/lib/usage-stats-events.test.tsx`: a suggestion counts as shown then accepted, or dismissed when another target is picked, and nothing without one. Full vitest and Playwright suites green | PASS | 2026-10-18 | claude-opus |

## Every metric and where it is counted

| Metric | Counted in | Where |
|---|---|---|
| `accounts.configured` | gauge (week close) | Accounts per Provider |
| `provider.cli_version` | gauge (week close) | `--version` per Provider, once a week |
| `launch_settings.count` | gauge (week close) | launch settings per Provider |
| `projects.count` | gauge (week close) | projects |
| `fleet.role` | gauge (week close) | none / node (Link on) / console |
| `fleet.paired_machines` | gauge (week close) | paired Macs |
| `cleanup.tier` | gauge (week close) | Accounts opted in to safe cleanup, and the rest |
| `auto_restart.enabled` | **not emitted yet** | AI Juggler has no auto-restart setting yet |
| `pack.redaction_policy` | gauge (week close) | the global redaction choice |
| `renewal_dates.entered` | gauge (week close) | Accounts with a renewal date |
| `worktree.isolation_enabled` | gauge (week close) | on once Juggler has made a worktree |
| `update.channel` | gauge (week close) | the build's channel |
| `update.auto_check` | gauge (week close) | check on launch |
| `ui.density` | gauge (week close) | the interface, on change |
| `ui.theme` | gauge (week close) | the interface, on change |
| `crash_reports.setting` | gauge (week close) | `never` until AJ-E1107 adds crash reports |
| `instance.peak_concurrent` | gauge (week close) | the 5 s resource sample, raised as it grows |
| `instance.started` | backend | every place an Instance row is created (7) |
| `instance.ended` | backend | commands::apply_lifecycle_event |
| `instance.minutes` | backend | commands::apply_lifecycle_event |
| `instance.state_entered` | backend | apply_attention_transition; quota exits |
| `instance.auto_restarted` | **not emitted yet** | AI Juggler doesn't auto-restart Instances yet |
| `instance.running_model` | backend | commands::persist_running_model (family only) |
| `instance.wrong_account_blocked` | backend | identity::assert_identity |
| `session.tokens` | backend | usage_pipeline: new Measured rows only |
| `switch.started` | backend | commands::switch_execute_core |
| `switch.completed` | backend | switch_execute_core; switch_undo_impl |
| `switch.failed_stage` | backend | commands::switch_execute_core |
| `switch.back` | backend | commands::switch_back_execute_impl |
| `switch.divergence_shown` | backend | commands::switch_back_plan_impl |
| `switch.duration_s` | backend | commands::switch_execute_core |
| `pack.size_kb` | backend | commands::pack_export_impl |
| `pack.redaction_reviewed` | backend | commands::record_redaction_decision |
| `context_sync.applied` | **not emitted yet** | there is no context-sync operation yet |
| `usage.wall_hit` | backend | commands::step_account_alerts |
| `usage.forecast_shown` | backend | the quota_display command (Usage tab) |
| `usage.suggestion` | interface | SwitchTargetRows |
| `usage.threshold_alert` | backend | commands::step_account_alerts |
| `usage.parse_degraded` | backend | usage_pipeline::record_health |
| `renewal.warning_shown` | backend | commands::step_account_alerts |
| `process.children_per_instance` | backend | the 5 s resource sample, hourly |
| `leftover.detected` | backend | the orphan tracker's confirmations |
| `leftover.cleaned` | backend | kill_execute_as; orphan_stop_audited |
| `leftover.verify_refused` | backend | kill_execute_as; orphan_stop_audited |
| `cleanup.memory_freed_mb` | backend | kill_execute_as; orphan_stop_audited |
| `cleanup.ports_freed` | backend | kill_execute_as; orphan_stop_audited |
| `resource.alert` | backend + interface | cleanup_alerts_inner (shown); Cleanup Centre (acted) |
| `cleanup_centre.opened` | interface | App |
| `onboarding.scan_completed` | backend | commands::environment_scan_full_impl |
| `onboarding.found` | backend | commands::environment_scan_full_impl |
| `onboarding.source_detected` | backend | commands::environment_scan_full_impl |
| `onboarding.proposal` | backend | the funnel event the wizard records |
| `onboarding.import` | backend | the account_import command |
| `onboarding.login` | interface | OnboardingWizard |
| `onboarding.expert_opened` | backend | the funnel event |
| `onboarding.first_launch_s` | backend | the funnel's first_launch |
| `onboarding.flow` | backend | the funnel's flow steps |
| `fleet.pairing` | backend | link: host and joiner outcomes |
| `fleet.remote_action` | backend | remote::send_action; fleet_switch_execute |
| `fleet.link_degraded` | backend | link: adopt_connection; a dropped link |
| `ui.screen_opened` | interface | App |
| `ui.inspector_tab` | interface | RightInspector |
| `ui.rail_tab` | interface | App |
| `ui.notification` | interface | NotificationRegion |
| `ui.shortcut_used` | interface | App; RightInspector |
| `app.launch` | backend | usage_stats::start |
| `app.recovered_after_crash` | backend | usage_stats::start (the recovery pass) |
| `error.class` | backend | AppError crossing to the interface |
| `update.flow` | backend + interface | updates (shown, installed, failed); the notice (deferred) |
| `storage.migration` | backend | usage_stats::start (opening the database) |
| `stats.dropped` | backend | juggler-stats: the recorder's flush |
| `stats.truncated` | backend | juggler-stats: assembly |
| `stats.invalid_report` | backend | juggler-stats: assembly |
| `perf.cold_start_ms` | backend | usage_stats::start |
| `perf.board_interactive_ms` | interface | App: the first board load |
| `perf.rss_peak_mb` | backend | the hourly tick: this app's memory |
| `perf.cpu_overhead_pct` | backend | the hourly tick: this app's CPU |

Notes:

- `onboarding.flow`: first install, new project and add Provider are
  counted; add Account, add Instance and import have no flow of their
  own in the interface yet.
- `leftover.verify_refused`: refusals are counted as
  `ownership_unverified`; the kill path doesn't yet tell a reused PID or
  a gone process apart.
- `fleet.role` is `node` while Link serves this Mac, `console` when it
  has paired Macs with Link off, `none` without peers.
- A switch's trigger comes from the target the user picked (the
  suggested one → `suggestion`, any other → `manual`).
