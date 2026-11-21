# Validating screens against the design system (2026-11-21)

**Status (D-142).** N1 ships in 0.5.0 with the one dialog frame; N2–N20 follow after 0.5.0 (AJ-E3618, AJ-E3619). The screen-review checklist (§5) applies now: every UI change is scored on it by Opus, at 4 or more on every line, before any baseline.

Every rule in `DESIGN-SYSTEM.md` is checked one of three ways:
- **A, automated, existing:** a gate or test in the repo today.
- **N, automated, to build:** a new lint, unit test, Playwright check or visual check. Each is specified in §3 precisely enough to implement.
- **R, review:** an Opus design review against the screen-review checklist (§5), from captures and a hand walk.

Most rules get two of these, because a gate proves a screen is "unchanged" and "accessible", not that it is good (D-57, M:design-quality-review).

Paths are repo-relative. All new checks follow the repo's existing pattern for lints, `scripts/*_lint.py` with an allowlist that **only shrinks**, so they can land before every screen is fixed without blocking unrelated work (the same rollout Carbon's "light-touch" preset and Polaris's migrator use).

---

## 1. How a screen is validated, in order

| Step | What | Who (model tiering) | Blocks? |
|---|---|---|---|
| 1 | Types: a component can't be written wrong (e.g. `PickerKind` forces a top group; `disabled` needs `reason`) | compiler | yes (ui build) |
| 2 | Static lints in `just verify-static` / `just preflight` | scripts | yes |
| 3 | Unit and role tests (vitest) | CI `ui` | yes |
| 4 | Measured checks in a real renderer (Playwright: fit, contrast, budget, kit) inside the a11y gate's job | CI `a11y` shards | yes |
| 5 | Visual regression (AJ-E803) and Happo, both themes, 1280×800 and 1024×680 | CI `visual` | yes (maintainer-approved baselines) |
| 6 | Captures drafted, critique drafted, persona and hand walk scored | Sonnet | no (input to 7) |
| 7 | Design review against §5, side by side with the mock-up in both themes; verifies Sonnet's verdicts | Opus | yes: a line under 4 is fixed before baselines |
| 8 | Critique stamped (`scripts/critique_check.py --stamp`) after staging | Opus | yes (`just evidence`) |

---

## 2. Existing gates and what they already cover

