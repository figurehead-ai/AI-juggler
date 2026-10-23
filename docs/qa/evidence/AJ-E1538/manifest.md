# Evidence manifest — AJ-E1538

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1538, search the words of sessions started outside AI Juggler, opt-in (P2).
Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR). 2026-10-23, claude-opus.

Gallery: `docs/qa/evidence/_gallery/90-outside-search/`, in both themes, with axe showing 0 critical or serious issues:
- Settings › Privacy, off: the size the index would take;
- Settings › Privacy, on: the index's size as it builds;
- History finding an outside session by its words, with the words around the match.

## What exists now

1. **Off by default** (Req.1). Settings › Privacy has **Search the words of every session**:
   - Before it's on, it says the index's likely size and how many sessions it covers.
   - While it's on, it shows the index's real size and progress ("212 MB so far · 1180 of 3412 sessions").
   - It says turning it off deletes the index.
   - Components: `OutsideSearchSetting.tsx`, in `PrivacyCard.tsx`.
2. **Its own file** (Req.2): `outside-search.sqlite` in AI Juggler's data folder, mode 0600.
   - `juggler-core` `SearchIndexFile` has the main store's index tables (schema v27, applied as is), so `TranscriptIndexRepo` reads and writes it unchanged.
   - Turning it off closes the file and deletes it with its `-journal`, `-wal` and `-shm` files. Deleting rows would leave their pages in the file; deleting the file leaves nothing.
   - The switch and the index share one lock, so a background pass can't open the file again after it was deleted.
3. **Built in the background** (Req.2). `features::outside_search::spawn` runs a thread:
   - It works at most 2 s a pass, rests 3 s while there's work, and checks once a minute after it has caught up. Turning the switch on starts it at once.
   - It covers only the conversations no AI Juggler session owns; AI Juggler's own sessions are AJ-E1007's index.
   - It works newest first and incrementally: a plain transcript is read only from where it stopped.
   - A compressed Codex rollout (`.jsonl.zst`) is read whole, once, since a finished rollout doesn't grow. Its offsets are in the decompressed text.
   - A conversation whose file is gone is forgotten.
   - After a pass that wrote, the index is optimized into one segment, so it's as small as it can be.
4. **Words in context for outside sessions** (Req.3):
   - `TranscriptHitInfo` names either an AI Juggler session or an outside conversation (account and id).
   - History shows the matching words for both, read from the tool's own file. The index is contentless: it holds terms and positions, never the text.
   - History's description and its "nothing matches" line say where words are searched and how to search the rest, instead of "aren't searched".
5. **Shared reading code:** `transcript_text.rs` holds the line reader and the snippet, now able to read compressed files, for both indexes. It moved out of `history.rs`.

## The quality gates

- **The size shown first matches the built index within 10 %.**
  - The estimate indexes a sample of the files in memory, spread by size so it's the same on every run. Compressed and plain files are sampled separately. It indexes up to 32 MB of each, all of them on a small setup, and scales by bytes.
  - `the_size_shown_first_is_the_built_size_within_ten_percent` covers the whole sample.
  - `a_sampled_estimate_is_within_ten_percent_too` samples a quarter of 41 files of varied length, and comes out 4.4 % over.
  - On the way there, a two-point growth fit and recency-ordered sampling were tried: 16–19 % off, and unstable from run to run. They were dropped.
  - **Not measured on a real setup yet:** on one with thousands of sessions, 32 MB is hundreds of files. QA's next real-data pass should record the estimate against the built size.
- **Turning it off leaves no index behind:** `turning_it_off_leaves_no_index_behind` checks that no `outside-search*` file is left and that a later pass doesn't make one. The core test `a_search_index_file_is_the_same_index_and_removes_whole` also removes a planted `-journal` file.
- **AJ-E1529's first pass is as fast with it on:**
  - The builder does nothing until the conversations index has read anything (`nothing_is_built_before_the_first_conversations_pass`).
  - It skips any round while a conversations pass is reading (`ConversationIndex::is_reading`).
  - It only ever reads the conversations index; it never calls it to read files.

## Tests

- **`juggler-core` `a_search_index_file_is_the_same_index_and_removes_whole`:** the same search, mode 0600, reopened intact, and removed whole.
- **`juggler-app` `transcript_text::tests::compressed_and_plain_files_give_the_same_lines_and_snippets`**
- **`crates/juggler-app/tests/outside_search.rs`**, over real storage, an in-place Claude Code account and 41 transcripts:
  - off by default, with no file and no hits;
  - both estimate tests;
  - finding words in context;
  - new words indexed incrementally;
  - a deleted transcript leaving the index;
  - an AI Juggler-owned conversation left to its own index;
  - turning it off;
  - nothing built before the first pass.
- **`session_lifecycle.rs`:** AI Juggler's own transcript search is unchanged.
- **UI** (`outside-search.test.tsx`, `history.test.tsx`):
  - the setting's states and words, and the on/off calls;
  - the hit key;
  - History finding an outside session by its words alone.
- **Hand walk in the preview:** off showed "about 684 MB … for 3412 sessions"; turning it on showed the progress line. The walk found two wording problems, both fixed: "Off by default: it keeps an index" read as if it kept one while off, and an empty index said "1 KB".
