# What AI Juggler touches

AI Juggler sits next to your coding tools and your accounts, so you should know exactly what it reads, changes and sends. This page says it, and each line is held to a test that runs on every change to AI Juggler. A line that loses its test fails the build.

The same list is in the app: More › **Privacy**.

## What it reads, on your Mac only

- It never reads your passwords, tokens or API keys, or the tools' Keychain items. It checks only whether a sign-in is there.
  <sub>Checked by [`no_sign_in_file_is_read`](../../crates/juggler-app/tests/no_bare_tools.rs), [`login_state_is_metadata_only`](../../crates/juggler-adapters/tests/claude_code_adapter.rs), [`scan_code_never_reads_credential_contents`](../../crates/juggler-adapters/tests/scan.rs), [`the_environment_scan_runs_no_vendor_cli`](../../crates/juggler-app/tests/environment_scan_no_cli.rs).</sub>
- From a tool's settings file it reads only the settings it needs, each one named in its code and checked.
  <sub>Checked by [`no_sign_in_file_is_read`](../../crates/juggler-app/tests/no_bare_tools.rs).</sub>

## What it changes

- Its own data stays in its own folder: ~/Library/Application Support/AI Juggler (AI Juggler Nightly for Nightly).
  <sub>Checked by [`ai_juggler_keeps_its_existing_folder_keychain_items_and_port`](../../crates/juggler-app/src/edition.rs).</sub>
- A setup you use where it is, is never written to unless you turn on a change that says so first.
  <sub>Checked by [`a_setup_used_where_it_is_is_never_written_by_ai_juggler`](../../crates/juggler-app/tests/in_place_accounts.rs), [`a_codex_home_used_where_it_is_is_never_written_by_ai_juggler`](../../crates/juggler-app/tests/codex_in_place.rs).</sub>
- It changes your setups and projects only when you apply a change, shows it first, and can undo it.
  <sub>Checked by [`turning_it_on_changes_only_the_status_line_and_off_puts_every_byte_back`](../../crates/juggler-app/tests/outside_capture.rs), [`nothing_is_written_until_the_user_applies`](../../crates/juggler-app/tests/project_sync.rs), [`the_other_claude_account_is_offered_what_it_lacks_and_gets_only_what_is_picked`](../../crates/juggler-app/tests/switch_setup.rs), [`a_new_account_gets_the_usual_settings_and_never_a_credential_history_or_cache`](../../crates/juggler-app/tests/account_settings.rs).</sub>
- Copying settings or MCP servers never takes a secret along: the copy names it, and you provide it.
  <sub>Checked by [`a_setting_with_a_secret_written_in_it_is_never_copied_or_recorded`](../../crates/juggler-app/tests/account_settings.rs), [`a_server_is_copied_across_tools_without_its_secret_and_undone`](../../crates/juggler-app/tests/project_sync.rs), [`the_other_claude_account_is_offered_what_it_lacks_and_gets_only_what_is_picked`](../../crates/juggler-app/tests/switch_setup.rs).</sub>
- Before you uninstall, it can take out the one setting it may have added to a Claude account, and keep any edits you made since.
  <sub>Checked by [`preparing_to_uninstall_puts_status_lines_back_and_lists_whats_left`](../../crates/juggler-app/tests/outside_capture.rs).</sub>

## What it sends

- It makes no requests to Anthropic, OpenAI or any other AI provider. Your tools talk to their providers themselves.
  <sub>Checked by [`only_the_listed_crates_can_reach_the_network`](../../crates/juggler-app/tests/trust.rs), [`the_app_reaches_only_its_own_addresses`](../../crates/juggler-app/tests/trust.rs), [`the_window_can_reach_nothing_but_the_app`](../../crates/juggler-app/tests/trust.rs), [`no_http_client_anywhere_in_the_adapters_crate`](../../crates/juggler-adapters/tests/quota_ingest.rs).</sub>
- It never sits between your tools and their providers: it sets no proxy or provider address for them.
  <sub>Checked by [`ai_juggler_never_points_a_tool_at_another_address`](../../crates/juggler-app/tests/trust.rs).</sub>
- It checks GitHub for updates, and only while update checks are on.
  <sub>Checked by [`check_off_means_zero_connections_and_on_reaches_the_feed`](../../crates/juggler-app/tests/updates_plugin.rs), [`the_app_reaches_only_its_own_addresses`](../../crates/juggler-app/tests/trust.rs).</sub>
- Usage statistics are off until you choose. When on, they're counts only: never your words, files, paths or names.
  <sub>Checked by [`without_a_choice_nothing_is_created_and_nothing_is_sent`](../../crates/juggler-app/tests/stats_guardrails.rs), [`nothing_the_user_owns_reaches_the_stats_store_or_a_report`](../../crates/juggler-app/tests/stats_guardrails.rs), [`off_creates_no_folder_and_records_nothing`](../../crates/juggler-stats/src/tests.rs).</sub>
- Crash reports are kept on your Mac, with names and paths taken out. This version can't send them anywhere.
  <sub>Checked by [`a_seeded_crash_is_scrubbed_before_it_is_kept`](../../crates/juggler-app/src/crash.rs), [`this_build_cannot_send`](../../crates/juggler-app/src/crash.rs), [`the_app_reaches_only_its_own_addresses`](../../crates/juggler-app/tests/trust.rs).</sub>
- The Figurehead account is optional. Signed out, or with a feature off, nothing is sent to Figurehead.
  <sub>Checked by [`g1_signed_out_is_silent`](../../crates/juggler-app/tests/cloud_guardrails.rs), [`g2_a_feature_that_is_off_never_calls_its_endpoint`](../../crates/juggler-app/tests/cloud_guardrails.rs).</sub>
- Fleet connects only Macs you paired, directly and encrypted, each checking the other's key. No server sits in between.
  <sub>Checked by [`mutual_tls_with_pinned_keys_connects_and_serves`](../../crates/juggler-link/tests/transport.rs), [`a_node_presenting_an_unpinned_key_is_refused_at_tls`](../../crates/juggler-link/tests/transport.rs), [`only_the_listed_crates_can_reach_the_network`](../../crates/juggler-app/tests/trust.rs).</sub>

## What it never does

- It never signs in for you. A sign-in is always the tool's own, and AI Juggler never reads what the tool saves.
  <sub>Checked by [`login_spec_points_at_isolated_dir`](../../crates/juggler-adapters/tests/claude_code_adapter.rs), [`login_spec_runs_the_vendor_flow_in_the_isolated_home`](../../crates/juggler-adapters/tests/codex_adapter.rs), [`no_sign_in_file_is_read`](../../crates/juggler-app/tests/no_bare_tools.rs).</sub>
- It never moves your work to another account by itself. Every switch starts from your click.
  <sub>Checked by [`nothing_switches_on_its_own`](../../crates/juggler-app/tests/trust.rs).</sub>
- The test bridge the developers use is not in the app you download.
  <sub>Checked by [`the_dev_bridge_is_not_a_default_feature`](../../crates/juggler-app/tests/dev_bridge_feature.rs).</sub>

## Your accounts and the vendors' terms

Use only accounts that are yours. AI Juggler never shares, pools or rotates accounts. When you add one, it shows what the tool's terms say about having several, with a link to the terms themselves. What the terms say is recorded, with its source and the day it was checked, in the [terms register](../qa/evidence/AJ-E1518/terms-claims-register.md).

## Found something that doesn't match?

Please report it privately, as [SECURITY.md](../../SECURITY.md) describes. A trust claim that isn't true is a security bug.
