# AI Juggler design principles

Written 2026-10-30 for 0.4. It answers the maintainer's question that day: "Have you actually researched how to design well functional and usable designs?"

Code references are to `release/0.3` at `deb209e39`, which is what the maintainer runs. Paths are relative to `ui/src/` unless they start with `docs/` or `crates/`.

The worked example (New session) and the wireframes are on the review page part `design-part.html`.

Each principle has four parts:
- **Rule:** the principle in one line.
- **Why:** the research behind it, with a link.
- **Here:** a do and a don't from our own screens, with file:line.
- **Check:** how a reviewer tests a change against it.

At the end there is a checklist that a reviewer scores from 1 to 5.

These principles extend the screen spec's own (`docs/review/deep/11-ux-redesign-screen-spec.md` §1: calm supervision, glanceable, keyboard-first, progressive disclosure). They don't replace them. SPEC §9 and the wireframes still decide *what* a screen holds. This file decides *how it reads*.

---

## The principles in one line each

1. **Serve the goal, and fit the whole.** Every feature states its user value, how it fits the other capabilities in both directions, its dependencies, how it works end to end, and what changes for existing users.
2. **Prominence follows frequency.** What people choose most is biggest, first and in colour. What they rarely choose is small, later or folded.
3. **Make the common choice, and say it.** Pre-fill the likely answer, state it in a sentence, and make accepting it one keystroke.
4. **Fold the rare.** Advanced and occasional options sit one clearly labelled step away, never on the default path.
5. **Say a thing once.** No row, card or column repeats what its heading, neighbour or tab already says. Unknown values stay silent.
6. **Distinct areas.** Areas separate by surface, spacing and a heading, so the eye can tell where one ends without reading.
7. **A colour hierarchy, not one grey band.** Four text levels and four surfaces, each a visible step apart. All of them clear WCAG AA, and APCA in dark mode.
8. **Headings, labels and values look different.** A heading names an area, a label names a field quietly, and a value is the loudest text in its row.
9. **One primary action per view.** Exactly one filled accent button, and it names what it does. Return or ⌘↩ triggers it.
10. **Pickers show suggested, then All, with search when long.** The likely few first, the full list after, and type-to-filter once a list passes about 8.
11. **Lists show active first, then search, filters and pages.** What's live or needs you leads, and every growing list is searchable and paged.
12. **Every object has a door; everything related links.** A name you can see is a link to that thing's page, and each page links to everything related.
13. **Plain words, one word per thing.** Account means a subscription, App an app or profile, Project a code folder, Session a conversation. No internal words.
14. **Empty, loading and unknown states teach once.** One empty state per area, which says what fills it and links to that. Never a wall of "nothing" boxes.

---

## Why these, and what good tools do (sources)

Every claim below was checked on the linked page on 2026-10-30.

