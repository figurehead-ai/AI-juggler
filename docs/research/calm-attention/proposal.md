# Proposal: an attention budget for Home and the rail

> **Ruled: D-117 (2026-11-14); tasks E36 and AJ-E3701.** Copied from the
> design session's working folder; its text is kept as written. Where it
> and D-117 differ, D-117 wins:
> - `attentionLevel()` lives in `ui/src/lib/attention.ts`, beside
>   `attentionWords`, not in a new `lib/attention-level.ts`.
> - Phase C is not "if the scope gate allows": every screen is in 0.5.
> - `.dot-live` is not removed: 0.4.5's three-iteration rule and
>   `idle.spec.ts` stay.
> - The count rule and the D-47 point 2 amendment are settled by D-117.
> - The mock (`mock.html`) and the scripts `tools/contrast.py` and
>   `tools/colourful.py` stayed in the design session's folder; the
>   colourfulness script is rebuilt in the repository by AJ-E3601. The
>   round-2 captures are in `docs/qa/evidence/AJ-E3601/`.

**Goal:** keep every piece of information, but make only what needs the user loud. This answers the maintainer's "Christmas tree": "if users feel everything is a disaster then nothing is a disaster".

**Inputs:**
- `research.md`: ISA-101, ISA-18.2, EEMUA 191, alarm fatigue, calm technology, how other tools do it, and modes.
- `audit.md`: 15 loud and 4 semi-loud elements for 2 urgent facts; one need shouted 5–6 times; the dark alarm red is the quietest status colour.

**Rulings this proposal honours:**
- **Readable and visual:** the four text levels stay distinct, and colour stays meaningful. We stop wasting colour, not removing it.
- **UI consistency:** reuse the existing row, chip and marks.
- **Progressive disclosure.**
- **AA contrast:** every number is computed in §c.
- **Desktop-first:** one click to go to anything.
- **D-69:** colour is the Apps'.
- **PRINCIPLES §9:** one primary action per view.

---

## (a) Attention model: four levels and a budget

ISA-18.2 ranks alarms by **consequence × time to respond**. Applied to a supervised AI session, that becomes:

