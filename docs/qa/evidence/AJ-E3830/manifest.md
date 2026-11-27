# Evidence manifest: AJ-E3830 (Bot fills the Watches form)

Built 2026-11-26 on release/0.5.2 (Opus): code done, the eval round pending. Nothing was run against a model. The harness is v6 (`../AJ-E3709/harness/v6/`, runbook `v6-runbook.md`); the app's prompts are the harness's, as measured (a test keeps them equal). The v6 blind sets were not opened by the builder.

R1: PASS (code) — the draft's schema: call 1 `{intent, target}` with exactly one target, an enum of the request's refs (`watches::draft::tests::two_calls_fill_the_form_the_target_first`; harness `selftest_v6.py` "watch2 schema: exactly one target"); what can't be watched a short enum, said in the request's own words by code (`what_cannot_be_watched_is_said_in_the_requests_words_with_the_nearest`); the condition from the target's catalogue, its number from the request (`the_forms_own_check_and_parameters_are_held`). Commits 98d775f96 (harness), d22ffb585 (app).
R2: PASS (code) — two calls, the target first (`two_calls_fill_the_form_the_target_first`); a failed check retries that call, sampled with a seed (`a_target_the_request_does_not_name_is_asked_again_sampled`; `engine::Decoding`, Apple's helper `"sampling"`, a local model's seeded sample: 2eaaf6070; harness "sampling: a retry samples"). In the app: Bot's request opens the Watches form filled in, once; nothing is added without the user's click (`bot::watch_draft::tests::a_watch_request_fills_the_form_and_adds_nothing`, vitest `bot-watch-draft.test.tsx`); without a passing verdict Bot answers as before (`without_the_jobs_verdict_bot_answers_as_before`).
R3: FAIL (2026-11-26, Apple v6 round, Sonnet run, Opus to verify): 63.5 % exact (strict 19.2 %) vs bar 90 %; see `../AJ-E3709/v6-results/summary.md`. Not shipped; verdict stays `not_yet`. (Was: the eval round: 60 fresh requests (`heldout/v6/watch-v6.json`) on Apple's model by the runbook, a Sonnet round, Opus verifying; ships at >= 90 % exact (D-149), recorded as the `watches` verdict (`beta`) in the `models` section with no code change (fe6fb87cf). Below 90 %: reported to the coordinator.

Usability answers (the task's six): (1) "tell me when the build finishes" sets up the watch from the words: PASS, the form opens filled in; (2) the user sees every field and clicks to add: PASS, the Watches form itself, no auto-add; (3) no second editor: PASS, `openWatches({kind: "new", draft})`; (4) a request Bot can't turn into a watch says why in one line and opens the form (nearest or empty): PASS, `WatchDrafted::CantWatch` / `Form`; (5) none; (6) a wrong session is refused and asked again, a watch is never added without the click, the "can't watch" words are the request's: PASS, the tests above.

v6 round (2026-11-26): 63.5 % exact against 90 % (`../AJ-E3709/v6-results/`): not shipped. v7 (D-149's amendment, code decides and the model picks): harness `../AJ-E3709/harness/v7/` (6d76e51c5, `selftest_v7.py` 62/62, runbook `v7-runbook.md` with v6's no-draft causes), the app c307b2e81 (`watches::draft::tests`, `watches::resolve`). R3 (v7, 2026-11-26, Apple, Sonnet run, Opus to verify): FAIL: 77.1 % exact with the ADR default (strict 56.3 %) vs bar 90 %; unsupported claimed 5 (bar 0); nearest offered 30 % (bar 90 %); see `../AJ-E3709/v7-results/summary.md`. Verdict stays `not_yet`.

v7 round (2026-11-26): 77.1 % exact, 5 unsupported claimed (`../AJ-E3709/v7-results/`). v8: never widen, thresholds unsupported with the nearest, "always": harness `../AJ-E3709/harness/v8/` (cc492caac, `selftest_v8.py` 45/45, runbook `v8-runbook.md`), the app's draft with v7's five rows as regression tests. R3: PENDING the v8 round on `heldout/v8/watch-v8.json` (0 unsupported claimed first, then >= 90 % exact).

R3 (v8, 2026-11-26, Apple, Sonnet run, Opus to verify): FAIL: unsupported claimed 3 (V8-W27 exclusion dropped, V8-W54 excluded project taken as target, V8-W60 tokens drafted as usage limit; bar 0); 75.0 % exact with the ADR default (strict 66.7 %) vs bar 90 %; nearest offered 83.3 % (bar 90 %); see `../AJ-E3709/v8-results/summary.md`. Verdict stays `not_yet`.
v8 round (2026-11-26): 3 unsupported claimed, 75 % exact (`../AJ-E3709/v8-results/`). v9 (D-149 amendment 2, fail closed): harness `../AJ-E3709/harness/v9/` (da54ea7f9, `selftest_v9.py` 27/27, runbook `v9-runbook.md`), the app 60af89bf4 (`watches::fail_closed`, a regression test for every unsupported-claimed miss so far) and 9675d33f2 (the drafted watch in plain words under Bot's line). G11 in Bot's written-answer check: 46879aeb1; its offline re-run `../AJ-E3709/harness/v9/g11-rerun.md`. R3: PENDING the v9 round on `heldout/v9/watch-v9.json`.
R3 (v9, 2026-11-26, Apple, Sonnet run, Opus to verify): FAIL on one bar: unsupported claimed 0 (bar 0, PASS); 68.8 % exact with the ADR default (22 of 32; strict 46.9 %) vs bar 90 % (FAIL); nearest offered 92.0 % (23 of 25; bar 90 %, PASS); median 2.4 s per item, p95 2.7 s; the misses are a lifetime said in words ("permanently", "dauerhaft", "einmal") read as today, one invented wait parameter, and 8 over-refusals; see `../AJ-E3709/v9-results/summary.md`. Verdict stays `not_yet`.

v10 (2026-11-26, the coordinator's calls on v9): harness `../AJ-E3709/harness/v10/` (4c17f060f, `selftest_v10.py` 31/31, runbook `v10-runbook.md`): lifetimes said in words (always, once, today; unsaid stays the ADR default), a named entity wins over a broad word only as its clause's subject or object and never beside a meaning-changer, a minutes value only with a time unit, the longer of two nested names. The app carries the same rules in `watches::draft`, `fail_closed` and `resolve`, with a regression test for each v9 miss (`v9s_misses_draft_as_said`). R3: PENDING the v10 round on `heldout/v10/watch-v10.json`.

R3 (v10, 2026-11-26, Apple, Sonnet run, Opus to verify): FAIL on all three bars: unsupported claimed 3 (R10-W03 exclusion became the target, R10-W40 "unless" dropped, R10-W60 retry count dropped; bar 0); 85.7 % exact with the ADR default (36 of 42; strict 76.2 %) vs bar 90 %; nearest offered 72.2 % (13 of 18; bar 90 %); median 1.1 s per item, p95 3.4 s; see `../AJ-E3709/v10-results/summary.md`. Verdict stays `not_yet`.

v11 (2026-11-26, the coordinator's design after v10's FAIL: 3 unsupported claimed, nearest 72 %): **full coverage, failing closed by construction.**
- Code claims a request only when it places every word (`watches::draft::cover`). Otherwise the model labels spans with a guided schema, and code reads the labels only for what to tell back and the nearest target, so the model never makes a request supported.
- The nearest option always comes with an unsupported answer, and it is never widened or the thing left out.
- v7's call 2 (the model picking a condition) is removed.
- Harness `../AJ-E3709/harness/v11/` runs the app's own drafting through `examples/watch_round.rs`; `selftest_v11.py` covers every fixture of v5 to v11 with three labellers. Runbook: `v11-runbook.md`.
- Blind set `heldout/v11/watch-v11.json`, written by a Sonnet writer that never saw prompts or code.

R3 (v11, 2026-11-26, Apple, Sonnet run, Opus verified): FAIL on exact only. Unsupported claimed 0 (PASS). Exact 65.8 % with the ADR default (25 of 38; strict 57.9 %) against 90 % (FAIL). Nearest offered 95.5 % (21 of 22, PASS). Watches written 0. Every miss is code under-reading a supported request: German "-Sitzung" compounds, ways of asking that aren't recognised, context clauses, paraphrases and two composite conditions. See `../AJ-E3709/v11-results/summary.md`. Verdict stays `not_yet`; v12 waits for the coordinator.

v12 (2026-11-26, the agreed design: verified labels may draft, a negation guard whatever the labels say): app 1adb29763, harness `../AJ-E3709/harness/v12/` (b4f8cdefa, `selftest_v12.py` 75/75, five labellers), blind set 190e8f434 (Sonnet writer). R3 (v12, Apple, Sonnet run, Opus verified): FAIL on exact only. Unsupported claimed 0 (PASS). Exact 31.6 % with the ADR default (12 of 38) against 90 % (FAIL). Nearest 95.5 % (PASS). Watches written 0. The labels never verified: Apple's model repeats the request word for word in about 30 of 84 attempts. The negation guard over-refuses scope-neutral "don't care / don't matter", and code's word lists are short of the set's paraphrases. See `../AJ-E3709/v12-results/summary.md`. Verdict stays `not_yet`; v13 waits for the coordinator.

v13 (2026-11-26, word-indexed labels, a second guard on filler): app 97d7cc978, harness `../AJ-E3709/harness/v13/` (bccba2b07, `selftest_v13.py` 115/115, six labellers), blind set 97302bbd7 (Sonnet writer, no example phrasings). R3, Apple only (Sonnet run, Opus verified):

- unsupported claimed 0 of 22 (PASS);
- exact 12 of 38 = 31.6 % with the ADR default (strict 28.9 %) against 90 % (FAIL);
- nearest offered 18 of 22 = 81.8 % (FAIL);
- latency median 634.5 ms, p95 775 ms;
- watches written 0.

The labels can't add a draft. Apple's model labels the words code can't place by meaning (condition, target, lifetime; filler once in 56), and code can't verify an unknown word in those slots, so 26 of 26 were told back. The rest is vocabulary and ways of asking that the blind set uses and code lacks. See `../AJ-E3709/v13-results/summary.md`.

**Status: watches stay `not_yet`, pending the maintainer's choice.** The hard bar held in every round from v9 to v13. Exact on fresh blind sets: v11 65.8 %, v12 31.6 %, v13 31.6 %; the later sets asked for more paraphrase.


## 0.5.2: Bot pre-fills the Watches form; the user saves (Opus, 2026-11-27)

The coordinator's and maintainer's call after v13: what ships is the form pre-filled with what code verified, each other field left empty for the user; the "Bot drafts a watch" verdict stays `not_yet` (builtin.json and the remote list). Code: e923067e2 (`watches::draft::prefill`, Bot's path, the Watches row, the note), d3ccfce2e (the form's empty fields and Bot's note, the nearest tag, Bot's confirm card as the Watches Beta, the Models row, the form folding by its room beside Bot), the gallery `99-watch-prefill` (critique there).

R4 (the bar, never a wrong pre-filled field): **PASS.** Unit: `watches::draft::prefill::tests` runs every request of harness v6 to v13 (480) through the app's drafting with nine lying labellers (all filler, all lifetime, all target, all quiet hours, all condition, all other, all exclusion, a mix, a wrong length) plus a target pick code never checks, and with no model: no filled field differs from the gold. Measured (below): 0 wrong fields on Apple's model (v6 to v13), on Qwen3 1.7B and Granite 4.1 3B (v13), and code alone (v13).

R4 (per-field fill, through the app's own path): PASS (measured, 0 wrong), measured by `crates/juggler-app/examples/watch_prefill_round.rs` on the app's own engines (Apple's helper; the local helper with the approved model files by sha256 name, hard-linked in scratch, nothing downloaded), jobs from the harness's own candidates (`prefill/prefill_jobs.py`), scored by `prefill/prefill_score.py`; reports `prefill/reports/report-*.json`. macOS 27.0.1, Apple M5 Ultra, one run each. **Not blind:** the v13 misses were read before this work and v6 to v12's shaped the code; no vocabulary was added (the reading changes only leave more empty: ordinals are counts, "even/selbst/sogar" never filler, a name after a negation never the target).

v13, 38 supported of 60 (filled and right; lifetime plus the form's labelled suggestion and how many suggestions equal the gold; quiet hours as verified, "on" right of the 6 that asked):

| R4 · engine | Target | Condition | Lifetime | Quiet hours | Wrong | Every field right | Card offered (all right) | Nearest offered | First call median |
|---|---|---|---|---|---|---|---|---|---|
| Apple on-device | 21 (55.3 %) | 16 (42.1 %) | 6 (+9 of 21 suggestions) | 27 (on: 3 of 6) | **0** | 11 (28.9 %) | 13 (11) | 18 of 22 | 592 ms |
| Qwen3 1.7B Q8 (local) | 19 (50.0 %) | 15 (39.5 %) | 6 (+8 of 20) | 24 (on: 3 of 6) | **0** | 11 (28.9 %) | 12 (11) | 18 of 22 | 179 ms |
| Granite 4.1 3B Q4_K_M (local) | 17 (44.7 %) | 17 (44.7 %) | 6 (+8 of 22) | 26 (on: 3 of 6) | **0** | 13 (34.2 %) | 15 (13) | 18 of 22 | 167 ms |
| No model (code alone) | 20 (52.6 %) | 12 (31.6 %) | 4 (+7 of 8) | 12 (on: 2 of 6) | **0** | 11 (28.9 %) | 12 (11) | 18 of 22 | n/a |

Earlier sets on Apple's model (same code; v6/v7's gold reads "idle" as stuck, scored as `idle_for` as the app reads it):

| R4 · set | Supported | Target | Condition | Lifetime | Quiet hours (on) | Wrong | Card offered (all right) | Nearest offered |
|---|---|---|---|---|---|---|---|---|
| v6 | 52 | 23 | 26 | 6 (+3 of 28) | 39 (1 of 3) | 0 | 17 (7) | 5 of 8 |
| v7 | 48 | 24 | 30 | 11 (+9 of 26) | 35 (1 of 3) | 0 | 20 (18) | 10 of 12 |
| v8 | 36 | 23 | 23 | 12 (+8 of 17) | 31 (3 of 4) | 0 | 19 (16) | 22 of 24 |
| v9 | 32 | 16 | 25 | 15 (+7 of 12) | 28 (3 of 3) | 0 | 16 (16) | 26 of 28 |
| v10 | 42 | 28 | 35 | 11 (+10 of 28) | 40 (3 of 3) | 0 | 25 (21) | 17 of 18 |
| v11 | 38 | 28 | 38 | 10 (+15 of 28) | 38 (5 of 5) | 0 | 28 (22) | 22 of 22 |
| v12 | 38 | 30 | 36 | 10 (+15 of 27) | 37 (4 of 4) | 0 | 30 (25) | 21 of 22 |

On unsupported requests no condition is filled anywhere; the targets filled there were checked by eye, each the entity the request names (v12's W39, "es sei denn, es ist die Rechnungsexport-Sitzung", named the excluded session before the after-negation rule and is now left). A card's unsaid lifetime is ADR-0036's default, which the card says; it differs from these sets' "today" gold in 2 of Apple's 13 v13 cards (v6's "once" gold in 10 of 17).

R5 (the card is opt-in Beta, off by default; the verdict stays `not_yet`): PASS (code): `prefill.card` only with every field verified, the form's own check passing and `ai_beta_watches` on (`bot::watch_draft::tests::the_card_needs_the_beta_and_the_form_s_check`); vitest `bot-watch-prefill.test.tsx` (the card writes only on Watch; Change… opens the form filled in; Cancel writes nothing; a problem keeps Watch off). The Watches row (Settings › AI › Models) shows Bot's state and its Beta switch (`models-bot-card.test.tsx`).

R4 (the form keeps empty what Bot left): PASS, vitest `watch-prefill.test.tsx`: an unread lifetime chooses nothing and Watch waits for a pick; an unread condition chooses no card (no default either) and tags the nearest; an unread target leaves the picker empty; unread quiet hours stay off with "Check it"; each leaves Bot's list once set. Gallery `99-watch-prefill`, critique there.

Watches written by the job: 0 (it never writes; only the user's click in the form or on the card adds one).

R3 (>= 90 % exact on a fresh blind set) was not met in any round (v6 to v13 above) and is superseded for 0.5.2 by the coordinator's and maintainer's ruling (requirements 4 and 5: the pre-filled form, the card as an opt-in Beta, the verdict `not_yet`). It needs the maintainer's D-number to be recorded as deferred; the rounds' lines above that say PASS for single bars are not R3's PASS.

The task's six usability answers (2026-11-27) hold on the built screens (the gallery's critique, line 18).
