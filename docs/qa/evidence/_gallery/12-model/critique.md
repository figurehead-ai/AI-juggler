<!-- captures: 7324d2bf34fc -->
# 12-model: what a session's model is: a model change seen in the transcript, a model the tool doesn't report, and the rail with and without the change mark

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

overview-drift-1440x900 states the change plainly in the Where it runs card: "Launched with opus · now running sonnet-5 (observed in transcript)" under "Model sonnet-5", wrapped onto two lines inside the card, which is easy to read. not-reported-1440x900 says "Model: not reported by Claude Code" and adds a quota box with a "Park until reset" button; the box's paragraph still wraps into a narrow six-line column beside the button. The same capture's terminal card says "Ended" while the meta line says "Running in AI Juggler · started 5h ago". rail-drift-1440x900 and rail-plain-1440x900 still differ only by a tiny "…" mark under the feat/read-model error row's status icon, which is nearly invisible and has no words, so the rail barely shows the model change. The design fixes show as the lighter accent on the logo mark, the selected tab, "Open its page" and the runway bar, a one-row top bar, and rail error rows that read "Stopped with an error" with no cut age. All captures were re-rendered. The second pass shows only in the two overview captures: the terminal's black now runs to its card's edges, with black padding before the text instead of a lighter frame.

## Weak

- rail-drift-1440x900: the model-change mark is a tiny unlabelled dot glyph, hard to see next to rail-plain.
- not-reported-1440x900: the terminal says "Ended" while the meta line says "Running in AI Juggler".
- not-reported-1440x900: the quota box's paragraph wraps into a narrow column beside "Park until reset".

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B3)

Re-captured (four states at 1440×900); opened at full size: not-reported and rail-drift. overview-drift keeps "Launched with opus · now running sonnet-5" under Model in Where it runs. not-reported now shows F66's question strip at the conversation's foot (a tone strip with an amber dot before "Claude asks you", no amber wash) and this batch's park note as a step of tone.

- **Fixed** (51ea3254f): the quota note ("Blocked by its quota. Park it until the reset…" with Park until reset) was an outlined box inside Where it runs (box in box); it is a step of tone, no outline (`ParkCard`). Gate 53-parked re-rendered.
- **F66 here:** the question strip under the conversation is tone with an amber mark (it was an amber-brown wash).
- **Weak, logged F77:** the rail's model-changed mark is "⇄" in an outlined pill named only by a `title` that no capture draws: the Switch account glyph, an outline, and no tip on focus. rail-drift and rail-plain still differ by that mark alone.
- **Weak, logged F80:** the Home behind rail-drift pins three L2 needs and folds both "Stopped with an error" sessions under "+2 more need you".
- **Closed since 0.3.2:** not-reported's terminal no longer says "Ended" beside "Running" (the conversation shows instead).

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Which model a session runs, and when it changed. |
| 2 | Prominence | 4 | The model line sits in Where it runs; the change said in words there. |
| 3 | Default | 4 | n/a: nothing to choose. |
| 4 | Mode | 4 | Calm. |
| 5 | Mock-up | 4 | I-03's spec (the change said, never guessed). |
| 6 | Areas | 4 | After the fix no box in a box in Where it runs. |
| 7 | Colour | 4 | Amber only as marks (F66's strip, the limit dot); the header chip's fill is F76's, scored on 05. |
| 8 | Type | 4 | Model line muted under its label. |
| 9 | Actions | 4 | Park until reset a quiet tinted action in its note; no second primary. |
| 10 | Serious | 4 | Parking says what is kept and when it comes back. |
| 11 | Doors | 4 | Account and Project are doors. |
| 12 | Words | 4 | "Model: not reported by Claude Code" plain. |
| 13 | States | 4 | Unknown said once. |
| 14 | Consistency | 3 | The rail's drift mark reuses ⇄ (Switch account's icon) with an outline and a title-only name (F77). |
| 15 | Accessibility | 4 | axe clean in the capture runs (no serious or critical); keyboard and zoom not re-walked this round. |
| 16 | Hand walk | 4 | Not re-walked by hand here; captured from the fixtures, both themes. |
| 17 | System prompts | 5 | Causes no system prompt. |
| 18 | Usability | 4 | Re-read against PAT-USE: the session page says the change in words; the rail's mark is the gap (F77). Match on the page. |

Lowest: 3, each line under 4 logged (F76, F77, F80); the rest 4 or 5.
