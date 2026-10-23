# The read-only test bridge

The real AI Juggler UI, running in a normal browser, against a running **AI Juggler Dev** build: its real backend and its real data. QA can walk a real setup this way without giving the app macOS Accessibility. That grant would cover every Claude session on the Mac, so we don't ask for it.

The bridge only reads. It can't launch, stop, import, remove, apply, type into a terminal, relaunch, quit or change a setting, and it starts no vendor CLI. Every write in the UI says **"Read-only test bridge — not available"** and sends nothing.

It is never in a release build.

## Run it

1. Start AI Juggler Dev with the bridge:

   ```bash
   just dev-bridge
   ```

   That runs `cargo tauri dev` with the Dev config, the `dev-bridge` feature and `AIJ_DEV_BRIDGE=1`. It also starts Vite on `http://localhost:5173`. The app's own window opens as usual.

   Its log line says where the token is, never the token itself:

   ```text
   ai-juggler: read-only dev bridge on 127.0.0.1:<port> for http://localhost:5173; token in …/AI Juggler Dev/dev-bridge.json
   ```

2. Get the page:

   ```bash
   just dev-bridge-url
   ```

   This reads `~/Library/Application Support/AI Juggler Dev/dev-bridge.json`, which is `0600` and per run. It prints:

   ```text
   http://localhost:5173/?devBridge=<port>#token=<64 hex>
   ```

   The token is in the fragment, so the browser never sends it to Vite. `?devBridge=<port>&token=<…>` works too. Either way, the page takes the token out of the address bar on load and keeps it for that tab only, so a reload still works.

3. Open that URL in any browser on this Mac. A strip at the foot of the page says it's the read-only bridge, and names each write it refused.

To use a different page origin (for example `vite preview` on 4173), set `AIJ_DEV_BRIDGE_ORIGIN=http://localhost:4173`. Only `http://localhost:<port>` and `http://127.0.0.1:<port>` are accepted.

### On a real setup

The coordinator and the QA session run it against the maintainer's real home, under the read-only rules:

- no launches, sign-ins, or writes to vendor folders;
- no credential reads;
- a before-and-after snapshot of the vendor folders.

Start the Dev build Finder-like (`env -i HOME USER LOGNAME TMPDIR SHELL LANG PATH …`) so agent-shell variables such as `CLAUDE_CONFIG_DIR` don't skew what it sees.

### On fixtures, or next to another AI Juggler Dev

Point `HOME` at a throwaway folder. The Dev data folder and every vendor folder then live there. Put stand-in `claude` and `codex` scripts first on its `PATH`.

The single-instance socket is shared across HOMEs, and Vite's port 5173 may be taken. So build with an identifier and a dev URL of your own:

```bash
TAURI_CONFIG='{"identifier":"ai.figurehead.juggler.dev.bridgeproof","build":{"devUrl":"http://localhost:5199","beforeDevCommand":""}}' cargo build -p juggler-app --features dev-bridge
```

```bash
pnpm --dir ui exec vite --port 5199 --strictPort
```

```bash
env -i HOME="$H" USER="$USER" TMPDIR="$TMPDIR" PATH="$H/bin:/usr/bin:/bin" AIJ_DEV_BRIDGE=1 AIJ_DEV_BRIDGE_ORIGIN=http://localhost:5199 "$CARGO_TARGET_DIR/debug/juggler-app"
```

```bash
python3 scripts/dev_bridge_url.py "$H/Library/Application Support/AI Juggler Dev"
```

This is how the PR that added the bridge proved it.

## What it serves

`GET /v1/commands` lists them. Each one is classed `Read` in the command registry. That registry is `app_commands!` in `crates/juggler-app/src/lib.rs`: every registered command, each marked `Read` or `Write`. A command can't be registered without a class.

