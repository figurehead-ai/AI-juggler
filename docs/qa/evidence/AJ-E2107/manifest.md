# Evidence manifest — AJ-E2107

Task: `docs/tasks/E21-desktop-first.md` AJ-E2107, Needs you from real
signals. Branch `e21-rail`. Status: **IN-PROGRESS**: the signals, the
"probably" rule, the seen mark and the count are built and tested below;
not yet seen on the real setup (a session waiting on a permission in
Terminal, on the maintainer's Mac), and the Codex CLI has no tail reader
yet (its adapter answers "nothing pending", so it never says Asks you
from a tail).

## What exists now

1. **The apps' recorded words.** The Claude app's `status_category`
   (AJ-E2101's reader) maps blocked → Asks you, review_ready → Done,
   review it, completed → Idle. New: Claude Code's own session file
   records `status: "waiting"` when a session waits on a permission or a
   question; it reads as `LiveStatus::Waiting` → Asks you, said plainly
   (recorded, not inferred). Only the status word is kept: the file's
   `waitingFor`, `needs` and `detail` text never are
   (`crates/juggler-adapters/src/claude_live.rs`).
2. **Terminal sessions' tails.** For a live, interactive session in no
   desktop app whose tool didn't say it asks, AI Juggler reads the last
   64 KB of its transcript, again only when the file changed
   (`crates/juggler-app/src/features/conversations/asks.rs`). The
   adapter says what the tail waits on
   (`HarnessAdapter::pending_ask`; Claude Code: `claude_asks.rs`, keeping
   only line types and tool ids, never text): an open question tool
   (AskUserQuestion, ExitPlanMode) asks you at once; another tool call
   with no result asks you once nothing was written for a minute while
   it works (a long run is not mistaken for a question within that
   minute). Either says **"Probably asks you"**
   (`ConversationInfo.probably`). The live tick re-checks tails even when
   nothing new opened.
3. **Seen mark.** Opening a session another app runs, or going to it,
   marks it seen (`ui/src/lib/seen.ts`, kept in AI Juggler's settings,
   the newest 300 marks, read guarded). "Done, review it" clears once the
   seen mark, or the app's own last view, is after its last activity.
4. **The count.** The rail's Needs you, Home's heading and Needs you
   card count AI Juggler's sessions and the sessions other apps run that
   ask you or have something to review (`needsYou` in
   `ui/src/lib/rail-items.ts`).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 the recorded words map, "waiting" asks you | `cargo test -p juggler-app --lib features::conversations` (`the_states_map_to_e21s_words`); `cargo test -p juggler-adapters --lib claude_live` (`a_session_waiting_on_the_user_reads_as_waiting_and_its_words_are_never_kept`) | blocked→Asks you, review_ready→Done, completed→Idle, waiting→Asks you; no summary text kept | pass (39 + 20) | PASS | 5f8b2904, 13e58677 | 2026-10-24 | agent |
| Req.2 a terminal tail: a question or a pending call | `cargo test -p juggler-adapters --lib claude_asks` (4 tests); `cargo test -p juggler-app --lib features::conversations::asks` (3 tests) | question at once; pending only after a quiet minute; desktop apps, background runs and recorded asks untouched; ≤ 64 KB, re-read only when changed | pass | PASS | 5f8b2904, 13e58677 | 2026-10-24 | agent |
| Req.3 Done, review it clears once seen | `cd ui && npx vitest run src/lib/rail-model.test.ts` ("Done, review it clears once the user looked") | clears on the seen mark or the app's view; comes back after new activity | pass | PASS | cdcad91b | 2026-10-24 | agent |
| Req.4 anything inferred says probably | `rail-model.test.ts` ("anything worked out says probably"); `place-board.test.tsx` ("a row's label leads with its state; one worked out says probably") | "Probably asks you" on the row and in its label | pass | PASS | cdcad91b | 2026-10-24 | agent |
| Needs you counts sessions other apps run | `rail-model.test.ts` ("Needs you counts sessions other apps run…"); `home-places.test.tsx` ("one sentence about what needs you…") | outside asks and reviews counted on the rail and Home | pass | PASS | e1baac01 | 2026-10-24 | agent |
| By hand: the seen mark clears Done | browser preview (places data), click "Tidy the blog's images" | Needs you 3 → 2, Home "4 things" → "3 things" | as expected | PASS | cdcad91b | 2026-10-24 | agent |
| Real setup: a Terminal session waiting on a permission | the maintainer's Mac, read-only | Asks you within a tick | not run | UNPROVEN | — | 2026-10-24 | agent |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
