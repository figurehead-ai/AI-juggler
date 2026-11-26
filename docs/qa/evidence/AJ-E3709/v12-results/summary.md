# v12 watch round (AJ-E3830, D-149), Apple on-device model

Sonnet run; Opus to verify; no verdict recorded. Run 2026-11-26 by a Sonnet agent, per `harness/v12/v12-runbook.md`; set `heldout/v12/watch-v12.json` (60 requests, sha256 db947628a92f369ed3df3699e2cbca7fc5723960ced2a59073bfd384a6a173f1). App drafting binary `watch_round` built from commit 1adb29763 (not rebuilt). Machine: macOS 27.0.1 (26A434), Apple M5 Ultra (Mac17,15), 96 GB. Selftest `selftest_v12.py` 75/75 (no model, honest labeller and three liar labellers: 0 unsupported claimed each; exact 73 of 86, nearest 51 of 52); `--check` clean (60 watch items, 0 unknown gold conditions, 0 unlisted languages); `replay_mismatches`: empty; meta says harness v12, harness_today 2026-11-26. One run only. Nothing committed; no verdict or builtin.json change.

## Bars

| Bar | Score | Observed |
|---|---|---|
| 1. Unsupported claimed == 0 (hard bar, first) | 0 of 22 unsupported requests | holds |
| 2. Exact on supported, ADR-0036 default for an unsaid lifetime, >= 90 % | 12 of 38 = 31.58 % (strict 11 of 38 = 28.95 %) | below 90 % |
| 3. Nearest offered on unsupported, >= 90 % | 21 of 22 = 95.45 % (equal to gold 17 of 22 = 77.27 %) | holds |
| Watches written by the job == 0 | 0 | holds |

Other counters: planted-order mismatches 0; quiet hours broken unasked 0; asked (picker) 0; critical reaching the user 0; omissions 25 (19 false_unsupported, 6 no_watch).

## Latency

30 of 60 decided by code (10 told back as unsupported by code, 13 drafted by code, 7 not_a_watch); model called on 30 (labelling): 3 passed first attempt, 27 after retries (attempts 3 each), 0 fell back. Decided by verified labels (a draft built from labels): 0. Told back after labels: 30 (all 30 label items ended unsupported). First attempt median 2391.5 ms, p95 3687 ms; item total median 7257 ms, p95 10689 ms. Model calls: 84 (3 x 1 + 27 x 3).

## Unsupported claimed (0)

None.

## Exact misses on supported requests (26 with the ADR default; 1 more only strict)

Format: request -> [kind shown, decided by] what was shown. Gold: target, conditions, lifetime ("until:2026-11-26T23:59:00Z" is today).

