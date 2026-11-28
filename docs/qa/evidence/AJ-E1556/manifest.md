# Evidence manifest — AJ-E1556

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1556, no tooltip or tip is
clipped, and nothing overlaps one (P1). Found by the maintainer on the
0.5.1 Nightly (2026-11-25): Bot's "New topic" tooltip read "Nothing is
do…" with a "23" over it. Status: **DONE (0.5.2)**, 2026-11-25.

Cause (Opus): the tip was drawn inside Bot's head, whose `whitespace-nowrap`
it inherited, so its sentence ran 176 px past its 250 px box on one line
and whatever clipped the panel cut it; drawn in the head's stacking layer
(z-30), the panel's own counts could sit over it.

Commits: a97128d78 (the one tip; New topic, the folded rail's card and a
chart's tip moved onto it), 43eced8b0 (the overflow check, in the
accessibility gate), 1b26459cf (the rule in the design system; lint).

## Requirements

| R | Claim | Verdict | Proof |
|---|---|---|---|
| R1 | The New topic tooltip is whole, with nothing over it | PASS | `TopicButton` (`bot/BotHead.tsx`) uses the app's one tip (`useTip`, `TipLayer` in `WhyTip.tsx`): portalled over the page on the popover layer (z-50), `TIP_LOOK` (overlay tone, `whitespace-normal wrap-break-word`, never ellipsised), under the button's right end, kept 8 px inside the window once drawn (`keepInWindow`). `tooltip-whole.test.tsx` R1 (on the body, not in the head; its words whole; nothing it sits in keeps them on one line or cuts them; z-50; hover and focus); `tooltip-overflow.spec.ts` "Bot's panel at 1280" and "at 1024" (pass; on the old tip both fail: "words past its edge: \"Bot stops using what came before\" (176 px over)") |
| R2 | A Playwright overflow check opens every tooltip and tip in the gallery states and fails on clipped text or an overlapping element; every finding fixed | PASS | `ui/tests/visual/tooltip-overflow.spec.ts`, in `scripts/a11y_gate.sh`: ten gallery states (Home and the rail's App lens at 1280 dark and 1024 light, the Project lens, the folded rail, Bot's panel at 1280 and 1024, a session's page, an ended session's page, Accounts, Usage's charts across their width) hover every control that can show a tip (316 controls) and measure every tip drawn (61 opened: the rail's head names, the folded rail's cards, the This Mac line's tip, New topic, the charts' tips): words cut or ellipsised (scrollWidth over clientWidth where overflow hides, `text-overflow: ellipsis`, a line clamp), words past its edge (each text run's box inside the tip's), outside the window or a clipping box, anything drawn over it (the topmost element at 15 points across it). Numbers in [tooltip-overflow.json](tooltip-overflow.json): 0 findings. Findings fixed on the way: the folded rail's card was drawn inside the rail's scroller with its title cut (`truncate`) and is now the one tip beside the icon; a chart tip's series names were cut (`truncate`) and now wrap; New topic (R1). Not vacuous: the old New topic tip, card and chart tip fail it |

## Usability answers, checked on the built app

1. A tooltip is read whole or not at all: every tip's words wrap inside it (0 findings over 61 tips opened). Match.
2. It wraps to its words, never ellipsised, and sits above what it names: the popover layer, portalled, nothing drawn over it (the overlap check). Match.
3. One Tooltip pattern: `TipLayer` / `useTip` (WhyTip, the name tips, New topic, the folded rail's cards), the chart tip wrapping the same way; written down in DESIGN-SYSTEM §4.16 "Tip". Match.
4. Long words wrap (`wrap-break-word`). Match.
5. None. Match.
6. Watched for: clipping at the window's edges (`keepInWindow`, and the check's window test at 1024 and 1280) and under counts (the overlap check). Match.

## Checks run

- `tooltip-overflow.spec.ts` through `scripts/heavy.sh` on the host: 10 passed (and, run against the old New topic tip, the folded rail's old card and the old chart tip, it fails on each).
- vitest: the whole UI suite, 634 files, 4299 tests, pass (`tooltip-whole.test.tsx`: 3); `tsc -b --noEmit` clean; `scripts/ui_lint.py` OK; `debt_baseline.py` clean for these files (`below` no longer exported).
- The gate: no gate state frames an open tip, so no baseline changed for this item (the 0.5.2 re-render's notes, [rerender-0.5.2.md](../AJ-E803/rerender-0.5.2.md)).
- `plain_words_lint.py`, `design_tokens_lint.py`, `private_data_lint.py`, `terminology_lint.sh`: OK on these files.
