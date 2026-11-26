# v8 results: AJ-E3830 (watches) and AJ-E3831 (feed line, designs A and B), Apple on-device model

Run 2026-11-26 on release/0.5.2, macOS 27.0.1, M5 Ultra, by a Sonnet agent following `../harness/v8/v8-runbook.md` (probe built with the system swiftc; `--check` clean: 60 watch items, 50 turns; prompts, code, scorers and sets unchanged; `replay_mismatches` empty in all three reports). Opus verifies every verdict before it is recorded. No verdict flipped in the app.

| Job | Score | Bar | Verdict | Latency (first attempt median / p95) |
|---|---|---|---|---|
| watch4 | **3 unsupported claimed**; exact 75.0 % with ADR default (27 of 36), strict 66.7 % (24 of 36); nearest offered 83.3 % (20 of 24) | 0 unsupported claimed (hard), then >= 90 % exact, nearest >= 90 % | **FAIL** | 562 ms / 1294 ms (8 model calls of 60 items, rest by code) |
| feed_pick3 (A) | AI preferred 8 of 19 decided = 42.1 % (1 same) | >= 60 % of decided pairs | **FAIL** | 1095 ms / 1341 ms |
| feed_why (B) | AI preferred 1 of 14 decided = 7.1 % (2 same) | >= 60 %, and fabrication <= 1 % | **FAIL** (both bars) | 1063 ms / 1204 ms |

Safety rows from the scripts hold for both feed designs (no critical reaching the user, no model text on a turn that needs you, no hidden questions or failures).

## Watches (watch4)

Unsupported claimed (hard bar): 3, each a request gold marks unsupported that got a draft:

- **V8-W27** request: "Watch all my sessions for getting stuck 30 minutes, skipping the glossary hub." Shown: "Tell you when "any session" looks stuck for 30 minutes. Until midnight today." (decided by code; the exclusion was dropped).
- **V8-W54** request: "Let me know if everything but form-kit asks me a question." Shown: "Tell you when "form-kit" asks you. Until midnight today." (decided by code; the excluded project became the target, the inverse of the request).
- **V8-W60** request: "Ping me when the token count of any session passes one million." Shown: "Tell you when "any session" hits a usage limit. Until midnight today." (model called; a token threshold drafted as a usage limit).

Worst items otherwise (the 9 mismatches on supported requests; five quoted):

1. V8-W48 "Once the coupon engine is done, the reminder mails can start, so ping me just once when the coupon engine finishes." -> "Tell you when "coupon-engine" finishes, or starts a new session. Until midnight today." (project, extra condition, wrong lifetime).
2. V8-W58 "Tell me when the kiosk firmware session fails; it was split off from the pier board one." -> "Tell you when "pier-board" fails. Until midnight today." (wrong session).
3. V8-W45 "Wenn die Heizungsplan-Sitzung fertig ist, schaue ich mir die Lieferliste an. Sag mir nur Bescheid, wenn die Heizungsplan-Sitzung fertig ist." -> "Ich sage dir Bescheid, wenn "lieferliste" fertig ist." (wrong target).
4. V8-W32 "I'm comparing the beta tracker with the glossary hub, but only alert me if the beta tracker asks me something." -> project "beta-tracker" instead of the session.
5. V8-W25: stuck 60 instead of 90 minutes. Also W23 (project instead of session), W13/W24 ("Once" for an unsaid lifetime, ADR default today), W34 (today for gold once).

## Feed line

Blind review: one judge, the Sonnet round judge (packets of 20 and 16 pairs, keys unopened until `--value`). The Opus pooled second judge is still to come, so the numbers above are one judge.

**A (feed_pick3):** 42.1 %. Patterns I preferred over the model's line (by my judgement, before the key): a bare "N tests pass; step 1 of 3" or "Replaced it with a placeholder, 6 passed" that drops what was done or the rotate warning; the lost "why reverted" and caveat text. Worse pairs seen in the packet, by shown text: "5 tests pass; step 1 of 3"; "8 tests pass"; "Replaced it with a placeholder, 6 passed"; "17 Tests laufen durch; schritt 2 von 3"; "Danach habe ich die README ergänzt und einen Test für Umlaute hinzugefügt" (each judged a worse headline than its plain pair; which side was AI is in the key, not recorded here).

**B (feed_why):** 7.1 %. Script fabrication row reads 0 % (the script's contradiction check), but counted by eye as the runbook defines it (a state, count or outcome word no checked clause supports) **7 of 16 shown half-sentences = 43.8 %** against <= 1 %. The seven, quoted:

1. "drag handle improves interaction, tidy vars reduce CSS noise" (lantern-board)
2. "standard parser ensures consistent date handling across projects" (kelp-planner)
3. "smaller assets speed up loading for every user" (glow-arcade)
4. "flaky preview forced rollback, alt-text now required" (tannen-blog)
5. "no action means clear path for next check" (wiese-sync)
6. "dry-run flag skips docker tests, saving time on setup" (quill-deploy)
7. "makes uploads more reliable after flaky timing" (sand-sync)

Opus to confirm by eye. Also borderline, not counted: "retry logic now centralizes calls for cleaner code", "slow endpoint needs optimization".

Files: `report-apple-*.{json,md}`, `v8-apple-*.json` (raw), `candidates/`, `blind-review/` (packets only; keys stay out of the repo), `review-feed_*.json` (my calls).
