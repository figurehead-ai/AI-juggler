# A alone, the non-AI default, on round 3's sets: the baseline row

This scores A (no model text) on heldout/v5r3/turns-v5r3.json and chat-v5r3.json. These sets are allowed for A, because A's rules were not tuned on them; the two fixes below were found on them, though, so A's critical row is a floor too. Synthetic held-out only; not checked on real messages.

Tool: `harness/v5/a_baseline.py`. Every item runs its job's code path with an engine that never answers, so a user without the AI opt-in sees exactly this. Script flags are candidates, and Opus confirmed each by eye. Results: A-baseline.json (code at 97a1f3780) and A-before-fix.json (A's outputs from a947c45cb, scored by the same detectors).

| Job (50 items each) | Critical items: before → after | Omission items: before → after | Questions or failures hidden |
|---|---|---|---|
| (a) feed: excerpt, git's lead, facts' lines | 9 → **0** | 19 → **18** (36 %) | 0 |
| (b) review, code-only | 4 → **0** | 0 → **0** | 0 |
| (c) attention from the facts | 0 → **0** | 11 → **11** (22 %) | 0 |
| (d) Chat by code | 0 → **0** | 11 → **11** (22 %) | n/a |

## What A showed before the fixes (all confirmed by eye)

**Review: an invented "Failed:" line (fixed in e13a38ee0).** On all four items the facts show no failure.

| Item | What A showed | What A shows now |
|---|---|---|
| R3T13 | "Failed: The branch is clean again apart from a new regression test that documents the bug." | no line; the facts' "Tests: 62 passed, 0 failed" |
| R3T28 | "Failed: Two: the contact form shows an inline error …" (a feature) | no line |
| R3T29 | "Failed: Third, I need to know whether failed jobs … should be purged" | only the facts' Question line |
| R3T43 | "Fehlgeschlagen: Begonnen mit dem Umbau … fehlt noch." | under "Offen" |

**Feed: an agent claim the facts or git contradict, shown as fact (fixed in 1782dda5b).** A now shows each one as "The agent says: “…”" (German: „…“), with the contradicting fact beside it.

| Item | The claim | The fact | Verdict |
|---|---|---|---|
| R3T48 | committed and pushed | git: uncommitted | REAL |
| R3T49 | fixed the lint errors, lint clean | lint error, lint exit 1 | REAL |
| R3T02 | now rounds half-up for every locale | 2 failed | REAL |
| R3T03 | alle Tests sind grün | 3 failed | REAL |
| R3T06, 08, 10, 12 | reverted work stated as done, beside git's lead | git reverted it | REAL |
| R3T14 | "Done with the avatar upload." | git reverted avatar/crop.js | NOT REAL, a borderline check hit: the attribution it now gets is harmless |

## What A still shows (omissions, all by code)

- **Feed, 18 items: a gold feed_must word is missing.** A's excerpt is the agent's opening sentence, and the gold's words sit in later sentences (examples: R3T13 "regression test, join"; R3T28). These are omissions by construction of a one-sentence excerpt, not errors.
- **Attention, 11 items: gold-vs-ADR.** All are L1 turns per D-117; see verdicts-opus.md.
- **Chat, 11 items:**
  - 6 are questions in FR, ES, NL or IT (gold-vs-ADR).
  - R3C10: "which files did turn 7 … change" fell outside the lexicon; A shows the found rows.
  - R3C12 (why), R3C50 (a count of records), R3C48 and R3C49 (lexicon window, kind word).
