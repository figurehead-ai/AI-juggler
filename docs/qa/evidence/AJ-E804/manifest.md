# Evidence manifest — AJ-E804

Task: `docs/tasks/E8-validation-visual-performance-quality.md` AJ-E804
(accessibility verification programme). 2026-10-19, reviewer claude-opus.

Run: `just a11y-gate` (`scripts/a11y_gate.sh`). It also runs as the
`a11y` jobs of `promote.yml` (D-20), which the approval step waits on, so no
build reaches beta or stable with a failing accessibility gate. Headless
Chromium, production build, mocked bridge.

Evidence:
[`axe-summary.md`](axe-summary.md) and [`axe-gallery.json`](axe-gallery.json) (every scan; the per-test files under `axe/` are not committed),
[`keyboard/`](keyboard/) (one traversal record per flow),
[`contrast.md`](contrast.md), [`zoom-motion/`](zoom-motion/) (captures),
[`voiceover-script.md`](voiceover-script.md) (the manual pass).

## The finding behind this task: contrast was never really checked

The app's background is a layered CSS gradient. axe-core can't compute
contrast over a gradient, so it files those text nodes as "incomplete"
rather than as violations. Every earlier "axe 0 serious" result
therefore skipped contrast for nearly all text. The gallery scan now
swaps the gradient for its conservative end (`--color-canvas-hi`:
lighter in dark, darker in light) during the scan only, and takes the
capture after removing the override.

On the first honest run, **920 gallery states produced 1015 serious
violations**. 317 states failed colour contrast, 72 states failed
target size and 6 states failed scrollable-region-focusable.

## Claims

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Req.1: axe-core runs on every gallery state; 0 critical, 0 serious; moderate rules need a task ID | Req.1 | `just a11y-gate`: **920 gallery states in 721 tests — 0 critical, 0 serious, 0 moderate, 0 minor** ([`axe-summary.md`](axe-summary.md)). The first honest run found 1015 serious; every one was fixed in the product, none waived or excluded | PASS | 2026-10-19 | claude-opus |
| Req.2: keyboard traversals of the six core flows — every action reachable, focus visible at every stop, no trap, Esc closes dialogs and returns focus to the invoker | Req.2 | `keyboard-flows.spec.ts`, 6 flows: launch, switch 1-click, switch wizard, cleanup confirm, onboarding, inspector tab walk. Every stop shows a visible indicator ([`keyboard/`](keyboard/)); three runs in a row passed | PASS | 2026-10-19 | claude-opus |
| Req.3: a manual VoiceOver pass per release over rail, inspector and one wizard, with the script and findings stored | Req.3 | Script in [`voiceover-script.md`](voiceover-script.md). The pass itself needs a person at a Mac with VoiceOver: a maintainer step per release | PENDING (maintainer) | 2026-10-19 | claude-opus |
| Req.3: status text uses `role="status"` / live regions without stealing focus | Req.3 | The undo toast is `role="status"` `aria-live="polite"`; it takes focus only when the dialog that started it has gone and focus would otherwise fall to the page. Wizard steps are announced politely (`switch-steps`) | PASS | 2026-10-19 | claude-opus |
| Req.4: body and faint text measured in both themes; nothing under 4.5:1 | Req.4 | [`contrast.md`](contrast.md): every text token against all five surfaces, from the computed colours of the running app, all ≥ 4.5:1. Before: `faint` 2.6–3.3:1 in both themes, light `live` 3.1–3.7:1, light `accent` and `warning` 4.2:1 on surface-3, Account initials down to 3.4:1 (light, yellow/green hues) | PASS | 2026-10-19 | claude-opus |
| Req.5: reduced motion and 200 % zoom captured for rail, inspector and wizard at 1280x800, both themes; nothing only in animation, colour or hover | Req.5 | `zoom-motion.spec.ts`: 200 % = 640x400 CSS at device scale 2 — no horizontal page scroll, every top-bar menu and New fully on screen, the wizard's Continue in view; reduced motion — no running animation. Colour-only status is guarded by the AJ-E803 `status is never colour-only` check | PASS | 2026-10-19 | claude-opus |
| Performance: the accessibility gate's run time | budget (D-22: ≤ 15 min) | The whole gate (733 tests) took **10.1 min** on a 30-core Mac; the release job runs it in four parallel shards. The original < 3 min budget predated the 920-state gallery; the maintainer accepted the measured time and set ≤ 15 min (decision D-22, 2026-10-19) | PASS | 2026-10-19 | claude-opus |