- R12-W01 "Bei allen Apps egal, aber wenn OpenCode abstürzt, will ich es wissen" -> [unsupported, via model labels] I can't watch for “egal, aber”. Nearest: (the form asks) · fails · today.  Gold: app app-oc, fails, until:2026-11-26T23:59:00Z.
- R12-W02 "Meld dich, wenn die Render-queue-Sitzung 45 Minuten still ist" -> [not_a_watch, via code] (no text shown)  Gold: session s-ren, idle_for 45, until:2026-11-26T23:59:00Z.
- R12-W04 "Tell me when the checkout-api session fails, even during quiet hours" -> [watch, via code] project checkout-api · fails · today · even during quiet hours  Gold: session s-chk, fails, until:2026-11-26T23:59:00Z.
- R12-W09 "I don't care about the rest of my apps, just alert me when Cursor needs an approval" -> [unsupported, via code] I can't watch for “don't care about the rest of my apps”. Nearest: (the form asks) · asks_you · today.  Gold: app app-cur, asks_you, until:2026-11-26T23:59:00Z.
- R12-W12 "Bis ich es wieder lösche: warne mich, wenn das Depot-Pro-Konto sein Limit erreicht" -> [unsupported, via model labels] I can't watch for “Bis ich es wieder lösche: warne mich, wenn das Depot-Pro-Konto”. Nearest: (the form asks) · hits_limit · today.  Gold: account acct-depot, hits_limit, until_removed.
- R12-W16 "Tell me when the Render queue session finishes, but keep it silent at night" -> [unsupported, via model labels] I can't watch for “but keep it silent at night”. Nearest: (the form asks) · finishes · today.  Gold: session s-ren, finishes, until:2026-11-26T23:59:00Z.
- R12-W17 "Buzz me when a Claude Code CLI · Depot session ends and leaves processes running behind" -> [not_a_watch, via code] (no text shown)  Gold: app app-ccd, leaves_processes, until:2026-11-26T23:59:00Z.
- R12-W19 "Benachrichtige mich, wenn Depot Pro wieder Luft hat" -> [unsupported, via model labels] I can't watch for “wieder Luft”. Nearest: account Depot Pro · finishes · today.  Gold: account acct-depot, runway_resets, until:2026-11-26T23:59:00Z.
- R12-W20 "Heute noch benachrichtigen, sobald der Suchindex eine Rückfrage an mich hat" -> [unsupported, via model labels] I can't watch for “noch”. Nearest: session Suchindex · asks_you · today.  Gold: session s-suc, asks_you, until:2026-11-26T23:59:00Z.
- R12-W23 "One heads-up is enough: tell me the first time the Render queue session needs my input" -> [not_a_watch, via code] (no text shown)  Gold: session s-ren, asks_you, once.
- R12-W24 "Sag Bescheid, wenn die Rechnungsexport-Sitzung durch ist" -> [unsupported, via model labels] I can't watch for “durch”. Nearest: session Rechnungsexport · finishes · once.  Gold: session s-rex, finishes, until:2026-11-26T23:59:00Z.
- R12-W30 "Wake me up when the Garage account hits its limit, quiet hours don't matter" -> [unsupported, via code] I can't watch for “don't matter”. Nearest: (the form asks) · hits_limit · today.  Gold: account acct-garage, hits_limit, until:2026-11-26T23:59:00Z.
- R12-W31 "Im ganzen Team läuft viel, aber im Projekt report-lab: melde jede neue Sitzung" -> [not_a_watch, via code] (no text shown)  Gold: project p-ber, starts, until:2026-11-26T23:59:00Z.
- R12-W33 "Across all projects, the one I care about is pixel-forge: tell me when a new session starts there" -> [unsupported, via model labels] I can't watch for “Across all projects, the one”. Nearest: (the form asks) · starts · today.  Gold: project p-ren, starts, until:2026-11-26T23:59:00Z.
- R12-W34 "Permanent watch on the Garage account: say when it has room again" -> [unsupported, via model labels] I can't watch for “say”. Nearest: account Garage · runway_resets · today.  Gold: account acct-garage, runway_resets, until_removed.
- R12-W35 "Every account is fine, but ping me when Depot Pro runs below 15 percent" -> [unsupported, via model labels] I can't watch for “fine, but”. Nearest: (the form asks) · runway_below · today.  Gold: account acct-depot, runway_below 15, until:2026-11-26T23:59:00Z.
- R12-W38 "Gib mir Bescheid, wenn die Onboarding-flow-Sitzung nach 10 Minuten immer noch hängt" -> [unsupported, via model labels] I can't watch for “Onboarding-flow Sitzung nach 10 Minuten immer noch”. Nearest: (the form asks) · stuck · today.  Gold: session s-onb, stuck 10, until:2026-11-26T23:59:00Z.
- R12-W40 "Der Suchindex-Agent will meine Freigabe – sag mir das" -> [not_a_watch, via code] (no text shown)  Gold: session s-suc, asks_you, until:2026-11-26T23:59:00Z.
- R12-W42 "Sag mir, wenn irgendeine Sitzung zu Ende ist" -> [unsupported, via model labels] I can't watch for “Ende”. Nearest: (the form asks) · finishes · today.  Gold: any None, ends, until:2026-11-26T23:59:00Z.
- R12-W44 "Von allen meinen Sitzungen interessiert mich nur der Suchindex: sag Bescheid, wenn er fertig ist" -> [unsupported, via model labels] I can't watch for “interessiert”. Nearest: session Suchindex · finishes · once.  Gold: session s-suc, finishes, until:2026-11-26T23:59:00Z.
- R12-W48 "Shout if the Render queue session goes quiet for 20 minutes" -> [not_a_watch, via code] (no text shown)  Gold: session s-ren, idle_for 20, until:2026-11-26T23:59:00Z.
- R12-W49 "Melde dich, wenn Cursor neu gestartet wurde, auch nachts in der Ruhezeit" -> [unsupported, via model labels] I can't watch for “wurde”. Nearest: (the form asks) · restarted · today.  Gold: app app-cur, restarted, until:2026-11-26T23:59:00Z.
- R12-W50 "Sag mir, wenn der Suchindex fertig ist, aber in der Ruhezeit bitte nicht" -> [unsupported, via code] I can't watch for “nicht”. Nearest: (the form asks) · finishes · today.  Gold: session s-suc, finishes, until:2026-11-26T23:59:00Z.
- R12-W52 "Keep telling me whenever the checkout-api session fails, until I delete the watch" -> [unsupported, via model labels] I can't watch for “until I delete”. Nearest: (the form asks) · fails · today.  Gold: session s-chk, fails, until_removed.
- R12-W54 "Einmalig bitte: Bescheid, wenn die Rechnungsexport-Sitzung abstürzt" -> [unsupported, via model labels] I can't watch for “Einmalig”. Nearest: session Rechnungsexport · fails · today.  Gold: session s-rex, fails, once.
- R12-W59 "Alle Konten sind mir egal, nur beim Garage-Konto: sag mir, wenn wieder Platz ist" -> [unsupported, via model labels] I can't watch for “egal, nur beim Garage-Konto: sag mir, wenn wieder Platz”. Nearest: (the form asks) · finishes · today.  Gold: account acct-garage, runway_resets, until:2026-11-26T23:59:00Z.

