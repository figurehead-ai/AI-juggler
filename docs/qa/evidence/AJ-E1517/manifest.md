# Evidence manifest — AJ-E1517

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1517, Export diagnostics (P1).
Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR).

Gallery: `docs/qa/evidence/_gallery/85-diagnostics/` (the preview with a
file open and the settings left out; where it was saved; both themes).

## What exists now

1. **Settings › General › Report a problem**, under About & updates:
   **Report a bug** opens the repository's bug form
   (`issues/new?template=bug_report.yml`); **Export diagnostics…** shows
   what the zip will hold, file by file (each one opens to its whole
   text), and names the settings left out. **Save…** asks where (a
   `.zip`), then says where it went and its size.
2. **What's in it** (`features/app/diagnostics/`):
   - `about.json`: AI Juggler's name, version, edition and channel; the
     OS, its version and the architecture; the schema this build writes
     and the one on disk; the tool versions already known (no tool is
     started); counts of accounts, launch settings, projects and sessions
     by status (no names).
   - `app-settings.json`: the app settings, every key and value scrubbed;
     left out entirely (and named): what only means something on this Mac
     and what holds tokens, addresses or paths (`restore_token`,
     `history_tokens`, `figurehead_*`, `fleet_*`, `cloud_backup_*`,
     `credits:*`, `vault*`, identity pins, parked sessions, worktrees,
     the recovery summary, redaction overrides, the editor).
   - `logs/`: the newest 256 KB (whole lines) of the logs AI Juggler
     keeps: sessions started and ended, usage statistics sent, crashes
     kept, first-run steps, how the last run ended, reopens after a crash.
   - `crashes/`: the crash reports waiting to be sent or asked about.
   - Never: transcripts, terminal output, an Account's folder,
     credentials, the Keychain.
3. **Scrubbed like crash reports** (`crash::scrub`, line by line): the
   home folder becomes `~`, the user name `[user]`, links `[url]`, and
   secret-shaped strings are masked. The crash scrub now also masks a
   credential-like `NAME=value` within a line (the shared redaction's
   `.env` rule only reads one at a line's start; its name test is now
   `redaction::credential_name`, used by both).
4. **What you saw is what's saved**: the preview is kept in memory and
   the export writes exactly it; a stale preview is refused ("show it
   again"). Before writing, a last scan of every file refuses anything
   that still looks like a secret, the home folder or the user name, and
   nothing is written. It goes to `name.zip.partial`, then renamed.
5. **The zip**: a small writer of our own (deflate and CRC-32 from
   flate2, already in the tree); the `zip` crate stays out (its
   advisories, see Cargo.toml). Read back by the test's own reader and by
   the system's `unzip -t`.

## Proof

| Requirement | Evidence | Result |
|---|---|---|
| Req.1 one zip with versions, schema, settings and logs, previewed before saving | `diagnostics::tests::the_zip_holds_what_was_shown_and_nothing_it_shouldnt` (the files and their order; the zip read back equals the preview byte for byte; schema on disk = this build's); vitest `diagnostics` (every file shown, left-out named, nothing saved until Save, a cancelled sheet saves nothing); Playwright `diagnostics` | PASS |
| Req.1 scrubbed like crash reports — the scrub corpus run over the zip | same test: an `sk-ant-…` key, an `…_API_KEY=` value mid-line, a bearer token, a link with a one-time code, this Mac's home folder and user name, a restore token, a Figurehead device id and a Fleet address seeded into settings, logs and a crash report; none is anywhere in the zip, and the redaction scan finds nothing; `crash::tests::a_secret_assignment_within_a_line_is_masked_and_the_rest_kept` | PASS |
| Req.2 no transcripts or credentials; paths as `~` | same test: an Account folder's credentials file and a terminal log are seeded and never opened (`export_reads`); `~/.local/bin/claude` and `~/Dev/…` in place of the home folder | PASS |
| A leak is never saved | `it_saves_only_a_zip_only_the_preview_shown_and_never_a_leak` (a preview holding a key: refused, naming the file, the key not in the message, no file or `.partial` left); a non-`.zip` and a stale preview refused | PASS |
| Performance < 3 s | the first test times preview and export together | PASS |
| The zip opens anywhere | `zip::tests::what_goes_in_comes_back_out` (CRC checked, deflated), `the_system_unzip_reads_it_where_there_is_one` (`unzip -t`: No errors detected), `dos_time_reads_the_utc_day` | PASS |
| Report a bug | vitest and Playwright (the form opens; a fixed URL, nothing from the UI reaches it) | PASS |
| The preview, both themes, axe | Playwright `diagnostics` (axe 0 critical/serious) | PASS |

## Walked

On the preview at 800×600: Settings › General shows Report a problem
under About & updates. Export diagnostics… lists about.json,
app-settings.json and logs/supervision.jsonl with sizes, and "Left out: 3
settings that hold tokens, addresses or paths (…)". about.json opens to
its text. Save… with a chosen file says "Saved to ~/Desktop/AI Juggler
diagnostics 2026-10-23.zip (4.7 KB). Attach it to your report." and the
preview closes.

## Notes

- AI Juggler keeps few logs today: its own messages go to the terminal
  it was started from, not to a file. The zip carries the journals and
  logs it does keep; a persistent app log is a follow-up.
