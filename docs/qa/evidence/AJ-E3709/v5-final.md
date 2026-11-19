# On-device AI jobs: the final table (harness v5, round 3, 2026-11-18)

Model testing ended with round 3 (D-131). **Synthetic held-out only; not checked on real messages** (the maintainer declined real transcripts, D-124).

Round 3 ran six models on fresh blind sets, with the facts guard on: 50 turns, 50 Chat questions, 30 digest scenarios and 60 watch requests. Every critical was judged by eye and re-checked by the main session (Opus). Value is the share of decided blind pairs in which the AI output beat the plain code fallback, judged by a fresh Opus with the keys unopened. The ≤1 % critical bar means 0 on sets of 50.

**Two numbers per critical cell:**
- **as run:** confirmed fabrications that reached the user in round 3;
- **after fixes:** the same outputs re-scored under the fixed rules, with no model re-run.
"After fixes" is a floor, not a blind measurement: the fixes were shaped by this set, and retries can't be replayed. The safety after the fixes rests on shared code rules, not on any model:
- an agent claim the facts or git contradict is never shown as fact (1782dda5b);
- "Failed:" comes only from the facts or a failure of this turn (e13a38ee0);
- a state word follows the facts (028d81f34);
- no model text appears for a language the checks can't read (d57828883).

Latency is the median per item, in ms. Disk is the model file: Apple's model is built in.

## Feed line, turn review, attention, Chat

| Job | Apple (0 GB) | Qwen3-1.7B Q8 (1.83 GB) | Nemotron 3 Nano 4B (2.84 GB) | Granite 4.1 3B (2.10 GB) | Qwen3-4B-2507 (2.50 GB) | Gemma 4 E2B (3.35 GB) |
|---|---|---|---|---|---|---|
| **Feed:** criticals as run → after fixes | 1 (2 %) → 0 | 0 → 0 | 1 (2 %) → 0 | 1 (2 %) → 0 | 1 (2 %) → 0 | 1 (2 %) → 0 |
| Feed: omissions / value / latency | 0 % / 43 % / 7240 | 0 % / 38 % / 2300 | 0 % / 55 % / 3260 | 0 % / 36 % / 1630 | 0 % / 56 % / 2918 | 0 % / 50 % / 2796 |
| **Review:** criticals as run (model + code template) → after | 3+4 (14 %) → 0 | 1+3 (8 %) → 0 | 3+3 (12 %) → 0 | 3+3 (12 %) → 0 | 5+4 (18 %) → 0 | 2+2 (8 %) → 0 |
| Review: omissions / value / latency | 0 % / 81 % / 2701 | 0 % / 89 % / 2034 | 0 % / 71 % / 1749 | 0 % / 77 % / 1339 | 0 % / 79 % / 1816 | 0 % / 74 % / 3652 |
| **Attention:** criticals; misses vs ADR | 0; 0 | 0; 0 | 0; 0 | 0; 0 | 0; 0 | 0; 0 |
| **Chat:** criticals as run → after | 1 (2 %) → 0 | 3 (6 %) → 0 | 1 (2 %) → 0 | 2 (4 %) → 0 | 1 (2 %) → 0 | 0 → 0 |
| Chat: omissions as run → after (of which other languages) | 42 → 20 % (12) | 42 → 20 % (12) | 46 → 22 % (12) | 44 → 22 % (12) | 42 → 20 % (12) | 40 → 18 % (12) |
| Chat: value / latency | 75 % / 1171 | 76 % / 450 | 80 % / 819 | 79 % / 282 | 83 % / 585 | 89 % / 828 |

**Notes:**
- **Review "code template":** "Failed:" lines our own excerpt code put on turns with no failures (R3T13/28/29/43). Fixed for every path, A included.
- **Review's model criticals:** echoes of an agent's false claim ("committed and pushed" with git uncommitted; "fixed the lint errors" with lint failing), plus Gemma's and Nemotron's Italian "In generale funziona".
- **Chat's criticals:** wrong picks between same-named sessions. Code now lists both.
- **Attention's 11 "misses"** are gold labels against D-117's levels (L1 turns); vs the ADR, none.
- **Chat's omissions "after"** include 12 points of French, Spanish, Dutch and Italian questions, which get the plain path by design (ADR-0033). English/German omissions are 6–10 %.

## Digest and watches

| Job | Apple | Qwen3-1.7B | Nemotron | Granite | Qwen3-4B | Gemma |
|---|---|---|---|---|---|---|
| **Digest:** criticals as run → after | 0 → 0 | 2 (7 %) → 0 | 2 (7 %) → 0 | 3 (10 %) → 0 | 4 (13 %) → 0 | 0 → 0 |
| Digest: omissions as run → after / value / latency | 3.3 → 3.3 % / 33 % / 4500 | 3.3 → 0 % / 42 % / 3400 | 10 → 6.7 % / 50 % / 4544 | 3.3 → 0 % / 50 % / 1055 | 6.7 → 3.3 % / 47 % / 3009 | 6.7 → 0 % / 53 % / 2747 |
| **Watch:** criticals as run → after | 0 → 0 | 2 → 0 | 0 → 0 | 0 → 0 | 1 → 0 | 0 → 0 |
| Watch: exact (en/de) after / false unsupported / latency | 19/27 / 3 / 3232 | 22/27 / 0 / 1316 | 23/27 / 1 / 2152 | 23/27 / 0 / 735 | 24/27 / 0 / 1257 | 19/27 / 2 / 1764 |

**Notes:**
- **The digest's criticals:** code rendered the model's caveat pick ("partly done", "blocked", "tests not run") without checking it against the facts. Fixed.
- **The watch's criticals:** were all other-language requests, which now open the Watches form.
- **Safe on every model:** no planted name changed a draft, no watch broke quiet hours unasked, and no quiet-hours request was missed after the fixes.

## Recommendation per job

| Job | Recommendation |
|---|---|
| Feed line | **Not yet, for every model:** the AI line isn't preferred (36–56 %). The corrected excerpt A is the default (0 criticals on round 3's sets). |
| Turn review | **Opt-in Beta.** Apple where available, else Qwen3-1.7B (89 % value, smallest, 2 s). Its safety is the shared claims-vs-facts rules. |
| Attention | **Ship.** It is facts-first code: 0 criticals, no misses against D-117. |
| Chat | **Opt-in Beta.** Apple where available, else Qwen3-1.7B (76 %, 0.45 s). Gemma reads best (89 %) but is the largest and its card sampling is unverified. English/German omissions 6–10 %. |
| Digest | **Not yet:** the AI digest isn't preferred (33–53 %). The code rollup stays. Five composition changes are listed in v5-round3-ef/verdicts-opus.md. |
| Watches from Chat | **Not yet as AI:** exact matches are 70–89 % against a 95 % bar. The Watches form (code) ships; every AI draft would need the user's click anyway. |

**Per Mac** (the Models area, D-131):
- with Apple Intelligence: Apple first, no download;
- otherwise: Qwen3-1.7B (1.83 GB) as the one recommended download;
- Granite 4.1 3B as the fast alternative.
Qwen3-4B, if ever offered, ships only as our own conversion of the official weights.

**Sources:**
- verdicts: v5-round3/verdicts-opus.md and v5-round3-ef/verdicts-opus.md;
- blind reviews: blind-review/v5r3*-verdicts-opus.json, plus v5r3-falsewords-opus.json;
- floors: v5-round3/rescored/ and v5-round3-ef/rescored/;
- the excerpt's baseline: v5-round3/rescored/A-baseline.md;
- the manifest: models-manifest.md.
