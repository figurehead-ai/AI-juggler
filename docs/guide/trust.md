# What AI Juggler touches

AI Juggler sits next to your coding tools and your accounts, so you should know exactly what it reads, changes and sends. This page says it, and each line names the tests that check it. Those tests run on every change to AI Juggler, and a line whose tests are gone fails the build.

The same list is in the app: More › **Privacy**, and Settings › Privacy links to it.

## What it reads, on your Mac only

- It never reads your passwords, tokens or API keys, or the tools' Keychain items. It reads only whether you're signed in, and as which account: its email and organisation.
  <sub>Checked by [`no_sign_in_file_is_read`](../../crates/juggler-app/tests/no_bare_tools.rs), [`login_state_is_metadata_only`](../../crates/juggler-adapters/tests/claude_code_adapter.rs), [`scan_code_never_reads_credential_contents`](../../crates/juggler-adapters/tests/scan.rs), [`the_environment_scan_runs_no_vendor_cli`](../../crates/juggler-app/tests/environment_scan_no_cli.rs), [`the_sign_in_probe_reads_kinds_never_values`](../../crates/juggler-adapters/tests/opencode_adapter.rs), [`the_sign_in_probe_reads_which_files_are_there_never_what_they_hold`](../../crates/juggler-adapters/tests/kimi_code_adapter.rs), [`the_sign_in_probe_reads_modes_never_values`](../../crates/juggler-adapters/tests/grok_build_adapter.rs).</sub>
- From Claude Code's settings.json it reads only the settings it needs, each one named in its code and checked.
  <sub>Checked by [`no_sign_in_file_is_read`](../../crates/juggler-app/tests/no_bare_tools.rs).</sub>
- With crash reports on, it looks through macOS's new crash reports for AI Juggler's own, and keeps only what failed and where.
  <sub>Checked by [`an_ips_crash_keeps_its_kind_and_frames_and_nothing_else`](../../crates/juggler-app/src/crash_native.rs), [`only_crashes_after_the_first_look_are_kept_and_never_reads_nothing`](../../crates/juggler-app/src/crash_native.rs), [`links_and_oversized_files_are_not_read`](../../crates/juggler-app/src/crash_native.rs).</sub>
- If you use Claude Squad or Vibe Kanban, it reads their own lists for which repository each of their worktrees belongs to and which config folders their profiles use, and nothing else they keep. It never opens their sign-in or key files.
  <sub>Checked by [`claude_squad_worktrees_come_with_their_repositories`](../../crates/juggler-adapters/tests/other_managers.rs), [`vibe_kanban_profiles_offer_the_folders_they_point_at`](../../crates/juggler-adapters/tests/other_managers.rs), [`the_importers_never_name_a_sign_in_or_key_file`](../../crates/juggler-adapters/tests/other_managers.rs).</sub>

## What it changes

- What it keeps lives in its own folder, ~/Library/Application Support/AI Juggler (AI Juggler Nightly for Nightly), and in its own Keychain items.
  <sub>Checked by [`ai_juggler_keeps_its_existing_folder_keychain_items_and_port`](../../crates/juggler-app/src/edition.rs).</sub>
- With Search the words of every session on, its index holds the words to search, not the conversations, in its own folder; turning it off deletes the index.
  <sub>Checked by [`turning_it_off_leaves_no_index_behind`](../../crates/juggler-app/tests/outside_search.rs), [`a_search_index_file_is_the_same_index_and_removes_whole`](../../crates/juggler-core/src/storage/transcript_index.rs).</sub>
- It never writes to a setup you use where it is, unless you turn on a change that says so first.
  <sub>Checked by [`a_setup_used_where_it_is_is_never_written_by_ai_juggler`](../../crates/juggler-app/tests/in_place_accounts.rs), [`a_codex_home_used_where_it_is_is_never_written_by_ai_juggler`](../../crates/juggler-app/tests/codex_in_place.rs), [`the_default_folder_is_used_where_it_is_and_never_written`](../../crates/juggler-app/tests/opencode_provider.rs), [`the_default_home_is_used_where_it_is_and_never_written`](../../crates/juggler-app/tests/kimi_provider.rs), [`its_sessions_become_projects_and_its_default_home_is_never_written`](../../crates/juggler-app/tests/grok_provider.rs).</sub>
