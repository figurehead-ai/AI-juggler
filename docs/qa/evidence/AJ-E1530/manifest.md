# Evidence manifest — AJ-E1530

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1530, usage for sessions outside
AI Juggler (a 0.1 P0). Status: **IN-PROGRESS**.
- **Req.1** (reading Claude limits for sessions started outside AI
  Juggler, by a status line added to the user's own `settings.json`)
  waits on the maintainer. It reverses ADR-0018 §2, which rejected
  exactly that, so the coordinator asked them first. Nothing is built
  for it, and the ADR is unchanged.
- **Req.2 and Req.3** are covered below.

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

## Proof

| Requirement | Evidence | Result |
|---|---|---|
| Req.1 the user-applied status line for outside sessions | waits on the maintainer (ADR-0018 §2 reversal) | BLOCKED |
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
- Two ISO parsers exist (`juggler_adapter::conversation::timestamp_ms`,
  UTC `Z` only; `quota_ingest::iso_to_epoch_ms`, now with offsets).
  Folding them into one is a follow-up.
