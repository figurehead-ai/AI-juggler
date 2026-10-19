# Usage statistics

AI Juggler can tell its maintainers which features people use. It does
this only if you choose to, and never by learning anything about your
work. This page lists **everything** it can ever send. The table is
generated from the schema the app and the server both enforce
(`stats/schema.v1.json`), so it can't drift from the code.

## The three modes

| Mode | What happens |
|---|---|
| **Off** (the default) | Nothing is counted, nothing is stored, and no request is made. There is no stats folder. |
| **Keep stats on this Mac only** | AI Juggler counts which features you use so you can see it in Settings ▸ Privacy. Nothing is sent. |
| **Share weekly usage stats** | Once a week, AI Juggler sends one small report of those counts. |

You choose in onboarding (no option is preselected) or at any time in
Settings ▸ Privacy. Switching to Off deletes everything stored locally.

## How a report works

- Counting happens on your Mac. Only weekly totals exist; individual
  actions are never stored or sent.
- One report covers one ISO week (UTC). It is sent at a random time in
  the 6 days after the week ends, so reports from many Macs spread out.
- A week is sent only if you were sharing for all of it. Data counted
  before you opted in is never sent.
- A report carries **no identifier**: no user, account, machine or
  install id. Its random report id is used once, to drop duplicates, so
  two reports from the same Mac can't be linked.
- Counts are rounded (for example, Account counts go into buckets like
  `4-5`; token totals keep two significant figures). Durations and sizes
  are only buckets.
- You can read every report before it is sent, as the exact JSON, in
  Settings ▸ Privacy.
- The service keeps raw reports for 35 days and only aggregates after
  that. Because a report has no id, a report that was already sent can't
  be found and deleted later.

## Never collected

- Prompts, responses, transcripts, code, diffs, file or folder names,
  paths, git remotes or branch names.
- Project names, Account labels, Launch-settings names, machine names,
  host names, user names, emails.
- Anything read from, derived from or hashed from a vendor's credential
  or identity files, including the account ids in `~/.claude.json` and
  `~/.codex/auth.json`.
- Environment variables, setting values beyond the choices listed
  below, command lines, ports, process ids.
- Error messages, stack traces and log lines. Errors are only a fixed
  code from the list below.
