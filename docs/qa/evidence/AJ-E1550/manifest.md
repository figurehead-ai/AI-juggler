# Evidence manifest — AJ-E1550

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1550 (calm surfaces: separation by tone, never bright borders; a bright line only for focus, hover or real attention; and, by the maintainer's amendment, requirement 5: balance, not monochrome). 2026-11-23, builder claude-opus.

## The rules

- **PAT-SURF**, `docs/design/DESIGN-SYSTEM.md` §5.12: the research table (Linear, Vercel Geist, Radix Themes, Apple's macOS HIG, the Claude and ChatGPT desktop apps), the tokens and PAT-SURF-1…8.
- **PAT-COLOR**, §5.13: one hue one meaning, a soft and a strong step each, Runway's bar by state, full-colour App icons, PAT-COLOR-1…7.
- Both mapped in `docs/design/VALIDATION.md` (§2 and the rule-to-check table).

## Tokens (`ui/src/styles.css`)

| Token (R1) | Dark | Light | Job |
|---|---|---|---|
| `border-subtle` (`hairline` is the same) | ink 7 % | ink 9 % | The one quiet line, only where tone can't separate (was `#2f3645` / `#d3d8e2`, 1.44:1) |
| `frame` | transparent | transparent | A card's or filled control's frame; faint under Increase Contrast |
| `overlay` | `#1b202b` | `#ffffff` | Menus, popovers, tips, the date picker |
| `hover` | ink 5 % | ink 5 % | A row's, tab's, segment's, menu line's hover |
| `control` / `control-hi` | ink 8 % / 12 % | ink 6 % / 10 % | Secondary buttons, inputs, picker fields, the segmented track |
| `selected` | `#292f3c` | `#dfe3ea` | Selected: a fill (1.26–1.44:1 off what's around it), semibold ink; no outline |
| `selected-edge` | transparent | transparent | `selected-ring` (≥ 3:1) under Increase Contrast |
| `focus`, `attention` | the accent | the accent | The only accent lines |
| `--shadow-popover`, `--shadow-dialog` (and `shadow-card`, now the floating layers' soft shadow) | soft, black | soft, ink | Floating layers only |
| `danger-strong`, `warning-strong`, `success-strong`, `live-strong` | `#db2943`, `#d37e01`, `#09af51`, `#059eb1` | `#c50031`, `#b46d10`, `#006e30`, `#028f95` | PAT-COLOR's strong step: Runway's bars |
| `.app-tone` | no filter | no filter | App icons always full colour (was saturate 0.25, opacity 0.72–0.78) |

## The palette validator (PAT-COLOR-6)

The dataviz skill's `validate_palette.js`, run locally, against each theme's surface.

Before (the identity slots as they were, and the text-step status colours):

```
identity, light and dark:   [FAIL] Normal-vision floor  worst adjacent #9d50ae↔#ca5b9f ΔE 9.7 (slots 2–3; 3–4 ΔE 11.0; 6–7 ΔE 9.7)
                            [WARN] CVD separation       #6868d3↔#9d50ae ΔE 6.5 (protan)
status text colours, light: [FAIL] Chroma floor, CVD separation (#9c4807↔#b91c1c ΔE 1.0 deutan), Normal-vision floor
status text colours, dark:  [FAIL] Lightness band, Chroma floor, Normal-vision floor (#7bbfcc↔#79c3a0 ΔE 6.8)
```

The status text colours are text (the validator's own scope note: a lone status or text colour is checked by contrast, and they are, at 4.5:1 on every surface, `attention-tokens.test.ts`). The fills that must be seen got their own strong step, validated as a set. Identity slots 3 (magenta → dark olive `{l: 0.56, c: 0.11, h: 110}`) and 7 (a second rose → deep blue `{l: 0.56, c: 0.2, h: 260}`) were replaced, keeping every rule in `mark-hues.ts` (the states' hue gaps, ΔE ≥ 0.12 from the status colours, 3:1 on every surface, AA letters).

After:

```
== identity slots, light, surface #ffffff  (#3b89e6,#54810d,#ca5b9f,#797a1e,#6868d3,#848f10,#bd67c2,#206ce7)
  [PASS] Lightness band         all 8 inside L 0.43–0.77
  [PASS] Chroma floor           all 8 >= 0.1
  [PASS] CVD separation         worst adjacent #206ce7↔#bd67c2 ΔE 8.4 (protan) · tritan 9.8
  [PASS] Normal-vision floor    worst adjacent #206ce7↔#bd67c2 ΔE 21.4 (normal)
  [PASS] Contrast vs surface    all 8 >= 3:1
== identity slots, dark, surface #151a23
  [PASS] Lightness band         all 8 inside L 0.48–0.67
  [PASS] Chroma floor           all 8 >= 0.1
  [PASS] CVD separation         worst adjacent #206ce7↔#bd67c2 ΔE 8.4 (protan) · tritan 9.8
  [PASS] Normal-vision floor    worst adjacent #206ce7↔#bd67c2 ΔE 21.4 (normal)
  [PASS] Contrast vs surface    all 8 >= 3:1
== strong semantic set + accent fill (red, amber, green, teal, violet), light, #ffffff
  [PASS] Lightness band         all 5 inside L 0.43–0.77
  [PASS] Chroma floor           all 5 >= 0.1
  [WARN] CVD separation         worst adjacent #006e30↔#b46d10 ΔE 6.8 (protan) · tritan 10.4
  [PASS] Normal-vision floor    worst adjacent #028f95↔#006e30 ΔE 15.7 (normal)
  [PASS] Contrast vs surface    all 5 >= 3:1
== strong semantic set + accent fill, dark, #151a23
  [PASS] Lightness band         all 5 inside L 0.48–0.67
  [PASS] Chroma floor           all 5 >= 0.1
  [WARN] CVD separation         worst adjacent #09af51↔#d37e01 ΔE 6.4 (deutan) · tritan 2.5
  [PASS] Normal-vision floor    worst adjacent #059eb1↔#09af51 ΔE 15.8 (normal)
  [PASS] Contrast vs surface    all 5 >= 3:1
```

No FAIL. The two CVD warnings sit in the validator's legal floor band (6–8), which needs secondary encoding: every status hue goes with its word or glyph (PAT-COLOR-5). The strong set stays ≥ 9 ΔE and ≥ 20° from the eight identity marks.

## The lint (PAT-SURF-8, PAT-COLOR-7)

`scripts/design_tokens_lint.py` (in `just verify-static`'s terminology step and CI's static job), allowlist `scripts/design_tokens.allow`, which only shrinks. Kinds: `bright-line` (an accent or bright-neutral border, ring, outline or divider outside a focus or hover variant; spinners and same-colour fills excepted), `sheen`, `grayscale`, `raw-hue`, `identity`. At first count there were 105 bright lines and 18 sheens in 95 files; after the pass, 37 allowed lines remain: 9 kept on purpose with their reasons (a swatch's chosen ring, a drag ghost, a drop target, a preview's lit node, a listening shortcut field, an open picker), 9 identity uses reviewed as marks, and the rest in files other 0.5.1 agents were editing (their one-line fixes are in the hand-back).

## Before and after

Captures of Home, a session's page, the switch dialog, Accounts (and its Out line at 1024 and 1280), Projects and Settings, both themes, generic fixture data: kept out of the repository (the baselines are re-shot once at the end of the 0.5.1 batch), in the session's scratch folder named in the hand-back.

## Gates

- vitest: the touched files' suites pass (controls, kit-controls, time-field, report-bug, one-mark-per-app, inspector-instance, attention-tokens, theme-namespaces, rail-model, marks, room-mark, home-runway-out, accounts-out-filter, conversation-switcher, ai).
- `tsc` clean; `scripts/ui_lint.py`: no new finding in these files.
- The palette validator, above.

## Close-out (0.5.1, 2026-11-23, Opus)

Each test named below was found by name and run, targeted, on `release/0.5.1` (vitest by file; cargo through `scripts/heavy.sh`, `--lib` with module filters). An OPEN line says where it closes.

R1: PASS — the research and the tokens are in `docs/design/DESIGN-SYSTEM.md` §5.12 (PAT-SURF) and §5.13 (PAT-COLOR); vitest `lib/attention-tokens.test.ts`, `lib/theme-namespaces.test.ts` (a121bf237).
R2: PASS — the shared components and every screen: vitest `controls/controls.test.tsx`, `kit-controls.test.tsx`, `time-field.test.tsx`, `lib/rail-model.test.ts`, `one-mark-per-app.test.tsx`, `pages/session/conversation-switcher.test.tsx`; `scripts/design_tokens_lint.py` (full run exit 0) (43371f885, 686ec8241, 4e804a25a, a2a0d68ac).
R3: PASS — `scripts/design_tokens_lint.py --selftest` and a full run: both exit 0. Its allowlist keeps only lines kept on purpose (a8b428be7, a2a0d68ac).
R4: OPEN — critique/baseline round: baselines (gallery and Happo) are re-shot once at the end of the 0.5.1 batch, after Opus's side-by-side with the Claude app in both themes.
R5: PASS — one hue one meaning, Runway's bar by state, full-colour App icons: vitest `lib/marks.test.ts`, `lib/room-mark.test.tsx`, `home-runway-out.test.tsx`, `accounts/accounts-out-filter.test.tsx`, `one-mark-per-app.test.tsx`; the palette validator above; the lint's `identity` and `raw-hue` kinds (D-145, 4ddc8d4d3).

### Usability answers, matched against the built tokens and components

1. A calm surface that recedes, like the Claude app's Code view: pending critique (screenshots, both themes, R4's round).
2. Hierarchy by tone and type, with selected and needs-you standing out: built (`selected` is a fill, attention keeps the accent); how it reads on screen: pending critique.
3. Tone, not lines; attention kept for what deserves it: matches (the lint fails any accent or bright line outside focus, hover and attention).
4. n/a.
5. None: matches (no system calls).
6. Watch-fors: lines everywhere (matches: the lint); an accent ring on "Suggested" (matches: the lint's `bright-line` kind allows the accent only in focus, hover and attention variants).
