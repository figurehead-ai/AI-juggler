# AI Juggler Design System (2026-11-21, D-142)

This is the single standard for how AI Juggler looks, reads and behaves. Every new feature, every existing screen and every idea follows it, and every screen is validated against it (`VALIDATION.md`). Where the app breaks it today is in `GAPS.md`.

It combines what is already decided (the decision log, the maintainer's recorded rules, `docs/design/PRINCIPLES.md`, the deep UX spec, SPEC §9, the calm and rich design references) with what the best design systems do (Apple HIG, Primer, Carbon, Fluent 2, Atlassian, Polaris, Material 3, Linear, Raycast). It builds on the tokens and components that exist in `ui/src`. It invents as little as possible.

**How to read a rule.** Each rule has an ID (for example `DLG-3`) so a review or a test can name it. Each is short and testable. The bracket after it says where it comes from.

**Source keys.**
- `D-nn`: `docs/review/decision-log.md`.
- `M:name`: the maintainer's recorded rule (memory note `name`).
- `PRIN §n`: `docs/design/PRINCIPLES.md`.
- `DEEP §n`: `docs/review/deep/11-ux-redesign-screen-spec.md`.
- `SPEC §9`: `docs/SPEC.md` §9. `QUAL`: `docs/plan/QUALITY.md`.
- `CALM`: the approved calm design (`calm-ui/proposal.md`, D-117). `KIT`: the approved D-126 making-and-editing kit (`d126-rich-ui/`). `AUDIT`: `docs/research/screen-audit.md`.
- External: `HIG` (Apple Human Interface Guidelines), `Primer`, `Carbon`, `Fluent`, `Atlassian`, `Polaris`, `M3`, `Linear`, `Raycast`, `WCAG 2.2`, `APCA`. URLs are in the appendix.
- `new`: introduced by this document, binding since D-142 (2026-11-21).

**Precedence.** A D-number beats this document. This document beats `PRINCIPLES.md`, where the two disagree (see `GAPS.md` §B). A change to a rule here is made by a D-number, then this file is updated the same day.

**Paths** are relative to `ui/src/` unless they start with `docs/`, `scripts/` or `crates/`.

---

## Contents

1. Principles
2. The two modes
3. Foundations: colour, type, spacing, radii, elevation, icons, motion, density and layout
4. Components
5. Patterns
6. Content rules
7. Accessibility
8. Worked example: the Move or Copy dialog
9. Appendix: external sources

---

## 1. Principles

| ID | Principle | Source |
|---|---|---|
| PR-1 | **One design language.** The same tokens, type, icons, radii, controls and craft on every screen. Only the amount of status and accent colour changes with the screen's job. A wizard next to Home reads as the same product. | D-126 (clarified), M:calm-attention-design |
| PR-2 | **Calm is not plain.** Calm limits alarm colour and noise, never design quality. Monitoring screens still have a real type scale, hierarchy, surfaces and spacing. | D-126 §2, M:readability-and-visuals |
| PR-3 | **Easy to read at a glance.** Heading, label, value and area are told apart in under a second, by size, weight, colour and surface. Never one grey band. | M:readability-and-visuals, PRIN §6–8 |
| PR-4 | **Where is what, one click to it.** Every screen answers what is open, where, on which App and Account, with what state, and gives one click to get there. Handle it here; Go to it is optional. | M:desktop-first-navigation, D-36 |
| PR-5 | **Prominence follows frequency.** The common choice is made for you, stated and biggest. Rare things are smaller, later or folded. | PRIN §2–4, M:ux-progressive-disclosure |
| PR-6 | **Colour means something.** Status colour is reserved for state (L1–L3). The accent is for action, selection, progress and preview. Identity colour belongs to Apps. Nothing is coloured for decoration. | D-117, D-69, D-126, ISA-18.2 (CALM) |
| PR-7 | **Honest and usable.** Show only what works on this setup. Say unknown once, never fake a number. Every alert carries its clearing action. | D-127, DEEP §0 and §1.1, M:accuracy-means-what-it-claims |
| PR-8 | **Serious choices are explained in place.** Consequence, example, "What does this mean?", and a docs link. | M:serious-choices-explained |
| PR-9 | **Instant.** The first frame shows a skeleton or cached data. Lists are paged. Feedback within 100 ms. | M:instant-and-paged, M:lazy-load-everywhere, M:test-by-hand |
| PR-10 | **Keyboard-first and accessible.** Every daily action is reachable in 3 keys or fewer. AA contrast in both themes. Never colour alone. | DEEP §1.3, D-16, WCAG 1.4.1 |
| PR-11 | **Built once, reused.** New UI reuses the closest existing component and pattern. A screen never hand-makes what the kit has. | M:ui-consistency, M:architecture-standard |
| PR-12 | **Prime space is for the common jobs.** Home, the rail, page headers and cards serve what most users do: what's open, what needs me, runway, switching. Niche features sit behind their object. | M:prime-ui-space, D-134 |

---

## 2. The two modes

There is one design language with two settings of the same dials (PR-1). The deciding question: **is the user watching, or deciding?**

### 2.1 What stays the same in both modes

Tokens, the type scale, icons and their meanings, radii, buttons, the one filled primary per view, focus rings, status colours and what they mean, content rules, accessibility. (D-126, KIT "Making vs monitoring")

### 2.2 The dials

| Dial | Calm (monitoring) | Rich (making and editing) | Source |
|---|---|---|---|
| Status colour | Only L2 and L3 are loud. Each need is loud once per area. | The same. Status colour still means status. | D-117 |
| Accent | The one primary button, links, keyboard focus. | Plus selection (the chosen card's accent ring), progress (the current step), and previews. | D-126 §1, `controls/selected.ts` |
| Selection look | Neutral: `SELECTED` / `SELECTED_FILL` (raised fill, `--color-selected-ring` edge, semibold ink). | Choice cards: `CHOSEN_CARD` (accent ring). Segments, tabs and rows keep `SELECTED`. | D-117 point 2, `controls/selected.ts` |
| App icons | Muted at rest (`.app-tone`), full colour at L2/L3, when selected, hovered or focused. 6 px App dot on rows. | Full colour, 32–40 px on choices (`ToolLogo`). | D-117, KIT |
| Icons | App icons stay. Other row icons show on hover, focus or selection. Section labels have no icons. | An icon or logo on every choice, row and section that holds choices. | CALM §b, D-126 §1 |
| Containers | Surface tone separates. No card borders, no row dividers. | Surface tone for areas. Choice cards and inputs keep their edge (they are controls). A tinted summary panel for consequences. | CALM §c2, KIT |
| Secondary buttons | Ghost (`.btn-ghost`). | Bordered secondary (`Button variant="secondary"`) beside the primary, ghost for Cancel. | CALM §c2, KIT, `FlowFoot` |
| Illustrations | None. | One small line illustration or a live preview per step, where it explains the choice. | D-126 §1, KIT |
| Space | Information-dense, top N plus "See all". | Generous: form to 680 px, preview 320–380 px, no narrow column on an empty canvas. | D-126 §1, `FlowLayout.tsx` |
| Motion | None at rest. One 600 ms fade when a need arrives. | Transitions on choice and step change (150 ms). | D-117, CALM §a.6 |

### 2.3 Which screens take which mode

Classification from the 0.5 screen audit (AUDIT §1), with D-126's list.

**Calm (monitoring):**
- Home: Needs you, the feed, Your apps, Runway (D-118).
- The rail in every density, its sections, View menu results, the folded rail.
- Open now, session lists, History, Archived, the Tasks board.
- Session page tabs: Overview, Usage, Quality, Processes, Subagents, the conversation.
- Account page, App page, Accounts list, Projects list, Project page (its overview).
- Usage, burn-down, cost, This Mac, Processes table, Fleet board, the Floor and its list.
- A running pool's canvas. The notification centre, the bell, the menu-bar menu, toasts.
- The command palette and Go to.

**Rich (making and editing):**
- Every wizard: the pool wizard, Switch (target, what's carried, confirm), Add a machine, first run.
- Create and add: New session, New project, Add an app, Add a fresh app, Add an API-key account, Log in.
- Edit: Account settings and renewal, launch settings and keep-alive, Copy settings into another account, Monthly caps, project sync, Watches (create and edit).
- Every Settings pane, including Settings › AI › Models.
- Decision dialogs: Hit its limit, identity mismatch, isolation offer, worktree offers, switch set-up offers, the folder trust ask, update available.
- Empty states that invite a first action (Home's first session, an empty Pools or Watches list).
- A pool still being set up (its canvas in setup state).
- Destructive confirmations take the **rich-compact** form: rich parts (icon, the consequence panel, tags) at confirm size (DLG-2, PAT-DES).

**Mixed.** A monitoring screen with one editing panel (the rail's View menu, the pool canvas inspector, the Pools list with Move to pool). The rule: **mode is decided per region.** The panel where the user edits is rich; the surface around it stays calm. (AUDIT §1, new)

**MD-1.** Every screen and dialog is listed in the mode registry with its mode (`VALIDATION.md` N20). A new screen declares its mode in its task before it is built. (new)

**MD-2.** Correct a screen by adjusting a dial, never by flipping it to the other mode's extreme. (D-126 clarified: "you keep swinging from one extreme to totally other side")

---

## 3. Foundations

All values below are the real tokens in `styles.css` (`@theme` and `:root`), unless marked **gap**. Token names never change; only values do (D-16).

### 3.1 Colour roles

**Surfaces (a ladder, each a visible step apart, 1.11–1.16:1):**

| Token | Dark | Light | Job |
|---|---|---|---|
| `--color-canvas` / `-canvas-hi` | `#0b0e14` / `#10141b` | `#eceef3` / `#e4e7ee` | The window. A flat canvas-hi to canvas step. |
| `--color-surface` | `#151a23` | `#ffffff` | An area or a card. |
| `--color-surface-2` | `#1e2430` | `#f5f6f9` | Inset in an area: an input, a hovered row, an info Alert, a summary panel. |
| `--color-surface-3` | `#242935` | `#e6e9ef` | A chip, a popover's hover, a ghost button's hover. |
| `--color-fill` | `#181d27` | `#ffffff` | A filled control in place of its outline (search, segments). |
| `--color-selected` | `#242935` | `#e6e9ef` | What is selected, chosen or current. |
| `--color-skeleton` | `#242935` | `#ced3dd` | Loading placeholders. |

**Text (four levels plus the section label):**

| Token | Dark | Light | Job | Never |
|---|---|---|---|---|
| `--color-ink` | `#f3f5f9` | `#0e1219` | Headings, names, values, a chosen item's words. | — |
| `--color-ink-2` | `#d2d8e1` | `#2c3341` | Body sentences, a row's second line. | — |
| `--color-section-label` | `#c3cad5` | `#3d4553` | Group labels in small capitals (Needs you, Your apps). | Body text. |
| `--color-muted` | `#b0b8c5` | `#424958` (D-142; was `#4f5767`) | Labels, column heads, captions, meta, help (two lines at most). | Body sentences. |
| `--color-faint` | `#8e97a6` | `#5f6777` | Placeholders, disabled words, tertiary times. | Anything the user must read to act. |

**Accent:**

| Token | Dark | Light | Job |
|---|---|---|---|
| `--color-accent-fill` | `#5b5bea` | `#4f4fd8` | The one filled primary button (white on it ≥ 5.09:1); the current step's dot. |
| `--color-accent` | `#8787f6` | `#4747d1` | Focus ring, accent icons, the chosen card's ring. |
| `--color-accent-hi` | `#a9a9ff` | `#4646e0` | Links and accent text ("Learn more", "Check again"). |
| `--color-accent-soft` | `#7c7cf41f` | `#5b5bf016` | Tinted surfaces: the stated choice, a done step, the accent tag, an icon chip that is on. |

**Status (the attention levels, `lib/attention.ts`):**

| Level | Token | Meaning | Look |
|---|---|---|---|
| L3 Broken | `--level-3` = `--color-danger`, `--color-danger-soft` | Work stopped or will be lost; only the user can recover it. | Red word, ✕/⊘ glyph, at most one tinted block per area. |
| L2 Needs you | `--level-2` = `--color-warning` | Work waits on a decision; nothing is lost while it waits. | Amber word and 6 px dot (hollow ring when "probably"). No amber fills. |
| L1 Note | `--level-1-done` = `--color-success`, `--level-1-live` = `--color-live` | Good to know; no action now. | A quiet glyph in its hue; words stay neutral. |
| L0 Calm | the text levels | The normal running picture. | No hue. |

**Identity:** each App has one colour (its profile badge and dot, `lib/mark-hues`), given once and the same on every screen. Accounts beside an App are neutral. Other groupings use icons, marks and tags, never a second colour. (D-69, SPEC §9.3 AJ-E3206)

**Lines:** `--color-hairline` (rules inside an area, input edges), `--color-divider` (the one rail/main split in calm screens, the FlowLayout step column and foot).

**Rules:**
- **COL-1.** Colours come only from tokens. No raw hex in components; the terminal palette is the one allowlisted exception. (D-16, QUAL)
- **COL-2.** Body sentences use `ink-2`. `muted` is for labels, meta and help of at most two lines. (PRIN §7)
- **COL-3.** Each area has exactly one `ink` heading. Values are `ink` and the loudest text in their row. Labels are never louder than their values. (PRIN §8)
- **COL-4.** Status hues appear only where `attentionLevel()` says L1–L3. A plain fact is never amber. (D-117, D-126 KIT "Remove amber from plain facts")
- **COL-5.** Amber is never a fill. Red tint: at most one block per area. (D-117 point 1)
- **COL-6.** One filled accent button per view (BTN-1). The accent never marks a status.
- **COL-7.** Every text token clears 4.5:1 on every surface it can sit on, in both themes. `ink-2` and `muted` also clear APCA Lc 60 in dark. (AJ-E804, PRIN §7, `tests/visual/contrast.spec.ts`)
- **COL-8.** `muted` text reaches 7:1 on every surface (the maintainer's decided scale). Dark passes (≥ 7.28:1). Light `muted` moves to `#424958` (7.29–9.03:1) in 0.5.0. (M:readability-and-visuals; D-142, `GAPS.md` B1)
- **COL-9.** Increase Contrast (`prefers-contrast: more`) raises the divider to hairline, the hairline to faint, the section label to ink-2, and gives ghost buttons an edge. Already in `styles.css`. (D-117 point 4)
- **COL-10.** Charts keep categorical colours (that is data, not status), the same colour per Account everywhere, and each has a table fallback. (CALM §b, M:readability-and-visuals, ADR-0021)

### 3.2 Type

The readability scale (`styles.css` `--text-*`, classes in `@layer components`). Native system font stack; mono for identifiers. One size per role. **Nothing under 12 px**, chips, captions and chart axes included.

| Role | Token / class | Size / line / weight | Colour | Use |
|---|---|---|---|---|
| Page or step title | `.type-title` (`--text-title`) | 22 / 1.25 / 600, −0.01em | ink | One per page, step or full-window sheet. |
| Section heading | `.type-section` (`--text-section`) | 15 / 1.35 / 600 | ink | An area's heading, a dialog's title. Sentence case. |
| Group label | `.section-label` | 12 / 1.4 / 500, capitals, 0.07em | section-label | A label over a group of rows or steps. Never the only heading of a form section. |
| Value or name | `text-body font-semibold` | 14 / 1.5 / 600 | ink | A session's title, "58% left". |
| Body | `text-body` | 14 / 1.5 / 400 | ink-2 | Sentences, table cells. |
| Help | `.type-help` (`--text-meta`) | 13 / 1.45 / 400 | muted | One line under a heading or field; two at most. |
| Meta | `text-meta` | 13 / 1.45 | muted | Second lines, column heads. |
| Label | `text-label` | 12 / 1.4 / 500 | muted | Field labels, tags, captions, axes. |
| Stat | `--text-stat` | 28 / 1.1 / 600, tabular | ink | A big number in a stat tile. |
| Mono | `font-mono` | at the size of its role | — | Paths, ids, commands (shown only where the user needs them). |

- **TYP-1.** Use the role classes and `--text-*` sizes. No Tailwind default sizes (`text-xs`, `text-sm`) or arbitrary `text-[Npx]` in new code. (readability scale, `styles.css` comment; new lint N3)
- **TYP-2.** Nothing under 12 px. (M:readability-and-visuals, D-57)
- **TYP-3.** At most three sizes in one view besides a stat; weight and colour do the rest. (PRIN §7, NN/g)
- **TYP-4.** Numbers that change or line up use `tabular-nums`. (DEEP §1.2)
- **TYP-5.** Times show relative with the absolute on hover and focus ("resets 17:00 · in 2h 14m"). (DEEP §1.2)
- **TYP-6.** Weights are 400, 500 and 600 only. No light or thin weights. (HIG Typography)
- **TYP-7.** Rich screens use `.type-section` sentence-case headings for every section that holds choices; `.section-label` capitals only label groups inside it (KIT: "YOUR APPS" over cards under the step's title). (M:readability-and-visuals, D-117, KIT)

Why 14 px body and not macOS's 13 pt: the maintainer's decided scale (2026-10-25) after "most of the text … very hard to read". Apple's 13 pt is the system default for native controls; this app reads at 14.

### 3.3 Spacing

Tailwind's 4 px scale is the spacing scale. Named tokens: `--spacing-card` (20 px card padding) and `--spacing-grid` (16 px between cards).

| Step | px | Use |
|---|---|---|
| 1 | 4 | Icon to its label inside a tag; a field label to its control (6 at most). |
| 2 | 8 | Between rows in a group, between buttons in a footer. |
| 3 | 12 | Between controls in a dialog section, an icon chip to its words. |
| 4 | 16 | Between cards, between sections inside a dialog, dialog side padding on small dialogs. |
| 5 | 20 | Card padding; dialog padding on M and larger (24 in the header and footer). |
| 6 | 24–32 | Between areas of a page. |
| 7 | 40 | FlowLayout form-to-preview gap. |

- **SPC-1.** The space between areas (24–32) is at least twice the space inside them (8–12). (PRIN §6, Fluent Layout)
- **SPC-2.** No arbitrary spacing values (`py-[9px]`) in new code; use the scale. (new; lint N3)
- **SPC-3.** Group by spacing first; a hairline only where spacing can't do it; a box only for a real area. No box inside a box of the same tone. (PRIN §6, CALM §c2, Carbon)

### 3.4 Radii

Today there is one token (`--radius-card`, 14 px) and ten radii in use (AUDIT; `GAPS.md` A9). The system has four roles:

| Role | Value | Today | Use |
|---|---|---|---|
| Control | 8 px | `rounded-lg` | Buttons (sm), segments, menu items, icon chips (30 px chip), tags with words that wrap. |
| Field and row | 10–12 px | `rounded-[10px]`, `rounded-xl` | Inputs, md buttons, list rows, ToggleRows, steps, Alerts. **Gap:** add `--radius-control: 8px` and `--radius-row: 10px`, and use `rounded-xl` (12) for inputs and md buttons only. |
| Card and dialog | 14 px | `--radius-card` | Cards, choice cards, dialogs, popovers. |
| Pill | full | `rounded-full` | Tags, counts, dots, avatars, step dots. |

- **RAD-1.** Only these four. No `rounded-[Npx]` except through the tokens. (new; lint N3)
- **RAD-2.** A nested shape's radius is smaller than its container's (card 14, control inside 8–10). (new, Raycast's one rule set for corners)

### 3.5 Elevation and layers

| Level | What | Look |
|---|---|---|
| 0 | Canvas | `canvas-hi` → `canvas` gradient. |
| 1 | Areas, cards | `surface`. Calm: no border, no shadow. Rich: may keep `--shadow-card` on a raised choice. |
| 2 | Menus, popovers, pickers, tips | `surface` with a hairline and a popover shadow. **Gap:** no `--shadow-popover` token. |
| 3 | Dialogs and sheets | `surface`, `--radius-card`, a dialog shadow over a dimmed, blurred canvas (`bg-canvas/70 backdrop-blur-sm`). **Gap:** the shadow is a raw value in `form.tsx`; add `--shadow-dialog`. |
| 4 | Toasts | Above everything but never over a dialog's footer (`data-toast-avoid`). |

- **ELV-1.** A z-index scale: base 0, sticky heads 20, rail overlays 30, popovers 40, dialogs 50, toasts 60. **Gap:** dialogs and toasts are both `z-50` today. (new)
- **ELV-2.** Depth comes from the surface ladder, not from glows. `--shadow-glow-*` is for the primary button only; the `.numeral` gradient and `.sheen` are retired from new work. (DEEP §2.1 "glow = live/primary only", D-126 KIT "Neither: gradients")

### 3.6 Icons

**The set.** AI Juggler's own inline SVG icons (`components/icons*.tsx`, about 156): a 24 grid, 1.75 stroke, round caps and joins, lucide-style. Not SF Symbols: the UI is a web view and ships on Windows and Linux too (D-52); the set is drawn to sit beside SF Symbols' regular weight. App and tool icons are the installed apps' own, read at run time, never bundled (D-38, M:app-icons-at-runtime); a CLI shows its vendor's desktop icon with a terminal badge at 24 px and up (KIT ruling).

| Size | Use |
|---|---|
| 12 | A glyph inside a tag or a step dot (✓), a level glyph. |
| 16 | Rows, buttons, menu items, Alerts, field adornments, icon chips' glyph (in a 30 px chip). |
| 18 | Top-bar and rail chrome (the set's default). |
| 20–24 | Section or dialog header icon tile (in a 36–40 px tile), empty states in calm screens. |
| 24 / 32 / 40 | `ToolLogo`: 24 in rows, 32 in cards, 40 on choices (KIT). |
| 40–48 | Empty-state icon in rich screens. |

- **ICN-1.** One meaning per icon, app-wide. The meaning registry is the icons file's names (`IconTrash` = move to Trash, `IconRemove` = remove from a list). A new meaning gets a new icon. (Carbon, Primer, HIG "use color consistently"; new registry check N19)
- **ICN-2.** ✕ means close in neutral chrome and never stands for "broken". L3 uses the danger glyph ⊘ (or the red ✕ mark only inside a level mark beside its word). **Gap:** `Alert` level `broken` uses `IconClose`. (CALM §b, DEEP §2.4)
- **ICN-3.** Icon-only buttons have an accessible name and a tooltip that shows on hover and on keyboard focus (`WhyTip` pattern, not `title`). Icon-only is banned in primary flows. (DEEP §2.2, Fluent, Carbon)
- **ICN-4.** Every menu line and action line has a 16 px icon (choice lines may show ✓ instead). (D-57, M:desktop-first-navigation)
- **ICN-5.** Status glyphs (✕ ⚠ ○ ● ◐ ✓) are `aria-hidden` and never part of an accessible name; the word carries the meaning. (D-117, a11y gate glyph check)
- **ICN-6.** Profile badges sit beside the app icon, never over it. (M:app-icons-at-runtime)
- **ICN-7.** Status mapping (always icon + word): Working ● static live dot, Asks you ● amber, Probably asks you ○ amber ring, May be stuck? ○ ring with "?", Out of room ⊘, Done ✓ (review it), Failed or stopped with an error ⊘/✕ red, Stopped ■, Handed over ↗. (DEEP §2.4, SPEC §9.3, D-117)

### 3.7 Motion

- **MOT-1.** Durations: 100 ms (hover, press), 150 ms (control state, choice change, icon tone), 250 ms (panel or step change); one 600 ms fade for an arriving need or feed item. Easing `--ease-out-soft`. **Gap:** no duration tokens; add `--duration-fast/base/slow`. (DEEP §2.1–2.2, D-117)
- **MOT-2.** No animation runs for ever. A live dot breathes three times, then rests. (0.4.5 AJ-E805, `idle.spec.ts`, D-117)
- **MOT-3.** Reduce Motion stops every animation and transition (the rule in `styles.css`). (WCAG 2.3.3, D-117)
- **MOT-4.** Motion explains a change (a step slides, a row fades in); it is never decoration. (CALM §a.6)

### 3.8 Density and layout

**Window:** default 1360×860, minimum **1024×680** (`tauri.conf.json`). Every screen is checked at 1024×680 and 1280×800 in both themes; monitoring dashboards are also judged at 1680 and 2000 wide. (M:readability-and-visuals, QUAL AJ-E803)

| Thing | Rule | Source |
|---|---|---|
| Content width | At most 1440 px (`--container-content`), centred; cards sized to content in a responsive grid, never full-width strips. | M:readability-and-visuals |
| Rail | Collapses to icons and badges under 1100 px; densities Comfortable (two lines), Compact (one line), Minimal (icons), ⌃D cycles. | DEEP §3, D-25, `DensityChoice.tsx` |
| Row heights | Comfortable two-line rows 44–48 px; compact rows 32 px; table rows 36 (compact) / 40 (default). | DEEP §2.2, Carbon |
| Control heights | md 36 px (`h-9`), sm 28 px (`h-7`); never under 24 px hit area, 28 px preferred on macOS. | HIG Buttons, WCAG 2.5.8, `styles.css` |
| Making screens | `FlowLayout`: steps column 240 px (280 from 1360), form to 680 px, preview 320–380 px sticky, max 1180 px, left-aligned; under 960 px the preview folds into a summary strip. | D-126, KIT, `FlowLayout.tsx` |
| Dialogs at 1024×680 | Height cap `min(92vh, 760px)` = 625 px: header ≤ 96, footer 56–68, so the body keeps ≥ 460 px. | new (DLG-3) |
| Tables | At most 5 columns at default width; overflow goes to the detail level. | DEEP §1.2 |

- **DEN-1.** No horizontal page scroll at 1024 px. Nothing is cut mid-word; a truncated value shows in full on hover and focus. (`fits-its-room.spec.ts`, `header-room-words.spec.ts`, Primer, HIG)
- **DEN-2.** Comfortable and Compact must look different (row height and second line), never "identical". (M:ux-progressive-disclosure)

---

## 4. Components

Each component lists its anatomy, when to use it, do's and don'ts, accessibility and its code. A screen uses these; it never hand-makes its own (PR-11).

### 4.1 Button

**Code:** `Button` in `components/primitives.tsx` (variants `primary`, `secondary`, `ghost`, `accent`, `danger`, `chosen`; sizes `sm` 28 px, `md` 36 px).

**Anatomy:** optional 16 px leading icon, label (verb first), optional trailing key hint or chevron/↗ for "opens more".

| Variant | Use |
|---|---|
| `primary` | The view's one most likely action. Filled accent. Marks itself `data-primary`. |
| `secondary` | Other actions in rich screens; a second real choice ("Add, sign in later"). |
| `ghost` | Cancel, Back, and every non-primary action in calm screens. |
| `accent` | An app-wide action that is always there (the top bar's New session). Tinted, never filled. |
| `danger` | A destructive action. Never the default. |
| `chosen` | A picked answer among buttons (an ask's answer). Selection, not action. |

- **BTN-1.** Exactly one filled `primary` per view (page, sheet or dialog). The top bar's New session is `accent` and doesn't count. (D-117, D-126, PRIN §9, HIG)
- **BTN-2.** The primary names its outcome: "Move to Personal", "Start in payments-api", "Make pool". Never "OK", "Yes", "Submit", "Continue" on a last step. (PRIN §9, Carbon, HIG Writing)
- **BTN-3.** Footer order (macOS): utility or tertiary on the left; Cancel, Back, secondary, then the primary on the right, last. (HIG Alerts, NN/g, `FlowFoot`)
- **BTN-4.** Return triggers the primary (⌘↩ where Return types text). Escape cancels. A `danger` button is never the default and Return never triggers it. (HIG Buttons, PRIN §9)
- **BTN-5.** A disabled button always shows why, in visible words near it (the footer's reason line, `FlowFoot.reason`) or through `WhyButton`; a bare greyed button is a defect. (D-57, D-127, Atlassian)
- **BTN-6.** A pressed action that needs the backend shows progress at once (label "Moving…" or a spinner in the button) and ignores a second press. (M:test-by-hand, Carbon)
- **BTN-7.** No hand-made button classes. **Gap:** `switch/SwitchDialogFrame.tsx` exports its own `BUTTON/SECONDARY/PRIMARY`, which `primariesIn()` can't see. (PR-11)
- **BTN-8.** Sentence case labels; an ellipsis (…) when the button opens more input before acting ("Switch account…"). (HIG Writing, content rules)

### 4.2 Dialog and sheet

**Code today:** `Modal` (`components/form.tsx`, 30 callers), `SwitchDialogFrame` (`components/switch/`), `FlowLayout` in a full-window sheet (`.window-sheet`, the pool wizard), and 35 hand-made overlays (`GAPS.md` A1–A2). **Target:** one `DialogFrame` with the anatomy below, which `Modal`, `SwitchDialogFrame` and every overlay use.

**Size classes:**

| Class | Width | Use |
|---|---|---|
| S (confirm) | 440 px | A yes/no decision, a destructive confirm. |
| M (form) | 560 px; 680 px for a decision with cards | A short form, a decision dialog (Switch, Hit its limit). |
| L (wide) | 768 px | Two columns of rows (Keyboard shortcuts). |
| XL (table) | up to 1024 px | A table of many columns. |
| Window sheet | the whole window | A flow of 4 or more steps or one with a live preview (`FlowLayout`). |

**Anatomy (top to bottom, a flex column):**
1. **Header, fixed.** A 36–40 px icon tile (rich) or none (confirm); the title (`.type-section`, 15 px, ink); one subtitle line (`.type-help`) naming the object ("payments-api · Claude app · Work"); step marks when it is a wizard; the close × (labelled "Close", Esc).
2. **Body, the only part that scrolls.** `min-h-0 flex-1 overflow-y-auto`, the `.scroll-edge` top shadow when scrolled and the bottom fade while more is below (`data-more`). An Alert about the whole dialog sits at the top of the body.
3. **Footer, fixed.** A hairline above, `surface` (or `surface-2`) behind; reason or note line; buttons per BTN-3.

**Rules:**
- **DLG-1.** Header and footer never scroll. Only the body scrolls, vertically, never sideways. (Carbon, Fluent, Atlassian, Primer; M2 "actions always stay in place")
- **DLG-2.** Every dialog is capped at `max-height: min(92vh, 760px)` inside a 16 px viewport gutter, and its overlay never centres content that is taller than the viewport. The primary action is visible at every scroll position, at 1024×680. (the 2026-11-21 report; new. **Gap:** `Modal` has no cap and centres with `items-center`, so a tall body runs above the top edge, out of reach, and the buttons fall below the bottom)
- **DLG-3.** The choices the dialog exists for are visible on open at 1024×680 without scrolling. Put them first in the body, details after. (new, PRIN §2)
- **DLG-4.** A footer divider and the bottom fade show whenever the body can scroll. (Primer, Carbon)
- **DLG-5.** Too much to fit: step up one size class, then move to a window sheet. Never more than two body heights of scrolling. (Carbon)
- **DLG-6.** One dialog at a time. A confirm may open over a sheet (two levels at most); never a third. (HIG Sheets, Primer)
- **DLG-7.** Focus moves into the dialog on open (the first `[data-autofocus]`, else the step heading, else the first control), Tab stays inside, Escape closes (except while an action that can't be stopped runs, which says so), and focus returns to what opened it. `role="dialog"`, `aria-modal`, named by its title. (DEEP §2.5, AJ-E1503)
- **DLG-8.** An invalid entry keeps the dialog open, marks the field and says how to fix it. A backdrop click doesn't close a dialog with unsaved input; closing one asks first. (Carbon, Primer, HIG Sheets)
- **DLG-9.** No modal for information only. Modals are for wizards, decisions the user asked for, and destructive confirms. (DEEP §3 disclosure rules, HIG Alerts)
- **DLG-10.** While a long action runs inside the dialog, the body shows determinate progress (steps done, the current one) and the footer is replaced by its one honest option ("Runs in the background" or none, never a dead Cancel). (HIG Progress, `MoveChoice` phases)

### 4.3 Wizard and FlowSteps

**Code:** `FlowLayout`, `FlowSteps`, `FlowFoot` (`components/FlowLayout.tsx`); `StepMarks` (switch dialog).

| Steps | Form |
|---|---|
| 1–2 | No step indicator; the header says "Step 1 of 2" if needed. |
| 3, in a dialog | `StepMarks` in the header: "Step 2 of 3: What's carried", then the steps as pills (done ✓, current selected, to-do muted). |
| 4 or more, or needs a preview | Window sheet with `FlowLayout`: `FlowSteps` beside (grouped, each step's value under its name), the form, the live preview, `FlowFoot`. |

- **WIZ-1.** Every step shows its value under its name once set ("Where · web"); done steps can be revisited; the current one is marked in the accent with `aria-current="step"`. (KIT, `FlowSteps`)
- **WIZ-2.** Back never loses input. Cancel on the first step, Back from the second; Cancel stays reachable on every step. (HIG multi-step sheets, Carbon)
- **WIZ-3.** Next names where it goes ("Next: Who leads"); the last primary names the outcome. When the rest is prefilled, offer finishing early as a secondary ("Make pool now") with a note ("The rest is prefilled"). (KIT who-dark, BTN-2)
- **WIZ-4.** A flow of 4 or more steps ends on a review step that looks like the result (the preview at full size, every value with a Change link). (KIT review, Carbon "summary")
- **WIZ-5.** A new step opens scrolled to its top with focus on its heading. (`FlowLayout` `at`, `useDialogKeys`)
- **WIZ-6.** Prefill everything a default can answer; group prefilled steps ("How they work · prefilled"). Ask only what changes the outcome. (M:serious-choices-explained "ask only when the answer changes what happens", HIG Onboarding)
- **WIZ-7.** Validate a step when the user presses Next; keep Next enabled for typed input and explain on press; for a choice step with nothing chosen yet, Next may wait, with its reason shown (BTN-5). (Primer, Atlassian, Carbon; `GAPS.md` B6)
- **WIZ-8.** The preview matches the form on every step, in both themes. (D-126 KIT, BUILD-BRIEF gates)

### 4.4 Form fields

**Code:** `Field`, `Input` (`components/form.tsx`); `DateField`, `TimeField`, `Picker`, `Segmented`, `Checkbox`, `Radio`, `Toggle` (`components/controls/`).

**Anatomy:** label (above) · help (one line, under the label) · control · error (replaces the help, with an icon).

- **FRM-1.** Labels above the control in dialogs, wizards and edit forms; one convention per surface. Settings rows use `ToggleRow`/`ControlRow` with the control at the right. (Carbon, Primer; HIG Settings)
- **FRM-2.** Labels are 1–3 words, sentence case, no colon. A placeholder is never the label; placeholders show format only, in `faint`. (Carbon, Primer, HIG Writing, PRIN §7)
- **FRM-3.** Help text is one line (two at most), never repeats the label, and hides while an error shows. (Primer, Carbon)
- **FRM-4.** Validate a field on blur once touched, live after its first error, and on submit; with 3 or more errors show an Alert summary at the top of the body and focus the first invalid field. (Carbon, Primer, HIG)
- **FRM-5.** Error messages say what happened and how to fix it ("Choose a name with at least 2 letters"), next to the field, without blame or "oops". (HIG Writing, Carbon)
- **FRM-6.** Mark the minority: if most fields are required, mark "(optional)"; required fields are also `aria-required`. (Carbon, Primer)
- **FRM-7.** No system select, checkbox, radio, date box or `<details>`: use the shared controls. (D-57, `native_controls_lint.py`)
- **FRM-8.** Text fields for names, paths, ids and keys have autocorrect, autocapitalise and spellcheck off; prose fields keep them. (`text_input_lint.py`)
- **FRM-9.** Prefer choosing to typing: pickers over free text, a folder picker or drop over a typed path. (HIG Entering data, SPEC §9.1 "never typed paths")
- **FRM-10.** Field look: 36 px high, `surface-2` fill (or `fill` in calm screens), hairline edge, 12 px radius, accent ring on focus. (`Input`, CALM §c2)

### 4.5 ToggleRow and CheckRow

**Code:** `ToggleRow`, `CheckRow`, `RowList` (`components/controls/ControlRow.tsx`).

**Anatomy:** a 30 px icon chip (accent-soft while on) · title (14 px medium ink) · one help line (13 px muted) · the control at the right. The whole row is the label.

- **TGL-1.** `ToggleRow` = a setting that takes effect at once ("Hold notifications at night"). `CheckRow` = an item in a set that is applied when the form is submitted ("Copy settings from Work"). Never a switch for a deferred change, never a checkbox for an immediate one. (KIT, HIG Toggles)
- **TGL-2.** In rich screens every row has an icon chip. A list of bare checkboxes is a defect. (D-126, AUDIT rank 1)
- **TGL-3.** Group rows into a `RowList` card under a `Section` heading; split long lists into meaningful groups ("Needs you" and "Just so you know"). (AUDIT rank 1, PRIN §6)
- **TGL-4.** A row that can't change now is faded with its reason said beside it and as its description; if the user can't clear the reason, the row is hidden (PAT-HIDE). (D-127)
- **TGL-5.** The help line says the consequence ("22:00 to 07:00 · nothing is dropped"), not a restatement of the title. (PRIN §5, NN/g info tips)

### 4.6 RadioCard and CheckCard

**Code:** `RadioCards` + `RadioCard`, `CheckCard` (`components/controls/RadioCard.tsx`); `ChoiceOption look="card"` is the older form to migrate.

**Anatomy:** logo or icon tile (40 px `ToolLogo` on a choice) · title (14 px semibold ink) · one sub-line (13 px muted) · optional tags or a count · the check mark in the corner (ring, filled ✓ when chosen) · `CHOSEN_CARD` accent ring when chosen.

- **RC-1.** Use cards for 2–5 options that differ in a way worth explaining (Move or Copy, which tool leads). For 2–4 short equal options with no explanation use `Segmented`; for more than 5, or a list of things, use `Picker`. (KIT, NN/g listbox vs dropdown, PRIN §10)
- **RC-2.** Every card has a logo or icon. (D-126)
- **RC-3.** The recommended card carries one accent tag "Recommended" and is preselected; never more than one recommendation. (PRIN §3, KIT pills; `GAPS.md` A6: today drawn ad hoc in the success tone)
- **RC-4.** The sub-line says what happens, in the user's terms ("The same conversation continues on Personal"). Shared facts go once below the group, not in every card. (PRIN §5)
- **RC-5.** The whole card is the hit target; arrows move and choose within a radio group, Space and Enter choose; the native input is the accessible control. (`RadioCard.tsx`, WCAG 2.5.3)
- **RC-6.** A card the user can't choose now and can unblock: faded, reason inline, plus a "Check again" link. One they can't unblock: hidden, and the remaining option is stated, not offered as a one-card choice. (D-127)

### 4.7 Tag, badge, chip and count

**Code today:** `Chip` (`primitives.tsx`) and `Pill` (`Pill.tsx`), two components with the same job and different tone maps; `FilterChip` (`related/`); the count badge (`--color-count*`); `.level-mark`. **Target:** one `Tag` (static word), one `FilterChip` (interactive), one `Count`, one `LevelMark`.

| Kind | Look | Use |
|---|---|---|
| Tag, neutral | `surface-2` or `surface-3`, muted words | An attribute: "web", "Beta", "CLI". |
| Tag, accent | `accent-soft`, accent-hi words | Selection, progress or a recommendation: "Recommended", "Setting up · 5 of 6", "prefilled". |
| Tag, L1 | quiet glyph + neutral word ("✓ Done") | A note. |
| Tag, L2 | amber dot + amber word ("● Asks you") | Needs you. Counts toward the attention budget. |
| Tag, L3 | ⊘ + red word ("⊘ Stopped with an error") | Broken. Counts toward the attention budget. |
| Count | neutral badge; amber dot at L2; red badge at L3 | A number; takes the level of the highest item it holds. |
| FilterChip | `fill`, `SELECTED` when on | A filter the user turns on and off. |

- **TAG-1.** 1–2 words (3 at most), sentence case, never wrapping in a row (ellipsis plus tooltip). (Polaris, Fluent, Carbon)
- **TAG-2.** At most two tags per row; only the exception is coloured; healthy states are no tag at all. (Carbon, Linear, D-117)
- **TAG-3.** Status never by colour alone: a glyph or a word with every hue. (WCAG 1.4.1, HIG Color, Carbon)
- **TAG-4.** System status uses Tags; user labels or categories use FilterChips or marks; groupings use icons and marks, never a second colour. (Polaris, Atlassian, D-69)
- **TAG-5.** Same state, same tag, same tone everywhere. (Polaris)

### 4.8 Alert

**Code:** `Alert` (`components/Alert.tsx`), levels `info`, `needs-you`, `broken`; `Banner` draws through it.

**Anatomy:** 16 px level icon (shape differs per level) · title in semibold ink · one sentence · actions at its end · optional ×.

- **ALR-1.** Three levels only. `info` is neutral (a fact, never amber). `needs-you` is amber and only when the user must act. `broken` is red when something failed. (D-126 KIT)
- **ALR-2.** Inline, beside what it is about. In a dialog it sits at the top of the body, under the header. (D-87, Primer)
- **ALR-3.** No page-top banner for an account's or a session's notice; those go to the bell and the notification centre. A slim app-wide bar only for states of the whole app (offline, update ready, heavy load), never with a session's action. (D-87)
- **ALR-4.** Every `needs-you` and `broken` Alert carries its clearing action, which really clears it (not only navigates). (DEEP §1.1.3, M:test-by-hand)
- **ALR-5.** At most one Alert per section and two per view; several of the same kind merge into one. (Primer "one banner per page", CALM flood rule)
- **ALR-6.** `broken` is announced at once (`role="alert"`); the others politely (`status`). (`Alert.tsx`, Fluent)
- **ALR-7.** Never end on a dead end ("Can't be set up"): set it up with consent and undo, or give numbered steps with an Open button and Check again; say whether it blocks. (M:serious-choices-explained addendum)

### 4.9 Section

**Code:** `Section`, `FoldSection` (`components/Section.tsx`).

**Anatomy:** heading (`.type-section`) · one help line beside it (`.type-help`) · one action at its end · content.

- **SEC-1.** Every area has a Section heading; at most 3–5 areas per view. (PRIN §6)
- **SEC-2.** The help line says what the section is for, once; never a glossary paragraph on every visit (those go behind a ?). (PRIN audit, M:ux-progressive-disclosure)
- **SEC-3.** A rare section folds with `FoldSection`; its closed label shows its current values ("Runs in its terminal · This Mac"). (PRIN §4)
- **SEC-4.** Never drop an existing collapsible or its chevron when restyling; chevrons are visible on hover and focus and work from the keyboard. (D-117 point 3)

### 4.10 List row

**Code:** `row/RowShell.tsx`, `row/RowParts.tsx`, `row/SessionRow.tsx`; the rail rows.

**Anatomy:** leading mark (app icon + profile badge, or the level mark) · name in ink · one secondary line (ink-2 or muted: project · App · window) · right-aligned values (tabular) and the state word · actions on hover and focus.

- **LST-1.** Reuse the session row's markup for any new list (two lines, mark, muted second line, relative time, hover and selection, fold). (M:ui-consistency)
- **LST-2.** One colour per row, its App's. (D-69)
- **LST-3.** No word repeats on every row or repeats its column head; blank means none; unknown is said once for the group. (PRIN §5, §14)
- **LST-4.** Row actions show on hover and focus and are reachable by keyboard; one action per row may stay visible when it is the row's main job. (PRIN §11, Carbon)
- **LST-5.** Selected rows wear `SELECTED_FILL` and semibold ink; focus is the accent ring, never the same look as selection. (D-117 point 2, `selected.ts`)
- **LST-6.** Each row's accessible name is its full sentence ("payments-api, main, 2 running, active 12 min ago"); state is in text. (DEEP §4.2)
- **LST-7.** Every visible name of an Account, App, Project, Session, Pool, Task or Mac opens its page. (PRIN §12)

### 4.11 Card

**Code:** `Card` (`primitives.tsx`), `ChartCard`, `StatTile`.

- **CRD-1.** A card is an area on `surface` over the canvas, padded 20 px, 16 px between cards, sized to its content in a responsive grid. (M:readability-and-visuals)
- **CRD-2.** No card inside a card of the same tone; inside a card, group with spacing or `surface-2` insets. (PRIN §6, AUDIT §3)
- **CRD-3.** Calm cards have no border (the surface step separates them); rich choice cards keep their edge. **Gap:** `Card` draws a hairline border, a shadow, `.sheen` and a hover border on every card. (D-117 round 2, `GAPS.md` A7)
- **CRD-4.** A stat tile shows a number only when it is non-zero or meaningful, with a sparkline when history exists; "Tasks 0" never takes stat size. (PRIN audit, M:readability-and-visuals)

### 4.12 Picker

**Code:** `Picker` (`components/controls/Picker.tsx`), with `PickerKind` forcing a top group for pickers of things.

**Anatomy:** a button with the value and a chevron · a menu with a search field (past 8 options), the top groups, then All, each line with icon, label, hint and ✓.

- **PCK-1.** Pickers of things (projects, sessions, Apps, Accounts, pools, Macs, tools, models) open on Suggested or the most related ("Open now", "Recent", "In this project"), then All. (M:pickers-suggested-then-all, PRIN §10, QUAL)
- **PCK-2.** A search field once the list can pass 8 options, with focus on open; type-to-filter; at most 60 lines drawn. (`Picker.tsx` `SEARCH_FROM`, NN/g)
- **PCK-3.** Options that would show nothing go last or behind "Show empty". (M:pickers-suggested-then-all)
- **PCK-4.** An option shown greyed must be one the user can unblock, with the how; otherwise it is hidden. (D-127, amends PRIN §10)
- **PCK-5.** A useful default selection, derived from real data, never a guess. (HIG Pop-up buttons, PRIN §3)

### 4.13 Empty, loading, error and unknown states

**Code today:** `Placeholder` (skeletons shaped like rows), and per-screen empties (`RelatedEmpty`, `WatchesEmpty`, `BotEmpty`, `FleetPeersEmpty`…). **Gap:** no shared `EmptyState` (DEEP §2.3 asked for one).

**Anatomy (inviting empty, rich):** icon (40–48) or a small line illustration · heading ("No watches yet") · one sentence on what fills it · one action · optional "Learn more". **Calm empty:** one line of text in the area, with a link.

- **EMP-1.** One state per area: loading (skeleton shaped like the content), error (`broken` Alert + Retry, data kept and marked stale), no results (with Clear filter), truly empty (teaches once), unknown (said once per group). (PRIN §14, DEEP §3, Carbon, Primer)
- **EMP-2.** Never flash empty: skeleton or cached data on the first frame. (M:instant-and-paged, DEEP §3)
- **EMP-3.** The empty state's action is the view's primary only when the view has no other primary; when several empties can show at once, they use text links. (Carbon, BTN-1)
- **EMP-4.** No filter bar, legend or column heads on an empty list. (M:ux-progressive-disclosure, Primer)
- **EMP-5.** Plain words: never an internal state name (`waiting_user`), never a code. (M:ux-progressive-disclosure, Carbon)
- **EMP-6.** Unknown is never drawn as 0, "—" or an empty ring in a big gauge; it is one line ("Not measured yet · Measure its limits"). (PRIN §14, DEEP §0)

### 4.14 Toast

**Code:** `Toast.tsx`, `ToastStack.tsx` (bottom right, three at most, a time bar paused on hover and focus, Undo or Retry, errors stay until closed).

- **TST-1.** Toasts confirm only the user's own actions. (D-87)
- **TST-2.** Never the only place information lives; never for anything that needs action later or must not be missed (that goes inline or to the notification centre). (Carbon, Atlassian, Primer)
- **TST-3.** About 6 s, paused on hover and focus; an error toast stays until closed. (DEEP §2.3, AJ-E2505)
- **TST-4.** Undo when the action can be undone; that beats a confirm dialog. (HIG Alerts, PAT-DES-1)
- **TST-5.** A toast never covers a dialog's or flow's footer (`data-toast-avoid`). (AJ-E2505 round 6)
- **TST-6.** Announced politely through the one live region. (AJ-E2505)

### 4.15 Menu

**Code:** `RowMenu`, `MenuItem`, `MenuSeparator` (`components/RowMenu.tsx`), `ItemMenu`.

**Anatomy:** an optional heading (what the menu is about) · lines: 16 px icon · label · optional hint · optional shortcut or ✓ · one level of submenus.

- **MNU-1.** Every action line has an icon (ICN-4). **Gap:** `MenuItem` has no `icon` prop, and 34 of 82 menu items have none. (D-57, M:desktop-first-navigation)
- **MNU-2.** Ordered by frequency, grouped by kind with separators; destructive actions last, in danger words, after a separator. (M:desktop-first-navigation "sorted and prioritised", HIG Menus)
- **MNU-3.** Every toolbar or header action also exists as a menu or palette command, with its shortcut shown. (HIG Toolbars, Raycast)
- **MNU-4.** Keyboard: arrows, Home, End, Escape and Tab close, → and ← for the submenu, focus returns to the opener. (`RowMenu.tsx`)
- **MNU-5.** Hints on one alignment grid; no wrapped or orphan hint. (D-57)

### 4.16 Smaller pieces

- **Segmented** (`controls/Segmented.tsx`): 2–4 short options; one look app-wide, filled with `fill`, the chosen segment `SELECTED`. (KIT, D-117)
- **Disclosure** (`controls/Disclosure.tsx`): a button with a chevron, never `<details>`; its closed label summarises what is inside. (D-57, AJ-E3011)
- **WhyTip / WhyButton** (`WhyTip.tsx`): a greyed control stays focusable, says `aria-disabled`, and shows its reason on hover and focus. (rc19/rc21 walks)
- **ToolLogo** (`ToolLogo.tsx`): the app's own icon read at run time; a neutral letter tile only when no icon exists and the owner allows none. (D-38, KIT)
- **ScrollEdge / MoreBelow**: the top shadow and bottom fade of any scroller under a fixed head. (0.4.1)
- **StatTile, ChartCard, charts** (`components/charts/`): hand-built SVG, table fallback, colour per Account. (ADR-0021, M:readability-and-visuals)

---

## 5. Patterns

### 5.1 A serious choice, explained (PAT-SER)

A serious choice changes what happens to the user's work, money, data or another company's access.

- **PAT-SER-1.** Show four things in place: the consequence in one line; a concrete example; a "What does this mean?" disclosure; a "Learn more" link to the guide's section, kept in sync with the docs. (M:serious-choices-explained; `switch/Explain.tsx` is the model)
- **PAT-SER-2.** Name the real actor: the App and Account ("Claude app · Personal"), never "target" or "provider". (M:serious-choices-explained, D-35)
- **PAT-SER-3.** Ask only when the answer changes what happens; the backend decides whether it applies. (M:serious-choices-explained)
- **PAT-SER-4.** Every option's words are true for each mechanism behind it. (M:serious-choices-explained)
- **PAT-SER-5.** The consequence of the chosen option shows in a tinted summary panel that updates as the choice changes ("This copies X, never Y"). (AUDIT §3, KIT preview)
- **PAT-SER-6.** **Gap:** there is no shared Explain or docs-link component outside the switch; build one (`Explain` promoted to `components/`). (`GAPS.md` A10)

### 5.2 Destructive and irreversible actions (PAT-DES)

- **PAT-DES-1.** Prefer undo over confirm: a reversible action happens at once with an Undo toast. Confirm only what can't be undone or is rare and large. (HIG Alerts, Atlassian)
- **PAT-DES-2.** The confirm is an S dialog: title "Delete payments-api?" (verb + object), the consequence in one line, what is removed and **what remains** ("Your folder isn't touched"), and where it goes (Trash). (DEEP §1.1.5, Fluent, `AccountManage.tsx` RemoveAccountModal words)
- **PAT-DES-3.** The danger button names the verb ("Move to Trash"), sits last, isn't the default; Cancel gets initial focus; Return doesn't trigger danger. (HIG, Carbon danger modal, BTN-4)
- **PAT-DES-4.** Use the least destructive option that does the job (Trash over delete, archive over remove) and say which it is. (HIG Alerts, `AccountManage.tsx` "Its folder is in the Trash")
- **PAT-DES-5.** Typing a name to confirm is reserved for bulk, unrecoverable actions (wiping a vault, deleting many items at once). (new; no external system mandates it)
- **PAT-DES-6.** Afterwards a toast says what was done and offers Undo when possible; the stop report or history keeps the record. (SPEC §9.4, TST-4)

### 5.3 Progressive disclosure and prominence (PAT-PD)

- **PAT-PD-1.** Three levels: glance (count and worst state), triage (why, forecast, one action), detail (history, lineage, raw paths). The daily loop never needs detail. (DEEP §1.4 and §3)
- **PAT-PD-2.** The common choice is made and stated in one sentence with a why of three words or fewer, accepted by Return or ⌘↩. (PRIN §3)
- **PAT-PD-3.** Options fewer than about 1 in 10 uses need go behind one labelled fold that shows its values. (PRIN §4)
- **PAT-PD-4.** Filters: basic first, "More filters" for the rest, and none on an empty list. Legends and help behind a ?. (M:ux-progressive-disclosure)
- **PAT-PD-5.** Say a thing when it changes the decision (tight, out of room, not signed in); stay silent when it's fine. (PRIN §4)

### 5.4 Hide what can't be used (PAT-HIDE)

- **PAT-HIDE-1.** A capability unsupported for the user's versions, tools or disks is hidden; the working choice is shown alone, with no "not proven" text. (D-127)
- **PAT-HIDE-2.** Disabled only when the user can unblock it, with how, in plain words ("4 sessions are working in this profile; Move becomes available once they finish"). (D-127)
- **PAT-HIDE-3.** No unfinished feature is visible outside its flag, and no "Beta · Not proven yet" on a route the product offers. (D-127 extension)
- **PAT-HIDE-4.** Version gates are watched; when a tool updates past one, the release scope check lists it. (D-127 §2)

### 5.5 View and edit in place (PAT-INP)

- **PAT-INP-1.** One surface to see and change a thing: select to inspect, an inspector beside it, ⋯ or right-click for actions; no separate setup tab for an object's own configuration. (M:view-and-edit-in-place, D-55)
- **PAT-INP-2.** "Change" turns the stated value into its picker in place; no second dialog. (PRIN worked example)
- **PAT-INP-3.** Every important object has an obvious door (a header entry, a chip on related pages). (M:view-and-edit-in-place)

### 5.6 Instant and paged lists (PAT-LST)

- **PAT-LST-1.** First frame: skeleton or cached data; then refresh in the background; the pick never jumps under the cursor (offer "acme-west now has more room · Use it"). (M:instant-and-paged, PRIN worked example)
- **PAT-LST-2.** Growing lists are keyset-paged with a hard maximum, virtualised or infinite, with a "Load more" fallback; dashboards use aggregates and top N plus "See all". (M:instant-and-paged)
- **PAT-LST-3.** Lists open on what's live or needs you, then recent, then the rest; search once a list can pass about 15; filters when they'd narrow something. (PRIN §11, M:desktop-first-navigation)
- **PAT-LST-4.** Data older than its freshness says so ("as of 12 min ago", "Stale · 12m"); offline disables actions with the reason and never hides data. (DEEP §3)
- **PAT-LST-5.** The feed is chronological, never ranked. (D-118, M:home-feed-design)

### 5.7 Attention budget (PAT-ATT), calm screens

- **PAT-ATT-1.** Each L2/L3 item is loud once per area (its Needs you row); its other appearances use the 6 px mark beside neutral words. (D-117)
- **PAT-ATT-2.** The shared count (rail, bell, title) counts L2 and L3 only and is identical everywhere. (D-117 point 5, DEEP §1.1)
- **PAT-ATT-3.** Flood: more than 5 needs group by kind; L3 is never grouped away. (CALM §a.5)
- **PAT-ATT-4.** Distribution target on reference fixtures: ≥ 80% of rows L0, ≤ 15% L1–L2, ≤ 5% L3. (CALM §a.7)
- **PAT-ATT-5.** Inferred states say "probably" and draw a hollow ring. (D-117, SPEC §9.3)

### 5.8 Doors and links (PAT-DOOR)

- **PAT-DOOR-1.** Every entity page links every related entity, both ways, as tabs or sections with counts. (M:desktop-first-navigation, PRIN §12)
- **PAT-DOOR-2.** Links look like links: accent-hi text or a chevron; external ones end in ↗. (PRIN §12)
- **PAT-DOOR-3.** Actions target their object by its unique id end to end. (M:actions-target-by-id)

---

## 6. Content rules

- **CNT-1. Plain words.** No internal words on screen: pack, target, recipe, instance, transplant, harness, proven, major, F1, tok. Use handover, the App's name, launch settings, session. (PRIN §13, `plain_words_lint.py`, `terminology_lint.sh`)
- **CNT-2. One word per thing.** Account = a subscription. App = a desktop or CLI app, or one of its profiles. Project = a code folder. Session = a conversation in an App. A profile is never called an account. (D-35, D-62, M:entity-model, `one-vocabulary.test.tsx`)
- **CNT-3. Sentence case** for titles, headings, buttons, labels, tags and menu items. Proper names keep their case (Claude Code, This Mac). Capitals only in `.section-label`, by CSS. (Carbon, Primer, Atlassian; app practice)
- **CNT-4. Short labels.** Buttons 1–4 words, verb first; labels 1–3 words; tags 1–2 words; titles under about 40 characters. Detail goes on the second line or behind disclosure. (M:ui-consistency, Carbon, Fluent)
- **CNT-5. Say it once.** No word repeats its heading, column or neighbour. (PRIN §5)
- **CNT-6. Name the outcome.** Questions and buttons say what will happen ("Move to Personal"), not the artifact. (M:questions-say-what-they-decide, BTN-2)
- **CNT-7. Honest numbers.** Measured, computed or estimated is said in text; never a number the backend didn't send; "accurate" means it measures what the label says. (DEEP §1.1.4, M:accuracy-means-what-it-claims)
- **CNT-8. Tone.** Calm, direct, no blame, no "oops", no exclamation marks. Errors say what happened and what to do. Inferred states say "probably" or end with "?". (HIG Writing, DEEP §2.4)
- **CNT-9. Generic examples.** Fixtures, previews and screenshots use generic names (Work, Personal, payments-api), never real people's names, emails or home paths. (D-101, M:no-real-names-in-outputs)
- **CNT-10. Numbers and time.** Tabular; relative plus absolute on hover; units spelled ("1.2 GB", "12 min ago"). (DEEP §1.2)

---

## 7. Accessibility

- **A11Y-1.** WCAG 2.2 AA in both themes on every gallery state: axe 0 critical and 0 serious. (D-16, AJ-E804, `a11y_gate.sh`)
- **A11Y-2.** Text ≥ 4.5:1; large text and icons ≥ 3:1; a control's identifying edge or state ≥ 3:1 (WCAG 1.4.11); selection edge ≥ 3:1 off its surroundings. (`contrast.spec.ts`, D-117)
- **A11Y-3.** Never colour alone: every state has a shape or word too; L2 and L3 differ by shape (dot or ring vs ⊘ and tint). Check with protan, deutan and tritan simulation. (WCAG 1.4.1, CALM §f.6)
- **A11Y-4.** Keyboard: every control reachable; visible focus ring (2 px accent, offset 2) only on keyboard focus; logical order; focus returned after dialogs and menus; no traps except modal dialogs. (DEEP §1.3, §2.5, `styles.css`)
- **A11Y-5.** Names: icon-only controls named; status glyphs out of names; each step's name holds its number, name and value (WCAG 2.5.3); rows named by their full sentence. (D-117, `FlowSteps`)
- **A11Y-6.** Live regions: one polite region for results and toasts, `role="alert"` for broken; arrivals batched at most once per 30 s. (DEEP §2.5, §4.1)
- **A11Y-7.** Target size ≥ 24 px with spacing (WCAG 2.5.8); 28 px preferred. (`styles.css`, HIG)
- **A11Y-8.** 200% zoom works without loss; Reduce Motion stops animation; Increase Contrast raises edges and labels. (`zoom-motion.spec.ts`, D-117)
- **A11Y-9.** Tabs follow the full ARIA tab pattern; splitters are keyboard-operable. (DEEP §2.5)
- **A11Y-10.** Disabled controls that explain themselves stay focusable (`aria-disabled`), so the reason can be reached. (`WhyTip.tsx`)

---

## 8. Worked example: the Move or Copy dialog, redesigned

### 8.1 What it is today

The switch dialog (`components/SwitchFlow.tsx` in `switch/SwitchDialogFrame.tsx`) asks how a Claude-to-Claude switch happens (`switch/MoveChoice.tsx`): **Move** the conversation to the other App (the default when it can) or **Copy** it. The frame itself is right: a fixed header, a scrolling body and a fixed footer. The problems are in the content and in the dialogs around it:

- "How" is a 13 px label; the two choices are `ChoiceOption` cards with a small icon, and "Recommended" is drawn by hand in the success tone (RC-3, TAG-5).
- "What Move does" (`MoveBody`) is four bullet paragraphs with bold "What moves:", "The original:", "Restarts:", "After:" labels in one ink, then muted 12 px notes and lists: grey text blocks with no icons, tags or areas (COL-2, TGL-2, PR-3).
- The fact that matters most, "anything running in them stops", is the third bullet's second sentence (PAT-SER-1, ALR-1).
- The choice label says "Move to account" while the destination is an App (a profile) on an Account (CNT-2).
- `ContinueButton` waits disabled with no reason (BTN-5); the frame's buttons are hand-made classes that `primariesIn()` can't count (BTN-7).
- The same choice opened through the shared `Modal` would overflow: `Modal` has no height cap and centres with `items-center` (DLG-2).

### 8.2 The redesign (rich mode, size M 680 px)

```
┌──────────────────────────────────────────────────────────── 680 ──┐
│ [⇄]  Continue on Claude app · Personal                        ×   │  header (fixed)
│      Payments refactor · now on Claude app · Work                  │  ~96 px
│      [◉ Claude · Work] ──────────▶ [◉ Claude · Personal]           │
├────────────────────────────────────────────────────────────────────┤
│ How it continues                                                   │  body (scrolls)
│ ┌─ ◎ Move ─────────── [Recommended] ✓┐ ┌─ ◎ Copy ──────────────  ○┐│
│ │ [⤓] The same conversation continues │ │ [⧉] A copy continues on   ││
│ │     on Personal. Work no longer     │ │     Personal. The original││
│ │     lists it.                       │ │     stays on Work.        ││
│ │ [No extra space] [Undo available]   │ │ [Uses 1.2 GB]             ││
│ └─────────────────────────────────────┘ └───────────────────────────┘│
│                                                                    │
│ ▲ Needs you  Claude app · Work has 2 conversations working. Both   │  Alert, only when true
│   apps quit and reopen; start that work again after.  [Wait]       │
│                                                                    │
│ What happens                                                       │
│ ┌ surface-2 summary panel ─────────────────────────────────────────┐│
│ │ [💬] Moves      The conversation and 2 sessions · 1.2 GB         ││
│ │ [↶] Original    Work no longer lists it · Undo puts both back    ││
│ │ [⟳] Restarts    Claude app · Work and Claude app · Personal      ││
│ │ [↗] After       Opens in Personal. Send a message to check it.   ││
│ └──────────────────────────────────────────────────────────────────┘│
│ [☐] Use this conversation's project notes on Personal              │  CheckRow, only on conflict
│     Every conversation of this project there reads them.           │
│ ▸ 3 conversations move                                             │  Disclosure
│ ▸ What does this mean?   Learn more in the guide ↗                 │  Explain
├────────────────────────────────────────────────────────────────────┤
│                                          Cancel   [Move to Personal ⏎] │  footer (fixed)
└────────────────────────────────────────────────────────────────────┘
```

**Frame (DLG-1 to DLG-4).** `DialogFrame` size M, `max-height: min(92vh, 760px)`, 16 px gutter. At 1024×680: 625 px tall, header about 96, footer 60, body about 469. The two cards and the start of the summary fit on open (DLG-3). The footer and its primary are on screen at every scroll position. The body shows the top edge shadow once scrolled and the bottom fade while more is below.

**Header.**
- A 36 px icon tile with the switch icon (accent-soft, rich mode).
- Title in `.type-section`: "Continue on Claude app · Personal", naming the App (CNT-2, PAT-SER-2).
- Subtitle in `.type-help`: the conversation's title and where it is now.
- A from → to strip: `ToolLogo` 24 px with the profile badge beside it, the App name, an arrow, the destination. It is the one picture of what happens.
- Close ×, labelled "Close", Esc.
- No step marks: this is the one-click door. On the wizard door (three steps), `StepMarks` sits under the title.

**Section 1, "How it continues"** (`Section` + `RadioCards`, RC-1 to RC-6):
- **Move** card. Icon `IconCarryIn`. Title "Move". Accent tag "Recommended" when nothing blocks it. Sub-line: "The same conversation continues on Personal. Work no longer lists it." Neutral tags: "No extra space", "Undo available".
- **Copy** card. Icon `IconCopy`. Title "Copy". Sub-line: "A copy continues on Personal. The original stays on Work." Neutral tag "Uses 1.2 GB" (from the offer's size).
- Move preselected when it can happen (PCK-5, PRIN §3).
- **Blocked by something the user can clear** (`user_can_clear`): the Move card is faded, its reason in one line under the sub-line ("2 conversations are working in Work. Move is ready once they finish."), with "Check again" (accent-hi link). Copy is preselected (RC-6, PAT-HIDE-2).
- **Blocked by the setup** (`setup`): no cards. The body states "Copies to Personal. The original stays on Work." as a sentence with the Copy icon, and the primary becomes "Copy to Personal" (PAT-HIDE-1, D-127).

**Alert** (only when true, ALR-1, ALR-4): `needs-you` when the move restarts apps with work running ("Claude app · Work has 2 conversations working. Both apps quit and reopen; start that work again after."), with a "Wait for them" action that returns to the card's waiting state. `broken` when the destination has no room (from `TargetRoom` `risky`), with its action ("Choose another app"). Nothing when nothing applies (PAT-PD-5).

**Section 2, "What happens"** (a `surface-2` summary panel, PAT-SER-5). Four fact rows, each a 16 px icon, a label in `muted` and a value in `ink`:
- **Moves:** "The conversation and 2 sessions · 1.2 GB". Archived or missing-folder members appear as neutral tags ("1 archived").
- **Original:** "Work no longer lists it · Undo puts both back".
- **Restarts:** "Claude app · Work and Claude app · Personal".
- **After:** "Opens in Personal. Send a message to check it answers there."

With Copy chosen, the panel updates in place: Copies · Original "Stays on Work, unchanged" · Space "Uses 1.2 GB" · After.

**Options and details:**
- `CheckRow` with the notes icon, only when the notes conflict (`notes_conflict`): "Use this conversation's project notes on Personal", help "Every conversation of this project there reads them. Undo restores them."
- `Disclosure` "3 conversations move": the members as small rows, each with its App icon and tags (archived, already on Personal, folder gone).
- Tool output left behind is a muted line inside the disclosure, not a paragraph in the body.
- `Explain`: "What does this mean?" with two or three tips, and "Learn more in the guide ↗" (PAT-SER-1).

**Footer** (BTN-1 to BTN-5): the primary "Move to Personal" (or "Copy to Personal"), filled, last, answers Return. Cancel is ghost, to its left. On the wizard door, Back sits between them. When the primary can't act yet, the reason is a visible line before the buttons, never a silent greyed button.

**Running and done** (DLG-10):
- **While moving:** the body becomes a three-step progress list (Quit both apps ✓ · Move the conversation ● · Reopen both). The footer is removed and Esc says "It finishes first". The dialog title stays.
- **Moved:** a success summary (✓ "On Personal now"), the same four fact rows in past tense, `Undo` as a secondary, `Done` as the primary.
- **Failed:** a `broken` Alert with what was left and Undo, then Close.

**What makes it pass the system.**

| Rule | How it shows |
|---|---|
| PR-3, COL-2, COL-3 | Title ink 15 px; card titles ink 14 semibold; sub-lines muted 13; summary labels muted, values ink; no muted body paragraphs. |
| D-126 rich | Icon tile, logos, choice cards with icons, accent ring on the chosen card, an accent "Recommended" tag, a summary panel. |
| PR-6, COL-4 | Amber appears only in the needs-you Alert, and only when something running will stop. |
| D-127 | Move hidden when it can't work; faded only for what the user can clear, with how. |
| DLG-1 to DLG-3 | Fixed header and footer; body scrolls; choices visible at 1024×680; primary always visible. |
| PAT-SER | Consequence (summary panel), example (named Apps and sizes), "What does this mean?", guide link. |
| CNT-2 | "Continue on Claude app · Personal", "Move to Personal"; never "Move to account". |

---

## 9. Appendix: external sources

Read 2026-11-21 (web pages only).

- Apple HIG (through Apple's JSON endpoints): Sheets, Alerts, Buttons, Toolbars, Sidebars, Settings, Typography, Color, Labels, Accessibility, SF Symbols, Writing, Onboarding, Progress indicators, Entering data. `https://developer.apple.com/design/human-interface-guidelines/`
- Primer: Dialog guidelines, Label, Banner, Empty states, Forms, Notification messaging, Progressive disclosure, Button; `@primer/stylelint-config`, `eslint-plugin-primer-react`, primitives. `https://primer.style/`
- Carbon: Modal usage, Dialog pattern, Progress indicator, Forms pattern, Status indicator pattern, Notification pattern, Empty states pattern, Data table, Disclosures, Spacing; `stylelint-plugin-carbon-tokens`. `https://carbondesignsystem.com/` and `github.com/carbon-design-system/carbon-website`
- Fluent 2: Dialog, Message bar, Badge, Layout, Typography, Accessibility. `https://fluent2.microsoft.design/`
- Atlassian: Modal dialog, Lozenge, Section message, Flag, Button, Tokens. `https://atlassian.design/`
- Polaris: Badge, `stylelint-polaris`. `https://shopify.dev/docs/api/app-home/` and `github.com/Shopify/polaris`
- Material: dialogs (M2 page; M3 site did not render). `https://m2.material.io/components/dialogs`
- Linear: "How we redesigned the Linear UI", "Behind the latest design refresh". `https://linear.app/now/`
- Raycast: "A fresh look and feel", Manual: Search bar. `https://www.raycast.com/blog/a-fresh-look-and-feel`
- WCAG 2.2 (1.4.1, 1.4.3, 1.4.6, 1.4.11, 2.3.3, 2.5.3, 2.5.8); APCA in a Nutshell; NN/g articles as cited in `docs/design/PRINCIPLES.md`.

Where the systems disagree, this system takes the macOS reading: Cancel then the primary at the trailing edge, in every dialog, sheet and flow. It takes sentence case over Apple's title case for buttons. Toasts are only for the user's own actions. Required data may hold Next on a choice step, with its reason shown; typed input keeps Next enabled and explains on press.
