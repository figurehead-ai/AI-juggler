# Evidence manifest — AJ-E3713

Task: `docs/tasks/E37-home-activity-feed.md` AJ-E3713, Home's feed tells
the story: posts with a gesture bar, threads instead of rows, a feed that
learns from more and less (P0, 0.5.2). Found by the maintainer on the
0.5.1 Nightly (2026-11-25); the mock approved 2026-11-26
(`AIJuggler-design-refs/story-feed/`).
Status: **not DONE**: R3's Remind me waits on a reminder condition in the
watches (below); every other requirement passes.

Commits: 6d5f5c3b1 (the leans kept through the write gate; the tabs and
state chips in the read model), 2fd36363a (the posts, threads, gesture bar,
story templates, filters, keys and the old bug), d1464c11e, 6b56f2c59,
fbed81794, 4b1fc279b, a004f664e and the verify-static fix (fixes found on the captures and by the
debt baseline), e0cc71c23 and 8e1d7b5a7 (the gallery specs; ADR-0037's
`feed` slot), 23a8024b4 (59 gate baselines, notes), fcd5fd268 (the
gallery, eight families' critiques).

## Requirements

| R | Claim | Verdict | Proof |
|---|---|---|---|
| R1 | Mock first, both themes, 1280 wide, approved | PASS | `AIJuggler-design-refs/story-feed/mock-story-feed{.html,-dark.png,-light.png}`, approved by the maintainer 2026-11-26 (a1a962ef3) |
| R2 | Posts: the App's real icon or AI Juggler's mark, a calm line, a 1–3 sentence story with chips, an optional attachment, follow-ups threaded | PASS | `HomeFeedCard.tsx`, `HomeFeedParts.tsx`, `lib/feed-story.ts`: the author is `AppIconTile` read at run time (the rail's and Your apps' path, nothing in the repo), AI Juggler's posts its mark on its tile; "**Claude app** · work-max · 9 min"; the story's things are chips that open them; attachments: a switch's before → after, the agent's quoted reply ("Codex CLI's last reply", "Builder's words"), an Account's room as a meter. Threads: `threadOf` (one session's or one task's items, one event told twice once). Tests: `home-feed-story.test.tsx` (posts, thread), `feed-story.test.ts` (threads). **Not drawn:** the mock's diff stat (files, +/−): the feed keeps no git line yet (AJ-E3708's fact); it shows when that fact is recorded |
| R3 | Gesture bar: More / Less like this as `preference` memory through ADR-0037's gate, scoped to the kind, the App, the project; ranking and what gets posted read them; "Why am I seeing this?"; Remind me (a one-time reminder watch with a time picker); Follow; Ask Bot; Open | FAIL | More / Less: `features/feed/commands/prefs.rs` (`feed_lean`, `feed_leans`, `feed_unlean`): a `preference` record in the `feed` slot, source `home_feed`, written with the memory writer through the store's gate; Undo forgets it through the one forget path. Tests: `prefs::tests::a_lean_is_a_preference_written_through_the_gate_and_read_back`, `the_gate_refuses_another_writer_and_an_ask_that_isnt_a_lean`. Ranking and posting: `lib/feed-leans.ts` `rank` (more first, quiet "less" posts folded into "N posts shown less · Show", level 2–3 never moved or folded); why panel with Undo and "Manage in What Bot remembers". Follow through the Watches flow (`lib/feed-watch.ts`, Notifications primed first); Ask Bot (Bot on the post's thing, the post in the question box, only while Bot is ready); Open. **FAIL only for Remind me:** built (`RemindMenu`, `remindAt`: a `once` watch with a `remind` condition and the picked minutes), hidden until the watches offer a `remind` condition (D-127). That condition belongs in `features/watches` (the Watches agent's module; my edit there was refused): the proposed change is `aj-e3713-remind-condition.patch` (a `remind` rule armed at the start on any target, one SHIPPED line, its words). Note also `features/bot/memory.rs`'s Forget allows only `alias:`/`focus:`/`note:` keys, so a feed lean listed in What Bot remembers can't be forgotten there until `feed:` is added (Undo in the feed works) |
| R4 | The story from D-149's feed job when it passes; until then templates; no AI: still a story | PASS | `lib/feed-story.ts`: `setStorySource` is the seam (a picked story must name the item's thing, else the template's); every kind has a narrative template ("You answered X — it's moving again", never "Resolved: …"). Tests: `feed-story.test.ts` "the template stories", "D-149's story seam, and the fallback". Wiring the seam to the feed job and its real-model check are AJ-E3831's (`features/feed/pick.rs`, the AI agent's) |
| R5 | Filters: All · Needs you · Sessions · Tasks · Pools · Switches & limits · This Mac; Running / Finished / Failed; Showing by App / Account / Project, suggested then all | PASS | Read model: `in_chip`, `in_state`, `Ask::with_running` (Running: the sessions running now); `feed::tests::the_tabs_and_state_chips_keep_what_they_say`. Window: `HomeFeedFilters.tsx`, `lib/feed-chips.ts`, the preview's stand-in by the same rules. Tests: `home-feed-story.test.tsx` "each tab keeps its posts", "each state chip keeps its posts, says its count today, and is remembered"; `feed-story.test.ts` R5. A filter with nothing says so and offers Show all (`home-feed.test.tsx`) |
| R6 | The ChatGPT · Codex row bug fixed | PASS | The story's thing was a `<button>` drawn inline: a long name wrapped inside its own box and centred mid-row. Now a chip: one `inline-block`, `whitespace-nowrap`, `text-left`, `truncate` box, its whole name in its tooltip; the author's bold name its own `shrink-0` box. Test: `home-feed-story.test.tsx` "0.5.1 Nightly: the ChatGPT · Codex row's title is one left-aligned box that never wraps inside itself" |
| R7 | Calm surfaces, PAT-COLOR; paged or infinite; j/k between posts, l/m for more/less | PASS | Posts are a surface on the canvas, no border, no shadow (PAT-SURF-1/-2; the mock's card shadow left out for PAT-SURF-2); chips the accent's soft step, state chips their soft tints with a ✓ when on (colour never alone). Paging unchanged (three pages by themselves, then Load more; `home-feed.test.tsx`). Keys: `home-more` / `home-less` in the registry (rebindable), `use-home-keys`; test `home.test.tsx` "AJ-E3713 R7: l and m lean the focused post" |
| R8 | Tests: preference writes through the gate; ranking after a "less"; thread grouping; filters; the template fallback; the old bug | PASS | As above: `prefs::tests` (2), `feed::tests::the_tabs_and_state_chips_keep_what_they_say`, `feed-story.test.ts` (14), `home-feed-story.test.tsx` (10), `home.test.tsx` (l/m). The real-model check for AI stories is AJ-E3831's |

## Usability answers, checked on the built screens

1. A colleague telling what happened: who (the App's icon and name, or AI Juggler), what came of it, the thing named as a chip. Match (story-1280, both themes).
2. Each post: author, calm line, 1–3 sentences with chips, an attachment where facts carry one, follow-ups as "↳ 2 min later · …", a calm gesture bar. Match, except the diff stat (no fact yet, R2).
3. Replaces the log rows on Home, the pinned zone above; one stream. Match.
4. No AI: the same posts from templates; nothing yet: the one calm line; a filter with nothing says so and offers Show all. Match.
5. Remind me and Follow make watches through the Watches flow, the Notifications prompt primed first (PAT-OSP). Follow: match. Remind me: hidden until the watches offer a reminder (R3).
6. Watch-fors: stories from facts only (a picked story must name the item's thing); more/less shows at once (the why panel, the fold line); a thread never hides a follow-up that needs you (level 2–3 always shown); Needs you stays pinned above; ranking never moves or folds level 2–3. Match.

## Checks run

- vitest: 637 files, 4332 tests pass; `tsc -b --noEmit` clean; `ui_lint.py` OK; `rich_mode_lint.py`, `plain_words_lint.py`, `design_tokens_lint.py`, `size_budget.py`, `private_data_lint.py`, `critique_check.py`: OK; `debt_baseline.py`: none of this work's (it lists `features/bot/commands.rs` and `juggler-core/src/storage/memory/world.rs`, the AI agent's work in progress).
- cargo (through `scripts/heavy.sh`): `feed::tests` and `feed::commands::prefs::tests` pass (26); `cargo clippy -p juggler-app -p juggler-core --all-targets -D warnings` was clean on 6d5f5c3b1's tree; a later run stops in the untracked `crates/gyrus` (another agent's, two float-comparison lints), before reaching this work; rustfmt clean on every file changed.
- `visual_gate.sh` (pinned container): the compare run failed 41 states, all Home's feed or the Keyboard pane; re-rendered with `--update` by file and line (59 baselines), each opened; the compare run afterwards passed all 348.
- `just keep-going verify-static`: this work's findings fixed (`platform_lint`: "This Mac" in words for any computer; `js_number_lint`: `FeedLeanInfo.at_ms` listed; `ipc_audit`: `feed_leans` listed as bounded; `evidence_check`: the index row); the rest are others' (below in the hand-back): `cargo fmt` and clippy in the untracked `crates/gyrus` and `juggler-core/src/storage/memory/{store.rs,tests/world.rs}`, `architecture_fresh` and `check_layering` (gyrus), `check_task_counts` (README's D-147 vs the log's D-149), `debt_baseline` (`bot/commands.rs`, `memory/world.rs`).
- Gallery: eight families re-captured on the host, every changed capture opened (contact sheets; story-1280 both themes at full size beside the mock); a 0.5.2 critique each, stamped.