- **Accounts and tools:** `app_info`, `providers_list`, `providers_capabilities`, `tools_catalog`, `accounts_list`, `account_audit`, `account_storage`, `launch_recipes_list`.
- **Sessions and projects:** `instances_list`, `instance_quality`, `instance_processes`, `conversations_list` (sessions started outside AI Juggler), `conversations_resume_command`, `sessions_resume_command`, `projects_list`, `projects_git_status`, `ranking_rows`, `switch_back_suggestions`, `parked_list`, `overnight_summary`, `dashboard_stats`.
- **History:** `transcript_search`, `instance_replay`.
- **Usage:** `quota_display`, `quota_display_all`, `cost_rollups`, `pricing_catalog_info`, `usage_source_health`, `reset_queue_list`.
- **This Mac:** `this_mac`, `machine_health`, `machine_advice`, `port_inventory`, `cleanup_inventory`, `cleanup_status`, `cleanup_tiers_get`, `recovery_summary_get`.
- **Settings reads:** `app_setting_get` (only `go_to_hotkey` and `renewal_warn_days`), `notification_prefs_get`, `cleanup_default_get`, `redaction_policy_get`, `os_access_list`, `update_status`, `going_back_options`, `crash_info`, `usage_stats_info`, `link_status`, `editor_choices`, `session_prefs_get`.

The refresh events go out over `GET /v1/events` (SSE), behind the same token: `app://ready`, `session://updated`, `session://resources`, `session://quota`, `machine://this-mac`, `update://status`, `conversations://changed`. Terminal output, preview frames and the progress of writes are never sent.

### What a read still does

A read changes nothing a person or a later answer would see. Some reads do, on the way, the bookkeeping the app's own timers already do. The Dev window does all of this when it opens anyway:

- **`instances_list`, `projects_list`, `overnight_summary`:**
  - sample the session process trees, which brings the spawn ledger up to date;
  - refresh a `git status` that's more than 30 s old, for a live session.
- **`accounts_list`, `ranking_rows`:** use the login-state probe:
  - it opens a vendor sign-in file only to read its expiry, and returns nothing from it;
  - it asks the Keychain whether an item *exists*, with `security find-generic-password` and never `-w`.
- **`transcript_search`:** brings the search index up to date first, with the same incremental pass the app runs every minute.
- **`this_mac`, `machine_advice`:** advance the in-memory overload-alert state. Delivering an alert (`machine_alerts_take`) is a write and isn't served.
- **`quota_display`:** may add one usage-statistics count. It records only if usage statistics are on.

### What the Dev build does on its own

The bridge adds a way in. It doesn't stop the Dev build's normal background work, which runs while bridged exactly as in any Dev run:

- the usage ingest pipeline;
- the 2 s processes tick, and the 5 s and 30 s resource and quota ticks;
- the attention heuristics;
- the transcript search index.

They read the vendor transcripts and write only AI Juggler Dev's own data folder.

The cleanup tiers act only on AI Juggler's own spawn ledger: the recorded descendants of sessions *it* launched, for accounts opted in to cleanup. A fresh Dev data folder has an empty ledger and no opted-in accounts, so the tiers can't signal anything there. The bridge can't launch a session or change an opt-in.

`environment_scan` (classed `Read`, not served) runs no vendor CLI: `tests/environment_scan_no_cli.rs` holds that, with stand-ins first on `PATH`. On macOS it asks the Keychain whether sign-in items exist (`security find-generic-password`, never `-w`).

### Left out on purpose

These commands look like reads, but each has an effect a read must not have.

| Left out | Why |
|---|---|
| `environment_scan` | Scans every vendor folder under `$HOME` with the login-state probe. It is classed `Read`, but left off the bridge until the coordinator says otherwise. |
| `vault_status`, `cloud_backup_status` | Read the vault key from the Keychain, which can prompt. |
| `fleet_self`, `fleet_peers`, `fleet_remote`, `fleet_discovered` | Create the Fleet identity keys on first use. |
| `figurehead_status`, `pairing_host_state` | Return a live device-link or pairing code. |
| `expert_setup_info`, `launch_recipe_env_get` | Return Launch-settings environment values, which can hold keys. |
| `terminal_snapshot` | Returns raw terminal output. |
| `providers_discover`, `recommendation_for`, `environment_scan_full`, `validation_matrix`, `fleet_switch_targets` | Run (or can run) vendor CLIs with `--version`. |
| `isolation_offer`, `git_status_refresh`, `worktree_removal_preview` | Run `git`. |
| `instance_delete_preview`, `retention_preview` | Issue one-time confirmation tokens. |
| `alerts_tick`, `cleanup_alerts`, `parked_offers`, `usage_stats_overview`, `switch_preflight`, `cost_csv_export`, `isolation_check` | Write settings, logs, files or overlays. |

## How it is kept safe

