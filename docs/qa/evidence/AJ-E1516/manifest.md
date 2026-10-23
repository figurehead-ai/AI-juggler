# Evidence manifest — AJ-E1516

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1516 (the crash report sender and native crash dumps). Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR). 2026-10-23, claude-opus.

The service and its address are the maintainer's step. No build has the address, so no build has a sender, and the Privacy page still says "This version has nowhere to send them."

## Done

| Requirement | Proof | Result |
|---|---|---|
| Req.1 a sender for the scrubbed reports | `crash_send::drain` sends what waits in `queued/` — a report the user answered Send to (Ask), or one kept under Always — exactly as stored (the JSON the user could read), as a `POST` with no credentials, over HTTPS only. A report is removed once the service took it (2xx). One refused for good (a 4xx other than 408/429) is dropped, so it can't hold up the others. Busy, down or not reached, it stays for the next try. Each report is read just before it goes, so one removed meanwhile (the user chose Never) doesn't go. Unit tests: `each_queued_report_goes_exactly_as_stored_and_is_then_removed`, `a_report_not_taken_stays_for_the_next_try`, `a_report_refused_for_good_is_dropped_and_the_rest_go`, `only_https`. | PASS |
| Req.1 Never / Ask / Always | Never sends nothing and reads nothing (Never already deletes every kept report). Ask sends only the reports the user answered Send to: one still waiting for the answer (`asking/`) is never sent. Always sends what's kept. `never_sends_nothing_and_reads_nothing`, `a_report_waiting_for_the_users_answer_is_never_sent`; the keeping side is unchanged from AJ-E1107. | PASS |
| Req.1 the daily limits | At most 5 sent a day (`sent.log`, days only, a week kept), on top of keeping's limits (one per crash signature a day, five a day in total). `at_most_five_go_a_day`, `the_sent_log_keeps_a_week` | PASS |
| Req.1 the address in release builds only | `build.rs` sets `cfg(crash_sender)` only for a release build whose `AI_JUGGLER_CRASH_ENDPOINT` is an `https://` address; the sender module is declared only under that cfg (and in its own tests), and only `lib.rs` starts it. `crash.rs` still holds no way to send, and reads the address only for `can_send`, under the same flag, so "can send" and the sender never disagree. The sending loop (`run`) is compiled in every test build, so CI keeps it honest; only the address and the thread are behind the flag. No workflow, script, Cargo config or the justfile sets the address. `trust.rs` `crash_reports_have_nowhere_to_go` (unchanged), `the_crash_sender_is_built_only_with_an_address` (new). | PASS |
| Req.1 nothing sent in debug builds | `build.rs` requires the release profile; `service_address()` is `None` in debug builds as before. | PASS |
| Req.2 native crash dumps, scrubbed the same way | A crash outside Rust leaves no panic. macOS writes its own report: `~/Library/Logs/DiagnosticReports/<name>-<time>.ips`, a header line (`bundleID`, `bug_type` `309`) and a body (`exception`, `faultingThread`, `threads[].frames`, `usedImages`). At the next launch, with reports on, this build's own new reports become crash reports through the same `report_of` (scrub, signature) and `keep` (Ask/Always, limits). Only the kind (`EXC_BAD_ACCESS (SIGSEGV) …`) and the crashing thread's frames (library and function) are kept. The body's user id, process path, crash reporter key, incident id, image ids, registers and memory summary are not. `crash_native` `an_ips_crash_keeps_its_kind_and_frames_and_nothing_else` (a report in macOS's shape, checked against a real one's keys). | PASS |
| Req.2 only this app's, only new, nothing under Never | Another app's report is told apart by its first line alone, and a non-crash (hang) report is left alone. Links and files over 4 MiB aren't read. Under Never nothing is read. The first look only marks where to start, so crashes from before the user turned reports on aren't picked up. Turning reports on marks it at once, so a crash later in that same session is picked up. A report already looked at isn't kept twice. A report macOS is still writing (under a minute old, not parsing yet) is looked at again: the app looks at launch and a minute later, since the watchdog can reopen it before macOS writes the report. The report keeps the version that crashed. `another_apps_report_or_a_non_crash_is_left_alone`, `only_crashes_after_the_first_look_are_kept_and_never_reads_nothing`, `a_crash_after_turning_reports_on_is_picked_up`, `a_report_still_being_written_waits_for_the_next_look`, `links_and_oversized_files_are_not_read`, `under_always_a_native_crash_is_queued_with_the_version_that_crashed`. | PASS |
| Trust page | A line under "What it reads": "With crash reports on, it looks through macOS's new crash reports for AI Juggler's own, and keeps only what failed and where." The crash-reports line keeps "This version has nowhere to send them" and now also names the sender's build test. `trust.rs` (every line names its tests). | PASS |

## Checked

- The sender compiles and passes clippy in a release build given an address (`AI_JUGGLER_CRASH_ENDPOINT=https://crash.invalid/v1 cargo clippy --release -p juggler-app --lib -- -D warnings`), so the code no build ships today is still kept honest.
- Performance: Never costs nothing. With reports on, one folder listing at launch; reports over 4 MiB aren't read.

## After the independent review

The review found no blockers. Fixed from its list:
- **One flag for "can send" and the sender.** A custom profile could have made them disagree.
- **The sending loop is compiled in CI.**
- **A refused report no longer blocks the queue.**
- **Never mid-send sends nothing more.**
- **The session where reports are turned on keeps its native crash.**
- **The watchdog's relaunch no longer outruns macOS's report.**
- **Other apps' reports are read to their first line only.** The Privacy line now says what it looks through.
- **More tests:**
  - a waiting report is never sent;
  - the 4xx handling;
  - the log's week;
  - Always;
  - links and oversized files.
- **Smaller fixes:**
  - the report keeps the version that crashed;
  - the first look's mark is the newest report there (the clock can be wrong).

Left as known:
- `kept.log` has no lock between the panic hook and the launch scan. At worst, one extra report past the day's limit.
- A native report found after the launch question showed waits for the next launch.

## Research basis

AJ-E1107 (the setting, scrub and limits). macOS's crash report format: the keys of a real `.ips` crash report from a public repository (noah-nuebling/mac-mouse-fix, `Crash Reports/`). The fixtures are synthetic, in that shape.

## Left (the maintainer's step)

- **The service.** Its address goes into the release workflow as `AI_JUGGLER_CRASH_ENDPOINT`. Then:
  - the Privacy line's "This version has nowhere to send them" changes, with `crash_reports_have_nowhere_to_go`;
  - the service's contract (status codes, size limit) can tighten `drain`, which today treats any non-2xx as "try again".
- **Other platforms.** Native crash reports are read on macOS only.
