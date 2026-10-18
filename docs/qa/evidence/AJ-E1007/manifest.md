# Evidence manifest — AJ-E1007

Task: `docs/tasks/E10-derived-workflows-and-beta.md` AJ-E1007 (session
lifecycle: search, archive, replay, retention). 2026-10-18, reviewer
claude-opus.

## How it works

**History** is a global view (header) listing every Instance that has
ended, with separate **Ended** and **Archived** lists. One search box
covers:
- project, Account, Provider, model, branch and date (YYYY-MM-DD), matched
  on the read model in the view;
- transcript text, from a local index. A matching row shows the words
  around the match.

Each row offers **Replay**, **Archive** or **Unarchive**, and **Delete…**.

- **Transcript index (V27).** A contentless FTS5 table, so it holds the
  search terms and never a second copy of anyone's conversation. Each row
  points back at one line of the original transcript, and the snippet is
  read from there. Only message text is indexed: the `text`, `content`
  and `thinking` fields, never ids, paths or signatures. Indexing is
  incremental: each transcript remembers how many bytes are indexed, and
  a file that shrank or moved is reindexed. It runs in the background
  every 60 s, and a search brings it up to date first unless the
  background pass is already running. The user's words are always
  quoted as plain terms, so FTS syntax cannot be injected.
- **Archive and unarchive** are lifecycle events in the V26 journal.
  Nothing about the Instance changes. Archived Instances leave the rail
  and stay in History. Only an ended Instance can be archived.
- **Delete** is separate, irreversible, and has no single-click path:
  1. The preview names everything it removes (the record, N history
     events, N index lines, account checks) and everything it keeps (the
     Provider's own transcript file, by path; the process and cleanup
     audit; usage totals).
  2. The dialog arms only once "delete" is typed.
  3. The backend requires the preview's one-time token, which is
     single-use and expires after 10 minutes.

  A live Instance is refused, and so is one that is part of a switch
  history, because deleting it would break the lineage; archiving is
  offered instead. AI Juggler never deletes Provider data.
- **Replay** opens an ended or archived Instance read-only, in the same
  terminal view as a live one with input disabled. Alongside it: its
  history (status changes and lifecycle steps), its switch lineage and
  its process record from the spawn ledger. The header shows the
  transcript's size and sha256. Replay shows at most the last 16 MB, and
  says so when it does.
- **Retention** is set per Account in Settings › Storage & retention:
  keep forever (the default), 1 year, 6 months, 90 days or 30 days. The
  card shows what is stored per Account: Instances, history events,
  index lines, and the measured size of the Provider's transcripts.
  A policy never deletes on its own. **Review cleanup…** runs a dry run
  listing every Instance it would remove and every old one it keeps,
  with the reason. Only that dry run's token can apply it, and the
  cleanup removes AI Juggler's records only (the E5 never-touch-unowned
  rule).

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Search covers project, Account, Provider, model, branch, date and transcript text, with a snippet, in < 500 ms over 1,000 sessions | Req.1 | `session_lifecycle :: search_finds…`: 1,000 fixture sessions; "stripe webhook" finds exactly the one session, with its snippet, in **15 ms**. Appended text is found on the next search. vitest `history.test.tsx`: every field matches; the transcript hit shows its snippet | PASS | 2026-10-18 | claude-opus |
| Archive removes from active views without deleting; unarchive restores exactly; delete is separate, confirmed, irreversible, naming the data destroyed | Req.2 | `archive_round_trips_exactly…`: the snapshot after archive and unarchive equals the snapshot before (lifecycle fields aside), in under 1 s; a live Instance is refused. Delete: a guessed token is refused and nothing goes; the preview names what is removed and kept; with the token, the record is gone and the Provider's transcript file still exists; the token is single-use; a live Instance's preview is refused with no token. vitest: the button stays disabled until "delete" is typed, and a refused preview has no delete button | PASS | 2026-10-18 | claude-opus |
| Replay opens an archived or ended session read-only, with history, lineage and process record | Req.3 | `search_finds…`: an archived session's replay transcript is **byte-identical** to the file, its sha256 matches, entries parse with roles, and the history includes `archived`. vitest: titled "Replay · read-only", with history, lineage and process rows. The terminal has `disableStdin` | PASS | 2026-10-18 | claude-opus |
| Retention per Account (keep forever by default), storage used per Account, a dry run before any bulk cleanup | Req.4 | `retention_lists_first…`: forever by default, with no token to apply; 90 days lists only the 100-day-old ended Instance and keeps the old live one with its reason; the dry run deletes nothing, and a wrong token deletes nothing; the dry run's token removes exactly 1; transcripts stay. vitest: forever by default, Review disabled until a policy is set, the dry run lists deletes and kept rows before apply | PASS | 2026-10-18 | claude-opus |
| Index build < 30 s per 1,000 sessions, incremental thereafter; search < 500 ms; archive < 1 s; storage accounting < 2 s | perf | Index build **127–212 ms** for 1,000 small fixture transcripts; a second pass reads 0 lines; search **15–17 ms**; archive and unarchive well under 1 s; accounting runs inside the same tests. Log: `fixture-runs.log`. Real transcripts are larger: the build runs in the background, so a search never waits for a first build | PASS | 2026-10-18 | claude-opus |
| Quality gates: byte-identical replay; no single-click delete path; 100% index correctness on the fixture | quality gate | As above. Every probe (`note 0/1/500/999 about`) finds its own session | PASS | 2026-10-18 | claude-opus |
| Visual: search results, archived list, replay view and retention settings, 1280×800 and 1024×680, both themes; state text explicit; delete consequence copy exact | visual / a11y | Gallery `56-history` (search, archived, replay, delete, retention, dry run: 24 captures). Playwright `history and delete dialog — axe 0` (both themes): search focused on open, and the delete button is disabled until confirmed. The captures caught three problems, all fixed: dry-run rows showed bare session ids (now the project name and date), the storage row was cramped at 1024 (now on two lines), and an eighth header item truncated the screen title at 1024 (header spacing tightened below xl) | PASS | 2026-10-18 | claude-opus |
| A dogfood user finds a 3-week-old session unaided (AJ-E809 script) | evidence | `docs/qa/dogfood-backlog.md` | DEFERRED-TO-DOGFOOD | 2026-10-18 | claude-opus |

## Test counts

- `cargo test -p juggler-app --test session_lifecycle`: 3/3.
- `cargo test -p juggler-core`: all green at schema v27, including the
  index module's unit tests (FTS syntax never passes through; JSON lines
  index message text only).
- vitest `history.test.tsx` 6/6. Playwright `56 history` 4,
  `history and delete dialog — axe 0` 2.