| Gate (command) | What it checks | Rules it covers |
|---|---|---|
| `scripts/ui_lint.py` (oxlint, baseline shrinks) | TypeScript strictness, react hooks, jsx-a11y, 400-line files, complexity 25, only `lib/ipc` calls the backend | A11Y-4/5 partly (jsx-a11y), PR-11 partly (size budget) |
| `scripts/native_controls_lint.py` | No system select, checkbox, radio, date box or `<details>` outside `components/controls/` | FRM-7, §4.16 Disclosure |
| `scripts/text_input_lint.py` | Text inputs have autocorrect, autocapitalise and spellcheck off | FRM-8 |
| `scripts/plain_words_lint.py` + `plain-words-on-screen.test.tsx` | No developer terms in on-screen strings | CNT-1 |
| `scripts/terminology_lint.sh` + `one-vocabulary.test.tsx` | Legacy nouns (harness, profile-as-account) in docs and UI | CNT-2 |
| `scripts/tool_literal_lint.py` | No tool ids outside adapters (so per-tool UI branches can't creep in) | PR-11 (architecture) |
| `scripts/critique_check.py` (D-57, AJ-E2814) | Each gallery family has a critique stamped for its current captures | R process (§5), D-57 |
| `scripts/check_baseline_notes.py` | Baseline changes carry a per-screen note | Screenshot notes rule |
| `scripts/a11y_gate.sh` (AJ-E804) | axe AA on every gallery state, both themes; keyboard flows; 200% zoom; reduced motion; status glyphs out of names; `fits-its-room`; `header-room-words`; Models and Watches galleries; contrast table | A11Y-1/3/4/5/8, ICN-5, DEN-1, MOT-3 |
| `ui/tests/visual/contrast.spec.ts` | Every text token ≥ 4.5:1 on every surface, count badges, selected edge ≥ 3:1 | COL-7, A11Y-2 |
| `ui/tests/visual/attention-budget.spec.ts` (D-117) | Per area on Home, rail, header: warning/danger text ≤ L2/L3 items; no amber fill; ≤ 1 danger-soft block; ≤ 1 filled accent button; no infinite animation; `data-level` matches `attentionLevel()`; chevrons kept; selection never loud; feed quiet | COL-4, COL-5, BTN-1 (calm areas only), MOT-2, SEC-4, LST-5, PAT-ATT-1/2 |
| `ui/tests/visual/idle.spec.ts` | No endless animation at rest | MOT-2 |
| `ui/tests/visual/menus-a11y.spec.ts` (AJ-E3011) | Every menu and popover: options exposed, hits land, the rest inert, Escape restores | MNU-4 |
| `ui/tests/visual/keyboard-flows.spec.ts`, `zoom-motion.spec.ts` | Six core keyboard flows; zoom and motion captures | A11Y-4, A11Y-8 |
| `ui/tests/visual/regression.spec.ts` (AJ-E803) + Happo | Core screens unchanged, both themes, both densities, 1280×800 and 1024×680; text in chrome 0 px | DEN (sizes), regression only |
| `scripts/visual_sweep.sh` (D-21) | Whole gallery base vs head, report | regression only |
| `ui/src/lib/attention-tokens.test.ts`, `theme-namespaces.test.ts` | Token contrast pairs; `--text-*` vs `--color-*` name clashes | COL-7, TYP-1 (namespace) |
| `primariesIn()` in 6 component tests | One primary in those views | BTN-1 (6 views only) |
| `controls/picker-groups.test.tsx` + `PickerKind` type | Pickers of things name a top group; search past 8 | PCK-1, PCK-2 |
| `controls/kit-controls.test.tsx`, `flow-layout.test.tsx`, `scroll-edge.test.tsx`, `alert.test.tsx` | Kit roles and keys | RC-5, TGL-1, WIZ-1, DLG-4 (rail only) |
| `just load-gate` (AJ-E805), `just polling-audit` | Start, render and selection budgets at scale; no sub-second polling | PR-9, PAT-LST-2 |
| `scripts/release_scope.py` | Version gates past the reference Mac's tools listed | PAT-HIDE-4 |

**What nothing checks today:** dialog fit and sticky footers (the reported bug), one primary outside six views and the calm areas, body text in `muted`, the 12 px floor in a renderer, raw sizes, radii and shadows, icons on menu lines, rich-mode kit use, one meaning per icon, disabled-without-reason, serious-choice explanations, the mode of each screen.

---

## 3. New checks to build

Each has an ID (N1…), a type, what it fails on, where it runs, and the rules it covers. Specs that run in a renderer reuse the gallery fixtures (`ui/tests/visual/fixtures.ts`, `axe-gallery.ts`) and run inside the a11y gate's shards, so they add no new CI job.

### N1. `dialog-fit.spec.ts` (Playwright), the reported bug

- **Scope:** every gallery state that shows a `[role="dialog"]` or `[role="alertdialog"]` (the gallery already opens them), plus the window sheets.
- **Sizes and themes:** 1024×680 and 1280×800, dark and light.
- **For each dialog:**
  1. Its bounding box is inside the viewport: `top ≥ 0`, `bottom ≤ innerHeight`, `left ≥ 0`, `right ≤ innerWidth`.
  2. It has exactly one header (`[data-dialog-part="header"]`), one body (`…="body"`) and at most one footer (`…="footer"`). `DialogFrame` sets these attributes; the hand-made frames fail until they adopt it (that is the point).
  3. The body's computed `overflow-y` is `auto` or `scroll`; the dialog's own and the overlay's are not.
  4. Scroll the body to its end, then to the middle. At each position, the header's and footer's boxes are unchanged (±1 px), and every footer button passes a hit test (`document.elementFromPoint` at its centre is the button or inside it).
  5. If `body.scrollHeight > body.clientHeight`: the footer has a top border ≥ 1 px, and the body carries `data-more` while not at its end.
  6. No horizontal overflow: `body.scrollWidth ≤ body.clientWidth + 1`.
  7. The first choice group (`[role="radiogroup"]`, `[data-choices]`) is fully inside the body's visible rect on open (DLG-3).
  8. Exactly one `[data-primary]` inside the dialog when it has a footer.
- **Fails on** any of these. Writes `docs/qa/evidence/design-system/dialog-fit.json` with each dialog's measurements.
- **Covers:** DLG-1, DLG-2, DLG-3, DLG-4, BTN-1 (dialogs), DEN-1.

### N2. `scripts/dialog_frame_lint.py` (static, allowlist shrinks)

- Counts per file in shipped UI code: `fixed inset-0`, `role="dialog"` and `aria-modal` outside the frame components (`components/DialogFrame.tsx`, and until it merges, `components/form.tsx` and `components/switch/SwitchDialogFrame.tsx`).
- Allowlist `scripts/dialog_frame.allow` starts with today's 35 files (`GAPS.md` A2) and only shrinks.
- **Covers:** PR-11, DLG-1 to DLG-7 (by forcing one frame).

### N3. `scripts/design_tokens_lint.py` (static, allowlist shrinks)

Counts per file, in `.tsx` and `.ts` under `ui/src` (tests, preview and generated code exempt):

| Kind | Pattern | Allowed |
|---|---|---|
| `hex` | `#[0-9a-fA-F]{3,8}\b` in a class or style string | the terminal palette files (`TerminalView.tsx`, `TerminalSearch.tsx`, `TerminalWindow.tsx`), `charts/colors.ts`, `lib/mark-hues.ts` |
| `px-text` | `text-\[\d+(\.\d+)?(px|rem)\]` | none; anything under 12 px is a hard fail with no allowlist |
| `tw-size` | `\btext-(xs|sm|base|lg|xl)\b` | baseline only; new code uses `text-label/meta/body/section/title` |
| `radius` | `rounded-\[[^\]]+\]` | none once `--radius-control` and `--radius-row` exist |
| `shadow` | `shadow-\[[^\]]+\]` | none once `--shadow-popover` and `--shadow-dialog` exist |
| `spacing` | `\b(p|px|py|pt|pb|pl|pr|m|mx|my|mt|mb|gap|gap-x|gap-y)-\[\d+px\]` | baseline only |
| `z` | `z-\[\d+\]` and `z-(10|20|30|40|50)` | only the values in ELV-1, through named classes once added |

- Today's baseline (measured 2026-11-21 on `release/0.5`): 14 hex, 27 arbitrary text sizes (7 under 12 px), 326 `text-xs`, 120 `text-sm`, 10 distinct radii (`GAPS.md` A5, A9).
- **Covers:** COL-1, TYP-1, TYP-2 (static half), SPC-2, RAD-1, ELV-1, ELV-2.

### N4. One primary per view, everywhere (Playwright + static)

- **Runtime:** extend `attention-budget.spec.ts` (iv) to every gallery state: per view (the main page region, each open dialog, each window sheet) count visible elements that are `[data-primary]` **or** whose computed background equals `--color-accent-fill`. Must be ≤ 1, and exactly 1 in a dialog with a footer. The top bar's `accent` button is excluded by its variant.
- **Static:** fail any use of `PRIMARY_FILL` or `bg-accent-fill` outside `primitives.tsx` and the step dot in `FlowLayout.tsx` (allowlist shrinks; `switch/SwitchDialogFrame.tsx` is on it until BTN-7 is fixed).
- **Covers:** BTN-1, BTN-7, COL-6, EMP-3.

### N5. Readable text roles (Playwright)

On every gallery state, both themes, for each visible text element with its own text:
1. **Floor:** computed `font-size ≥ 12px`, except `aria-hidden` glyphs. (TYP-2)
2. **Body not in muted:** an element whose computed colour equals `--color-muted` or `--color-faint` and whose rendered height exceeds 2 × its line height (a paragraph of three lines or more) fails. Help may be muted for up to two lines. (COL-2, FRM-3, SEC-2)
3. **One heading per area:** each `section`, `[role="dialog"]` and `[data-area]` has at least one element with `.type-title` or `.type-section` (or an `h1`–`h4` in ink). (COL-3, SEC-1)
4. **Hierarchy present:** in each view, at least three distinct text colour tokens among visible text (ink plus two of ink-2, section-label, muted). A view in one or two greys fails. (PR-3, COL-3)
5. **Weights:** computed `font-weight` in {400, 500, 600}. (TYP-6)

Writes a per-state table; a new screen can't add findings; old findings sit in a shrinking list.

### N6. Rich-mode kit check (Playwright, rich and mixed regions only)

Uses the mode registry (N20). In each rich view or rich region:
1. Every group of 2–5 mutually exclusive choices with a sub-line is a `RadioCards` group (`[data-kit="radio-card"]`); a `radiogroup` of bare radios with hints fails. (RC-1)
2. Every `RadioCard`/`CheckCard` contains a `ToolLogo` or an icon (`svg` or `img`) ≥ 24 px. (RC-2)
3. Every checkbox or switch in a list of 2 or more is inside a `ToggleRow`/`CheckRow` with an icon chip. (TGL-2)
4. Each `Section` that holds choices has an icon or logo somewhere in it. (MD dials, D-126)
5. There is at least one accent-coloured element (the primary, a chosen ring, a current step, an accent tag or link), so a rich screen can't render "monochrome". (D-126 §1)
6. The form column is ≥ 520 px wide at 1280×800 and nothing sits in a narrow centred column on an empty canvas (form box's left edge within 48 px of the steps column or the dialog's padding). (D-126 "good use of the space")

### N7. Status never colour alone (Playwright)

For every element whose computed colour or background is a status token (`--color-warning`, `--color-danger`, `--color-success`, `--color-live` or their soft fills): it, or a sibling within its row, has visible text of at least 2 characters, or a `.level-mark` / glyph with a distinct shape per level. L2 and L3 marks must differ in shape (`data-level` 2 is round; 3 is the ⊘/✕ mask). Runs once with a grayscale filter to assert the L2 and L3 marks still differ in their mask, not only colour. **Covers:** TAG-3, A11Y-3, ICN-7.

### N8. Menu lines carry icons (types + unit test)

- Add `icon: ControlIcon` to `MenuItem`, required unless `checked` is set (a choice line shows ✓) or `submenu` is set with an icon on its parent. A discriminated union makes a missing icon a compile error, the way `PickerKind` forces a top group.
- Until then, a static count of `<MenuItem` blocks with no `Icon`, `ToolLogo` or `AppIcon` inside (today 34 of 82), allowlist shrinks.
- **Covers:** ICN-4, MNU-1.

### N9. Disabled needs a reason (types + lint)

- Types: the shared controls (`Button` when disabled, `ChoiceOption`, `RadioCard`, `CheckCard`, `ToggleRow`, `CheckRow`, `MenuItem`, `PickerOption`, `FlowAction`) take `disabled?: false` or `{ disabled: true; reason: string }`.
- Lint: a JSX element with `disabled={…}` and none of `reason=`, `aria-describedby=`, `title=` is counted per file (allowlist shrinks). `switch/SwitchDialogFrame.tsx` `ContinueButton` is the first to fix.
- Runtime (in N5's pass): every visible `:disabled` or `[aria-disabled="true"]` control has an accessible description, or a visible text node within its row or footer.
- **Covers:** BTN-5, TGL-4, RC-6, PCK-4, PAT-HIDE-2, A11Y-10.

### N10. Destructive defaults (Playwright)

For every dialog whose footer contains a `danger` button: on open, focus is not on it; pressing Return does not activate it (assert the dialog is still open and no IPC call for the action was made, through the mock bridge's call log); Escape closes. **Covers:** BTN-4, PAT-DES-3.

### N11. Serious choices are explained (unit + registry)

- A registry `ui/src/lib/serious-choices.ts` lists each serious choice by test id (switch How, secrets in handovers, cleanup tiers, agent access levels, privacy and stats, keys questions, move vs copy, delete with folder).
- A vitest renders each and asserts: a consequence line (`[data-consequence]`), a "What does this mean?" disclosure and a "Learn more" link with a guide id that exists in the docs' guide index.
- **Covers:** PAT-SER-1, PAT-SER-5, PAT-SER-6.

### N12. Tags stay tags (Playwright + static)

- Static: `Tag`/`Chip`/`Pill` children that are string literals have ≤ 3 words and start with a capital, followed by lowercase except a proper-name list read from the tools catalog. (TAG-1, CNT-3)
- Runtime: per row (`[data-row]`, `li`), ≤ 2 visible tags; a tag never wraps (`scrollWidth ≤ clientWidth` or a tooltip present). (TAG-2, TAG-1)
- Static: one tag component. After `Chip` and `Pill` merge, importing the old names fails. (TAG-5, PR-11)

### N13. Sentence case on screen (static, extends `plain_words_lint.py`)

For string literals in JSX text and in `label`, `title`, `aria-label`, `summary`, `heading` props: words after the first are lowercase unless in the proper-name list (tool and app names from the catalog, "AI Juggler", "This Mac", "Mac", month and day names, acronyms such as MCP and CLI). Title Case Headings fail. **Covers:** CNT-3.

### N14. Empty-state count (Playwright)

Load each screen with an empty fixture and a partial fixture (`allStatuses` without accounts, without sessions). Count elements `[data-empty-state]` per area: ≤ 1. No filter controls (`[data-filter]`) on an empty list. No "0" or "—" in a stat tile or gauge whose source is unknown (the tile carries `data-unknown`). **Covers:** EMP-1, EMP-4, EMP-6, PRIN §14.

### N15. Truncation shows the whole (extends `fits-its-room.spec.ts`)

Every visible text element with `scrollWidth > clientWidth + 1` (cut) has a `title`, an accessible description or a hover/focus tooltip that holds its full text; none is cut mid-word in the header (already) or in a dialog title. **Covers:** DEN-1, TAG-1.

### N16. Icon meanings are unique (unit test)

- `ActionIcon` and every action map (`place-actions.ts`, row menus' action tables) map an action key to an icon. A test asserts no two different action keys share one icon component, except pairs listed with a reason (e.g. Close × in chrome).
- A test asserts `Alert`'s `broken` icon is not `IconClose` (ICN-2).
- `docs/design/icons.md` (generated by a script from the icon files' names and doc comments) is the meaning registry reviewers check.
- **Covers:** ICN-1, ICN-2.

### N17. Paged lists (static, Rust side)

Every IPC command whose return type is a `Vec<…>` of sessions, events, messages, tasks or projects takes a cursor and a limit with a hard maximum, or is listed in an allowlist with its bound and reason. (Parses `#[tauri::command]` signatures in `crates/juggler-app/src`.) **Covers:** PAT-LST-2.

### N18. Toast discipline (unit)

`toast()` calls carry an `origin: "user-action"`; a call from a backend event handler (files under `lib/ipc/` listeners, `lib/notifications*`) fails the test. Error toasts must set `sticky`. **Covers:** TST-1, TST-3.

### N19. Colour-vision and squint captures (report, not a gate)

For the core screens and every dialog, `pnpm visual:capture` also writes protan, deutan and tritan filtered copies and an 8 px blur. The Opus review (§5 line 6 and 14) looks at them. **Covers:** A11Y-3, PAT-ATT (squint), PRIN §6.

### N20. The mode registry and per-mode runs

- `docs/qa/evidence/_gallery/modes.json`: each gallery family → `calm`, `rich`, `mixed` (with the selector of its rich region), or `rich-compact` (destructive confirms). Seeded from the screen audit's 63 rows (`docs/research/screen-audit.md` §1).
- `scripts/visual_specs_check.py` fails a family missing from the registry (a new screen must declare its mode: MD-1).
- `attention-budget.spec.ts` runs its invariants on every `calm` family (today only Home, rail and header); N6 runs on every `rich` and `mixed` region.
- **Covers:** MD-1, PAT-ATT-1 to 4 across all calm screens, and routes N6.

### N21. Switching is one visible click (vitest now, Playwright with N1's pass)

- **Unit (in place, `components/switch-account.test.tsx`):** "Switch account…" is the first child of a session's and a conversation's side column; accent with room, `data-primary` when its account is out of room or low, and then the view's only `[data-primary]`; its press opens the switch list.
- **Runtime (to add to N1's gallery pass):** on every gallery state whose page is a session or a conversation (`[data-testid='session-page'], [data-testid='conversation-page']`), `[data-testid='switch-account']` is visible without scrolling at 1024×680 and 1280×800 in both themes, and passes a hit test. A session on a paired Mac is exempt.
- **Covers:** PAT-SW-1 to PAT-SW-3, BTN-1 on session pages.

### Rollout order

1. N1 + N2 with `DialogFrame` (the reported bug, and the frame every later fix uses).
2. N3, N4, N5 (tokens, one primary, readable roles): they catch most "monochrome" and off-scale regressions.
3. N9, N8, N16 (types first; small).
4. N20, then N6 and N7 (they need the registry).
5. N10–N15, N17, N18; N19 as a report.

Each lands warn-first with today's counts as its allowlist (shrink-only), then fails on new findings.

---

## 4. Every rule mapped to its check

Legend: **A** = existing automated, **N#** = new automated (§3), **R** = Opus review (§5 line number).

| Rule | Check |
|---|---|
| PR-1 One design language | R5, R14; N3; N20 |
| PR-2 Calm not plain | N5.4; R4, R5 |
| PR-3 Easy to read | N5; R6, R7 |
| PR-4 Where is what | R2, R11; hand walk 5-second test |
| PR-5 Prominence by frequency | R2, R3 |
| PR-6 Colour means something | A attention-budget; N4, N7; R7 |
| PR-7 Honest and usable | N9, N14; A release_scope; R12 |
| PR-8 Serious choices explained | N11; R10 |
| PR-9 Instant | A load-gate; N17; R13 |
| PR-10 Keyboard and accessible | A a11y gate, keyboard-flows; R15 |
| PR-11 Built once | N2, N3, N12; A ui_lint size budget, tool_literal_lint; R14 |
| PR-12 Prime space | R2 |
| MD-1 Mode declared | N20 |
| MD-2 Adjust, don't flip | R5 |
| COL-1 Tokens only | N3; A D-16 rule in review |
| COL-2 Body in ink-2 | N5.2 |
| COL-3 One ink heading, values loudest | N5.3, N5.4; R8 |
| COL-4 Status hue only at L1–L3 | A attention-budget (calm); N7; N20 extends to all calm |
| COL-5 No amber fill, one red block | A attention-budget |
| COL-6 Accent not status | N4; R7 |
| COL-7 4.5:1, APCA Lc 60 | A contrast.spec, attention-tokens.test |
| COL-8 muted 7:1 | contrast.spec raised to 7:1 for `muted` (B1 decided, D-142) |
| COL-9 Increase Contrast | A (styles.css media rule); add a contrast.spec pass with `prefers-contrast: more` |
| COL-10 Chart colours | A charts.test; R7 |
| TYP-1 Role classes | N3 (`tw-size`, `px-text`); A theme-namespaces.test |
| TYP-2 ≥ 12 px | N3 (static), N5.1 (rendered) |
| TYP-3 ≤ 3 sizes | R8 |
| TYP-4 Tabular numbers | R8 |
| TYP-5 Relative + absolute time | R12 |
| TYP-6 Weights | N5.5 |
| TYP-7 Sentence-case section headings in rich | N5.3; R8 |
| SPC-1 Between ≥ 2× inside | R6 |
| SPC-2 No arbitrary spacing | N3 |
| SPC-3 No box in box | R6 |
| RAD-1 Four radii | N3 |
| RAD-2 Nested radius smaller | R14 |
| ELV-1 z-index scale | N3 |
| ELV-2 No glows, sheen, gradients | N3 (`shadow`, `.numeral`, `.sheen` usage count); R14 |
| ICN-1 One meaning | N16; R14 |
| ICN-2 ✕ not broken | N16 |
| ICN-3 Icon-only named, tooltip on focus | A axe (names); R15 |
| ICN-4 Icons on action lines | N8 |
| ICN-5 Glyphs out of names | A a11y gate glyph check, glyph-names.test |
| ICN-6 Badge beside icon | A app-icon tests; R14 |
| ICN-7 Status mapping | N7; A attention-budget (vi) |
| MOT-1 Durations | N3 (once tokens exist); R14 |
| MOT-2 No endless animation | A idle.spec, attention-budget (v) |
| MOT-3 Reduce Motion | A zoom-motion.spec |
| MOT-4 Motion explains | R14 |
| DEN-1 No sideways scroll, nothing cut | A fits-its-room, header-room-words; N1.6, N15 |
| DEN-2 Densities differ | A rail-density.test; R13 |
| BTN-1 One primary | A attention-budget (calm), primariesIn (6 views); N4 (all) |
| BTN-2 Primary names outcome | N13 list of banned labels ("OK", "Submit", "Yes", "Continue" as last-step primary); R9 |
| BTN-3 Footer order | N1 (button order: primary is the last footer button) |
| BTN-4 Return and danger | N10 |
| BTN-5 Disabled says why | N9 |
| BTN-6 Progress at once | hand walk measure (≤ 100 ms); R13 |
| BTN-7 No hand-made buttons | N4 static |
| BTN-8 Sentence case, ellipsis | N13; R12 |
| DLG-1 Fixed header and footer | N1 |
| DLG-2 Height cap, primary visible | N1 |
| DLG-3 Choices visible on open | N1.7 |
| DLG-4 Divider and fade | N1.5 |
| DLG-5 Step up size | R6 |
| DLG-6 One dialog | N1 (count of open dialogs ≤ 2) |
| DLG-7 Focus, Esc, return | A keyboard-flows (core flows); N1 adds focus-in and focus-return per dialog |
| DLG-8 Invalid stays open; unsaved asks | unit tests per form; R13 |
| DLG-9 No info-only modal | R9 |
| DLG-10 Progress inside | R13; unit test of the flow's phases |
| WIZ-1 Step values, current marked | A flow-layout.test |
| WIZ-2 Back keeps input | unit test per wizard (step back and forth, values kept) |
| WIZ-3 Next names next step | R9 |
| WIZ-4 Review step | R9 |
| WIZ-5 Step opens at top | A flow-layout.test (`at`) |
| WIZ-6 Prefill | R3 |
| WIZ-7 Validation on Next | N9; R13 |
| WIZ-8 Preview matches form | R5 (side by side) |
| FRM-1 Labels above | R8 |
| FRM-2 Short labels, no placeholder label | N12-style static check on `Field label`; R12 |
| FRM-3 Help one line | N5.2 |
| FRM-4 Validation timing | unit tests per form; R13 |
| FRM-5 Error words | R12 |
| FRM-6 Mark the minority | R12 |
| FRM-7 No native controls | A native_controls_lint |
| FRM-8 Autocorrect off | A text_input_lint |
| FRM-9 Choose over type | R3 |
| FRM-10 Field look | A visual regression; R14 |
| TGL-1 Toggle vs check | R10 |
| TGL-2 Icon chip | N6.3 |
| TGL-3 Grouped | R6 |
| TGL-4 Reason or hidden | N9 |
| TGL-5 Help says consequence | R12 |
| RC-1 When cards | N6.1 |
| RC-2 Logo or icon | N6.2 |
| RC-3 One accent Recommended | N12 (tone check: "Recommended" uses accent); R7 |
| RC-4 Sub-line says what happens | R12 |
| RC-5 Whole card, keys | A kit-controls.test |
| RC-6 Blocked cards | N9; unit test of `MoveChoice` blocked states |
| TAG-1 Short, no wrap | N12, N15 |
| TAG-2 ≤ 2 per row | N12 |
| TAG-3 Not colour alone | N7 |
| TAG-4 Status vs labels | R7 |
| TAG-5 Same state, same tag | N12 (one component); R14 |
| ALR-1 Three levels | A alert.test; N7; R7 |
| ALR-2 Inline placement | R6 |
| ALR-3 No page-top banners | A banner-band.test; R6 |
| ALR-4 Clearing action | hand walk (click it, does it clear?); R11 |
| ALR-5 ≤ 1 per section | N5 pass counts `[data-level]` Alerts per section; R6 |
| ALR-6 Live roles | A alert.test |
| ALR-7 No dead ends | R10 |
| SEC-1 Heading per area, ≤ 5 areas | N5.3; R6 |
| SEC-2 Help once, no glossary | N5.2; R5 |
| SEC-3 Folds show values | R3 |
| SEC-4 Keep collapsibles | A attention-budget (vii) |
| LST-1 Reuse row | R14 (side by side with the session row) |
| LST-2 One colour per row | A one-mark-per-app.test |
| LST-3 No repeated words | extend plain-words-on-screen.test: the same phrase on every row of a list fails (PRIN §5) |
| LST-4 Hover actions, keyboard | A keyboard-flows; R15 |
| LST-5 Selection vs focus | A attention-budget (viii), contrast.spec |
| LST-6 Row names | A axe; R15 |
| LST-7 Names are doors | R11 (click every name) |
| CRD-1 Card size and grid | R6 |
| CRD-2 No card in card | R6 |
| CRD-3 Borders by mode | N20 + computed border check in N6/attention-budget; R7 |
| CRD-4 Stat tiles | N14 |
| PCK-1 Suggested then All | A PickerKind type, picker-groups.test |
| PCK-2 Search past 8 | A picker-groups.test |
| PCK-3 Empty last | A picker-groups.test |
| PCK-4 Greyed only if clearable | N9; R12 |
| PCK-5 Useful default | R3 |
| EMP-1 One state per area | N14 |
| EMP-2 Never flash empty | A load spec (first frame); R13 |
| EMP-3 Empty action primary only if alone | N4 |
| EMP-4 No filters on empty | N14 |
| EMP-5 Plain words | A plain_words_lint |
| EMP-6 Unknown never 0 | N14 |
| TST-1 User actions only | N18 |
| TST-2 Never the only place | R10 |
| TST-3 6 s, errors stay | A toast tests; N18 |
| TST-4 Undo | R10 |
| TST-5 Avoid footers | A toast-narrow.spec |
| TST-6 Announced | A toast tests |
| MNU-1 Icons | N8 |
| MNU-2 Order, danger last | R14 |
| MNU-3 Also a command | A palette-commands.test; R15 |
| MNU-4 Keys | A menus-a11y.spec |
| MNU-5 Hint grid | A visual regression; R14 |
| PAT-SER-1…6 | N11; R10 |
| PAT-DES-1 Undo over confirm | R10 |
| PAT-DES-2 Says what remains | R12 |
| PAT-DES-3 Danger not default | N10 |
| PAT-DES-4 Least destructive | R10 |
| PAT-DES-5 Type to confirm only for bulk | R10 |
| PAT-DES-6 Toast and record | R10 |
| PAT-PD-1…5 | R2, R3; A attention-budget |
| PAT-HIDE-1…3 | N9; A release_scope (gates); R12 |
| PAT-HIDE-4 Gates watched | A release_scope.py |
| PAT-INP-1…3 | R11 |
| PAT-LST-1 Cache first, no jump | A load spec; R13 |
| PAT-LST-2 Paged | N17; A load-gate |
| PAT-LST-3 Live first, search, filters | R2 |
| PAT-LST-4 Stale said | R12 |
| PAT-LST-5 Feed chronological | A home-feed tests |
| PAT-ATT-1…5 | A attention-budget; N20 extends it to every calm family |
| PAT-DOOR-1…3 | R11; A actions-by-id tests where they exist |
| PAT-SW-1…5 | N21; walk step "from any session page, the switch is one visible click"; R2, R9 |
| CNT-1 Plain words | A plain_words_lint, plain-words-on-screen.test |
| CNT-2 One word per thing | A terminology_lint, one-vocabulary.test |
| CNT-3 Sentence case | N13 |
| CNT-4 Short labels | N12; R12 |
| CNT-5 Say it once | LST-3 extension; R5 |
| CNT-6 Name the outcome | R9 |
| CNT-7 Honest numbers | N14; R12 |
| CNT-8 Tone | R12 |
| CNT-9 Generic examples | A private_data_lint, the denylist hook, CI pattern check |
| CNT-10 Numbers and time | R8, R12 |
| A11Y-1…10 | A a11y gate (axe, keyboard, zoom, motion, contrast, glyphs); N7; N9; R15 |

---

## 5. The screen-review checklist (Opus)

**When:** every new or changed screen, dialog, menu or popover, before baselines; the result goes into the family's `critique.md` beside its captures (D-57), then `--stamp` after staging.

**Inputs:** captures at 1024×680 and 1280×800, dark and light (calm dashboards also at 1680 and 2000); the matching mock-up from `~/Dev/AIJuggler-design-refs/` side by side at full size; the screen's mode from the registry; Sonnet's draft critique and hand-walk scores; N19's colour-vision and blur copies.

**Score each line 1–5. Ship only at ≥ 4 on every line.** A line that doesn't apply says "n/a" with why.

| # | Question | 1 = | 5 = |
|---|---|---|---|
| 1 | **Job.** Why would the user open this? Does it do that job end to end (every promise kept, not just a navigation)? | works but useless | the job done in one place |
| 2 | **Prominence and prime space.** Does the visual order match frequency? Is anything niche on prime space? (PR-5, PR-12, PAT-LST-3) | rare things first or loudest | the common path is the most prominent thing |
| 3 | **Default and folding.** Is the likely choice made, stated with a short why, accepted by Return? Is the rare folded behind a fold that shows its values? (PAT-PD) | user assembles it | one line, one key |
| 4 | **Mode.** Is the screen in its registry mode, with the dials of §2.2? Calm: crafted, not plain. Rich: icons, logos, cards, preview, summary, good use of space. (MD, D-126) | wrong mode or an extreme | the dials exactly as specified |
| 5 | **Matches the mock-up.** Every deviation listed with its reason; nothing specced is missing (wireframes, deep/11, SPEC §9). (M:design-refs-build-to-mockups, M:read-specs-and-wireframes) | drifted | matches, deviations reasoned |
| 6 | **Areas and spacing.** ≤ 5 areas, surface steps, more space between than inside, no box in a box, one alignment grid. (SPC, SEC-1, CRD-2) | one grey band | areas obvious when blurred |
| 7 | **Colour.** Status hue only for L1–L3, amber only where the user must act, accent only for action, selection, progress, preview; App colour once per row. Holds in grayscale and colour-vision copies. (COL, TAG) | Christmas tree or monochrome | each colour means one thing |
| 8 | **Type and hierarchy.** Heading, label and value distinct by size, weight, colour; body in ink-2; ≤ 3 sizes; nothing under 12 px; tabular numbers. (TYP, COL-2/3) | all alike | values loudest, labels quietest |
| 9 | **Actions.** Exactly one filled primary naming its outcome, last, Return; footer order; no info-only modal; wizard Next names its step; review step for 4+ steps. (BTN, WIZ, DLG-9) | 0 or 2+ primaries, "OK" | one, named, last |
| 10 | **Serious and destructive choices.** Consequence, example, "What does this mean?", guide link; undo over confirm; danger never default; no dead ends. (PAT-SER, PAT-DES, ALR-7) | bare choice | explained in place |
| 11 | **Doors and in-place.** Every name opens its page; related things linked both ways; editing happens where the thing is shown; alerts' actions really clear them. (PAT-DOOR, PAT-INP, ALR-4) | dead names | every name a door |
| 12 | **Words.** Plain words, one meaning per entity, sentence case, short labels, said once, honest numbers, errors that say how to fix, no "not proven" text, hidden what can't be used. (CNT, PAT-HIDE) | jargon and repeats | clean |
| 13 | **States and speed.** Skeleton or cache on the first frame; one empty state per area; unknown said once; stale said; every control responds within 100 ms; tried with real-sized data. (EMP, PAT-LST, M:test-by-hand) | flashes, filler walls, slow | as the rule |
| 14 | **Consistency.** Uses the kit and closest existing pattern; icons from the set, one meaning each, on every menu line; radii, shadows and motion from tokens; side by side with the app's best screens (the row menus, the rail's order menu, Usage, Processes, the pool wizard). (PR-11, ICN, M:ui-consistency) | hand-made look | indistinguishable from the best screens |
| 15 | **Accessibility by hand.** Keyboard through every control; focus visible and returned; names make sense read aloud; 200% zoom; Reduce Motion; Increase Contrast. (A11Y) | traps, unnamed | smooth |
| 16 | **Hand walk.** Every element clicked in the real Dev app or the preview, every path (success, refusal, error, empty, many), at 1024 and 1280, both themes; each scored works / speed / clarity / layout. (M:test-by-hand) | not walked | walked, each element ≥ 4 |

**Dialog add-on (every dialog, sheet and window sheet):** header and footer fixed; the body alone scrolls; the choices visible on open at 1024×680; the primary visible at every scroll position (scroll it and look); divider and fade when it scrolls; focus in on open and back on close; Escape behaviour honest during long actions. N1 measures these; the reviewer still looks.

**Writing the critique.** Each capture gets what changed and what was judged, concretely ("the How cards moved above the summary; the summary is now four icon rows"), not "looks good" (M:screenshot-notes-say-what-changed). Every finding is fixed before the screen is called done; "unchanged" and "accessible" are not "good" (QUAL D-57).

**Model tiering.** Sonnet captures, runs walks and persona testers, and drafts the critique and scores. Haiku counts lint findings and formats the tables. Opus scores the checklist, verifies every PASS or FAIL Sonnet reported before it is recorded, and approves baselines (M:model-tiering, M:screenshot-approval-delegated, M:happo-approval-mine).
