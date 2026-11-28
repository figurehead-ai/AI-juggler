# G11 re-run (D-149 amendment 2): the fabrication gate checks support, offline

Written by Opus, 2026-11-26. **No model was run.** G11 (`g11.py`): every claimed outcome, benefit, state, count or cause in a model's text must trace (by stem; a number by its value) to a checked clause: the turn's clauses that passed every v5 check, the caveats the composer keeps in the agent's own words, and the facts' sentence. In the app, Bot's written-answer check gains the same rule for causes, benefits and outcomes (`bot::check`, `Refused::Claim`; its states and numbers were already held to the cited records).

| Re-run | Model lines | Refused by G11 | Verdict effect |
|---|---|---|---|
| The digest's v6 round, replayed from `../../v6-results/v6-apple-digest_mr.json` (30 windows, the round's recorded attempts; the corpus read by the script only) | 25 picked lines shown | **0** (0 %) | **None:** the digest's 64 % pooled Beta stands. Its lines are the agents' own checked clauses with the facts' test counts; nothing claims beyond them. |
| Bot's AJ-E3829 / AJ-E3832 R6 real answers on Apple's model (the five record answers in `../../../AJ-E3832/manifest.md`, against their fixture records) | 5 | **0** | None. The one general answer ("which model are you") carries no record claim and is the general path's own check. |
| For scale: v8's design B (`../../v8-results/summary.md`) | 16 half-sentences | the seven by-eye fabrications each refused (`g11.py` selftest: "speed up", "ensures", "improves", "reliable"…) | B was already off (7 %). |

Commands: `python3 g11.py` (selftest), `python3 g11.py --digest-v6`, `python3 g11.py --bot-e3829`. Opus verifies by eye before anything is recorded; nothing changed in a verdict.