**Hierarchy**
- NN/g, [Visual Hierarchy in UX](https://www.nngroup.com/articles/visual-hierarchy-ux-definition/):
  - Hierarchy comes from contrast with the background and neighbours, not from the colour itself.
  - Use about three sizes, and make only one or two elements large.
  - Tight spacing groups things and whitespace separates them. Borders and backgrounds should be used sparingly. Check with the squint test.
- Refactoring UI, [Building your colour palette](https://refactoringui.com/previews/building-your-color-palette): greys make up most of a UI, and it needs 8–10 grey shades. The book's hierarchy chapter ([refactoringui.com](https://www.refactoringui.com/)) includes "De-emphasize to emphasize" and "Labels are a last resort".

**Defaults and disclosure**
- NN/g, [The Power of Defaults](https://www.nngroup.com/articles/the-power-of-defaults/): people tend to stick to defaults, so the default *is* the experience for most users. Pre-fill the most likely value.
- NN/g, [Progressive Disclosure](https://www.nngroup.com/articles/progressive-disclosure/):
  - Defer rare or advanced features to a secondary step.
  - The initial view holds what most people need, and the way to the rest is clearly signposted.
  - Decide what goes where by frequency of use.
- NN/g, [8 Design Guidelines for Complex Applications](https://www.nngroup.com/articles/complex-application-design/): reduce clutter without reducing capability, make important information stand out by removing decoration, and surface faster methods in context.

**Choice**
- Laws of UX, [Hick's Law](https://lawsofux.com/hicks-law/) and [Choice Overload](https://lawsofux.com/choice-overload/): more options means slower, worse decisions. Highlight the recommended option, show what's relevant now, and give narrowing tools (search, filters).
- NN/g, [Website Forms Usability](https://www.nngroup.com/articles/web-form-design/): list the most common values first.

**Recognition over recall**
- NN/g, [10 Usability Heuristics](https://www.nngroup.com/articles/ten-usability-heuristics/):
  - #6: keep options visible instead of making people remember them.
  - #7: accelerators that stay out of novices' way.
  - #8: aesthetic and minimalist design, because irrelevant content competes with what matters.
- NN/g, [Memory Recognition and Recall](https://www.nngroup.com/articles/recognition-and-recall/).

**Grouping**
- Laws of UX, [Proximity](https://lawsofux.com/law-of-proximity/), [Common Region](https://lawsofux.com/law-of-common-region/), [Similarity](https://lawsofux.com/law-of-similarity/) and [Chunking](https://lawsofux.com/chunking/).
- NN/g, [Proximity](https://www.nngroup.com/articles/gestalt-proximity/): whitespace separates groups, and items far apart get missed.
- NN/g, [Common Region](https://www.nngroup.com/articles/common-region/): a shared background makes a group, but too many borders add clutter and false stopping points.
- NN/g, [Similarity](https://www.nngroup.com/articles/gestalt-similarity/): reserve one distinct colour for the primary action, because buttons of the same colour read as equally important.

**One primary action**
- Apple HIG, [Buttons](https://developer.apple.com/design/human-interface-guidelines/buttons): "Keep the number of prominent buttons to one or two per view". Give the most likely action the prominent style. On macOS the default button answers Return. Never make a destructive action the default.
- NN/g, [Button States](https://www.nngroup.com/articles/button-states-communicate-interaction/) describes primary, secondary and tertiary emphasis. [OK–Cancel or Cancel–OK?](https://www.nngroup.com/articles/ok-cancel-or-cancel-ok/) says to follow the platform: on Apple, the confirming button goes last.

**Desktop density**
- Apple HIG, [Designing for macOS](https://developer.apple.com/design/human-interface-guidelines/designing-for-macos): large displays show more content in fewer nested levels, keyboard and pointer come first, and density should stay comfortable "that doesn't make people strain".

**Colour in dark UIs**
- Apple HIG, [Dark Mode](https://developer.apple.com/design/human-interface-guidelines/dark-mode): base and elevated backgrounds, with elevated ones brighter. Contrast no lower than 4.5:1, and strive for 7:1 for small text.
- Apple HIG, [Color](https://developer.apple.com/design/human-interface-guidelines/color): never rely on colour alone.
- Google, [Design a dark theme with Material](https://codelabs.developers.google.com/codelabs/design-material-darktheme):
  - The base surface is #121212, and surfaces lighten as they rise.
  - Text emphasis is 87% (high), 60% (medium) and 38% (disabled).
- [Material 3 in Compose](https://developer.android.com/develop/ui/compose/designsystems/material3): a more raised surface takes "a more prominent tone".
- [material-web colour roles](https://github.com/material-components/material-web/blob/main/docs/theming/color.md): surface-container-lowest, -low, -container, -high and -highest.
- W3C, [1.4.3 Contrast (Minimum)](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html): 4.5:1 for text and 3:1 for large text.
- W3C, [1.4.6 Contrast (Enhanced)](https://www.w3.org/WAI/WCAG22/Understanding/contrast-enhanced.html): 7:1.
- W3C, [1.4.11 Non-text Contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html): 3:1 for what identifies a control and its state.
- APCA, [APCA in a Nutshell](https://git.apcacontrast.com/documentation/APCA_in_a_Nutshell):
  - Lc 90 is preferred for body text, Lc 75 is the body minimum, Lc 60 for other content text, Lc 45 for headlines and Lc 30 for disabled text.
  - It says WCAG 2's ratio overstates contrast for dark colours. APCA is a WCAG 3 candidate, not a standard, so we use it *in addition to* AA.

**Scanning and words**
- NN/g, [Layer-cake scanning](https://www.nngroup.com/articles/layer-cake-pattern-scanning/): distinct, descriptive headings that front-load key words.
- NN/g, [Data Tables](https://www.nngroup.com/articles/data-tables/): frozen headers, row-tracking aids, and related columns kept together.
- NN/g, [Rewriting for Brevity](https://www.nngroup.com/articles/rewriting-content-brevity/): cut restated ideas.
- NN/g, [Info Tips](https://www.nngroup.com/articles/info-tips-bad/): tips that repeat visible labels erode trust.
- NN/g, [Information Scent](https://www.nngroup.com/articles/information-scent/): people choose where to go next by the label, so labels must be accurate and free of jargon.

**Empty states**
- NN/g, [Designing Empty States in Complex Applications](https://www.nngroup.com/articles/empty-state-interface-design/):
  - Show the system's status (loading, failed or truly empty).
  - Teach what the area is for.
  - Link straight to the task that fills it.

**Pickers**
- NN/g, [Dropdowns](https://www.nngroup.com/articles/drop-down-menus/): avoid long scrolling dropdowns, and let people type when that's faster.
- NN/g, [Listboxes vs. Dropdowns](https://www.nngroup.com/articles/listbox-dropdown/): 5 or fewer options suit radios, and more than 15 need a listbox with search. Use a dropdown when there's a clear default.
- Apple HIG, [Pop-up buttons](https://developer.apple.com/design/human-interface-guidelines/pop-up-buttons): "Provide a useful default selection".
- Apple HIG, [Combo boxes](https://developer.apple.com/design/human-interface-guidelines/combo-boxes): the most likely choices first, plus your own entry.

**What well-regarded desktop tools do, and why it works**

| Tool | What it does | Why it works | Source |
|---|---|---|---|
| Linear | Builds themes in LCH from three inputs (base, accent, contrast) instead of 98 variables. Surface elevation comes from the same scale. The 2024 redesign cut visual noise in the sidebar, tabs and headers, with careful alignment. | Steps that look even to the eye across hues, so surfaces and text levels read as distinct. A contrast setting gives high-contrast themes for free. | [How we redesigned the Linear UI](https://linear.app/now/how-we-redesigned-the-linear-ui) |
| Raycast | Root search learns which result you pick for a query and ranks by "frecency" (frequency and recency). Favourites stay at the top while the search is empty. | The common choice is already first, and typing narrows the rest. Two keystrokes for daily actions. | [Raycast Manual: Search Bar](https://manual.raycast.com/search-bar) |
| VS Code | The Command Palette keeps a most-recently-used list of commands across restarts, and shows a "commonly used" section to new users. Quick Open (⌘P) is separate from commands (⇧⌘P). | Recognition over recall, plus suggested-then-all with search for thousands of commands. | [1.14 release notes](https://code.visualstudio.com/updates/v1_14), [1.75 release notes](https://code.visualstudio.com/updates/v1_75), [User interface](https://code.visualstudio.com/docs/getstarted/userinterface) |
| Arc (now sunset) | Pinned tabs keep frequent sites within reach. Unpinned tabs archive themselves daily. Spaces are separate sidebars for separate areas of life. | It folds the rare *automatically*, and the areas are unmistakable. | [Cure tab overload](https://start.arc.net/cure-tab-overload) |
| Things 3 | "Today" is the one daily hub. Upcoming is for planning. Type Travel jumps anywhere by typing. | One clear home for the frequent job, with the rest one step away. | [Things features](https://culturedcode.com/things/features/) |

---

## 1. Serve the goal, and fit the whole

**Rule.** Every feature and every screen change says five things before it's built:
1. Its **user value**: the job it does for the multi-subscription vibe-coder.
2. Its **fit with the other capabilities, in both directions**: what it uses, and what it changes for them.
3. Its **dependencies**.
4. **How it works end to end.**
5. **What changes for existing users.**

**Why.** It's the maintainer's ruling (2026-10-30): "all of our decisions are to be made serving user value and goal. Every feature … needs to flow together". NN/g's complex-application guidelines point the same way. People keep using "satisfactory (often inefficient)" paths unless the faster path sits in context ([NN/g](https://www.nngroup.com/articles/complex-application-design/)), so a feature that doesn't connect to the flows around it isn't used.

**Here.**
- *Don't:* New session ranks accounts with its own list (`components/RankedRows.tsx:17`). The session page's "Continue on another account" has its own list too (`components/HandoverSection.tsx:132`). The two orders and words can drift, and the user learns two pickers for one decision: "where should this work run?"
- *Do:* one destination picker, used by New session, Switch account… and the limit dialog. The engine is already shared: `lib/room.ts:43` `rankByRoom`, `lib/launch-pick.ts:38` `pickLaunch`.
- See the New session fit section below for the full form.

**Check.** The change's task or PR has a "Fit" paragraph that answers all five points. A reviewer can name every screen it touches and every screen that touches it.

## 2. Prominence follows frequency

**Rule.** Size, position, weight and colour are spent in order of how often, and how urgently, a thing is chosen.

**Why.** Defaults and progressive disclosure, with placement decided by frequency of use ([NN/g](https://www.nngroup.com/articles/progressive-disclosure/)). Highlight the recommended option ([Hick's law](https://lawsofux.com/hicks-law/)). Only one or two elements should be large ([NN/g](https://www.nngroup.com/articles/visual-hierarchy-ux-definition/)). Raycast and VS Code put the frequent and recent first.

**Here.**
- *Don't:*
  - New session opens with "How it runs" and "Start from a saved handover…" *above* "Where do you want to work?" (`screens/NewSession.tsx:262-264`). The rarest choices come first.
  - Home puts "Your apps" full-width above "Needs you" (`screens/Home.tsx:431` before `:433`).
- *Do:* the most common path (last project, usual app) is one stated line and one button. The rare ones go under "More options".

**Check.** List the view's choices by expected frequency. The visual order (squint test) must match. No rare choice sits above or louder than a common one.

## 3. Make the common choice, and say it

**Rule.** Pre-fill the most likely answer, state it as a sentence the user can verify at a glance, and let ⌘↩ or Return accept it. Say *why* it's the pick in at most three words.

**Why.**
- People tend to stick to defaults ([NN/g](https://www.nngroup.com/articles/the-power-of-defaults/)).
- "Provide a useful default selection" ([Apple pop-up buttons](https://developer.apple.com/design/human-interface-guidelines/pop-up-buttons)).
- A default button answers Return ([Apple buttons](https://developer.apple.com/design/human-interface-guidelines/buttons)).
- Our spec's copy is "Start names destination `Start in payments-api on Work Max`" (deep/11 §4.8).

**Here.**
- *Don't:* New session opened from the top bar starts with **no folder** (`screens/NewSession.tsx:51`, `initialDirectory` is `null`). The footer says nothing until you pick one (`:460`). The stated pick is buried in a card among 14 others (`:325-341`).
- *Do:* "Start in **jive-code-graph** on **bookauth-luka**", with "used last here" beside it and ⌘↩ to start.

**Check.**
- Can the user tell what will happen without reading anything but one line?
- Is the default derived from real data (last used, project default, most room), never a guess?
- Does the "why" fit in three words?

## 4. Fold the rare

**Rule.** An option that fewer than about 1 in 10 uses need goes behind one clearly labelled disclosure. The closed disclosure shows its current values ("Runs in its terminal · This Mac"), so nothing hides silently.

**Why.**
- Defer rare features and clearly signpost the way to them ([NN/g, progressive disclosure](https://www.nngroup.com/articles/progressive-disclosure/)).
- Reduce clutter without reducing capability ([NN/g, complex apps](https://www.nngroup.com/articles/complex-application-design/)).
- Arc archives what isn't pinned.

**Here.**
- *Do:* `components/ExpertSetup.tsx:19` is already "collapsed by default so the guided path stays simple". That's the model to follow.
- *Don't:*
  - "How it runs" (`components/RuntimePick.tsx:30-45`) is always shown at the top of New session.
  - The machine advice line "This Mac can take another session · about 2.0 GB a session" is always shown (`screens/NewSession.tsx:342`), even when nothing is wrong.
- *Rule of thumb:* say it when it changes the decision (tight, out of room, not signed in), and stay silent when it's fine.

**Check.** Every element on the default view is needed by most uses, or warns of something that is true right now. Everything else is folded, and the fold's label summarises what's inside.

## 5. Say a thing once

**Rule.**
- No text repeats what its heading, column, tab or neighbour says.
- Unknown or zero values are silent: an empty cell, or one footnote for the group. They're never repeated on every row.
- A choice is never shown twice on one view.

**Why.**
- Irrelevant content competes with what matters (heuristic #8, [NN/g](https://www.nngroup.com/articles/ten-usability-heuristics/)).
- Cut restated ideas ([NN/g, brevity](https://www.nngroup.com/articles/rewriting-content-brevity/)).
- Tips that repeat labels erode trust ([NN/g, info tips](https://www.nngroup.com/articles/info-tips-bad/)).

**Here.**
- *Don't:*
  - Every ranked row says "Not enough use to forecast" (`lib/quotaWindow.ts:93`, drawn at `components/RankedRows.tsx:94`).
  - "Which account?" repeats the ranked list's top row as a card (`screens/NewSession.tsx:322-341`).
  - Projects rows say "Nothing waiting", "Nothing open" and "No open tasks" on every row (`screens/projects/ProjectRow.tsx:95`, `lib/project-summary.ts:90, 104`).
  - History rows say "Ended" under the Ended tab (`lib/session-history-words.ts:16`).
- *Do:* blank means none. The column head says what it is. One footnote, "Room not measured yet for 4 accounts", covers the group.

**Check.** Read each row aloud. If a word is the same on every row, or the same as its header, delete it. Count a choice's appearances on the view: there must be one.

## 6. Distinct areas

**Rule.**
- A view has at most about 3–5 areas.
- Each area is a **surface step** (not just a 1 px border) with a heading.
- Space between areas (24–32 px) is at least twice the space inside them (8–12 px).
- Inside an area, rows separate by spacing or hairlines, not by boxes inside boxes.

**Why.**
- Proximity and common region ([Laws of UX](https://lawsofux.com/law-of-common-region/), [NN/g](https://www.nngroup.com/articles/common-region/)), and too many borders create clutter.
- Chunking into visually distinct modules ([Laws of UX](https://lawsofux.com/chunking/)).
- Apple's base and elevated backgrounds ([Dark Mode](https://developer.apple.com/design/human-interface-guidelines/dark-mode)).
- M3's tonal surface containers ([material-web](https://github.com/material-components/material-web/blob/main/docs/theming/color.md)).

**Here.**
- *Don't:*
  - Today the canvas and a card are 1.05:1 apart in dark (`styles.css:14,16`: `#0a0d13` against `#10141c`), and surface → surface-2 is 1.07:1. Areas show only by a hairline, so everything is "one band".
  - New session draws 6 folder cards, 7 ranked rows and a "Which account?" card, all as equal bordered boxes (`screens/NewSession.tsx:269-341`, `components/RankedRows.tsx:79`).
- *Do:* steps of about 1.11–1.14:1 between levels (tokens below). The chosen thing sits on a tinted surface. Lists sit flat with hairlines between rows.

**Check.** Blur the screenshot (squint test). You should still count the areas, and the count should be 5 or fewer. No card sits inside a card. Borders appear only where a surface step can't do the job.

## 7. A colour hierarchy, not one grey band

**Rule.** Use four text levels and four surfaces, each with one job. **Body sentences are `ink-2`, not `muted`.** `muted` is for labels and meta only. `faint` is for placeholders, disabled text and tertiary timestamps.

**Why.**
- Material's text emphasis is 87%, 60% and 38% ([Google codelab](https://codelabs.developers.google.com/codelabs/design-material-darktheme)).
- Refactoring UI's 8–10 greys ([palette](https://refactoringui.com/previews/building-your-color-palette)).
- Apple asks for at least 4.5:1, and 7:1 for small text ([Dark Mode](https://developer.apple.com/design/human-interface-guidelines/dark-mode)).
- WCAG [1.4.3](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html) and [1.4.6](https://www.w3.org/WAI/WCAG22/Understanding/contrast-enhanced.html).
- [APCA](https://git.apcacontrast.com/documentation/APCA_in_a_Nutshell) for dark mode, where WCAG 2 overstates contrast.
- Linear's LCH themes make even steps.

**What's wrong today, measured.**
- **Dark `muted` is APCA Lc 57–59.** It passes WCAG with 7.3–9.0:1, but it falls below Lc 60 for content text and far below Lc 75 for body text.
- **`text-muted` is used 889 times in screens, pages and components, against 809 for `text-ink`.** Most sentences are muted, so headings, body, labels and values collapse into two greys, and one of them is too dim to read comfortably. That's the "monochrome" the maintainer reported.
- **Dark `faint` is Lc 38–40**, below Lc 45.
- **Surfaces are 1.05–1.10:1 apart** in both themes.

### Proposed tokens

The names stay as they are (D-16). One is added: `ink-2`.

**Dark**

| token | value | job |
|---|---|---|
| canvas | `#0b0e14` | the window: base level |
| surface | `#151a23` | an area or card |
| surface-2 | `#1e2430` | inset inside an area, input, hovered row |
| surface-3 | `#282e3b` | selected row, chip, popover |
| hairline | `#353d4e` | rules between rows. 1.60:1 on surface, decorative (WCAG 1.4.11 asks 3:1 only where a border alone identifies a control) |
| ink | `#f3f5f9` | headings, names, values |
| **ink-2 (new)** | `#d2d8e1` | body sentences, a row's second line |
| muted | `#b0b8c5` | labels, column heads, captions, meta |
| faint | `#8e97a6` | placeholder, disabled, tertiary time |
| accent (fill) | `#5b5bea` | the one primary button. White on it is 5.09:1, Lc 80 |
| accent-hi (text, links) | `#a9a9ff` | links and selected-state text |

**Light**

| token | value | job |
|---|---|---|
| canvas | `#eceef3` | the window: a grey base, so white areas stand out |
| surface | `#ffffff` | an area or card |
| surface-2 | `#f5f6f9` | inset, input, hovered row |
| surface-3 | `#e6e9ef` | selected row, chip, popover |
| hairline | `#d0d5df` | rules (1.47:1 on surface) |
| ink | `#0e1219` | headings, names, values |
| **ink-2 (new)** | `#2c3341` | body sentences |
| muted | `#4f5767` | labels, meta |
| faint | `#5f6777` | placeholder, disabled |
| accent (fill) | `#4f4fd8` | primary button. White on it is 6.10:1, Lc 85 |
| accent (text) | `#4747d1` | links |

### Contrast, computed

Computed with the WCAG 2 relative-luminance ratio, and with APCA 0.0.98G (sRGB coefficients 0.2126729, 0.7151522 and 0.0721750, exponent 2.4, black soft-clamp 0.022, ^1.414), shown as |Lc|. Each cell gives the WCAG ratio, then the APCA Lc. Any APCA calculator reproduces them.

**Dark, proposed**

| text | canvas `#0b0e14` | surface `#151a23` | surface-2 `#1e2430` | surface-3 `#282e3b` |
|---|---|---|---|---|
| ink `#f3f5f9` | 17.7:1 · Lc 101 | 16.0:1 · Lc 100 | 14.2:1 · Lc 98 | 12.5:1 · Lc 97 |
| ink-2 `#d2d8e1` | 13.5:1 · Lc 82 | 12.2:1 · Lc 81 | 10.8:1 · Lc 80 | 9.5:1 · Lc 78 |
| muted `#b0b8c5` | 9.7:1 · Lc 63 | 8.7:1 · Lc 62 | 7.8:1 · Lc 61 | 6.8:1 · Lc 59 |
| faint `#8e97a6` | 6.6:1 · Lc 45 | 5.9:1 · Lc 44 | 5.3:1 · Lc 43 | 4.6:1 · Lc 41 |
| accent-hi `#a9a9ff` | 9.0:1 · Lc 60 | 8.1:1 · Lc 59 | 7.3:1 · Lc 58 | 6.4:1 · Lc 56 |

Surface steps: canvas → surface 1.11:1, surface → surface-2 1.12:1, surface-2 → surface-3 1.14:1.

**Light, proposed**

| text | canvas `#eceef3` | surface `#ffffff` | surface-2 `#f5f6f9` | surface-3 `#e6e9ef` |
|---|---|---|---|---|
| ink `#0e1219` | 16.2:1 · Lc 95 | 18.8:1 · Lc 105 | 17.4:1 · Lc 100 | 15.4:1 · Lc 92 |
| ink-2 `#2c3341` | 10.9:1 · Lc 89 | 12.7:1 · Lc 99 | 11.7:1 · Lc 93 | 10.4:1 · Lc 86 |
| muted `#4f5767` | 6.3:1 · Lc 75 | 7.3:1 · Lc 85 | 6.7:1 · Lc 80 | 6.0:1 · Lc 72 |
| faint `#5f6777` | 4.9:1 · Lc 68 | 5.7:1 · Lc 78 | 5.3:1 · Lc 73 | 4.7:1 · Lc 65 |
| accent `#4747d1` | 5.9:1 · Lc 73 | 6.8:1 · Lc 83 | 6.3:1 · Lc 77 | 5.6:1 · Lc 70 |

Surface steps: canvas → surface 1.16:1, surface → surface-2 1.08:1, surface-2 → surface-3 1.13:1.

**Today, for comparison**

| dark today | canvas `#0a0d13` | surface `#10141c` | surface-2 `#161b26` | surface-3 `#1d2331` |
|---|---|---|---|---|
| ink `#e8ecf3` | 16.4 · Lc 95 | 15.6 · Lc 95 | 14.5 · Lc 94 | 13.3 · Lc 93 |
| muted `#a8b1c0` | 9.0 · Lc 59 | 8.5 · Lc 59 | 8.0 · Lc 58 | 7.3 · Lc 57 |
| faint `#848c99` | 5.7 · Lc 40 | 5.4 · Lc 40 | 5.1 · Lc 39 | 4.6 · Lc 38 |

Today's dark surface steps are 1.05, 1.07 and 1.10:1. Today's light theme is fine for text: muted is Lc 78–90. Its surfaces are 1.04–1.10:1 apart, and the canvas `#f7f8fb` is almost white, so the white cards don't stand out.

### What every level clears

| level | WCAG 2.2 | APCA | Apple |
|---|---|---|---|
| ink and ink-2 | AAA (≥ 7:1) on every surface, both themes | dark ink-2 Lc 78–82, close to the preferred body Lc 90 that only ink reaches in dark | clears "strive for 7:1" |
| muted | AA on every surface. Dark ≥ 6.8:1, light ≥ 6.0:1 | about Lc 60 (dark) or above, the level for content text that isn't body | clears 4.5:1 |
| faint | AA (≥ 4.5:1) on every surface, so the existing AJ-E804 test still holds | dark Lc 41–45. For non-essential text only | clears 4.5:1 |

### Type steps

Size steps of 1 px (15/14/13/12, `styles.css:61-73`) don't read as levels. Keep three sizes per view ([NN/g](https://www.nngroup.com/articles/visual-hierarchy-ux-definition/)), and let weight and colour do the rest:

| role | size | weight | colour | example |
|---|---|---|---|---|
| section heading | **16 px** (from 15) | 600 | ink | an area's heading |
| value or name | 14 px | 600 | ink | a project's name, "58% left" |
| body | 14 px | 400 | ink-2 | a sentence, a row's second line |
| label or meta | 12–13 px | 500 | muted | "Folder", "used last here", column heads |

**Check.**
- No body sentence uses `text-muted`.
- Each area has exactly one ink heading.
- Grep a changed file for `text-muted` and make sure each use is a label or meta.
- Run the contrast test (AJ-E804) against the new values.
- Look at the screen in grayscale: four levels should be visible.

## 8. Headings, labels and values look different

**Rule.**
- **Heading:** names an area. 16 px semibold ink, sentence case, one per area.
- **Label:** names a field. 12–13 px medium muted, never louder than its value.
- **Value:** the thing you came for. 14 px semibold ink, `tabular-nums` for numbers.
- Where the value is self-explanatory ("58% left"), drop the label: "Labels are a last resort" (Refactoring UI).

**Why.** Layer-cake scanning reads headings, then values ([NN/g](https://www.nngroup.com/articles/layer-cake-pattern-scanning/)). De-emphasize to emphasize ([Refactoring UI](https://www.refactoringui.com/)).

**Here.**
- *Don't:* the session page's "Where it runs" (`pages/session/WhereItRuns.tsx:28-35`) shows App, Profile, Account, Model and Folder at 13 px. Labels and values differ only by grey, and none of the values is a link.
- *Do:* labels muted at 12 px, values ink at 14 semibold, each value a link (principle 12).

**Check.** In a screenshot, cover the labels: can you still read the values and know what they are? If yes, the labels may be redundant. If the labels are the loudest thing, the hierarchy is inverted.

## 9. One primary action per view

**Rule.**
- Each view (page, sheet or dialog) has exactly one filled accent button.
- It names its outcome ("Start in jive-code-graph", not "OK" or "Launch").
- It sits last in the footer on macOS and answers Return or ⌘↩.
- Everything else is secondary (surface button) or tertiary (text link).
- A destructive action is never primary.

**Why.**
- [Apple buttons](https://developer.apple.com/design/human-interface-guidelines/buttons).
- [NN/g button states](https://www.nngroup.com/articles/button-states-communicate-interaction/), and [OK–Cancel](https://www.nngroup.com/articles/ok-cancel-or-cancel-ok/).
- Reserve a distinct colour for the primary action ([NN/g similarity](https://www.nngroup.com/articles/gestalt-similarity/)).

**Here.**
- *Don't:*
  - Home shows a primary "Sign in" and a primary "Allow" (for the permission ask, `lib/bridge.ts:1203`) on one view, plus the top bar's primary "New session".
  - The project page's "New session here" is secondary (`pages/ItemHeader.tsx:266-268`), although it's that page's most common act.
- *Do:* the top bar's New session is the app-wide primary. A page's own primary is its most frequent act, and a permission ask is a quiet secondary.

**Check.** Count the filled accent buttons in a screenshot of the view. There must be exactly one (the top bar's New session is chrome, and counts only on Home).

## 10. Pickers show suggested, then All, with search when long

**Rule.**
- Every chooser opens on the likely few: **Suggested** (the pick and why), then **Recent** or **Used here**, then **All**.
- Lists of more than about 8 get a search field that has focus on open. More than 15 is a listbox with search, never a scrolling dropdown.
- Unavailable options stay visible, greyed, with their reason.

**Why.**
- Most common first ([NN/g forms](https://www.nngroup.com/articles/web-form-design/)).
- [Listbox vs dropdown](https://www.nngroup.com/articles/listbox-dropdown/) thresholds.
- Avoid long dropdowns and grey out what's unavailable ([NN/g dropdowns](https://www.nngroup.com/articles/drop-down-menus/)).
- Apple's combo box puts likely choices first.
- VS Code's MRU and Raycast's frecency.

**Here.**
- *Don't:* New session's folder grid shows exactly 6 recents (`screens/NewSession.tsx:193`) with no search and no "All projects". On the real setup, a seventh project is only reachable through Finder.
- *Do:* the folder step opens on Recent (5), then Pinned, then All projects (n) with search and paging, then Choose folder…

**Check.** Open the picker with real data (24+ projects, 7+ apps). The common choice is visible without scrolling, and any choice is reachable by typing 3 letters.

## 11. Lists show active first, then search, filters and pages

**Rule.**
- Rows that are live or need you come first, in their own group.
- Then recent, then the rest, paged or infinite. Never load everything (memory: Instant and paged).
- A search field once the list can pass about 15. Filters appear when they'd narrow something.
- Row anatomy:
  - a leading mark (state);
  - a name (ink);
  - one secondary line (ink-2 or muted);
  - right-aligned values (tabular);
  - actions on hover or focus.

**Why.** [NN/g data tables](https://www.nngroup.com/articles/data-tables/) (frozen headers, row tracking, related columns together). [Choice overload](https://lawsofux.com/choice-overload/) (narrowing tools). Things' Today.

**Here.**
- *Do:* History has search, filters and day groups (`screens/History.tsx`).
- *Don't:*
  - Processes repeats a "Stop…" button on every child row.
  - Tasks shows Project, Pool and Holder filters with nothing to filter (`screens/tasks/TaskBoard.tsx`).

**Check.** With 300 rows the view opens instantly on the first page, and active rows are on top. Every row's actions are reachable by keyboard.

## 12. Every object has a door; everything related links

**Rule.**
- A visible name of an Account, App, Project, Session, Pool, Task or Mac opens that thing's page.
- A page links to everything related to it, both ways (`docs/research/relations-map.md`).
- Links look like links: accent text, or a chevron.

**Why.** Recognition over recall ([NN/g](https://www.nngroup.com/articles/recognition-and-recall/)). [Information scent](https://www.nngroup.com/articles/information-scent/). The maintainer's ruling (memory: View and edit in place, Desktop-first navigation).

**Here.**
- *Don't:* "Where it runs" names App, Profile and Account as plain text (`pages/session/WhereItRuns.tsx:28-35`, audit G11 in `docs/research/entity-model-audit.md`).
- *Do:* the project row's name opens the project. The session header's tool door (`pages/session/HeaderDoors.tsx:48-70`).

**Check.** Click every name on the view. Each opens the right page, not "the first folder's" (audit G1/G5).

## 13. Plain words, one word per thing

**Rule.**
- **Account** = a subscription. **App** = a desktop or CLI app, or one of its profiles. **Project** = a code folder. **Session** = a conversation in an App.
- A list of apps is never titled "Which account?".
- No internal words (pack, target, recipe, instance).

**Why.** Labels are the strongest cue, so be accurate and avoid jargon ([NN/g information scent](https://www.nngroup.com/articles/information-scent/)). The maintainer's model, 2026-10-30, and audit G16 in `docs/research/entity-model-audit.md`.

**Here.**
- *Don't:*
  - "Which account?" lists folders' launch settings (`screens/NewSession.tsx:322`).
  - Every row says "Launch settings: Main" (`:398`), even when each App has only "Main".
- *Do:* "Choose app and account". Account rows hold their Apps, and launch settings show only when an App has more than one.

**Check.** Every noun on the view is one of the four entities (or Mac, Pool, Task), used in its one meaning.

## 14. Empty, loading and unknown states teach once

**Rule.**
- An empty area shows **one** state: what it's for, and the one action that fills it.
- Loading shows the layout as a skeleton or the cached data (memory: Instant and paged).
- Unknown is said once per group, never per row, and is never shown as 0 or a dash in a big gauge.

**Why.** [NN/g empty states](https://www.nngroup.com/articles/empty-state-interface-design/): show status, teach, and give a direct path.

**Here.**
- *Don't:*
  - Tasks with no tasks draws four columns with four filler sentences (`screens/tasks/TaskBoard.tsx:24-27`).
  - An unmeasured account draws a big empty ring with "—" (`components/usage/Runway.tsx:85-86`).
- *Do:* Home's first-run card, "Start your first session", which gives the purpose and one primary (`screens/Home.tsx:421-427`).

**Check.** Load the view with no data, then with partial data. Count the "nothing" messages: there must be one per area at most.

---

## Reviewer checklist (score 1–5 each; ship at ≥ 4 on every line)

| # | Question | 1 = | 5 = |
|---|---|---|---|
| 1 | **Fit:** are user value, two-way fit, dependencies, end-to-end flow and the change for existing users all stated, and true? | not stated | all five, and checked against the other screens |
| 2 | **Prominence:** does the visual order match how often things are chosen? | rare things first or loudest | the common path is the most prominent thing |
| 3 | **Default:** is the likely choice made, stated in a sentence with a 3-word why, and accepted by ⌘↩? | user must assemble it | one line, one key |
| 4 | **Folding:** is everything rare behind one labelled fold that shows its values? | rare always open | nothing rare on the default view |
| 5 | **Once:** does any word, value or choice repeat? | many repeats | none |
| 6 | **Areas:** 5 or fewer areas, surface steps, more space between areas than inside, no box in a box? | one grey band | areas obvious when blurred |
| 7 | **Colour:** ink, ink-2, muted, faint used for their jobs, and contrast tests pass both themes? | body in muted | four visible levels, AA everywhere |
| 8 | **Heading, label, value:** distinct by size, weight and colour? | all alike | values loudest, labels quietest |
| 9 | **One primary:** exactly one filled button, naming its outcome? | 0 or 2+ | 1, named, last, Return |
| 10 | **Pickers and lists:** suggested, then All; search past about 8; active first; paged? | flat list | as the rule |
| 11 | **Doors:** does every visible name open its page, and are related things linked? | dead names | every name a door |
| 12 | **Words:** one meaning per entity word, no internal words? | mixed | clean |
| 13 | **States:** one teaching empty state per area; unknown said once; skeleton or cache on open? | filler walls | as the rule |
| 14 | **Hand check:** walked by hand at 1280 and 1024, both themes, with real-sized data (memory: Test by hand, Design-quality review)? | not walked | walked, screenshots described |

---

## Worked example: New session

The wireframes, with both themes at 1280 and 1024, are on the review page. This section is the text form.

### What's there today, top to bottom (`screens/NewSession.tsx`)

1. How it runs: a picker (`:263`, `components/RuntimePick.tsx:30`).
2. "Start from a saved handover…" (`:264`, `components/OpenPack.tsx:62`).
3. "Where do you want to work?": 6 recent-folder cards plus "Choose folder…" (`:266-313`). No folder is selected when it opens from the top bar.
4. "Ranked for you": one bordered button per launch setting, 7 on the real setup. Each one says "Not enough use to forecast" (`components/RankedRows.tsx:49-111`).
5. "Which account?": a tinted card that repeats the ranked list's top row. It holds its why, room, launch-settings name, model, verdict, Mac advice and Customise (`:322-368`).
6. With Customise open: one more row per launch setting, each with its own Launch → (`:382-428`).
7. Expert setup (`:441-453`).
8. Footer: Close, and "Launch on X" (`:458-470`).

That's about 14 equal boxes before Customise and about 21 after. Three of the four rarest choices are on top. The account choice is shown twice, and its list is titled "account" but lists Apps.

### Proposal

**Opens with** a stated choice and one button. It shows at once, from cache.

> **Start in jive-code-graph**
> ~/Dev/jive-code-graph · worked here 12 min ago · [Change folder]
>
> **Claude Code · bookauth-luka** · *used last here*
> Account Luka Max · 58% left · resets 20:52 · [Change app]
>
> More options: runs in its terminal · this Mac ▸
>
> [Cancel]  **[Start in jive-code-graph ⌘↩]**

- **Folder**, in this order:
  - the folder it was opened for (`initialDirectory`: the project page, a drop, a handover);
  - else the project with the latest `last_activity_ms`. It counts other apps' sessions too, so it's the truest "last used" (`types/generated/ProjectInfo.ts`).
- **App**: `pickLaunch` (`lib/launch-pick.ts:38`) as it is today: this project's default, then your default, then the App used last here (`accounts_used`), then the most room. The why is `pickWhyWords` (`:85`), which is already three words or fewer.
- **Room** is said once, for the **Account** (subscription) the App belongs to. Two Apps on one plan share room, so it's grouped through `accountOfProfile` (`lib/subscriptions.ts:44`).
  - If room is unknown, nothing is said.
  - If it's tight, a warning line appears in place of the meter (principle 4).
- **Mac advice** speaks only when this Mac is short (`components/MachineAdvice.tsx`).

**One step away: Change folder.** The stated block becomes the picker in place. It doesn't open a second dialog.
- A search field has focus: "Search projects or paste a path".
- Then **Recent** (5), **Pinned**, and **All projects (24)**, paged and filtered by typing.
- Then **Choose folder…** (drop anywhere still works).
- Each row has:
  - a project mark;
  - the name (ink 14/600);
  - the path (muted);
  - right-aligned "12 min ago" and a live dot when working;
  - "folder missing" greyed, with Relocate.
- Picking a folder returns to the stated choice and re-runs `pickLaunch` for that project. The why can change ("this project's default").

**One step away: Change app.** The picker is grouped by **Account**:
- One header row per Account: its name, plan and one room meter, "58% left · resets 20:52". Unknown room is silent, with one footnote for the group.
- Under each Account, its **Apps** (Claude Code · bookauth-luka, Codex · ~/.codex). Launch settings show only when an App has more than one.
- Sections:
  1. **Suggested**: the pick, plus the most-room Account if that's a different one, with the reason.
  2. **Used in jive-code-graph**.
  3. **All (7)**, with search once past 8.
  4. **Needs sign-in (2)**, folded, each with Sign in. The sheet stays open, as Req.8 asks today.
- An out-of-room App stays visible, greyed, with "out of room until 20:52" and "Use anyway".
- "Why this order?" becomes a (?) beside the Suggested heading, with `ROOM_ORDER_WORDS` (`lib/room.ts:79`).
- "Ranked for you" and "Which account?" both go away. This picker is the one list.

**Folded: More options.** One row, with the current values in its label. It holds:
- How it runs (`RuntimePick`);
- Runs on, shown in the label only when paired Macs exist (`MachinePicker`);
- Start from a saved handover… (`PackPicker`; on screen it's "handover", never "pack");
- Expert setup (`ExpertSetup`, collapsed inside as it is now).

**Areas:**
1. The stated choice, on accent-soft over surface. It's the only tinted block.
2. More options, a single row on surface.
3. The footer, on surface-2 with a top hairline.

The picker states replace area 1's content. There are no other boxes.

**Box count:**

| state | areas | primary buttons | rows shown |
|---|---|---|---|
| today | about 14 equal boxes | 1, plus up to 7 Launch → buttons with Customise open | — |
| proposed default | 3 | 1 | — |
| proposed folder picker | — | — | 5 to 12 rows, all with search |
| proposed app picker | — | — | 5 to 12 rows, all with search |

### Fit (principle 1)

- **User value.** The daily case is "carry on where I was, on the account I use there": ⌘N, then ⌘↩. Two keys, no reading beyond one line. Choosing another folder or app is one click and three typed letters. Rare needs (handover, terminal-less run, another Mac, env overrides) are still one click away and say what they're set to.

- **Switching.** "Switch account…" and Continue on another account (`components/HandoverSection.tsx`, `pages/session/ContinueOn.tsx`) choose a destination too.
  - The **Change app** picker becomes one shared component, `DestinationPicker`, for New session, switching, and the limit dialog's suggestion (`components/LimitDialog.tsx`). It replaces `RankedRows`, whose own doc already plans that second host (`components/RankedRows.tsx:1-4`).
  - Switching adds what it carries (history, tasks, context) and the switch preflight for missing MCP servers and skills (memory: Switching is a 0.1 core feature). New session never shows those.

- **Room and credits ranking.**
  - Unchanged as an engine: `rankByRoom` (`lib/room.ts:43`) and `pickLaunch`.
  - Changed as a display: one meter per Account, not per App, because Apps on one plan share room.
  - An account through a service (D-34) shows credits left in the same slot.
  - Home's "Best for new work" (`components/AccountRunway.tsx:164`) and the Suggested section must name the same Account. One order, as F17 asked.

- **Projects.**
  - The folder picker reads `ProjectInfo`: `pinned`, `last_activity_ms`, `missing` and `default_account_id`.
  - The app picker offers "Always use for jive-code-graph". It sets the project default (`components/ProjectDefaultAccount.tsx`), which `pickLaunch` already honours first.
  - The project page's "New session here" opens with the folder fixed. It should become that page's primary (principle 9).

- **The App and Account model.**
  - It groups Apps under Accounts and says "app" and "account" in their one meanings, which closes audit G16.
  - Until 0.4 stores Accounts and Apps (`entity-model-audit.md` §0), grouping uses `useSubscriptions` and lists the launchable Apps: the CLI folders with launch settings.
  - The Claude app's own profiles aren't launchable from here today (G3). When they're stored, they join under their Account with "Opens in the Claude app".

- **Cache.**
  - Opens from the cached projects, launch settings and room (memory: Local cache ruling, Instant and paged), so the stated choice is there on the first frame.
  - Fresh reads refresh in the background. **The pick never jumps under the cursor.** If a fresh read would change it, a quiet line says "bookauth-mia now has more room · Use it".
  - Room older than 5 minutes says "as of 12 min ago".

- **Handovers.** Choosing one under More options:
  - adds a third line to the stated choice: "with the handover from *Refactor settings*";
  - takes the folder from the handover when it's on this Mac (`components/OpenPack.tsx:39`);
  - keeps the button as "Start in … with handover".

- **Fleet's other Macs.**
  - With paired Macs, "on This Mac" joins the stated sentence as a changeable token.
  - Choosing another Mac swaps both pickers to that Mac's projects and launch settings (`components/RemoteLaunch.tsx`, `RemoteSessionSheet`). That Mac re-checks everything and answers in its own words.
  - Without paired Macs, nothing about Macs is shown.

- **Dependencies:**
  - `pickLaunch` and `rankByRoom`;
  - `useSubscriptions` and `accountOfProfile`;
  - `quotaDisplayAll`;
  - `machineAdvice`;
  - `runtimeChoices` and folder trust (`FolderTrustAsk`);
  - `usePack`;
  - `fleetLaunchOptions`;
  - `projects` and `launchRecipesList` from cache.
  - No backend change is needed for 0.4's first cut. The shared picker needs the Account grouping, which exists today as `SubscriptionInfo`.

- **What changes for existing users:**
  - ⌘N then ⌘↩ still starts the preselected session. **New:** a folder is preselected from the top bar too (the last project), and the button names it, so a start is never in an unseen folder.
  - "Ranked for you", "Which account?" and Customise are replaced by Change app.
  - "How it runs" and the handover link move under More options.
  - The tests to update: `components/launch-sheet.test.tsx` (`launch-now`, `launch-preselected`, `sign-in-first`), `serde-null-ranked-surfaces.test.tsx`, and the visual baselines for the sheet. They are gallery shots, so stage by name (memory: Playwright rewrites the gallery).

---

## Audit: the other main screens (2026-10-30, preview data at 1280 × 860, dark)

Scores are 1–5 against the checklist, rounded. Each screen lists its biggest violations, with a one-sentence fix.

| Screen | Score | Biggest violations (file:line) | Fix |
|---|---|---|---|
| **Home** | 3 | (1) "Your apps" spans full width *above* "Needs you", even with one app (`screens/Home.tsx:431` before `:433`). (2) Three primaries on one view: "Sign in", "Allow" (permission ask, `lib/bridge.ts:1203`) and New session. (3) Runway, Needs you, Pick up and Your apps are all the same card on a canvas 1.05:1 away, so nothing reads as the area that matters. | Put Needs you first and loudest, and apps as a compact strip. Make the permission ask a quiet secondary row. Use the new surface steps. |
| **Session page** | 3 | (1) The facts line packs tool, where, start time, CPU, memory and process count into one muted run. "Switch account…" and Stop are equal secondaries (`pages/ItemHeader.tsx:250-253`). (2) "Where it runs" repeats the header's profile and project as plain text, with no doors (`pages/session/WhereItRuns.tsx:28-35`). (3) Runway and "Continue on another account" are two cards that both say "Work Max" and "not measured". | Keep three facts in the header and move resources to Processes. Make each "Where it runs" value a link and drop what the header shows. Merge into one "Room" card with Switch inside. |
| **Accounts** | 2.5 | (1) A glossary paragraph on every visit (`screens/Profiles.tsx:171`). (2) An unmeasured account's most prominent spot is an empty ring with "—" (`components/usage/Runway.tsx:85-86`). (3) An account card titled by its state, "Not signed in", with no name. | Move the glossary into a (?). Make unknown room one "Measure its limits" line. Group sign-in needs at the top, with Sign in as the page's primary. |
| **Projects** | 3.5 | (1) Every row says "Nothing waiting", "Nothing open" and "No open tasks" (`screens/projects/ProjectRow.tsx:95`, `lib/project-summary.ts:90,104`). (2) No "Working now" group. With hundreds of projects, active ones are found only by sort. | Leave blank cells blank. Add a Working now group, then Recent, then All, paged. |
| **Project page** | 3 | (1) Four equal stat tiles, including "Tasks 0" and "Accounts used 1", at stat size (`pages/project/Overview.tsx:78-97`). (2) "New session here" is secondary (`pages/ItemHeader.tsx:266-268`). (3) The Activity chart sits above Open sessions, which falls below the fold. | Make tiles speak only when non-zero, and open sessions first. Make "New session here" the page's primary. |
| **Tasks** | 3 | (1) No tasks means four columns of filler (`screens/tasks/TaskBoard.tsx:24-27`). (2) Project, Pool and Holder filters show with nothing to filter. | Use one empty state that teaches ("Agents write tasks here when …") and links to Pools. Fold the filters until there are tasks. |
| **Pools** | 3 | (1) Card thumbnails are mostly empty dot grids: decoration where state should be. (2) "project pool" and "quiet" are filler labels (`screens/pools/PoolsScreen.tsx:175`, `:57`). | Show members as state rows (needs you, working) on the card. Say nothing when it's quiet. |
| **Usage** | 4 | (1) The chart header has three control groups: 5 splits, Tokens or Cost, and table. (2) "By account" splits by profile (folder) names, audit G4/G5. | Use one "Split by ▾" pop-up. Group by Account (subscription), with Apps inside. |
| **Processes** | 3 | (1) Four equal tallies, so "24 running" weighs as much as "7 to clean up" (`screens/processes/ProcessesScreen.tsx:85-88`). (2) "Stop…" repeats on every child row, and the amber "Left behind" marks fill the column. (3) A definition paragraph is always open above the table. | Make "Clean up 7" the page's one primary, with the rest as a summary line. Show row actions on hover. Move the definitions into (?). |
| **History** | 4 | (1) "Ended" on every row under the Ended tab (`lib/session-history-words.ts:16`). | Show only what differs: the branch and the app. |
| **Settings** | 3.5 | (1) Theme and Order take three large radio cards each, where a three-way pick fits a pop-up or segmented control (`screens/settings/GeneralPane.tsx`). (2) The pane descriptions sit *under* their cards, where they read as footers. | Use segmented controls for three-option picks, and keep cards where a preview helps (density). Put the description under the heading. |

**Across all screens:**
1. **Colour.** Dark `muted` (Lc 57–59) carries most body text, and the surface steps are 1.05:1. Adopting the tokens above, plus `ink-2` for sentences, fixes "monochrome" on every screen at once.
2. **Repeated filler.** Make "unknown or zero is silent" a lint: the `plain-words-on-screen` test (`plain-words-on-screen.test.tsx`) can learn to flag the same phrase drawn on every row of a list.
3. **Doors.** Every name is a link. This depends on 0.4's App and Account pages (audit G1, G2, G5).
