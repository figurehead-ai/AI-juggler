# AJ-E3709 v3 local tier: format-failure classes (draft, Sonnet; Opus to check)

Source: results-*-v3.json write drafts. A draft is format-failed when it is not counted in report_v3 `format_pass`: `checks.format_check(strip_quote_pair(text).replace("`",""))` fails (newline, list, >1 sentence, >20 words, >160 chars, empty), or finish=length, or it was dropped earlier as a non-answer (`checks.non_answer`: refusal phrase, <3 words, error). Classes assigned by script in priority order C, A, F, B, E, D, G (one class each). Choose calls are not format-counted; counts of invalid picks are shown for information.

Class key: A think/preamble, B stop/limit (finish=length or at 160-char cap), C template artefact (role/special tokens, think tags, quotation marks), D too long but complete (one sentence >20 words), E several sentences, F wrong content type (non-answer, refusal, other script, list, newline), G other.

## Counts

| model | corpus | drafts | format-failed | A | B | C | D | E | F | G |
|---|---|---|---|---|---|---|---|---|---|---|
| qwen | v2 | 306 | 276 | 0 | 171 | 0 | 45 | 12 | 48 | 0 |
| qwen | turnends | 204 | 111 | 0 | 33 | 0 | 30 | 39 | 9 | 0 |
| qwen | heldout | 72 | 30 | 0 | 12 | 0 | 6 | 12 | 0 | 0 |
| qwen | **all** | 582 | 417 | 0 | 216 | 0 | 81 | 63 | 57 | 0 |
| nemo | v2 | 306 | 69 | 0 | 21 | 0 | 45 | 3 | 0 | 0 |
| nemo | turnends | 204 | 9 | 0 | 0 | 0 | 9 | 0 | 0 | 0 |
| nemo | heldout | 72 | 3 | 0 | 0 | 0 | 3 | 0 | 0 | 0 |
| nemo | **all** | 582 | 81 | 0 | 21 | 0 | 57 | 3 | 0 | 0 |

Choose calls (not format-counted), invalid/length-cut picks: qwen: v2 0/312, turnends 0/204, heldout 0/72; nemo: v2 0/312, turnends 0/204, heldout 0/72

## Rejecting checks per class

- qwen class B: N words > N x207, finish=length x207, more than one sentence x105
- qwen class D: N words > N x81
- qwen class E: more than one sentence x57, N words > N x54
- qwen class F: N words > N x54, finish=length x54, more than one sentence x30, non-answer x3
- nemo class B: N words > N x15, finish=length x12, more than one sentence x3
- nemo class D: N words > N x57
- nemo class E: more than one sentence x3

## Examples (verbatim; id, rep, finish, rejecting checks)

