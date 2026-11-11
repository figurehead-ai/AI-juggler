# Evidence manifest — AJ-E2801 Shared controls: choice line, checkbox, radio and picker

Task: `docs/tasks/E28-finding-your-way.md` AJ-E2801, judged against the
requirements as amended 2026-11-06. The controls live in
`ui/src/components/controls/` (ChoiceLine, ChoiceOption, Checkbox, Radio,
Picker, Segmented, Disclosure), built in 0.3 (1274ff77b, shipped in
ca2247e04) and finished on `release/0.4.1`; re-checked against the code, its
tests, the lint and the committed gallery, 2026-11-06.

Run for this manifest (one command):
`cd ui && pnpm -s exec vitest run src/components/controls/controls.test.tsx src/components/controls/picker-groups.test.tsx src/components/rail-group-switch.test.tsx src/components/item-menu.test.tsx`
→ 4 files, 55 tests passed. Playwright was not run here: its tests run in the
gallery gate, and the captures cited are the committed ones.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 `ChoiceLine` draws icon · label · hint · ✓ on one grid (promoted from the rail's order menu); a `menuitemradio` / `menuitemcheckbox` in a menu, a `radio` / `checkbox` in a form | `ChoiceLine.tsx` (`grid-cols-[16px_minmax(0,1fr)_auto]`, ✓ column kept); `RailOrderMenu.tsx` on the shared line (0fed85fd4); vitest `controls/controls.test.tsx`: "is named by its label and described by its hint, with its reason when greyed" (radio in a form), "are the native control, drawn by the shared style" (checkbox), "shows the value, opens on the chosen line, and picks" (`menuitemradio` with `aria-checked`); `rail-group-switch.test.tsx`: "is a menu: Show lines turn on and off, Only opens a submenu, Reset shows only when changed" (`menuitemcheckbox`); gallery `_gallery/97-controls/view-open-1280x800-dark.png` | the four roles; one grid | all pass; the View capture shows icon, label, hint and ✓ columns aligned | PASS | 0fed85fd4 | 2026-11-06 | Claude (self) |
| R2 `Checkbox` / `Radio` in the accent, the app's focus ring, a disabled state visibly off with its reason read by screen readers; a disabled choice that is on keeps a muted fill (amended) | vitest `controls/controls.test.tsx` "says why it is greyed", `ChoiceOption` "is named by its label and described by its hint, with its reason when greyed"; `controls-gallery.spec.ts` "a choice on but disabled, and one focused from the keyboard" → `_gallery/97-controls/control-states-1280x800-{dark,light}.png` | on-but-disabled reads apart from off; focus ring shown; reason read | the order's chosen line disabled keeps a muted fill, the others dashed rings; the chosen theme reached by Tab wears the ring; both captures looked at, axe 0 critical/serious; vitest pass | PASS | 2f0f81289 | 2026-11-07 | Claude (self) |
| R3 `Picker` shows value and chevron, opens a menu of choice lines with ✓ on the current one (`aria-haspopup="menu"`, arrows, type-ahead, Home/End, Esc returns focus); replaces `form.tsx`'s `Select` | vitest `controls/controls.test.tsx`: "shows the value, opens on the chosen line, and picks", "opens from the keyboard, moves with the arrows, and Escape closes only the menu", "jumps to a line by its first letter, round from the one in focus (AJ-E2801 R3)", "is greyed with a reason"; `item-menu.test.tsx`: "is a role=menu that takes focus and gives it back on Escape" (Home/End in the shared `RowMenu` the Picker opens); `grep -n "export function Select" ui/src/components/form.tsx` (none); Playwright `controls-gallery.spec.ts` "a Picker open: History's filter"; `_gallery/97-controls/picker-open-history-1280x800-dark.png` | every key; no `Select` left | all pass; `Select` is gone from `form.tsx`; the capture shows the value, chevron and ✓ on All (spec runs in the gallery gate) | PASS | 0fed85fd4 | 2026-11-06 | Claude (self) |
| R4 the system selects, checkboxes and radios replaced (How it runs, History's filters, the stop sheet's "What to stop", Settings…) | `python3 scripts/native_controls_lint.py` (counts every raw `<select>` / checkbox / radio outside `ui/src/components/controls/`); `scripts/native_controls.allow` (empty); `KillSheet.tsx:447` "What to stop" on `ChoiceOption type="radio"` | 0 raw controls in shipped UI code | `native-controls: OK (0 allowed)`, exit 0 | PASS | 1274ff77b | 2026-11-06 | Claude (self) |
| R5 a lint fails a new raw `<select>`, checkbox or radio outside the shared controls; its allowlist only shrinks | `python3 scripts/native_controls_lint.py`; `python3 scripts/native_controls_lint.py --selftest`; wired in `justfile` (lines 155-156) | exit 0; selftest OK; allowlist ≤ its first count | `native-controls: OK (0 allowed)` exit 0; `native-controls selftest: OK` exit 0; the allowlist is empty | PASS | 659b29131 | 2026-11-06 | Claude (self) |
| R6 both themes; 12 px minimum; a hit area of at least 24 × 24 px | Playwright `controls-gallery.spec.ts` "Settings › General: equal cards" (asserts each radio's label box ≥ 24 × 24); `_gallery/97-controls/*-{dark,light}.png` (every state captured in both themes); `grep -rnE "text-\[(8\|9\|10\|11)px\]\|text-\[0\.[0-6]" ui/src/components/controls/` (none; the smallest step is `text-label`, 12 px, `styles.css:67,82`) | ≥ 24 × 24; nothing under 12 px; both themes | grep finds nothing under 12 px; both themes captured; the 24 × 24 check runs in the gallery gate | PASS | 94a1a1d4c | 2026-11-06 | Claude (self) |
| Gate: the controls' tests and the lint | the vitest command above; `python3 scripts/native_controls_lint.py` and `--selftest` | pass | 55 / 55 tests pass; lint and selftest OK | PASS | 94a1a1d4c | 2026-11-06 | Claude (self) |

## Binary artifacts

None added; cited: `_gallery/97-controls/view-open-*.png`,
`_gallery/97-controls/picker-open-history-*.png`,
`_gallery/97-controls/settings-general-cards-*.png`,
`_gallery/96-switch-setup/where-no-desktop-*.png`.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none)