| Level | Name | Meaning (consequence × urgency) | App states mapped to it |
|---|---|---|---|
| **L3** | **Broken** | Work has stopped unexpectedly, or will be lost, and only the user can recover it. | Stopped with an error / crashed; Limit reached **and no Account has room** (nothing to continue on); an Account **Out while a session is running on it**; a switch that failed or rolled back; the ledger or readings degraded (Home's `ledger-degraded` card, all readings stale); This Mac at **critical** memory pressure. |
| **L2** | **Needs you** | Work is waiting on a decision. Nothing is lost while it waits. | Asks you; Probably asks you; Probably stuck; Limit reached **with somewhere to continue**; Cut off: Resume it; sign-in needed for an App that has sessions; a budget cap hit; an agent's message waiting; an Account at ≤ 10% left serving a live session; This Mac working hard (the "heavy" note, warning pressure). |
| **L1** | **Note** | Good to know at a glance. No action is needed now. | Done, review it; low runway (11–25%) that no live session depends on; leftovers to clean up (not heavy); update available or restart ready; not measured yet; renewal in ≤ 7 days; kept alive or paused; uncommitted files. |
| **L0** | **Calm** | The normal running picture: the periphery. | Working; Idle; Ended; healthy runway (> 25%); App running or not running; counts, places, times, projects, Accounts; identity (App icons and colours); "Plenty of room". |

**Inferred states are a step softer than confirmed ones.** "Probably asks you" and "Probably stuck" stay L2. They draw a hollow ring instead of a filled dot, and they sort after confirmed needs. ISA rationalisation keeps alarms for confirmed conditions, and the word "Probably" already says it's a guess.

### The budget (what may be loud at once)

These are the rules a test can check (§f).

1. **One loud mark per fact per surface.** Each L2 or L3 item is loud **once in the rail** (its Needs you row) and **once on Home** (its Needs card row). Its other appearances use the level's **mark**, a 6 px dot or ring, beside neutral text. Those appearances are the Open now copy (D-47), the App group header, the App tile, the bell and the header.
2. **Fills and tints are for L3 only.** There are no amber fills anywhere. At most **one tinted block per surface**: several L3 items group into one block.
3. **One filled accent button per view.** On Home that is the first need's primary action. Every other button is bordered or ghost.
4. **Counts take the colour of the highest level they contain.** A count is neutral at L0 and L1, has an amber dot at L2, and is red at L3. A monochrome count badge (ink on canvas) is still the most luminous thing in the header. It pops by **luminance**, not hue.
5. **Flood rule** (adapted from ISA-18.2's "more than 10 in 10 minutes"). When **more than 5** L2/L3 items are open, Needs you groups them by kind ("4 ask you · 2 probably stuck · 1 limit reached"). Amber goes on the group heads only, and the rows inside show the L2 mark with a neutral word. The Home title says the same ("7 need you: 4 ask, 2 probably stuck, 1 limit"). L3 items are never grouped away. They stay on top, in red.
6. **Motion is for change only.** No animation loops in lists. When an item enters L2 or L3, its row fades in one tint for 600 ms, once, and at most once per 30 s, which matches the existing aria-live batching. Reduced motion turns this off.
7. **Distribution target** (the EEMUA 80/15/5 adapted). On the reference fixtures, **≥ 80% of rows are L0**, ≤ 15% are L1 or L2, and ≤ 5% are L3. If a fixture breaks that, it means a state is mapped too high, not that the screen needs more colour.

**On the reference Home** (3 needs, 2 urgent), today's 15 loud and 4 semi-loud elements become **5 loud**:
- the rail's Needs you count;
- 2 amber words in the rail;
- 2 amber words on Home's Needs card.

They sit beside 1 filled primary button, which is accent, not alarm. The review item and the counts move to L1 or L0.

---

## (b) Visual language per level

| | **L0 Calm** | **L1 Note** | **L2 Needs you** | **L3 Broken** |
|---|---|---|---|---|
| **Text colour** | The 4-level hierarchy: ink for titles and values, ink-2 for sentences, muted for labels and state words, faint for hints | Neutral (ink-2 or muted). The hue is only on its glyph. | State word in **warning** (amber), semibold | State word in **danger** (red), semibold |
| **Mark** | Idle and ended: faint 6 px dot. Working: 6 px dot in calm **live**, **static** | A 10 px glyph in its quiet hue: ✓ success for review, ↓ live for an update, ⌁ warning-quiet for low runway | Filled amber dot for confirmed, **hollow amber ring** for "probably" | Red ✕ or ⊘ glyph, 12 px |
| **Weight** | Titles medium (ink-2 in the rail, ink on Home); the selected row semibold ink | Medium | Title semibold ink, word semibold | Title semibold ink, word semibold |
| **Fill / tint** | none | none | **none** | Row tint `danger-soft` (one block per surface) |
| **Borders** | Cards: hairline. Lists: spacing, or the new `divider` token. No dashed borders. | – | – | – |
| **Icons** | App icons stay (real icons, run-time ruling). Other icons in rows show on hover, focus or selection (↗, pin, drag). Section headers have no icons. | One glyph | Alert icon only on the **Needs you header** | ✕ or ⊘ glyph on the row and the Needs you header |
| **Identity colour** | **One** App dot per group header, tile or need row (D-69). No row edges. | – | – | – |
| **Motion** | none (no breathing) | none | one 600 ms tint fade on entry | one 600 ms tint fade on entry; tray icon and notification sound (L3 only, existing settings) |
| **Aggregates** (bell, header, App header, tile) | number in muted | number in muted | number in ink + amber dot | number in danger, red badge |

**Why this still meets "Readable and visual" and isn't "make everything grey":**
- **Text** keeps four steps, from Lc 98 down to Lc 45 in dark.
- **Weight** keeps three steps.
- **Surfaces** keep four steps.
- **Status** keeps a three-step chroma ladder:
  - L3 red, OKLCH C 0.15, the most saturated;
  - L2 amber, C 0.12;
  - L1 calm live and success, C 0.07–0.09.

  Today the ladder is inverted: cyan C 0.13 and amber C 0.16 outshine red C 0.17, which has the lowest luminance and so reads as quietest in dark.
- **Identity** colour stays, just once per group.
- **Charts** on Usage keep their categorical colours, since that is data, not status.

---

## (c) Token changes (`ui/src/styles.css`), contrast-checked

**Method:**
- WCAG 2.2 ratios and APCA 0.0.98G Lc, computed by the scratchpad script `tools/contrast.py` against all five surfaces (canvas, canvas-hi, surface, surface-2, surface-3), the same set `ui/tests/visual/contrast.spec.ts` uses.
- OKLab ΔE to the App mark colours (`lib/mark-hues.ts` requires ≥ 0.12).

### Status colours: dark theme (light unchanged)

| Token | Before | After | WCAG min → max (after) | APCA Lc (after) | Chroma (OKLCH C) before → after | ΔE to App marks (≥ 0.12) |
|---|---|---|---|---|---|---|
| `--color-danger` (L3) | `#f87171` (5.26–6.98:1, **Lc 45–49**) | **`#ff8578`** | 6.15 → 8.17:1 | **Lc 52–56** | 0.166 → **0.151 (now the highest)** | 0.166 (was 0.132) |
| `--color-warning` (L2) | `#fbbf24` (8.71–11.57:1, Lc 70–73) | **`#e8b25a`** | 7.58 → 10.07:1 | Lc 62–66 | 0.164 → 0.123 | 0.196 |
| `--color-live` (L1, Working dot) | `#22d3ee` (8.05–10.69:1, Lc 66–69) | **`#7bbfcc`** | 7.04 → 9.35:1 | Lc 58–62 | 0.134 → **0.071** | 0.182 |
| `--color-success` (L1) | `#34d399` (7.57–10.05:1) | **`#79c3a0`** | 7.01 → 9.31:1 | Lc 58–62 | 0.153 → 0.090 | 0.172 |
| `--color-danger-soft` (L3 row tint) | `#f871711a` | **`#ff857824`** (14%) | danger on it: 5.86:1 on surface, 4.84:1 on surface-3; ink on it 12.7:1 | Lc 48–52 | – | – |

**Light theme:** `--color-warning #9c4807` (5.07–6.28:1), `--color-danger #b91c1c` (5.23–6.47:1), `--color-live #056074` and `--color-success #065f46` stay as they are. They already form a sensible ladder, and the light theme's noise comes from usage: tints, chips and repetition. The L3 tint `#b91c1c14` keeps danger at 5.66:1 on white and 4.69:1 on surface-3, with ink at 13.6–16.4:1.

**All four dark status colours stay ≥ 6:1 on every surface, above the AA 4.5:1.** Danger gains Lc 7 in APCA. Warning stays at Lc 62 or more, so it still clears PRINCIPLES §7's Lc 60 for content text in dark.

### New tokens

| Token | Dark | Light | Use | Contrast note |
|---|---|---|---|---|
| `--color-divider` | `#232a36` | `#e2e6ed` | row dividers that remain (Runway accounts, Pick up); replaces `border-hairline/40–/60` | decorative, so WCAG 1.4.11 doesn't apply; card and control edges keep `--color-hairline` |
| `--color-count` / `--color-count-ink` | `#d2d8e1` / `#0b0e14` | `#2c3341` / `#ffffff` | neutral count badge (bell at L0–L2) | 13.5:1 dark, 12.7:1 light |
| `--color-count-alarm` / ink | `#ff8578` / `#0b0e14` | `#b91c1c` / `#ffffff` | the count badge at L3 | 8.2:1 dark, 6.5:1 light |
| `--level-3`, `--level-2`, `--level-1-done`, `--level-1-live` | aliases of danger, warning, success, live | same | semantic names for components and tests | – |

### Removed decoration

- `body` radial washes. The violet `#7c7cf40d` and cyan `#22d3ee08` go in both themes, leaving the flat `canvas-hi → canvas` gradient.
- `RailHeader` logo glow (`drop-shadow-[0_0_10px_#7c7cf466]`).
- The breathing animation on rows (`animate-breathe`) and on the This Mac dot (`.dot-live`). The keyframes can stay for the Floor or for one-off use.
- `UserMenu` avatar gradient (`from-accent to-live`) becomes `bg-surface-3 text-ink`.

**Unchanged:**
- ink, ink-2, muted and faint;
- the surfaces and hairline;
- the accent (`#8787f6` / `#4747d1`);
- the App and Account mark colours, which are measured, and the new status colours keep ΔE ≥ 0.166 from them.

**Mark-hue tests:** `lib/mark-hues.ts` `STATUS_HUE_GAPS` must be re-run against the new status hues:
- new warning h 78 (was 84);
- new danger h 28 (was 22);
- live h 211 and success h 163 are unchanged.

---

## (c2) Round 2: calmer surroundings (token and usage changes over round 1)

After reviewing the round-1 mock, the maintainer said: "way better, but … calm it even more. The subheadings and other borders and things need to be even calmer. More subtle but still readable." They added that app icons should be muted unless that app needs attention or is selected.

**L2 and L3 are unchanged from round 1.** The amber and red words, glyphs, dots and the L3 tint keep the same values. Only their surroundings get quieter, which is why they stand out more.

### New and changed tokens

| Token | Dark | Light | Contrast (min → max on canvas, canvas-hi, surface, surface-2, surface-3) | Use |
|---|---|---|---|---|
| `--color-section` **(new)** | `#c3cad5` | `#3d4553` | dark 8.82 → 11.71:1, **Lc 70–74**; light 7.80 → 9.66:1, Lc 78–92 | Section labels: Needs you, Open now, Your apps, Runway by account |
| `--color-fill` **(new)** | `#181d27` | `#ffffff` | non-text: 1.14:1 on canvas (dark), 1.16:1 (light) | Filled search field, segmented controls, the `?` button. Replaces their outlines. |
| `--tone-saturate` **(new)** | `0.25` | `0.25` | n/a (icons and marks are never the only carrier of a state) | App icon and mark tone at rest |
| `--tone-opacity` **(new)** | `0.78` | `0.72` | n/a | App icon and mark tone at rest |
| `--color-divider` (round 1) | `#232a36` | `#e2e6ed` | decorative | **Now used once**: the vertical rail/main split |
| Card and tile border | `--color-hairline` → **transparent** | same | the surface step separates them (dark surface on canvas 1.11:1, light 1.16:1, the prior "visible step apart" ruling) | Home cards, App tiles, window-access card |

### Type hierarchy (values brightest after the title, headings never one grey with labels)

| Role | Colour | Size / weight | Dark Lc | Light Lc |
|---|---|---|---|---|
| Page title | ink | 21 px / 600 | 100 | 105 |
| **Values**: session titles, App names, "58% left" | ink | 13–15 px / 500–600 | 98–101 | 92–105 |
| Sentences, rail row titles | ink-2 | 13 px / 400–500 | 79–82 | 84–99 |
| **Section labels** | **section** | **12 px / 500, capitals, tracking 0.07em** | **70–74** | **78–92** |
| Labels, meta, state words (L0/L1) | muted | 12–13 px / 400 | 60–63 | 71–85 |
| Hints ("To review", idle dot) | faint | 12 px / 400 | 42–45 (≥ 4.94:1) | 64–78 |

A section label sits above labels on four counts: colour (Lc ~73 against ~62), weight (500 against 400), capitals and tracking. It sits below values on colour and size.

### Contrast of every text token used in round 2 (dark / light, minimum over all five surfaces)

| Token | Dark | Light |
|---|---|---|
| ink | 13.33:1, Lc 98 | 15.16:1, Lc 91 |
| ink-2 | 10.15:1, Lc 79 | 10.24:1, Lc 84 |
| section | 8.82:1, Lc 70 | 7.80:1, Lc 78 |
| muted | 7.28:1, Lc 60 | 5.87:1, Lc 71 |
| faint | 4.94:1, Lc 42 | 4.59:1, Lc 64 |
| accent-hi (header New session text) | 6.79:1, Lc 57 | 5.28:1, Lc 67 |
| warning (L2) | 7.58:1, Lc 62 | 5.07:1, Lc 66 |
| danger (L3) | 6.15:1, Lc 52 | 5.23:1, Lc 66 |
| danger on the L3 tint | 5.86:1 on surface, 4.84:1 on surface-3 | 5.66:1 on white, 4.69:1 on surface-3 |
| success (✓ glyph) | 7.01:1, Lc 58 | 6.21:1, Lc 72 |

**Every meaning-bearing text token is at 4.5:1 or above in both themes.** Faint is the lowest at 4.59–4.94:1, and it is used only for hints that repeat a visible state.

### Usage changes

- **Section labels** use `--color-section` at 12 px, weight 500, in capitals with 0.07em tracking. On the Needs you label, the **count** carries the level: the amber ⚠ and number (L2), or the red ✕ and number (L3). The label itself stays calm.
- **Tab strip:** nav items are muted at weight 500. The current item is ink at 600, with no fill.
- **Borders:**
  - cards and tiles have no border;
  - there are no row dividers (the Runway accounts and Needs rows use spacing);
  - there is no rail indent guide;
  - the rail foot and header lose their rules;
  - **one hairline** (`--color-divider`) splits the rail from the main area.
- **Inputs and segmented controls** are filled with `--color-fill` and have no outline. The selected segment is `surface-3` with ink-2 text at weight 500.
- **Buttons:** every non-primary button is a ghost button (ink-2 text, `surface-3` on hover). The one filled accent button remains. The header New session is accent text with no fill.
- **App marks:**
  - Rail and need rows show a **6 px App dot** instead of the icon.
  - Group headers show a **13 px** icon and a 5 px dot.
  - The **full icon stays on the App tiles**, where the App is the subject.
  - Account discs on the Runway shrink to 8 px.

### App tone: muted at rest, full colour when it matters

This is a CSS treatment on whatever image `AppIcon` renders, because icons are read at run time and nothing is baked into assets:

```css
.app-tone        { filter: saturate(var(--tone-saturate)); opacity: var(--tone-opacity); transition: filter .15s, opacity .15s; }
.app-tone.is-lit { filter: none; opacity: 1; }
@media (prefers-contrast: more) { .app-tone { filter: saturate(.5); opacity: 1; } }
```

**Why `saturate(.25)` plus a little opacity, and not a tinted monochrome:**
- it keeps each icon's own luminance pattern, so the shape stays recognisable;
- it works on any bitmap or SVG;
- it leaves no colour from one App on another.

The global reduced-motion rule already cancels the transition.

**What lights an icon or mark to full colour (`is-lit`):**

| Lit | Muted |
|---|---|
| The App, profile or session is at **L2 or L3** (on its group header, App tile, need row and Needs card), from `attentionLevel()` | Working, Idle, Ended (L0), and L1 notes (to review, low runway) |
| It is **selected or current** (the selected row, the App whose page is open) | everything else at rest |
| On **hover or keyboard focus** of its row, header or tile | |

**Apply the filter to the icon image only.** The ProfileBadge dot gets the same class separately, so a lit badge never sits on a muted icon by accident.

**Never colour alone:**
- the L2/L3 word, glyph and dot still carry the meaning;
- a muted icon never hides a state;
- colour-blind users lose nothing.

In the mock, the two Work Max Apps, which need you, are in full colour on their tiles and rail headers, and ChatGPT and Personal are muted beside them.

### Measured: before, round 1 and round 2 (mock frames, headless Chrome)

**Element counts.** These come from an in-page script (`#metrics` in `mock.html`) that reads computed styles. They are the same in both themes.

| Metric | Before | Round 1 | Round 2 |
|---|---|---|---|
| **Loud elements**: amber or red text, accent fill, amber, red or accent tints, red badge | **18** (hand count: 15 + 4 semi) | **8** | **6**: the Needs you count, 2 rail words, 2 Home words, 1 primary button |
| Hue-coloured text elements | 20 | 9 | **8** |
| Tinted or filled backgrounds | 16 | 5 | **2** (the primary button and the bell badge) |
| Elements with a visible border | 36 | 29 | **2** |
| Semibold or bold text (non-heading) | 58 | 45 | **26** |
| Icons (app tiles and marks, SVG) | 16 | 15 | **9** |
| Infinite animations (from code) | 3 | 0 | 0 |
| Text elements (the information kept) | 131 | 131 | 125 (only the icon letters went; no fact removed) |

**Colourfulness** (Hasler–Süsstrunk M; loud = % of pixels with channel spread ≥ 70; rail = first 248 px):

| Frame | Rail M | Rail loud | Main M | Main loud |
|---|---|---|---|---|
| Dark, before | 28.6 | 2.91% | 20.9 | 1.78% |
| Dark, round 1 | 17.3 | 1.47% | 16.9 | 1.16% |
| **Dark, round 2** | **12.5** | **0.64%** | **14.7** | **0.77%** |
| Light, before | 23.0 | 2.64% | 19.1 | 1.74% |
| Light, round 1 | 17.1 | 1.45% | 15.7 | 1.17% |
| **Light, round 2** | **12.2** | **0.63%** | **13.3** | **0.78%** |

**Reading the numbers:**
- The rail is now calmer than the content (M 12.5 against 14.7), which meets the §f target "rail ≤ main + 1".
- The loud-pixel share is a quarter of today's, and it meets the original §f target of rail ≤ 1.0%.
- Whatever colour remains is, almost entirely, the needs and the Apps that need you.

---

## (d) Per-element remap (audit IDs)

| ID | Element | After | Level rule |
|---|---|---|---|
| H1 | This Mac dot + "Plenty of room" | Neutral faint dot, static, muted text. Amber dot at warning pressure (L2), red dot plus word at critical (L3). | L0 by default |
| H2 | "7 to clean up" | **Ghost button**: muted text + broom icon, no tint or border. A neutral ring when heavy (L2: holds ports or > 1 GB). | L1 |
| H3 | Header New session | Keep as is: accent text with an accent border, no fill. This is the app-wide entry. | – |
| H4 | Bell count | Neutral `--color-count` badge at L0–L2, red only at L3. | rule 4 |
| H5 | Floor button | Unchanged. | – |
| H6 | Avatar | `surface-3` disc, ink initial. | L0 |
| R1 | Brand mark | Accent, no glow. | – |
| R5 | Lens tabs | Selected tab: `surface-3` fill + ink text (no accent). The accent is for actions and focus. | L0 |
| R6 | Needs you header | **Kept loud**: alert icon + label + count of L2+L3. Amber if the highest level is L2, red if L3, muted with no icon if only L1 (reviews). Reviews are listed last, under a faint "To review" label. | rule 4 |
| R8 | Need-row corner dot | App colour, unchanged, and now the **only** meaning of a corner dot anywhere (see M28). | identity |
| R9 | Need-row state word | Amber (L2) or red (L3), semibold. "Done, review it" becomes muted with a ✓ success glyph (L1). "Probably …" gets the hollow ring. | (a) |
| R10 | ↗ | Shown on hover, focus or the selected row. The `G` key and the menu still work. | progressive disclosure |
| R11 / R20 | Row titles | Rail: medium ink-2. The selected row and Needs-you rows: semibold ink. | weight budget |
| R12 | Open now header | Muted semibold, **no icon, no cyan**. The same treatment as the other section heads (Pinned, Inactive apps). | L0 |
| R14 | App group header | App icon + App colour dot + profile (semibold ink) + app name (muted). The kind badge shows only where two Apps share an icon. | identity once |
| R15 | Group header `⚠ n` chip | `n` in ink + 6 px amber dot (L2) or red `n` (L3). No pill, tint or icon. Clicking it still unfolds the group. | rule 1 |
| R17 | Row left edge in the App colour | **Removed.** The group's indent guide becomes a 1 px `divider`. The App is already named by its header. ("Say a thing once", D-69 "one colour, its App's dot".) | identity once |
| R18 | Row state dot | Working: static calm-live dot. Idle and ended: faint. Needs (in Open now): the amber mark. Review: success ✓. | (b) |
| R19 | Row state word | Muted for L0 and L1. **In Open now, a need's word is ink-2 with the amber mark** (the loud copy is in Needs you, D-47 amendment, see below). | rule 1 |
| R21 | "18% left" / heavy | L1 at 11–25%: muted + ⌁ glyph. L2 at ≤ 10% on a live session: amber. | (a) |
| R22 | git dirty `· 3●` | Muted (L1). | L1 |
| R23–R24 | Accent links in the rail | ink-2, underline on hover. Accent only on focus. | accent budget |
| R25 | Folded `!n` | Same as R15. | rule 1 |
| R26 | Project lens state glyphs | Muted glyphs for L0 and L1, amber or red only for L2 and L3. | (a) |
| R28 | Project header colour ring | A neutral count. The Apps show on the rows (D-69: other groupings use their own identifiers). | identity |
| R29 | Rail-foot New session | Bordered neutral (ink text, terminal icon). The header keeps the accent. | one accent per action |
| R33 | Rail notices | Same level rules as rows (a cap or sign-in is L2, the word amber once). | (a) |
| M1 | Title | Counts L2+L3 ("2 sessions need you"). With an L3 item: "1 session stopped · 2 need you". Reviews (L1) move into the muted counts line ("1 to review"). | (a) |
| M4 | App tiles | Running: surface + hairline, **no shadow**. Not running: **no dashed border**: `canvas-hi` surface, muted text. | borders |
| M5–M6 | Tile icon, App dot, profile | Unchanged (identity once per tile). | – |
| M7 | Tile `⚠ 1 needs you` | "1 needs you" in ink-2 + amber dot (red text at L3). No pill, tint or icon. | rule 1 |
| M8 | Kept-alive chip | A muted replay glyph + "kept alive", no tint. | L1 |
| M9 | Paused or lost App | Paused: muted (L1). Lost while sessions were open: amber (L2). | (a) |
| M11 / M24 | Runway meters | Healthy: `ink-2` at 55% opacity in dark (`#848994`, 4.15:1 against the `surface-3` track) and 65% in light (`#6d737e`, 3.92:1). Both clear the 3:1 non-text minimum. Low: amber. Out with live work: red. **The accent is no longer used for meters.** | (a) |
| M13 | Need chips | No outer box. Selected chip: surface-3 + ink. Counts stay. | borders, accent |
| M15 | First need's state word | Amber (L2) or red (L3). | (a) |
| M16 | "Open its page ↵" | **Stays the one filled accent button.** | rule 3 |
| M19 | Other need rows | Dividers become 8 px spacing. Each row's state word is loud once (it is the Home surface's copy). | rule 1 |
| M20 | Their buttons | **Ghost** buttons (ink-2 text, border on hover or focus). The ↗ shows on hover or focus. They stay one click. | borders |
| M21 | Severe icon tile | L3: red glyph on the L3 tint. L2: amber glyph, no tile fill. Sign-in: neutral. | rule 2 |
| M22 | Runway card | Account dividers use the `divider` token. | borders |
| M23 | Account discs | Unchanged (D-69 allows Account colour on the Runway). | – |
| M25 | "Out" pill | L3 (a live session is on it): red word + ⊘, no pill. L1 (nothing running on it): muted "Out · resets 4:13 AM". | (a) |
| M26 | "Measure its limits ›" | ink-2 link. | accent budget |
| M27 | Window-access card | "Allow" becomes **bordered** (secondary). The card's tint goes, leaving a plain surface. Keyboard: ⌘↩ still allows when focused. | rule 3 |
| M28 | Pick-up rows' grey corner **state** dot | **Removed.** The section heading says the state ("ended"). The corner position is now App identity only (fixes root cause 6). | – |
| M29 | Since you last looked | Group heads (Crashed, Needs you) take their level's colour on the **heading only**. Sub-cards lose their borders and become rows. | rule 1 |
| M31–M32 | Page wash, card sheen | Removed (wash). Sheen kept only on the elevated header. | decoration |

### The one change that needs a ruling

**D-47 point 2** says a session that needs you "stays in its place in Open now, its state in amber". The proposal keeps the rest of D-47:
- the row stays in its place;
- its word still says "Asks you";
- it is still counted once.

It changes only the colour: in Open now the word becomes ink-2 with an amber mark. The loud copy is the one in Needs you, one line above.

The maintainer records this as an amendment D-number. Under the D-number rule, check every branch for the next free number first.

**The count rule, which keeps "one queue, one badge".** Deep/11 §1.1 and AJ-E3104 require the rail's Needs you number, the bell and the Home title to show the same number. Under this proposal all three count **L2+L3** only. "Done, review it" items stay listed at the end of Needs you, under a faint "To review" label, but are not counted, because they are L1. The numbers still agree everywhere. They now mean "things waiting on you", not "things to look at". I make that change as a design call under "Decide by the goal" and report it with the D-47 amendment.

Everything else is a design call under "Decide by the goal".

---

## (e) Modes: recommendation

| Option | For | Against |
|---|---|---|
| **1. One calm default (the attention budget above)** | Fixes the default, which is what over 95% of users keep (Spool; NN/g: users "rarely — if ever — customize"). One grammar for the legend, docs, screenshots and tests. Meets "design for the roadmap" and "no stopgaps". | Users who liked the brighter look lose it. Mitigated: every fact is still visible, and only its loudness changes. |
| **2. Calm/focus mode + "Classic" mode** | Lets the maintainer keep today's look as a choice. | The default problem stays for whoever ships with Classic, or the default becomes calm and Classic is dead weight. Two attention grammars means a forked legend. It doubles the Home, rail and status baselines on top of 2 themes × 3 densities (the gallery is already 1,974 PNGs), and "validate every combination" (D-62) applies to it. It says colour is a taste when the problem is a priority bug. |
| **3. Density settings** | Already exist (Comfortable, Compact, Minimal, D-25). They trade **space**, which people do legitimately choose. | They don't touch priority, so they don't fix the complaint. |
| **4. A "focus" filter** (show only L2 and L3) | Useful as a *view*, not a theme. It already exists as the rail's View › Only and the Needs-you lens. | Not a colour mode. Keep it as it is. |

**Pick Option 1: one calm default.**
- Keep the existing density setting as it is.
- Honour the OS: **Reduce motion** is already honoured. Add **Increase contrast** (`prefers-contrast: more`), which raises dividers to hairline and hairline to faint and turns ghost buttons into bordered ones, with no setting of our own.
- During the 0.5 beta only, keep a hidden developer flag (`localStorage aij.attention = "classic"`). It serves the side-by-side walks and the maintainer's own comparison, and it is deleted before 0.5 goes Stable. It is not a user setting, has no Settings UI and no baselines.

If the maintainer still wants a user-facing choice after living with the calm default, the honest version is a single "Emphasis: Standard / Strong" switch. Strong would *raise* L2 to a tint, so it adds emphasis and never brings back the repetition. It would be a 0.6 item, decided on evidence from the walks.

---

## (f) How to validate

Model tiering applies: Sonnet runs the walks, captures and testers, Haiku does counts and formatting, and Opus verifies every PASS or FAIL before it is recorded.

1. **Contrast and axe (gate, both themes).**
   - Run `ui/tests/visual/contrast.spec.ts` with the new values. Add the count badge pairs. Add an APCA assertion for dark: warning ≥ Lc 60 and danger ≥ Lc 52 on every surface.
   - Run the `scripts/a11y_gate.sh` axe gallery.
   - Re-run the `lib/mark-hues` tests (ΔE ≥ 0.12, hue gaps).
2. **Attention-budget test (new, Playwright, fixtures `allStatuses` + a "typical" fixture + a "flood" fixture).** On Home and the rail, read the computed colours. Assert:
   - (i) elements painted warning or danger ≤ (distinct L2/L3 items) per surface, plus the Needs you count;
   - (ii) zero amber fills;
   - (iii) ≤ 1 `danger-soft` block per surface;
   - (iv) exactly one filled accent button;
   - (v) `document.getAnimations()` has no infinite iterations in `[data-testid=rail]` or `[data-testid=home]`;
   - (vi) rows carry a `data-level` attribute that matches `attentionLevel()`.

   This turns the budget into a regression gate. Without it the noise will grow back.
3. **Colourfulness metric (scratchpad `tools/colourful.py`, cheap, objective).** Today's typical Home measures rail M 25.3 with 2.41% loud pixels, and main M 18.5 with 1.34%. **Targets with 3 needs:**
   - rail loud ≤ 1.5%;
   - main loud ≤ 1.2%;
   - **rail M ≤ main M + 1**, so the rail sits at or below the content;
   - the all-clear captures unchanged (rail 0.44%).

   Most of what stays loud is the real App icons, which are identity and stay by the run-time icons ruling.

   **Evidence from the mock.** The hand-built "before" in `mock.html` reproduces the real captures closely. I measured it in headless Chrome, with the frames cropped:

   | | Theme | Rail M | Rail loud | Main M | Main loud |
   |---|---|---|---|---|---|
   | Before | dark | 29.0 | 3.01% | 20.9 | 1.78% |
   | After | dark | **17.5** | **1.51%** | **17.2** | **1.20%** |
   | Before | light | 23.4 | 2.76% | 19.1 | 1.75% |
   | After | light | **17.4** | **1.50%** | **16.0** | **1.21%** |

   That is a 26–40% drop in colourfulness, with the rail no longer louder than the content.
4. **Squint test.** Blur each capture by 8 px. A reviewer lists what is still identifiable, and it must be exactly the L2/L3 items, the title and the primary button.
5. **Glance test (5 s and 10 s, after Kuric 2024).** Sonnet persona testers see the capture for 5 s, then for 10 s, in both themes. They are asked:
   - "How many things need you?"
   - "Which first?"
   - "Is anything broken?"
   - "What is running fine?"

   **Pass:** ≥ 4 of 5 personas answer the first three correctly. On the flood fixture (8 needs and 1 crash), they must name the crash first. Run the same test on the current build as the baseline.
6. **Colour-vision simulation.** Run protan, deutan and tritan filters on the captures before and after. L2 and L3 must stay distinct by **shape** (filled dot or ring vs ✕ glyph plus tint), not by hue.
7. **Persona walks by hand ("Test by hand", "Persona-based validation").**
   - Personas: the multi-subscription vibe-coder (8 sessions, 3 Apps), a newcomer (1 App), and a heavy day (14 open, 7 needs, 1 crash).
   - Click every element. Score works, speed, clarity, layout and feeling.
   - Low scores go to the maintainer, and bugs are fixed at once.
   - The "Design-quality review" critique covers every changed screen.
8. **Review page** with before/after pairs and "what changed" notes ("Screenshot notes say what changed"). That page is the record.

---

## (g) Phased plan

This plan respects "one release, not many", "design for the roadmap" and the release-scope gate, which belongs to the maintainer. Each step builds and is committed separately ("shared tree always builds").

### 0.5 — Phase A: tokens and quieting (small, low risk; about 1 day of Opus code + 1 Sonnet walk batch)

- `styles.css`: the four dark status values, `--color-danger-soft`, the `divider` and count tokens, the level aliases. Remove the body washes.
- `lib/rail-items.ts` `STATE_TONE`: Working's word becomes muted. Remove `animate-breathe` from `SessionRow` and `.dot-live` from `MachineStatusLine`.
- `PlaceBoardParts.tsx` SECTION: Open now becomes muted with no icon. Needs you takes the level colour.
- `PlaceHeader.tsx` needs chip, `rail-groups.tsx` `FoldedNeeds` and `HomeApps.tsx` needs chip: neutral count + level mark.
- `NotificationBell.tsx` count by level. `MainNav.tsx` clean-up becomes a ghost button. `UserMenu.tsx` avatar neutral. `RailHeader.tsx` glow removed.
- Rail-foot New session neutral. Window-access "Allow" secondary.
- Row titles medium ink-2 in the rail.
- Legend text updated (`IndicatorLegend`: drop "A row's left edge is its app's colour").
- One baseline regeneration at the end of A+B, not twice ("Walk more per RC").

### 0.5 — Phase B: the attention model in one place (medium; about 2–3 days of Opus code + walks)

- `lib/attention-level.ts`: `attentionLevel(item): 0|1|2|3`. It is the **single source**, adapter-agnostic, fed by `RailItem.need` / `state` / `probably`, runway, the machine status and notices. The rail, Home, the bell, the tray and the legend all read it. This follows the architecture standard: no per-tool branches.
- The L3 split:
  - failed and crashed become red;
  - a limit with no room becomes red;
  - Out with a live session becomes red;
  - review becomes L1;
  - probably-states get the hollow ring.
- Row edges removed. Corner dot = App only (Pick-up state dot removed). ↗, pin and drag show on hover, focus or the selected row.
- Home: ghost row actions, dividers become spacing, no chip box, App tiles without shadows or dashed borders, meters neutral.
- The flood rule (> 5) in Needs you and in the Home title.
- The D-47 colour amendment, once the maintainer rules.
- The attention-budget Playwright test (§f.2), and the colourfulness check in the visual job.

### 0.5 if the scope gate allows, otherwise 0.6 — Phase C: the rest of the app

- The same grammar on the Projects, Accounts, Usage (except chart data colours), Tasks, Pools and Session page headers, and on the Floor's signal mapping (its people already show states by face, so this means aligning its colour accents to L2 and L3).
- Tray icon states and notification sounds tied to L3 only (and L2 if the user's existing notification settings ask).
- Guide page "What the colours mean". PRINCIPLES.md gains "15. An attention budget" with the rules in (a).
- Windows (0.6) inherits everything through the tokens and `attentionLevel()`.

### Not proposed

- A user-facing colour mode (see e).
- Changing App or Account mark colours.
- Changing text levels or surfaces.
- Hiding information.
