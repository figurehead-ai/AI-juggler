# ADR-0024: The conversation feed — append-only, by offset, followed while shown

- Status: Accepted (0.3.5, 2026-11-02)
- Date: 2026-11-02
- Deciders: the maintainer (report of 2026-11-02: "it should append, and sync properly … the whole presentation layer of conversation, its architecture, needs to be rethought"); feed builder (draft)
- Relates to: D-14 (secrets redacted), D-37 (read while the page shows, kept nowhere), ADR-0023 (memory tiers), AJ-E2108 (a session's last messages), AJ-E1520 (subagents), ENGINEERING-GUIDE T1/T2/T3/I4/I6/U4

## Context

In 0.3.4 a session's page read its conversation by polling: every 4 s the page asked for "everything from where my first turn starts" and replaced its whole list with the answer. The backend read a fixed 2 MiB window, rebuilt every turn in it and sent up to 200 turns each time.

Watching a long Claude turn, the card emptied mid-turn ("The end of its transcript holds no messages." under "Claude is working") and the page flashed. The causes, reproduced with synthetic transcripts:

- **A fixed window that gave up.** When a turn grew more than 2 MiB past the page's start (big tool results, a subagent's lines in the main file, progress records, one 3 MiB tool output), the read fell back to the last 2 MiB and treated "no turn found there" as "no turns". The page then followed from the window's start, so the earlier turns never came back on their own.
- **Replace, not append.** Each answer gave every turn a new identity. Turns were keyed by their index, so rows were reused for other turns and what the reader had opened moved to another turn. The live last turn switched element trees when it finished.
- **Scroll.** Every update counted as a prepend, so a reader who had scrolled up was pulled down; any status other than "shown" swapped the list for a message, and the scroller remounted at the bottom.

## Decision

