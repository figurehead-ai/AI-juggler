# Calm attention: the glance test and the colour-vision check (AJ-E3601 R8)

A reusable walk sheet for D-117. Run it on a build before and after a
screen's calm pass. Sonnet persona testers run it; Opus checks every
verdict before it is recorded (model tiering).

## Captures to use

Take each in both themes, at 1280 × 800, from the gallery fixtures
(generic names only):

| Fixture | What it shows | Spec |
|---|---|---|
| typical | 8 open on 4 Apps, 2 ask you, 1 to review | `ui/tests/visual/places-gallery.spec.ts` (`home-typical`) |
| all statuses | every session state, 1 stopped with an error | `ui/tests/visual/home-gallery.spec.ts` (`needs-you`) |
| flood | 7 waiting, 1 stopped with an error | `ui/tests/visual/attention-budget.spec.ts` (`flood`) |
| all clear | nothing needs you | `ui/tests/visual/home-gallery.spec.ts` (`all-clear`) |

## The glance test (5 s, then 10 s)

Show each capture for 5 seconds, then hide it and ask the questions
below. Then show it again for 10 seconds and ask them again.

1. How many things need you?
2. Which one first?
3. Is anything broken?
4. What is running fine?

**Pass:** at least 4 of 5 personas answer questions 1 to 3 correctly at
5 s. On the flood fixture, they name the stopped session first. Run the
same sheet on the build before the change as the baseline.

Personas: the multi-subscription vibe-coder (8 sessions, 3 Apps), a
newcomer (1 App), and a heavy day (14 open, 7 needs, 1 crash).

## The colour-vision check

Run each capture through protan, deutan and tritan simulations, for
example with the macOS Accessibility Inspector's colour filters, or any
Brettel/Viénot simulator.

**Pass:** L2 and L3 stay distinct by **shape** and words, not by hue:
- L2: a filled amber dot, or a hollow ring for a guess, and its word;
- L3: a ✕ glyph, the row's tint, and its word.

## Recording

Write the verdicts into the task's evidence manifest
(`docs/qa/evidence/AJ-E36NN/manifest.md`):
- each persona's answers and timing;
- pass or fail per fixture and theme;
- the colourfulness table, from
  `python3 scripts/qa/colourfulness.py --check <captures>`.

Opus signs off each verdict.
