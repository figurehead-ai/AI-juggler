# Evidence manifest — AJ-E1107

Task: `docs/tasks/E11-usage-statistics.md` AJ-E1107 (crash reports
behind their own setting). 2026-10-18, reviewer claude-opus; R5's code
side 2026-11-13 (D-108). Captures: `_gallery/64-crash` (the setting and
the next-launch question, 1280×800, light and dark). **Status:
IN-PROGRESS.** R1–R4 and R5's code side pass on tests. Left: sending to
a real Sentry project from a signed build, which waits for the
maintainer's `AI_JUGGLER_CRASH_ENDPOINT` secret (D-108).

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Setting `never` (default) / `ask` / `always`; `DO_NOT_TRACK` forces `never` | R1 | `crash.rs` (per Mac; never in a backup). `the_setting_and_answers_move_reports_and_never_deletes_them`; `crash-reports.test.tsx` (Never by default; DO_NOT_TRACK forces Never and disables the choice). The `crash_reports.setting` gauge now reads it | PASS | 2026-10-18 | claude-opus |
| `ask` keeps the report locally and asks on next launch with a full read-only preview: Send / Don't send / Always send | R2 | A panic hook keeps the report under `crashes/asking/`. `CrashReportPrompt` shows the whole report and the three answers (Always send also changes the setting; Escape is Don't send). Tests: `crash-reports.test.tsx` ("the next launch shows the whole report and asks…"; nothing asked under Never or Always); capture `64-crash/ask-*` | PASS | 2026-10-18 | claude-opus |
| Scrubbing on the Mac before anything is kept: the home directory → `~`, no user name, no URLs or query strings, secret-shaped strings masked, no environment, host name or user field, ≤ 50 backtrace lines | R3 | `crash::scrub` / `report_of`: `a_seeded_crash_is_scrubbed_before_it_is_kept` (a home path, the user name, an `sk-ant-…` key, a URL with a token and a query string: none survive; `~/…` kept for readable paths; the report has no `server_name`, `user` or environment field). App-internal breadcrumbs: the app keeps none, so a report carries none | PASS | 2026-10-18 | claude-opus |
| On-device limits: one per crash signature a day, five a day in total | R4 | `the_limits_hold_one_per_signature_and_five_a_day` (a repeat signature refused; the sixth refused; a new day starts over; `never` keeps nothing). The signature hashes where it happened plus the failure's kind, never its message | PASS | 2026-10-18 | claude-opus |
| R5: the address (Sentry's DSN) compiled into release builds only; debug builds never send | R5 | `build.rs` sets `crash_sender` only for a release build whose `AI_JUGGLER_CRASH_ENDPOINT` is `https://…@…` (a DSN); `crash_send` is declared only under that flag (and in its tests) and started once from `lib.rs`; `crash::service_address()` is `None` otherwise. `this_build_cannot_send`, `the_crash_sender_is_built_only_with_an_address`, `crash_reports_address_reaches_only_the_release_build` (no workflow, script or Cargo config names the variable yet) | PASS | 2026-11-13 | claude-opus |
| R5: reports go through Sentry's own SDK with its defaults off (D-108) | R5 | `sentry = "=0.49.5"`, `default-features = false`, no features: the client only, no transport, integrations (OS, device, environment contexts, debug images, panic hook), sessions, logs or metrics. One file uses it (`crash_sentry.rs`); `sentry::init` is never called, so nothing is global and the SDK captures nothing by itself. Options: `send_default_pii = false`, no server name, no integrations, `release` = `ai-juggler@<version that crashed>`, `environment` = the edition (`main`, `nightly`, `dev`). The event: level, the failure's kind and message, its frames (oldest first, file and line from the Rust backtrace), the report's signature as Sentry's fingerprint, `channel` and `os` tags. `only_one_file_uses_sentrys_sdk_and_without_its_defaults` (trust.rs: one file, no global set-up, each option, the exact pin and no features), `a_screen_error_and_a_native_crash_say_what_they_are` | PASS | 2026-11-13 | claude-opus |
| R5: every event through our scrubber in `before_send`, breadcrumbs too | R5 | `before_send` empties `user`, `server_name`, `request`, `contexts` and `modules`, keeps at most 50 breadcrumbs and 50 frames a stack, and runs `crash::scrub` over every string of crash text (the SDK's own fields — event id, timestamp, level, SDK line, release, environment, fingerprint — left as they are); an event that doesn't read back is dropped, never sent unscrubbed. `a_seeded_crash_reaches_the_sdks_envelope_scrubbed`: a report whose scrub had missed everything (home path, user name, an `sk-ant-…` key, an `…_API_KEY=` value, a URL with a token, a query string) is built into the SDK's envelope and handed to a capturing transport (no network): none survive, and the event has no `user`, `server_name`, `request`, `contexts`, `modules` or `debug_meta`. `a_seeded_crash_leaves_scrubbed_through_the_sdk` (the same through the sender and a recording transport). `breadcrumbs_and_frames_are_capped_and_scrubbed` | PASS | 2026-11-13 | claude-opus |
| R5: only what the user agreed to: nothing on Never, on Ask only after Send or Always send, on Always after the same scrub; `DO_NOT_TRACK` forces Never | R5 | Never: nothing is kept (R1), so no event is built and nothing is read or sent (`never_sends_nothing_and_reads_nothing`; `crash_info` builds a preview only for a waiting report, of which Never has none). Ask: a report waiting for the answer is never sent (`a_report_waiting_for_the_users_answer_is_never_sent`); Send / Always send move it to `queued/` (`the_setting_and_answers_move_reports_and_never_deletes_them`). Always: kept reports go, scrubbed again (`a_seeded_crash_leaves_scrubbed_through_the_sdk`) | PASS | 2026-11-13 | claude-opus |
| R5: the preview shows exactly the event that would go | R5 | `CrashInfo.asking_event` is the SDK's prepared event for the report the question shows, as JSON; the prompt shows it (falling back to the stored report only where no backend runs). `the_preview_is_exactly_the_event_that_goes` (the preview and the envelope's event are the same value; the same report is always the same event id, so a retry is one issue). UI: `crash-reports.test.tsx` "the preview is the Sentry event exactly as it would go" (vitest, run by the preflight) | PASS | 2026-11-13 | claude-opus |
| R5: the queue sends when a DSN exists and the answer allows; kept until Sentry took it; the limits stay | R5 | `crash_send::drain` sends `queued/` oldest first as the SDK's envelope (`POST https://<host>/api/<project>/envelope/`, `X-Sentry-Auth` with the public key only, `application/x-sentry-envelope`) over the app's own HTTPS client (`juggler-account`'s ureq + rustls); removed on 2xx; a 4xx other than 408/429 is dropped; 429 (Sentry's rate limit), 5xx or no answer keeps it for the next try; at most 5 sent a day on top of keeping's limits. `each_queued_report_goes_as_its_sentry_event_and_is_then_removed`, `a_report_not_taken_stays_for_the_next_try`, `a_report_refused_for_good_is_dropped_and_the_rest_go`, `at_most_five_go_a_day`, `only_https`, `the_sent_log_keeps_a_week`, `the_limits_hold_one_per_signature_and_five_a_day` | PASS | 2026-11-13 | claude-opus |
| R5: minidumps are tested in a signed, notarized build before they are enabled | R5 | Not built, so not enabled (plan and cost below). Crashes outside Rust are covered by macOS's own report, read and scrubbed at the next launch (AJ-E1516) | PASS | 2026-11-13 | claude-opus |
| R5: a report reaches a real Sentry project from a signed build | R5 | The secret exists (2026-11-13; the client key keeps no IP addresses and is limited to 100 an hour). Wired the same day: `build-macos.yml` takes `AI_JUGGLER_CRASH_ENDPOINT` as a named secret for its compile step only, passed in by `nightly.yml` (no other workflow names it, none prints it: `crash_reports_address_reaches_only_the_release_build`); the Privacy line, README and SECURITY say release builds carry the address. Left: one deliberate test crash, crash reports on Always, from a signed nightly build of the 0.5 code (the Nightly edition, its own bundle id, with a throwaway HOME, never beside a running installed AI Juggler of the same identity), and one scrubbed event seen in Sentry with no IP, user, host or path | OPEN | 2026-11-13 | claude-opus |
| No cost when `never` | perf budget | The hook checks one atomic and returns; nothing is written or read, and nothing of the SDK is built | PASS | 2026-11-13 | claude-opus |

## Why the app's own HTTPS client, not one of the SDK's transports (D-106)

The SDK offers `reqwest`, `curl` and `ureq` transports. Each sends from a
background thread and reports nothing back: `send_envelope` queues the
envelope and drops it when the channel is full
(`sentry-0.49.5/src/transports/thread.rs`), so a report could be deleted
before Sentry took it, or lost. The SDK documents custom transports
(`ClientOptions::transport`, the `Transport` trait, `sentry-core-0.49.5/src/transport/mod.rs`),
and its public API gives each part separately: `Client::prepare_event`
(defaults and `before_send`), `Envelope::to_writer`,
`Dsn::envelope_api_url` and `Dsn::to_auth`. AI Juggler uses those and
sends over the HTTPS client it already has, reading the answer, so no
second HTTP or TLS stack enters the app and the queue keeps each report
until Sentry took it. The SDK's own client reports (counts of events it
dropped) aren't sent: the envelope holds the one event the preview showed.