1. **One turn builder, fed whole lines with their byte offsets** (`juggler-adapters/src/conversation_feed.rs`, `TurnLog`). Opening a page, following it and "Show earlier" use the same builder, so turn boundaries never disagree. The rules are the page's own: who spoke, one speaker's consecutive messages are one turn, a `<tag>` line of yours (a notification, a command's output) ends the turn before it, a message the person sent mid-turn is theirs, a subagent's lines stay out of a session's conversation. Lines are read one at a time (`conversation_lines.rs`), never matched to messages by position.

2. **A turn's id is `"{generation}:{offset of its first line}"`.** The offset is the same in every read of the file; `generation` changes only when the conversation's file changes (another file, a rewrite, a shorter one). A turn that grows keeps its id and its `revision` goes up.

3. **"Found nothing" is never "there is nothing".** The backward scan (`last_turns::scan_back`) reads windows of 256 KiB, 1, 4 and 16 MiB back from where a page ends, passes stretches with no turn (up to 48 MiB a call) and, when it stops before the file's start, says where (`earlier`). Only a read that reaches the start answers that there is nothing before. A window's first turn may have begun before it and is dropped when later ones follow (never when it is the only one: a run of replies longer than the window shows its end).

4. **The page follows; the backend sends only changes.**
   - `conversations_follow` / `subagents_follow` answer a snapshot (the latest turns, at most 60) and send each change after it on the page's own `tauri::ipc::Channel` (T3): `{generation, revision, kind: append | reset | state, upserts, dropped_before, status}`. Per subscriber, never broadcast, so conversation text never goes to a window that didn't ask.
   - `conversations_unfollow`, `conversations_keep` (a lease: the page says every 30 s which follows it still has; a page that went away without saying so is let go after 90 s) and `conversations_earlier` (a page of at most 60 turns before an offset: a keyset page, I6).
   - `conversations_last_messages` and `subagents_conversation` stay one release for the read-only preview callers, on the new reader; removed in 0.4.0 (I4).

5. **One registry, one thread** (`juggler-app/src/features/conversations/feed/`, managed as its own state, T2).
   - One feed per conversation (or subagent), shared by every page that shows it. It holds the transcript's tail and the turns read, **redacted as they are read** (D-14: the log holds no raw text past one line).
   - Bounded: a held line ≤ 8 MiB (longer ones are passed over and counted), text ≤ 16,000 characters a block, ≤ 200 blocks a turn, ≤ 300 turns and 4 MiB a feed (oldest let go first, a "Show earlier" away), ≤ 100 turns a change (more is sent as a reset), ≤ 16 MiB read a pass (more on the next), a gap past 64 MiB read again from the end. Unfollowed feeds: kept a minute, 4/8/16 by memory tier (ADR-0023), at most 32 in all.
   - The thread `conversation-feeds` starts with the first follow. It is woken by the followed files' folders changing (`folder_wake.rs`: only those folders, not recursively, only while followed) and checks the files every second as the fallback (events can be late or dropped). Changes are batched: 150 ms after the last, at most 500 ms after the first. With nothing followed it sleeps.
   - A file replaced, made shorter or swapped for another (a resumed Codex thread writes a new rollout) is read again from its end and sent as a `reset`. A file stored compressed (a finished Codex rollout) has `ended`; one gone keeps what was shown and says so. Turned off in Settings, every page hears `off` within a second and everything is let go.

6. **The page** (`ui/src/lib/conversation-store.ts`, `conversation-feed-state.ts`, `pages/session/useConversation.ts`).
   - A module-level store per conversation, read through `useSyncExternalStore` (U4); the last 8 conversations stay in memory, so going back shows at once while the follow catches up.
   - Pure steps (`applySnapshot`, `applyDelta`, `applyEarlier`): a change replaces only the turns it names, so every other turn keeps its object and React its row; new turns go by offset; a stale change is dropped; a missed one (a gap in revisions) follows again and the snapshot merges with what is shown; earlier pages merge by id.
   - Rows are keyed by id and memoized; markdown is parsed once per text. A turn has one element tree whether working or done.
   - Once turns were shown, nothing replaces them with an empty card: a problem is a line over them ("Couldn't read the latest changes. Trying again.").
   - Scroll (`useStickToBottom`): opening shows the bottom; new words keep it there only when the reader is there; a reader who scrolled up stays put and a "New messages" pill takes them down; earlier turns put above keep the turn being read in place (found by id, not CSS scroll anchoring).

7. **The turn carries what a renderer needs, and its layout.** Each block says its kind (text, narration, a run of work), each call its tool and the inputs a title needs (description, command, file, path, pattern, query, subagent type), whether it is done or failed and what it did to a file; text says whether thinking came first and its message id. The adapter says how the tool's own app draws a turn (`layout`: the Claude app's stretches, ChatGPT's Codex fold) and writes a run's title by the app's own rules (`claude_folds.rs`), so the page draws it without knowing which tool it is.

## Consequences

- A long turn appends: the open turn grows under its id; nothing else is re-sent or re-rendered, and the page never empties mid-turn. Following costs one `stat` a second per followed file while a page shows it, and only the appended bytes are read.
- D-37 holds in a new form: words are kept in memory while a page shows them and a minute after, redacted, and never written. Settings and the trust page say so.
- Very long single turns (more than 16 MiB with no other turn) open on their end; their start, a "Show earlier" away, can show as a second row of the same speaker.
- A page reloaded without unfollowing costs at most 90 s of reads for its follows.
- `content-visibility` was left out: with it, rows not yet drawn have estimated heights, which would move the bottom the scroll sticks to. A list of a few hundred memoized rows is cheap; a virtualizer comes only if the kept window grows past that.

## Alternatives considered

- **A bigger window.** Moves the cliff, keeps "nothing found" as "nothing there", and re-reads more each tick.
- **Polling deltas by offset (`since`).** No push, but still a timer per page and a read per tick; the Channel sends only when something changed.
- **An app-wide event with the text.** Every window would receive every followed conversation's words (T3 reserves events for app-wide notices, and D-37 for what a page shows).