Strict-only (lifetime unsaid, drafted once, gold today; the ADR default differs from gold):

- R12-W27 "Tell me once the Lint cleanup session is wrapped up" -> [watch, via code] session Lint cleanup · finishes · once  Gold: session s-lin, finishes, until:2026-11-26T23:59:00Z. | diffs: ["lifetime (unsaid; ADR-0036's default) once != today"]

## Nearest not offered or not equal to gold (5 of 22 unsupported)

- R12-W18 "Ping me when the Render queue session uses over 2 GB of memory" -> [unsupported, via model labels] I can't watch for “uses over 2 GB of memory”. Nearest: session Render queue · finishes · once.  Gold: unsupported; nearest stuck. | nearest shown: finishes
- R12-W29 "Melde mir alles, was fertig wird, abgesehen von Lint cleanup" -> [not_a_watch, via code] (no text shown)  Gold: unsupported; nearest finishes. | nearest shown: None
- R12-W45 "Warne mich, wenn eine Sitzung mehr als 2 GB Speicher belegt" -> [unsupported, via model labels] I can't watch for “mehr als 2 GB Speicher belegt”. Nearest: (the form asks) · finishes · today.  Gold: unsupported; nearest stuck. | nearest shown: finishes
- R12-W51 "Tell me when sessions end, but none of the ones on the Garage account" -> [unsupported, via model labels] I can't watch for “but none of the ones on”. Nearest: (the form asks) · finishes · today.  Gold: unsupported; nearest ends. | nearest shown: finishes
- R12-W57 "Alert me when more than 3 sessions are running at once" -> [unsupported, via model labels] I can't watch for “more than 3 sessions are running”. Nearest: (the form asks) · finishes · today.  Gold: unsupported; nearest starts. | nearest shown: finishes

## Notes for Opus (observations, not verified)

- No verified-label draft happened: all 30 items that went to the model ended as unsupported (27 with three attempts, 3 with one). The 13 drafts all came from code alone ("cover: claimed" 11, "cover: leftover" 2).
- Most label-path items show an unplaced span that is a filler or context phrase (e.g. "egal, aber", "durch", "noch", "wurde", "Einmalig", "until I delete") or a lifetime phrase, so the verification apparently refused; whether the labels or the verification are at fault is for Opus to judge.
- R12-W04 drafted target project checkout-api where gold is session s-chk (the only wrong-rule draft).
- Selftest exact is 73 of 86 (84.9 %) against 31.6 % here; the gap is in the label path.
- Request-reading by code gave not_a_watch on 7 items (W02, W17, W23, W31, W40, W48, W29).

## Files

`v12-apple-watch8.json` (raw), `report-apple-watch8.json` / `.md`, `candidates-apple-watch8.json`.

## Opus verification (2026-11-26)

I recomputed the bars from the raw results and the set (opened only after the run). They match:

- Unsupported claimed: 0.
- Exact: 12 of 38 with the ADR default (31.6 %), 11 strict.
- Nearest offered: 21 of 22.
- Watches written: 0.
- `replay_mismatches`: empty.

**Verdict: FAIL on the exact bar.** Watches stay `not_yet`. The hard bar and the nearest bar pass; no lying label drafted anything. The causes:

1. **The labels never verify.** Apple's model repeats the request word for word in about 30 of 84 label attempts. It drops words, rewords, and once copied the prompt's own example ("but hold it until quiet hours end"). When it did repeat the request, it put known words in the wrong slot ("wenn" as the condition). A sampled retry returned the same text. So 0 drafts came from labels. The fix is structural: label word by word, with code sending the numbered words and the schema asking for exactly one slot per word. Coverage then holds by construction, and code's own slot stands for every word it places.
2. **The negation guard over-refuses scope-neutral negations** (W09, W30, W50): "I don't care about the rest of my apps", "quiet hours don't matter", "in der Ruhezeit bitte nicht" (the default). None leaves a target or condition out.
3. **Code's word lists are short of the set's paraphrases and German forms:**
   - "still ist" (idle); "zu Ende", "durch ist", "wieder Luft/Platz", "neu gestartet wurde";
   - "Einmalig", "heute noch", "bis ich es wieder lösche", "until I delete the watch";
   - "buzz me", "the first time", "say when", "goes quiet for 20 minutes";
   - context in new forms: "… egal, aber", "every account is fine, but", "the one I care about is";
   - compounds whose name has a dash or a space ("Render-queue-Sitzung" for "Render queue"), and "-Agent".
4. **W04: wrong target** (a project for gold's session; seen in the report).

v13 is not started; the coordinator decides.