### qwen class A (0)
- none
### qwen class B (216)
- [v2 S03 rep0 finish=length] `The list remains unchanged, views do not truncate, fold control is present where children are shown, long prompts clamp to four lines and open in place, subagnt` -> 27 words > 20, finish=length
- [v2 S05 rep0 finish=length] `The release included two gate failures: v40 storage doc comments named tool ids, and GoToPalette.tsx had 402 lines. Two small helpers were moved to palette-hel-` -> more than one sentence, 25 words > 20, finish=length
- [v2 S08 rep0 finish=length] `The embedded terminal set its background, foreground, and cursor, causing tools' ANSI colours to revert to xterm's defaults (magenta and bright black at 3:1 on` -> 25 words > 20, finish=length
### qwen class C (0)
- none
### qwen class D (81)
- [v2 S07 rep0 finish=stop] `A full vitest run failed due to CPU contention, with settings affecting wait and test durations, and three runs under 2 × cores showed consistent results.` -> 26 words > 20
- [v2 S11 rep0 finish=stop] `Eight pairs were held back: one due to a header chip issue and six due to timing differences without code changes.` -> 21 words > 20
- [v2 S12 rep0 finish=stop] `The main window could stay behind the Floor after a door, with the stage manager (BetterStage) handling the navigation and display of the main window.` -> 25 words > 20
### qwen class E (63)
- [v2 S09 rep0 finish=stop] `A long name pushed '· Finished' onto a line alone. The session name, app, project, and timing are now displayed as specified.` -> more than one sentence, 22 words > 20
- [v2 M30 rep0 finish=stop] `The promotion rules updated STABLE_MIN_DAYS to 7, requiring blocker_fix to have a 3-day blocker and a stable candidate 7 on beta. Self-tests are in place.` -> more than one sentence, 25 words > 20
- [v2 M34 rep0 finish=stop] `At fit a card's words were 9-10 px. Below 1100 px, a pool opens in the List view until a view is chosen, with the choice per pool and winning.` -> more than one sentence, 30 words > 20
### qwen class F (57)
- [v2 S01 rep0 finish=length] `0.6's Stable work joins 0.5, Windows moves to 0.6, and Linux moves to 0.7; the roadmap is updated to 0.3.1–0.3.5, with the fast preflight now on GitHub and no再开` -> 29 words > 20, finish=length
- [v2 S04 rep0 finish=length] `The check identifies the default instance as the rail does, and on 0.4.1, Account C and D are listed as Running while E is absent and Codex is not one of the用户s` -> 32 words > 20, finish=length
- [v2 S10 rep0 finish=length] `The common::rpc_codec field includes requests with details like Outbox, error events, choices, and waiting asks, used by ZCode and Codex. The common::launch字段指明` -> more than one sentence, 22 words > 20, finish=length
### qwen class G (0)
- none
### nemo class A (0)
- none
### nemo class B (21)
- [v2 S01 rep0 finish=stop] `0.6's stable version joins 0.5, Windows moves to 0.6, Linux to 0.7, roadmap updated to 0.3.1–0.3.5, fast preflight with GitHub gates, promotion no longer open.` -> 25 words > 20
- [v2 S03 rep0 finish=length] `The agent returned the list unchanged with views, search, type, order, groups, no truncation, fold control, long prompt clamped, subagent page scrolled, gallery` -> 23 words > 20, finish=length
- [v2 S23 rep0 finish=stop] `polling-audit: drive.rs's two 1 ms waits indicate polling. release-selftest: comment no longer names floor_harness_check.sh's path. ci-parity: just verify runs.` -> more than one sentence
### nemo class C (0)
- none
### nemo class D (57)
- [v2 S04 rep0 finish=stop] `The check found Account C and D as running in inactive apps, with E absent and Codex not a user app.` -> 21 words > 20
- [v2 M02 rep0 finish=stop] `The agent found a move and undo refused during a running turn, leaving profiles closed and a failed undo with keep alive paused.` -> 23 words > 20
- [v2 M03 rep0 finish=stop] `The agent displays a speech-bubble card with state, session title, user query, or last step, stays 300 ms after leaving, and Esc closes it.` -> 24 words > 20
### nemo class E (3)
- [v2 A13 rep0 finish=stop] `The off-by-one in check_limit is fixed. test_limit_boundary passes; test_limit_negative fails with a panic.` -> more than one sentence
### nemo class F (0)
- none
### nemo class G (0)
- none

## Notes and read

Cross-check: failed counts equal writes minus report `format_pass` for every model x corpus (qwen 276/111/30, nemo 69/9/3). Every count is a multiple of 3 (3 repeats, greedy, so repeats agree). Priority resolves overlaps: a Qwen draft with CJK characters that also hit finish=length is class F, not B. Nemotron class B includes 9 cases cut by the 160-character grammar cap without a reported token stop (finish=stop reads as a stop at the cap) plus 12 finish=length cuts; a few B items are also over 20 words. Class A and C are 0 in both models: thinking is off through the template, and the grammar bans quotation marks and line breaks, so no think tags, preambles or role markup reached the text. Nemotron class B example S23 (two sentences, no cap hit) was placed in B by the 159+ characters rule; the length is the cause only in part.

Read: no failure is mechanical in the A/C sense (nothing fixable by chat template or reasoning config). The grammar-and-limit cut (B, 216 Qwen / 21 Nemotron drafts, mostly mid-word) is engine-caused only in the sense that the 160-character and 60-token caps clip lines that were already running past 20 words, so the root is the same as D: the model ignores the 20-word instruction (Qwen v2 especially, where 171 of 306 drafts were cut). Qwen F (57) is a decode artefact of its own: Chinese tokens appear near the cap, a model fault, not template. E (63 Qwen, 3 Nemotron) is the model writing two sentences despite "one sentence". Nemotron fails almost purely on D (57 drafts of 21 to 24 words, a slight overrun), so it is a near-miss on length; Qwen fails across B, D, E and F, a broad instruction-following failure. The Qwen v2 corpus is far worse than turnends and heldout, so long inputs drive it. Raising the cap would not rescue either model, as every non-B failure is a finished line that breaks the 20-word or one-sentence rule.