## Defects found and fixed

1. **Text contrast (WCAG 1.4.3).** New token values: `faint` is
   `#848c99` in dark and `#626879` in light. In light, `live` is
   `#056074`, `accent` is `#5555e1` and `warning` is `#a94e08`. Each was
   chosen to hold ≥ 4.5:1 on every surface. The Account-initials
   lightness was chosen for the worst of all 360 hues (dark ≥ 5.4:1,
   light ≥ 4.8:1). A `text-faint/80` rollup and a launch row dimmed with
   `opacity-75` now use full-strength tokens.
2. **Keyboard trap in the terminal (2.1.2).** Selecting an Instance
   focuses its terminal, and the terminal keeps Tab and Shift+Tab for the
   shell (completion, and the agent's own mode cycling). Ctrl+Tab and
   Ctrl+Shift+Tab now leave it. Terminals send no sequence for those
   keys, so no CLI loses one. The hint "⌃Tab leaves the terminal" shows
   while the terminal has focus and is its `aria-describedby`.
3. **Focus didn't move into the New session sheet (2.4.3).** The next
   Tab went to the page behind the sheet. Opening the sheet now focuses
   its first control, and a Tab from outside is pulled back in.
4. **The cleanup confirm sheet had no focus trap (2.1.2).** Tab left it
   for the page behind. Tab now cycles inside the sheet.
5. **Cancelling the switch dropped focus on the page (2.4.3).** It now
   returns to the row that opened the switch. A completed switch hands
   focus to the undo toast's Undo, never the page body. 0.3.2: the rows
   sit in Continue on another account's dialog, gone once one is picked,
   so Cancel returns focus to whatever opened that dialog.
6. **Target size (2.5.8, WCAG 2.2).** Native radios and checkboxes are
   16 px with a 5 px margin, so neighbours keep 24 px of clear space. The
   rail's row-actions ⋯ button, the inline "copy", "project" and "Why
   this order?" links, and "add date" now have 24 px hit areas. The
   Minimal rail's tabs no longer collapse to 17 px.
7. **Scrollable tables unreachable by keyboard (2.1.1).** The process,
   cost and provider-pair tables' scroll regions are focusable, labelled
   regions.
8. **The top bar clipped its menus at 200 % zoom (1.4.10, 2.4.11).**
   History through New were cut off, so they could be focused but not
   seen. The bar now wraps at narrow widths and stays a single row at
   normal widths.

## Found by the axe-core 4.14 upgrade (2026-10-19)

axe-core 4.14 reports `label-content-name-mismatch` (WCAG 2.5.3, Label in
Name) as serious under WCAG 2.1 A. A voice-control user says the words
they see, so a control's accessible name has to contain them. Elements
with a hand-written `aria-label` that reworded their visible text failed:

- **Instance rows** (compact, comfortable, Minimal, card), the **remote
  rows** on the board, and the **cross-machine switch targets** are now
  named by their visible content. Shorthand gets a screen-reader word
  (initials → the Account name, "—" → "resources unavailable"; "!" is
  hidden), and the rest of a row's detail (quota with confidence, tokens,
  CPU/RAM, children, age, the attention reason) is its
  `aria-describedby`. A Minimal row, which shows only icons, is named
  "Account, status" and described by the full summary.
- **Filter chips** read "Needs me 5 items"; **project tiles** "Work in
  <project> last used with <Account>"; the Fleet **Access…** button
  "Access… for <Mac> on this Mac"; **Import** "Import as “<name>” —
  <path>"; **＋ Renewal date** "Renewal date for <Account> (optional)".

Tests that read the old labels now read the name plus the description
(`SessionRow.test.tsx`, `density.test.tsx`, `sorting.test.tsx`, the
keyboard-flow name helper, `regression.spec.ts`'s colour-only check).

## Notes

- The gallery's other a11y specs (`a11y.spec.ts`) still scan the real
  page. They now pass with the new tokens too. The gallery scan is the
  gate.
- Rail rows announce their status in words and carry their attention
  reason (AJ-E405), so screen readers don't depend on the badge colours.
- The radio group in the Cleanup centre's filters takes one Tab stop per
  option rather than one per group. That isn't a WCAG failure, but it
  differs from the APG radio pattern. Recorded for a later polish task;
  it isn't a gate item.