- It changes your setups and projects only when you apply a change, shows it first, and can undo it.
  <sub>Checked by [`turning_it_on_changes_only_the_status_line_and_off_puts_every_byte_back`](../../crates/juggler-app/tests/outside_capture.rs), [`nothing_is_written_until_the_user_applies`](../../crates/juggler-app/tests/project_sync.rs), [`the_other_claude_account_is_offered_what_it_lacks_and_gets_only_what_is_picked`](../../crates/juggler-app/tests/switch_setup.rs), [`a_new_account_gets_the_usual_settings_and_never_a_credential_history_or_cache`](../../crates/juggler-app/tests/account_settings.rs).</sub>
- Copying settings or MCP servers never takes a secret along: the copy names it, and you provide it.
  <sub>Checked by [`a_setting_with_a_secret_written_in_it_is_never_copied_or_recorded`](../../crates/juggler-app/tests/account_settings.rs), [`a_server_is_copied_across_tools_without_its_secret_and_undone`](../../crates/juggler-app/tests/project_sync.rs), [`the_other_claude_account_is_offered_what_it_lacks_and_gets_only_what_is_picked`](../../crates/juggler-app/tests/switch_setup.rs), [`a_settings_file_with_a_key_written_in_it_stays_behind_named`](../../crates/juggler-adapters/tests/opencode_adapter.rs), [`a_copy_keeps_settings_but_never_a_sign_in_a_key_or_the_sessions`](../../crates/juggler-adapters/tests/kimi_code_adapter.rs), [`a_copy_keeps_settings_and_sessions_but_never_a_sign_in_or_a_key`](../../crates/juggler-adapters/tests/grok_build_adapter.rs).</sub>
- Before you uninstall, it can take out the one setting it may have added to a Claude account, and keep any edits you made since.
  <sub>Checked by [`preparing_to_uninstall_puts_status_lines_back_and_lists_whats_left`](../../crates/juggler-app/tests/outside_capture.rs).</sub>

## What it sends

- It makes no requests to Anthropic, OpenAI or any other AI provider. Your tools talk to their providers themselves.
  <sub>Checked by [`only_the_listed_crates_can_reach_the_network`](../../crates/juggler-app/tests/trust.rs), [`the_app_reaches_only_its_own_addresses`](../../crates/juggler-app/tests/trust.rs), [`the_window_can_reach_nothing_but_the_app`](../../crates/juggler-app/tests/trust.rs), [`no_http_client_anywhere_in_the_adapters_crate`](../../crates/juggler-adapters/tests/quota_ingest.rs).</sub>
- It never sits between your tools and their providers: it sets no proxy or provider address for them.
  <sub>Checked by [`ai_juggler_never_points_a_tool_at_another_address`](../../crates/juggler-app/tests/trust.rs).</sub>
- It checks GitHub for updates when it starts, if that's on, or when you choose Check now.
  <sub>Checked by [`check_off_means_zero_connections_and_on_reaches_the_feed`](../../crates/juggler-app/tests/updates_plugin.rs), [`the_app_reaches_only_its_own_addresses`](../../crates/juggler-app/tests/trust.rs).</sub>
- Once a day, if that's on, it asks npm's public registry for the newest version of each tool you have an account on, and shows it on that tool's page. It never updates a tool for you.
  <sub>Checked by [`one_request_per_tool_a_day_only_for_tools_in_use_and_none_when_off`](../../crates/juggler-app/tests/tool_updates.rs), [`no_update_is_ever_run`](../../crates/juggler-app/src/features/tool_updates/mod.rs), [`the_app_reaches_only_its_own_addresses`](../../crates/juggler-app/tests/trust.rs).</sub>
- Usage statistics are off until you choose. When on, they go to stats.figurehead.ai as counts only: never your words, files, paths or names.
  <sub>Checked by [`without_a_choice_nothing_is_created_and_nothing_is_sent`](../../crates/juggler-app/tests/stats_guardrails.rs), [`nothing_the_user_owns_reaches_the_stats_store_or_a_report`](../../crates/juggler-app/tests/stats_guardrails.rs), [`off_creates_no_folder_and_records_nothing`](../../crates/juggler-stats/src/tests.rs), [`the_app_reaches_only_its_own_addresses`](../../crates/juggler-app/tests/trust.rs).</sub>
