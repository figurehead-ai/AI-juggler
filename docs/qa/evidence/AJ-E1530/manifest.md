# Evidence manifest — AJ-E1530

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1530, usage for sessions outside
AI Juggler (a 0.1 P0). Status: **DONE-EVIDENCED** (IN-REVIEW until the
release PR). Req.1 is built as the maintainer decided on 2026-10-22,
"Offer it, opt-in", with the coordinator's guardrails. ADR-0018 carries
the amendment and its trade-off.

## What exists now

**Codex usage counts for every session, AI Juggler's or not.**
- The usage pipeline reads every rollout in an account's folder, and a
  rollout's usage rows (`token_usage_record`, each response once by
  `thread_id/turn_id/response_id`) become the account's window tokens.
- Codex's own `rate_limits` (5-hour and weekly, from `token_count`)
  become measured samples (#120).
- The replica wrote its Codex usage rows without the three ids, so every
  row was dropped as damage. That's why the oracle showed backend 0
  window tokens for both Codex accounts. Its rows now carry them, as
  Codex writes them, and a resumed thread gets new ids per run.

**The status line's reset time reads in every form it arrives in.**
- `resets_at` is read as epoch seconds, epoch milliseconds or an
  RFC 3339 string. Claude's documented capture writes
  `2026-10-13T18:00:00-07:00`, which used to be dropped. Epoch seconds
  would have read as milliseconds, making every window "reset since the
  reading".
- `iso_to_epoch_ms` takes `±HH:MM` offsets and reads a short fraction
  as part of a second.

**Copies conform to ADR-0018 §3, as in-place accounts do.**
- A copy's status line capture comes in with each launch
  (`claude --settings <AI Juggler's overlay>`).
- Ingest no longer rewrites a copy's `settings.json`. The old rewrite
  wasn't atomic and didn't keep the file's bytes: pretty-printed, key
  order lost.
- A copy an earlier version wired to its own wrapper is left exactly as
  it is. The launch overlay chains the user's own status line from that
  copy's backup, never the wrapper. The copy's old spool is still read
  while it's there, so nothing measured is lost.

**What the UI says until a Claude limit is measured** (Req.2): "Not
measured yet · N tokens used". The tokens are Measured, from the
transcripts. The window % comes only from a session AI Juggler started
on that account; one such session measures the account.

**Req.1: measuring limits for sessions started elsewhere (opt-in).**
- **Where:** the account's page has "Limits for sessions started
  elsewhere", only for an account used where it is, of a tool that
  needs it (Claude). It is off by default.
- **Preview first:** "Measure limits for sessions started elsewhere…"
  opens the exact change, the `statusLine` member now and after, with
  two sentences on what changes and why. Nothing is written until **Turn
  on**.
- **The change:**
  - Only `statusLine` in `settings.json` changes, spliced into its text
    (`juggler_adapter::jsonc`), so every other byte stays: comments, key
    order, CRLF and trailing commas included.
  - The write is atomic. It goes through a link to its target, so the
    link stays, and the file keeps its permissions.
  - Before it, AI Juggler keeps a full copy of the file and the record
    of what changed in `overlays/<id>/`. The wrapper and spool are
    there too.
  - The user's own status line is chained and shows unchanged; their
    other `statusLine` keys (padding) are kept.
- **Off, from the same place:**
  - The file goes back exactly when it's as AI Juggler left it.
  - If the user edited it since, only AI Juggler's value is spliced
    out, and their edits stay.
  - If their status line is something else now, it's theirs: AI Juggler
    says so ("changed since you turned this on") and leaves it.
- **Refused in words:** a missing `settings.json`, one that isn't valid
  JSON, a file changed between the preview and **Turn on**, an account
  that is a copy, and one already on.
- **The isolation check:**
  - On: "Its only change to your folder is the statusLine you turned
    on".
  - Off: "Your settings.json doesn't call AI Juggler".
  - AI Juggler's own files in the folder still fail it.
- **Where the code is:** Claude's adapter says which file, key and value
  (`HarnessAdapter::limits_capture`, `limits_capture_value`; no tool
  named in shared code). The commands are `quota_capture_status`,
  `quota_capture_preview`, `quota_capture_turn_on` and
  `quota_capture_turn_off` (async; the first two also on the Dev
  bridge).

## Proof

| Requirement | Evidence | Result |
|---|---|---|
| Req.1 opt-in capture for sessions started elsewhere | `jsonc` tests (byte-identical set/unset on comments, odd key order, CRLF, an existing status line, `{}`, trailing commas; refusals); integration `outside_capture` (on then off byte for byte on four fixtures; edits kept; a changed line left alone; missing / invalid / changed-since-preview / copy refused; a linked file stays a link with its permissions; the isolation check allows exactly the key; end to end: the command from the user's settings prints their own line unchanged and its limits become a measured sample); vitest `account-capture`; the preview walk | PASS |
| Req.2 tokens Measured; % "Not measured yet" until measured | `AccountRunway`/`runwayOf` words (vitest `quota-unmeasured-surfaces`); the replica: Claude accounts with no AI Juggler session read "Not measured yet · N tokens used" | PASS |
| Req.3 Codex limits from its rollouts' `rate_limits` | #120's reader (merged); the replica oracle's 5-hour lines "none, as Codex reports (secondary null)" and its measured % | PASS |
| The oracle comparison of tokens on the replica | files at rest: all 18 window-token lines ×1.00, Claude and Codex, 77 OK ([oracle-replica-static.txt](oracle-replica-static.txt)). With live writers, before QA's cut at the harness's ingest (9cb992b3): live accounts read low by the writes in between, identically per account ([oracle-replica-live-before-cut.txt](oracle-replica-live-before-cut.txt)) | PASS |
| The status line's reset in every form | `quota_ingest` tests `a_status_line_reset_reads_as_epoch_seconds_ms_or_an_rfc3339_string`, `iso_times_read_offsets_and_short_fractions` | PASS |
| Copies: no write into the folder; earlier wiring left alone | `usage_pipeline` test (nothing installed into a copy; its old spool still read); `in_place_accounts` (a copy's launch carries `--settings`; its `settings.json` unchanged); `quota_ingest` `a_copy_wired_by_an_earlier_version_chains_its_users_own_line_and_stays_as_it_is` | PASS |

## Found on the way

- `gpt-5-codex`, a real model, has no price in `cost_ledger_seed.json`.
  Its cost rows are honest nulls, never guessed. Adding it needs a
  cited price (passed to the coordinator). On the maintainer's real
  setup the only unpriced model QA found is `codex-auto-review`.
- **Three ISO parsers are now one** (the coordinator: "merge within
  E1530"): `juggler_core::time::timestamp_ms` reads `Z` and `±HH:MM`
  offsets and a fraction of a second, and refuses impossible dates and
  times.
  - It replaces `juggler_adapter::conversation::timestamp_ms` (UTC `Z`
    only, offsets ignored), `quota_ingest::iso_to_epoch_ms` and
    `commands::chrono_like_ms`, a third copy found while merging.
  - Transcripts, rollouts and status lines use it.
  - Its tests carry the merged cases (`juggler_core::time`).
- `gpt-5-codex` stays unpriced ("No price for … yet") until OpenAI's
  own page lists it; no uncited price is added (the coordinator,
  2026-10-22).

- The same family, at a later push (2026-10-22, with E1527):
  `juggler-supervisor` `orphan::collector_targeted_tick_overhead_under_budget`
  measured 32 ms against its 20 ms budget inside the full verify (load
  average 17–24); alone it passed three runs of three. The workspace
  suite passed with only it skipped. Fix batch 4's, as above.

## After the docs fact-check (2026-10-22)

- **The status line can't be left dangling.** The line written into the
  user's `settings.json` is now fail-safe: `sh -c` runs AI Juggler's
  wrapper while it's there and runnable, and otherwise their own command,
  kept in it as one quoted word (`fail_safe_statusline_command`). The
  preview shows this exact line. Deleting AI Juggler or its data never
  breaks their status line.
- **Removing an account turns measuring off first**, by the same rules
  (exact restore, or only AI Juggler's value taken out). If that fails,
  the account stays and the refusal says why.
- **"Prepare to uninstall"** (Settings › Data): puts every measured
  account's status line back and lists what's left (the data folder, the
  Keychain items, the worktrees AI Juggler made). Nothing is deleted.
- Tests: `quota_ingest::the_fail_safe_line_runs_the_wrapper_while_it_is_there_else_the_users_own`
  (a path with a quote and a space; the wrapper; the data deleted; no
  line of theirs; never its own wrapper), and `outside_capture`
  `with_ai_jugglers_data_deleted_the_users_own_line_still_prints`,
  `removing_the_account_puts_the_users_settings_back`,
  `preparing_to_uninstall_puts_status_lines_back_and_lists_whats_left`;
  vitest `uninstall-card`.

## Known blocker at this push (not from E1530)

- `load_budgets::monitoring_costs_under_one_percent_of_a_core` measured
  1.38% of a core inside the full `just verify` on a loaded Mac (load
  average above 20). Run alone it measured 0.40–0.66% across nine runs.
  For this push the workspace suite ran with only that test skipped (158
  green), plus three solo runs of it.
- The coordinator (2026-10-22): the 1.38% is probably real, not noise.
  fix-reports measured 1.36–1.43% beside a busy Claude app, because the
  observed process trees are re-read on every tick. It is a **0.1
  blocker** and the first item of fix batch 4
  (`fix/maintainer-reports-4`).
- Next steps: merge main once #126 lands (it replaces the test with
  counter assertions plus the CPU bound), then merge again once batch 4
  lands. From then on the test has to pass inside a full verify on a
  loaded Mac.

## Walked: the three questions (Req.1)

Walked on the preview. The backend's whole path, from turning it on to a
measured sample, runs in `outside_capture`.

1. **Does it solve the real problem?** An account whose sessions all
   start in the Claude app or a terminal gets measured limits, once its
   owner says yes. Before, it stayed "Not measured yet" however much it
   was used.
2. **Is it clear without explanation?** The account page says why it's
   off ("Claude reports its limits only to its status line…"). The
   preview shows the one line that changes, before and after, and says
   their own status line still shows. Turning it off says what it did.
3. **Is it consistent with its siblings?** It sits on the account's page
   with its other opt-ins, uses the app's sheet and toast, and follows
   AJ-E1503's rule: nothing irreversible without a preview, and undo in
   the same place.