| Guarantee | How | Checked by |
|---|---|---|
| Never in a release build | The module compiles only with `debug_assertions` and the off-by-default `dev-bridge` feature. With the feature and no debug assertions, `compile_error!` fires. | `scripts/dev_bridge_check.sh` (in `just verify` and CI) checks three things: the feature graph, that no release workflow uses the feature or `--all-features`, and that `cargo check --release --features dev-bridge` fails with that error. Also `tests/dev_bridge_feature.rs`. |
| Off unless asked | It starts only with `AIJ_DEV_BRIDGE=1`, in the Dev edition. | `dev_bridge::start_if_enabled` |
| Not reachable off this Mac | Binds `127.0.0.1` on a random port, and accepts loopback peers only. | Unit tests |
| Only the page that has the token | Every request needs `Authorization: Bearer <token>`. The token is 256 bits from the OS random source, compared in constant time (`subtle`), redacted from `Debug`, wiped on drop, and stored only in a `0600` file that is written atomically and removed when the app quits. A killed process leaves the file, but its token died with the run: the port is closed, and the next run writes a new one. | `the_token_*`, `a_request_without_the_right_token_*` |
| No DNS rebinding | `Host` must be `127.0.0.1:<port>` or `localhost:<port>`. | `a_request_without_the_right_token_host_or_origin_is_refused` |
| No other site | An `Origin`, when sent, must be the one dev origin, or the answer is 403. CORS answers that origin only. A preflight carries no token and gets nothing but the CORS headers. | Same test, plus `over_the_wire_cors_answers_only_the_dev_origin` |
| Read-only by construction | The bridge serves only its route table. Each route names the function it calls, and a test holds that function to `Read` in the registry. Anything else gets 403, before any argument is read: unknown commands, write commands, and settings keys off the list. | `every_route_calls_a_command_the_registry_classes_read`, `no_write_command_is_on_the_allowlist`, `every_write_and_unknown_command_is_refused_with_403`, `over_the_wire_reads_answer_and_writes_get_403` |
| Bounded | Limits on: request bodies (64 KiB, 413 above), request heads (16 KiB, 5 s to arrive), 32 connections, 4 event streams, and 200 requests in a burst refilled at 50 a second (429). Arguments are typed and `deny_unknown_fields`. | `the_rate_limit_runs_out_and_refills`, over-the-wire tests |
| Audited | Each request logs one line to stderr: the method, the command or route, and the status. Arguments and results are never logged. | — |
| Not in the shipped UI | `ui/src/lib/ipc/devBridge.ts` is imported only behind `import.meta.env.DEV`. A production bundle has no trace of it. | `ui/tests/bundle/no-dev-bridge.test.ts` builds both modes: production must lack `devBridge`, and development must have it. |
| Writes refused in the page too | Before sending, the UI transport checks each command against `/v1/commands`. It refuses the native file pickers. The refusal uses the app's error shape, so screens show "Read-only test bridge — not available". | `ui/src/lib/ipc/devBridge.test.ts` |

The event stream is read with `fetch`, not `EventSource`. `EventSource` can't send an `Authorization` header, and the token must not sit in a URL.

## Threat model

The repo's threat model is the STRIDE table in `docs/engineering/SECURITY-GUIDELINES.md`. This row belongs in it:

| Component | S | T | R | I | D | E |
|---|---|---|---|---|---|---|
| **Read-only test bridge** (Dev builds only; TB6, loopback HTTP) | A local process or web page calls it. Mitigations: 256-bit per-run bearer token in a `0600` file; `Host` check against DNS rebinding; one allowed `Origin` | Arguments typed and size-limited; no paths, programs or URLs accepted (ids only) | One stderr line per request: route and status, no content | The reads return real transcripts, accounts and usage to whoever holds the token. Mitigations: the token never leaves the Mac or the logs; served reads exclude secrets, codes and tokens | Rate and connection limits; slow heads time out | **A write through the bridge.** Mitigations: the route table is the allowlist, and a test holds every route to `Read` in the registry; unknown and write commands get 403; `compile_error!` in release builds; production UI bundles carry no client |

SECURITY-GUIDELINES C7 says AI Juggler opens no TCP or HTTP listener. A browser can't use a Unix socket, so this Dev-only bridge is the stated exception. It meets the rule's conditions for HTTP: `127.0.0.1` only, `Origin` validated (403), and a bearer token.
