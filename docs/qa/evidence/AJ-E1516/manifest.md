# Evidence manifest — AJ-E1516

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1516 (the crash report sender and native crash dumps). Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR). 2026-10-23, claude-opus.

The service and its address are the maintainer's step. No build has the address, so no build has a sender, and the Privacy page still says "This version has nowhere to send them."

## Done

| Requirement | Proof | Result |
|---|---|---|
| Req.1 a sender for the scrubbed reports | `crash_send::drain` sends what waits in `queued/` — a report the user answered Send to (Ask), or one kept under Always — exactly as stored (the JSON the user could read), as a `POST` with no credentials, over HTTPS only. A report is removed once the service took it (2xx); one it didn't take, or that didn't arrive, stays for the next try. Unit tests: `each_queued_report_goes_exactly_as_stored_and_is_then_removed`, `a_report_not_taken_stays_for_the_next_try`, `only_https`. | PASS |
| Req.1 Never / Ask / Always | Never sends nothing and reads nothing (Never already deletes every kept report). Ask sends only the reports the user answered Send to; Always sends what's kept. `never_sends_nothing_and_reads_nothing`; the keeping side is unchanged from AJ-E1107. | PASS |
| Req.1 the daily limits | At most 5 sent a day (`sent.log`, days only), on top of keeping's limits (one per crash signature a day, five a day in total). `at_most_five_go_a_day` | PASS |
| Req.1 the address in release builds only | `build.rs` sets `cfg(crash_sender)` only for a release build whose `AI_JUGGLER_CRASH_ENDPOINT` is an `https://` address; the sender module is declared only under that cfg (and in its own tests), and only `lib.rs` starts it. `crash.rs` still holds no way to send, and reads the address only for `can_send`. No workflow, script, Cargo config or the justfile sets the address. `trust.rs` `crash_reports_have_nowhere_to_go` (unchanged), `the_crash_sender_is_built_only_with_an_address` (new). | PASS |
| Req.1 nothing sent in debug builds | `build.rs` requires the release profile; `service_address()` is `None` in debug builds as before. | PASS |
| Req.2 native crash dumps, scrubbed the same way | A crash outside Rust leaves no panic. macOS writes its own report: `~/Library/Logs/DiagnosticReports/<name>-<time>.ips`, a header line (`bundleID`, `bug_type` `309`) and a body (`exception`, `faultingThread`, `threads[].frames`, `usedImages`). At the next launch, with reports on, this build's own new reports become crash reports through the same `report_of` (scrub, signature) and `keep` (Ask/Always, limits). Only the kind (`EXC_BAD_ACCESS (SIGSEGV) …`) and the crashing thread's frames (library and function) are kept. The body's user id, process path, crash reporter key, incident id, image ids, registers and memory summary are not. `crash_native` `an_ips_crash_keeps_its_kind_and_frames_and_nothing_else` (a report in macOS's shape, checked against a real one's keys). | PASS |
| Req.2 only this app's, only new, nothing under Never | Another app's report, or a non-crash (hang) report, is left alone. Under Never nothing is read. The first look only marks where to start, so crashes from before the user turned reports on aren't picked up; a report already looked at isn't kept twice. `another_apps_report_or_a_non_crash_is_left_alone`, `only_crashes_after_the_first_look_are_kept_and_never_reads_nothing`. | PASS |
| Trust page | A line under "What it reads": "With crash reports on, after AI Juggler crashes it reads the report macOS wrote about that crash, and keeps only what failed and where." The crash-reports line keeps "This version has nowhere to send them" and now also names the sender's build test. `trust.rs` (every line names its tests). | PASS |

## Checked

- The sender compiles and passes clippy in a release build given an address (`AI_JUGGLER_CRASH_ENDPOINT=https://crash.invalid/v1 cargo clippy --release -p juggler-app --lib -- -D warnings`), so the code no build ships today is still kept honest.
- Performance: Never costs nothing. With reports on, one folder listing at launch; reports over 4 MiB aren't read.

## Research basis

AJ-E1107 (the setting, scrub and limits). macOS's crash report format: the keys of a real `.ips` crash report from a public repository (noah-nuebling/mac-mouse-fix, `Crash Reports/`). The fixtures are synthetic, in that shape.

## Left (the maintainer's step)

- **The service.** Its address goes into the release workflow as `AI_JUGGLER_CRASH_ENDPOINT`. Then:
  - the Privacy line's "This version has nowhere to send them" changes, with `crash_reports_have_nowhere_to_go`;
  - the service's contract (status codes, size limit) can tighten `drain`, which today treats any non-2xx as "try again".
- **Other platforms.** Native crash reports are read on macOS only.