## The dependency

4 crates added: `sentry` 0.49.5, `sentry-core` 0.49.5, `sentry-types`
0.49.5 and `debugid` 0.8.0 (everything else they use was already in the
tree; `rand` 0.10 gains its `std_rng` feature). Licences MIT. cargo vet:
`debugid` 0.8.0 by Mozilla's imported audit, `sentry-types` as a delta from
Embark's imported 0.30.0 audit, `sentry` and `sentry-core` audited here
(`supply-chain/audits.toml`, notes say what was checked); no new
exemption. cargo deny: `debugid` ships a maintainer's version-bump script
(`scripts/bump-version.sh`, run by no build), allowed by name in
`deny.toml`.

## Native minidumps: the plan and its cost

Researched (D-106): the SDK's `minidump` feature brings `sentry-minidump`
0.49.5 (the successor to the deprecated `sentry-rust-minidump`), built on
Embark's `minidumper`, `minidumper-child`, `crash-handler` and
`minidump-writer`. From its source (`sentry-minidump-0.49.5/src/lib.rs`):

- It works inside `sentry::init`: the app re-executes itself as a second,
  long-running "crash reporter" process, which on a crash writes the
  minidump and **sends it at once** as an attachment through its own
  client. That skips Ask (no preview, no answer) and needs a global
  client, against this design (one seam, nothing global, lazy).