- Crash reports are kept on your Mac, with names and paths taken out. This version has nowhere to send them.
  <sub>Checked by [`a_seeded_crash_is_scrubbed_before_it_is_kept`](../../crates/juggler-app/src/crash.rs), [`crash_reports_have_nowhere_to_go`](../../crates/juggler-app/tests/trust.rs), [`the_crash_sender_is_built_only_with_an_address`](../../crates/juggler-app/tests/trust.rs).</sub>
- The Figurehead account is optional. Signed out, nothing goes to it; signed in, a feature you've turned off sends nothing.
  <sub>Checked by [`g1_signed_out_is_silent`](../../crates/juggler-app/tests/cloud_guardrails.rs), [`g2_a_feature_that_is_off_never_calls_its_endpoint`](../../crates/juggler-app/tests/cloud_guardrails.rs).</sub>
- Fleet connects only Macs you paired, directly and encrypted, each checking the other's key, and no server carries the traffic. To be found, a Mac announces itself on your local network, and in your Figurehead account if you turn that on.
  <sub>Checked by [`mutual_tls_with_pinned_keys_connects_and_serves`](../../crates/juggler-link/tests/transport.rs), [`a_node_presenting_an_unpinned_key_is_refused_at_tls`](../../crates/juggler-link/tests/transport.rs), [`only_the_listed_crates_can_reach_the_network`](../../crates/juggler-app/tests/trust.rs).</sub>
- A paired Mac sees which sessions run where, with their project and account names, states and usage totals. It never gets your sign-ins, files or prompts, and gets a conversation only when you switch one to that Mac.
  <sub>Checked by [`no_field_can_carry_a_secret_or_a_transcript`](../../crates/juggler-link/tests/proto_contract.rs), [`what_starting_there_and_usage_by_machine_send_is_numbers_and_an_id`](../../crates/juggler-link/tests/proto_contract.rs), [`a_view_peers_launch_is_refused_at_the_node_and_nothing_starts`](../../crates/juggler-link/tests/transport.rs), [`each_mac_reports_what_it_used_in_the_last_day`](../../crates/juggler-app/tests/fleet_switch.rs).</sub>

## What it never does

- It never signs in for you. A sign-in is always the tool's own, and AI Juggler never reads the secret the tool saves.
  <sub>Checked by [`login_spec_points_at_isolated_dir`](../../crates/juggler-adapters/tests/claude_code_adapter.rs), [`login_spec_runs_the_vendor_flow_in_the_isolated_home`](../../crates/juggler-adapters/tests/codex_adapter.rs), [`no_sign_in_file_is_read`](../../crates/juggler-app/tests/no_bare_tools.rs), [`sign_in_runs_opencode_auth_login_in_the_accounts_folders`](../../crates/juggler-adapters/tests/opencode_adapter.rs), [`sign_in_runs_kimi_login_in_the_accounts_home`](../../crates/juggler-adapters/tests/kimi_code_adapter.rs), [`sign_in_runs_grok_login_in_the_accounts_home`](../../crates/juggler-adapters/tests/grok_build_adapter.rs).</sub>
- It never moves your work to another account by itself. Every switch starts from a click, on this Mac or on a Mac you paired.
  <sub>Checked by [`nothing_switches_on_its_own`](../../crates/juggler-app/tests/trust.rs), [`only the switch screens start a switch`](../../ui/src/screens/privacy-page.test.tsx).</sub>
- Pools only group your sessions. Putting a session in a pool, giving it a role or deleting the pool never starts, stops or types into it.
  <sub>Checked by [`pool_operations_never_start_stop_or_type_into_a_session`](../../crates/juggler-app/tests/pools.rs), [`the_pool_code_has_no_path_to_a_session`](../../crates/juggler-app/tests/pools.rs).</sub>
- The test bridge the developers use is not in the app you download.
  <sub>Checked by [`the_dev_bridge_is_not_a_default_feature`](../../crates/juggler-app/tests/dev_bridge_feature.rs).</sub>

## Your accounts and the vendors' terms

Use only accounts that are yours. When you add one, AI Juggler shows what that tool's terms say about having several, with a link to the terms. What they say, where it comes from and the day it was checked are in the [terms register](../qa/evidence/AJ-E1518/terms-claims-register.md).

## Found something that doesn't match?

Please report it privately, as [SECURITY.md](../../SECURITY.md) describes. A line here that isn't true is a security bug.