- IP addresses (the service doesn't read or store them), and identifiers
  of any kind.

The app enforces this in its types: every value in a report is one of
the choices listed below or a bounded number, so free text can't be
sent. Tests check it on every build.

## Turning it off, or keeping it out entirely

- **Settings ▸ Privacy ▸ Off.** This also deletes local stats.
- **`DO_NOT_TRACK=1`** in your environment forces it off, along with
  crash reports.
- **`AI_JUGGLER_STATS=off`** forces it off; **`AI_JUGGLER_STATS=local`**
  never lets it go beyond local-only.
- **Builds from source:** `cargo build -p juggler-app --no-default-features`
  leaves usage statistics out of the app entirely.

Debug builds and CI never upload.

## Showing your own stats on figurehead.ai (optional)

If you share usage stats **and** this Mac is linked to a Figurehead
account, Settings ▸ Privacy offers "Show my stats in my Figurehead
dashboard". With it ticked, the weekly report goes to your account
(where only you can see it, and it is deleted with your account)
**instead of** the anonymous upload, never as well.

## Crash reports

Crash reports have their own setting: never (the default), ask (you see
the report and choose), or always. They are scrubbed on your Mac first:
your home folder becomes `~`, and no user name, environment or URLs are
included.

## A sample report

```json
{
  "schema": 1,
  "report_id": "0b6f2d4e-6a3c-4f0e-9d7b-3c1e5a9f8b21",
  "week": "2026-W42",
  "sample_rate": 1.0,
  "app": { "version": "0.3.1", "channel": "beta" },
  "os": { "name": "macos", "major": 15, "arch": "arm64" },
  "install_age": "1-4w",
  "days_active": 5,
  "consent_version": 1,
  "entries": [
    { "m": "instance.started", "d": { "provider": "claude-code", "kind": "terminal", "origin": "new" }, "v": 14 },
    { "m": "switch.duration_s", "d": { "kind": "provider" }, "h": [0, 2, 1, 0] },
    { "m": "accounts.configured", "d": { "provider": "claude-code" }, "g": "2" }
  ]
}
```

## Every metric

Each report carries the app version and channel, the OS name, major
version and chip architecture, how long ago AI Juggler was installed (as
a bucket), how many days that week it ran (0–7), the consent version and
the sampling rate. The entries below are the only other things it can
contain.

### Environment

| Metric | Kind | Dimensions | Value |
|---|---|---|---|
| `accounts.configured` | gauge | `provider`: `claude-code` · `codex` · `zcode` · `other` | `0` · `1` · `2` · `3` · `4-5` · `6-9` · `10+` |
| `provider.cli_version` | gauge | `provider`: `claude-code` · `codex` · `zcode` · `other` | `major.minor` of the CLI, or `other` |
| `launch_settings.count` | gauge | `provider`: `claude-code` · `codex` · `zcode` · `other` | `0` · `1` · `2` · `3` · `4-5` · `6-9` · `10+` |
| `projects.count` | gauge | — | `0` · `1` · `2` · `3` · `4-5` · `6-9` · `10+` |
| `auto_restart.enabled` | gauge | — | `0` · `1` · `2` · `3` · `4-5` · `6-9` · `10+` |
| `renewal_dates.entered` | gauge | — | `0` · `1` · `2` · `3` · `4-5` · `6-9` · `10+` |
| `worktree.isolation_enabled` | gauge | — | `on` · `off` |
| `crash_reports.setting` | gauge | — | `never` · `ask` · `always` |

### Instances and sessions

| Metric | Kind | Dimensions | Value |
|---|---|---|---|
| `instance.started` | counter | `provider`: `claude-code` · `codex` · `zcode` · `other`; `kind`: `terminal` · `desktop`; `origin`: `new` · `resume` · `switch` · `relaunch` | a weekly count (at most 10,000) |
| `instance.ended` | counter | `provider`: `claude-code` · `codex` · `zcode` · `other`; `outcome`: `finished` · `stopped` · `failed` · `crashed` | a weekly count (at most 10,000) |
| `instance.minutes` | counter | `provider`: `claude-code` · `codex` · `zcode` · `other` | a weekly count (at most 600,000) |
| `instance.peak_concurrent` | gauge | — | `0` · `1` · `2` · `3` · `4-5` · `6-9` · `10+` |
| `instance.state_entered` | counter | `provider`: `claude-code` · `codex` · `zcode` · `other`; `state`: `waiting_user` · `stuck` · `quota_blocked` | a weekly count (at most 100,000) |
| `instance.auto_restarted` | counter | `provider`: `claude-code` · `codex` · `zcode` · `other` | a weekly count (at most 10,000) |
| `instance.running_model` | counter | `provider`: `claude-code` · `codex` · `zcode` · `other`; `model_family`: `opus` · `sonnet` · `haiku` · `gpt-5` · `gpt-5-codex` · `gpt-5-mini` · `glm-4` · `glm-5` · `other` | a weekly count (at most 10,000) |
| `instance.wrong_account_blocked` | counter | `provider`: `claude-code` · `codex` · `zcode` · `other` | a weekly count (at most 10,000) |
| `session.tokens` | counter | `provider`: `claude-code` · `codex` · `zcode` · `other`; `direction`: `input` · `output` · `cache` | a weekly count (at most 10,000,000,000) |

### Switching

| Metric | Kind | Dimensions | Value |
|---|---|---|---|
| `pack.redaction_policy` | gauge | — | `redact` · `keep` · `ask` · `other` |
| `switch.started` | counter | `kind`: `account` · `provider` · `instance`; `trigger`: `quota_wall` · `suggestion` · `manual` · `remote` | a weekly count (at most 10,000) |
| `switch.completed` | counter | `kind`: `account` · `provider` · `instance`; `from`: `claude-code` · `codex` · `zcode` · `other`; `to`: `claude-code` · `codex` · `zcode` · `other`; `mechanism`: `native_resume` · `transplant` · `pack_replay`; `trigger`: `quota_wall` · `suggestion` · `manual` · `remote`; `outcome`: `success` · `rolled_back` · `failed` | a weekly count (at most 10,000) |
| `switch.failed_stage` | counter | `kind`: `account` · `provider` · `instance`; `stage`: `freeze` · `stage` · `verify` · `launch` | a weekly count (at most 10,000) |
| `switch.back` | counter | `kind`: `account` · `provider` · `instance` | a weekly count (at most 10,000) |
| `switch.divergence_shown` | counter | — | a weekly count (at most 10,000) |
| `switch.duration_s` | histogram | `kind`: `account` · `provider` · `instance` | counts per bucket: `<5` · `5-15` · `15-60` · `60+` |
| `pack.size_kb` | histogram | — | counts per bucket: `<16` · `16-64` · `64-256` · `256+` |
| `pack.redaction_reviewed` | counter | `outcome`: `kept` · `changed` | a weekly count (at most 10,000) |
| `context_sync.applied` | counter | — | a weekly count (at most 10,000) |

### Usage clocks

| Metric | Kind | Dimensions | Value |
|---|---|---|---|
| `usage.wall_hit` | counter | `provider`: `claude-code` · `codex` · `zcode` · `other`; `window`: `5h` · `weekly` · `credits` | a weekly count (at most 10,000) |
| `usage.forecast_shown` | counter | `provider`: `claude-code` · `codex` · `zcode` · `other` | a weekly count (at most 100,000) |
| `usage.suggestion` | counter | `provider`: `claude-code` · `codex` · `zcode` · `other`; `action`: `shown` · `accepted` · `dismissed` | a weekly count (at most 100,000) |
| `usage.threshold_alert` | counter | `provider`: `claude-code` · `codex` · `zcode` · `other`; `level`: `warn` · `critical` | a weekly count (at most 10,000) |
| `usage.parse_degraded` | counter | `provider`: `claude-code` · `codex` · `zcode` · `other`; `area`: `transcript` · `quota_feed` | a weekly count (at most 10,000) |
| `renewal.warning_shown` | counter | — | a weekly count (at most 10,000) |

### Processes and cleanup

| Metric | Kind | Dimensions | Value |
|---|---|---|---|
| `cleanup.tier` | gauge | `tier`: `notify_only` · `clean_safe` | `0` · `1` · `2` · `3` · `4-5` · `6-9` · `10+` |
| `process.children_per_instance` | histogram | — | counts per bucket: `0` · `1-2` · `3-5` · `6+` |
| `leftover.detected` | counter | `category`: `dev_server` · `mcp` · `browser` · `watcher` · `other` | a weekly count (at most 100,000) |
| `leftover.cleaned` | counter | `category`: `dev_server` · `mcp` · `browser` · `watcher` · `other`; `mode`: `manual` · `auto` | a weekly count (at most 10,000) |
| `leftover.verify_refused` | counter | `reason`: `ownership_unverified` · `pid_reused` · `gone` | a weekly count (at most 10,000) |
| `cleanup.memory_freed_mb` | counter | — | a weekly count (at most 10,000,000) |
| `cleanup.ports_freed` | counter | — | a weekly count (at most 100,000) |
| `resource.alert` | counter | `kind`: `cpu_hog` · `memory_hog` · `idle`; `action`: `shown` · `acted` | a weekly count (at most 10,000) |
| `cleanup_centre.opened` | counter | — | a weekly count (at most 10,000) |

### Onboarding

| Metric | Kind | Dimensions | Value |
|---|---|---|---|
| `onboarding.scan_completed` | counter | — | a weekly count (at most 10,000) |
| `onboarding.found` | counter | `provider`: `claude-code` · `codex` · `zcode` · `other`; `provenance`: `found` · `likely` · `guessed` | a weekly count (at most 10,000) |
| `onboarding.source_detected` | counter | `source`: `vendor_default` · `env_override` · `alias` · `cc_switch` · `t3code` · `other_manager` | a weekly count (at most 10,000) |
| `onboarding.proposal` | counter | `action`: `shown` · `accepted` · `customised` | a weekly count (at most 10,000) |
| `onboarding.import` | counter | `provider`: `claude-code` · `codex` · `zcode` · `other`; `outcome`: `completed` · `failed`; `reason_code`: `none` · `permission_denied` · `source_missing` · `copy_failed` · `unsupported_layout` · `verify_failed` · `other` | a weekly count (at most 10,000) |
| `onboarding.login` | counter | `provider`: `claude-code` · `codex` · `zcode` · `other`; `outcome`: `completed` · `skipped` · `expired` | a weekly count (at most 10,000) |
| `onboarding.expert_opened` | counter | — | a weekly count (at most 10,000) |
| `onboarding.first_launch_s` | histogram | — | counts per bucket: `<60` · `60-300` · `300-900` · `900+` |
| `onboarding.flow` | counter | `flow`: `first_install` · `new_project` · `add_provider` · `add_account` · `add_instance` · `import` | a weekly count (at most 10,000) |

### Fleet

| Metric | Kind | Dimensions | Value |
|---|---|---|---|
| `fleet.role` | gauge | — | `none` · `console` · `node` |
| `fleet.paired_machines` | gauge | — | `0` · `1` · `2` · `3` · `4-5` · `6-9` · `10+` |
| `fleet.pairing` | counter | `outcome`: `completed` · `failed` · `expired`; `reason`: `none` · `code_invalid` · `code_expired` · `network` · `version` · `tls` · `denied` · `other` | a weekly count (at most 10,000) |
| `fleet.remote_action` | counter | `kind`: `view` · `switch` · `cleanup`; `outcome`: `ok` · `denied` · `failed` | a weekly count (at most 10,000) |
| `fleet.link_degraded` | counter | `reason`: `version` · `network` | a weekly count (at most 10,000) |

### Interface

| Metric | Kind | Dimensions | Value |
|---|---|---|---|
| `ui.density` | gauge | — | `comfortable` · `compact` · `minimal` |
| `ui.theme` | gauge | — | `system` · `light` · `dark` |
| `ui.screen_opened` | counter | `screen`: `overview` · `accounts` · `usage` · `fleet` · `cleanup` · `settings` | a weekly count (at most 100,000) |
| `ui.inspector_tab` | counter | `selection`: `session` · `provider` · `account` · `instance` · `project`; `tab`: `overview` · `usage` · `history` · `quality` · `processes` · `actions` | a weekly count (at most 100,000) |
| `ui.rail_tab` | counter | `tab`: `projects` · `instances` · `sessions` | a weekly count (at most 100,000) |
| `ui.notification` | counter | `kind`: `quota_warning` · `quota_wall` · `waiting_user` · `stuck` · `leftover` · `renewal` · `update` · `fleet` · `other`; `action`: `shown` · `clicked` · `dismissed` | a weekly count (at most 100,000) |
| `ui.shortcut_used` | counter | `shortcut`: `toggle_rail` · `new_session` · `command_palette` · `switch` · `focus_window` · `inspector_tab` · `search` · `other` | a weekly count (at most 100,000) |

### Reliability

| Metric | Kind | Dimensions | Value |
|---|---|---|---|
| `update.channel` | gauge | — | `beta` · `stable` |
| `update.auto_check` | gauge | — | `on` · `off` |
| `app.launch` | counter | — | a weekly count (at most 10,000) |
| `app.recovered_after_crash` | counter | — | a weekly count (at most 10,000) |
| `error.class` | counter | `area`: `adapter` · `supervisor` · `storage` · `switch` · `fleet` · `updater` · `ui`; `code`: `parse_failed` · `io_error` · `permission_denied` · `timeout` · `not_found` · `version_unsupported` · `db_locked` · `db_corrupt` · `network` · `signature_invalid` · `spawn_failed` · `pty_failed` · `invalid_state` · `other` | a weekly count (at most 10,000) |
| `update.flow` | counter | `step`: `prompt_shown` · `installed` · `deferred` · `failed`; `reason`: `none` · `network` · `signature` · `checksum` · `disk` · `other` | a weekly count (at most 10,000) |
| `storage.migration` | counter | `outcome`: `ok` · `failed` | a weekly count (at most 10,000) |
| `stats.dropped` | counter | — | a weekly count (at most 10,000) |
| `stats.truncated` | counter | — | a weekly count (at most 10,000) |
| `stats.invalid_report` | counter | — | a weekly count (at most 10,000) |

### Performance

| Metric | Kind | Dimensions | Value |
|---|---|---|---|
| `perf.cold_start_ms` | histogram | — | counts per bucket: `<750` · `750-1500` · `1500-3000` · `3000+` |
| `perf.board_interactive_ms` | histogram | — | counts per bucket: `<250` · `250-500` · `500-1000` · `1000+` |
| `perf.rss_peak_mb` | histogram | — | counts per bucket: `<100` · `100-150` · `150-300` · `300+` |
| `perf.cpu_overhead_pct` | histogram | — | counts per bucket: `<0.5` · `0.5-1` · `1-3` · `3+` |