- A minidump is raw memory: every thread's stack, which can hold paths,
  prompts and tokens. `crash::scrub` can't scrub it and the preview can't
  show it meaningfully.
- Cost: 21 more crates (crash-context, crash-handler, error-graph,
  failspot, goblin, mach2, memmap2, minidump-common, minidump-writer,
  minidumper, minidumper-child, nix, num-derive, plain, procfs-core,
  range-map, scroll, scroll_derive, sentry-minidump, smart-default, uds),
  much of it unsafe FFI (Mach exception ports, reading another process's
  memory), each needing a cargo vet audit; a second process for the life
  of every launch.
- macOS: the reporter reads the crashed app's task through a port the app
  hands it; under the hardened runtime of a signed, notarized build that
  must be proven before anything is enabled (R5).

**Plan:** not built now. Crashes outside Rust already arrive as macOS's
own `.ips` report, read, scrubbed, previewed and sent like any other
(AJ-E1516). If minidumps are wanted later, as their own task (effort M–L):
use `minidumper` directly rather than the integration; keep dumps only on
the Mac (`crashes/`), never sent by the crash process; at the next launch
ask with what can be shown (size, thread count, the scrubbed `.ips`
summary); send the dump as the event's one attachment only after Send or
under Always; behind a code flag that stays off until a signed, notarized
build's test is recorded here.

## What waits on the secret

- The maintainer creates the Sentry project (sponsored open-source plan,
  D-108) and adds its DSN as `AI_JUGGLER_CRASH_ENDPOINT` to the release
  workflow.
- Then, in one change: the Privacy line (`ui/src/lib/trust-claims.json`,
  `docs/guide/trust.md`), README's row and SECURITY's "until a release
  build is given Sentry's address" lose their "not yet" part, and
  `crash_reports_address_reaches_only_the_release_build` becomes a check that only the release
  workflow names the variable.
- One crash report sent from a signed build and seen in Sentry closes the
  last R5 row.
