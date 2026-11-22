# Audit: what competes for attention on Home and the rail today

**Code:** `origin/release/0.5`. Paths below are relative to `ui/src/`.

**Captures:** I read the gallery fixtures with the Read tool:
- `94-places/home-typical-{dark,light}`
- `94-places/home-busy-dark`
- `95-app-icons/home-1280-dark`
- `99-home-chips/home-1440x900-{dark,light}`
- `99-inactive-apps/home-closed-apps-dark`
- `54-summary/home-1440x900-dark`, the newest, from 2026-11-10
- `01-rail/all-statuses-1440x900-dark`

All names in them are generic fixtures.

**Reference screen for the counts:** `94-places/home-typical-dark` at 1280×800. It shows 3 needs (2 Asks you and 1 Done, review it), 8 open sessions across 4 Apps, 2 Accounts on the Runway and the window-access card.

**Key to the "Kind" column:**
- **H**: a hue (a status or brand colour on text, a dot, an edge or a fill).
- **T**: a tinted or filled background (a chip, a pill, a badge or a filled button).
- **B**: bold or semibold weight on non-heading text.
- **I**: an icon or glyph.
- **R**: a border, frame or divider.
- **M**: motion.
- **Loud** means it claims attention before anything is read: a saturated warm hue, a tint, a filled accent, or a red or amber badge.

## 1. Root causes (what the inventory shows)

1. **One need is shouted 5–6 times.** Take a single "Asks you" session. Today it shows as:
   1. the rail's Needs you row (amber word);
   2. the same row again in Open now (amber word and amber dot, as D-47 requires);
   3. its App group header (amber `⚠ 1` chip);
   4. its Home App tile (amber `⚠ 1 needs you` chip);
   5. Home's Needs card (amber word);
   6. the header bell (red count) and the page title ("3 sessions need you").

   With 3 needs, that is about 16 loud elements for 3 facts.
2. **Every need is the same amber.** `rowState()` in `components/row/SessionRow.tsx` and `sessionState()` in `lib/rail-items.ts` paint every need `text-warning`. "Asks you" (a question), "Probably stuck" (a guess), "Limit reached" (blocked) and "Stopped with an error" (failed) all look alike in the rail. Only Home's first item and `FoldedNeeds` use red for severe cases. That is one priority level where ISA-18.2 expects three.
3. **Normal running is coloured and animated.** "Working" is cyan (`#22d3ee`, Lc 66–69) with a breathing dot (`animate-breathe` / `.dot-live`) on every working row and on the This Mac status dot. That is the brightest hue on screen after amber, and it moves. ISA-101 makes normal running grey.
4. **The alarm colour is the quietest status colour in dark mode.** Danger `#f87171` measures Lc 45–49 and 5.3:1 on surface-3. Warning `#fbbf24` measures Lc 70–73 and live `#22d3ee` Lc 66–69 (see the contrast tables in proposal §c). Today "crashed" is less salient than "working".
5. **Identity colour is repeated.** D-69 says "a row wears one colour, its App's dot". Yet in the rail the App's colour appears as the header dot, as a 2 px left edge on every row (`SessionRow` compact and cozy `border-l-2`, `style={edge}`), and as the corner dot on the Needs-row icon. That is three appearances per row group, and it sits beside the status dot.
6. **The same corner dot means two things.** On the rail and Home Needs rows the dot at an icon's corner is the App's colour. On `SessionRow` `density="list"` (Home's Pick up, a page's list) the same dot is the state's colour (`state.dot`).
7. **The accent is spent on chrome.** The violet accent tints the header New session, the rail-foot New session, the selected lens tab (App), the selected chip ("All 3"), the brand logo (plus a glow), the "Last 2 inactive ›" link, the "Measure its limits ›" links and the avatar gradient. It also fills two buttons on one screen: "Open its page" and "Allow". That breaks PRINCIPLES §9, "one primary action per view".
8. **Borders on everything.** Every Home card has a border and a shadow, every App tile has a border plus a shadow (dashed when not running), the chip row sits in a bordered box, and each Needs row has a divider and a bordered button. The header has four bordered controls. NN/g (common region) warns that this adds "false stopping points". Linear removed dividers that had "quietly proliferated".
9. **The rail is louder than the content.** On the same captures, the Hasler–Süsstrunk colourfulness of the rail is about 1.4–1.9× the main area. The rail measures M 24–28 against main 15–19, and its loud-pixel share is 2.2–2.9% against 0.9–1.4%. Linear's 2026 refresh made its sidebar "a few notches dimmer" for this reason.

## 2. Colourfulness baseline (gallery, measured)

The script is in the scratchpad: `tools/colourful.py`. M is the Hasler–Süsstrunk colourfulness. "Loud" is the % of pixels with channel spread ≥ 70 and max ≥ 60. The rail is the first 264 px.

| Capture | Whole M | Rail M | Main M | Rail loud | Main loud |
|---|---|---|---|---|---|
| 67-home/needs-you 1280 dark | 21.0 | 27.6 | 18.7 | 2.85% | 1.45% |
| 94-places/home-typical dark | 20.2 | 25.3 | 18.5 | 2.41% | 1.34% |
| 94-places/home-typical light | 17.4 | 19.8 | 16.8 | 2.20% | 1.37% |
| 99-home-chips 1440 dark | 18.4 | 28.1 | 15.0 | 2.93% | 0.88% |
| 54-summary/home 1440 dark | 13.9 | 22.6 | 10.7 | 1.98% | 0.35% |
| 67-home/all-clear 1280 light (nothing needs you) | 13.7 | 9.9 | 14.5 | 0.44% | 0.93% |

The all-clear rail (0.44% loud) shows what the rail looks like when nothing is wrong. The same rail with 3 needs reaches about 2.4–2.9%. Most of that rise comes from the repeated amber and the cyan, not from the 3 needs themselves.

## 3. Inventory: header (`AppHeader.tsx`, `MainNav.tsx`, `NotificationBell.tsx`, `MachineStatusLine.tsx`, `UserMenu.tsx`)

| # | Element | State shown | Treatment today | Kind | Loud | Competes with |
|---|---|---|---|---|---|---|
| H1 | This Mac status dot + "Plenty of room" | calm machine | green (`success`) dot, `.dot-live` **breathing**, muted text | H M | – | bell, clean-up pill |
| H2 | "7 to clean up" | leftover processes (housekeeping) | amber-tinted pill, `border-warning/40 bg-warning-soft`, ink text | T R H | **yes** | Needs you, bell |
| H3 | New session (header) | always | accent-tinted button with an accent border and a terminal icon | T R H I | semi | Home's primary "Open its page" |
| H4 | Bell count "3" | notices = needs | **red** filled badge (`bg-danger`) with a surface ring | T H | **yes** | rail Needs you count, page title |
| H5 | Floor split button | always | bordered two-part button with a cube icon and a chevron | R I | – | – |
| H6 | Avatar "S" | always | accent→live **gradient** disc | T H | semi | bell |
| H7 | Nav tabs | current = Home | surface fill on the current tab, medium weight | – | – | – |
| H8 | Header bottom rule | always | hairline | R | – | – |

## 4. Inventory: rail (`PlaceBoard*.tsx`, `PlaceHeader.tsx`, `PlaceRows.tsx`, `row/SessionRow.tsx`, `RailHeader.tsx`, `RailToolbar.tsx`, `WorkflowRail.tsx`, `rail-groups.tsx`)

| # | Element | State | Treatment today | Kind | Loud | Competes with |
|---|---|---|---|---|---|---|
| R1 | Brand mark | always | accent icon with a **violet glow** (`drop-shadow …#7c7cf466`) | H | semi | Needs you header |
| R2 | Edition chip (beta/dev builds) | build | accent-tinted pill | T R | semi | – |
| R3 | Search field | always | bordered | R | – | – |
| R4 | Filter button (+ dot when filtered) | filters | bordered, with an accent dot when on | R I | – | – |
| R5 | Lens tabs App / Project / Account | selected | segmented control in a bordered box, **accent tint + accent text** on the selected tab | T R H | semi | Needs you header |
| R6 | "Needs you n" header | ≥ 1 need | **amber** `IconAlert` + **amber semibold** label + count | H I B | **yes** | everything (correctly) |
| R7 | Needs-you row: App icon | each need | vivid real app icon, 16–20 px | H I | – | R8, R9 |
| R8 | Needs-you row: corner dot | each need | the **App's** colour (8 hues) | H | – | R9 |
| R9 | Needs-you row: state word | asks, stuck, limit, failed, review | **amber semibold** for every kind, review ("Done, review it") included | H B | **yes** | each other: 3 identical ambers |
| R10 | Needs-you row: ↗ go to app | each row | always-visible icon | I | – | – |
| R11 | Needs-you row: title | each | ink semibold | B | – | – |
| R12 | "Open now n" header | always | **cyan** `IconWindow` + **cyan semibold** label | H I B | semi | R6 |
| R13 | "Your order ▾" | always | muted text + chevron | I | – | – |
| R14 | App group header: fold chevron, App icon, App kind badge, App colour dot, profile **semibold ink**, app name muted | always | 3 marks before the name | H I B | – | R15 |
| R15 | Group header needs chip `⚠ n` | needs inside | **amber-tinted pill**, amber semibold, alert icon | T H I B | **yes** | R6, R9 (same needs again) |
| R16 | Group header count | open sessions | muted number | – | – | – |
| R17 | Row: left edge | every row | **2 px in the App's colour** | H R | – | R18 |
| R18 | Row: state dot | Working, Idle, Asks, Done | cyan (**breathing**) / faint / amber / green | H M | semi | R17, R19 |
| R19 | Row: state word | per state | **cyan** "Working", **amber** "Asks you" (again, D-47), green "Done, review it", muted Idle or Ended | H B | **yes** for needs | R9 (duplicate) |
| R20 | Row: title | every row | **ink semibold** on every row | B | – | all titles equally heavy |
| R21 | Row: low runway "18% left" / heavy load | exception | amber text | H | **yes** | R19 |
| R22 | Row: git dirty `· 3●` | dirty | amber number | H | semi | – |
| R23 | "Last 2 inactive ›" | inactive rows exist | accent link | H | – | – |
| R24 | "Also:" line parts | folded views | muted, accent-hi when on | H | – | – |
| R25 | Folded group `!n` (FoldedNeeds) | folded with needs | filled amber or **red** circle `!` + count | T H B | **yes** | R6 |
| R26 | Project lens: state glyphs (✕ ▷ ⏲ ✓ …) | per row | coloured like the state (amber, cyan) | I H | semi | R27 |
| R27 | Project lens: right-hand App mark + App colour dot | per row | letter or icon tile + 8-hue dot | H I | – | R26 |
| R28 | Project lens: project header mixed-colour ring | per project | multi-colour ring of its Apps' colours | H | semi | – |
| R29 | Rail foot New session | always | **accent-tinted** button with an accent border | T R H | semi | H3 (the same action twice) |
| R30 | Rail foot History / Projects buttons | always | bordered icon buttons | R I | – | – |
| R31 | `?` legend | always | bordered circle | R | – | – |
| R32 | Rail right edge, foot top rule | always | hairline | R | – | – |
| R33 | Rail notices (sign-in, cap, agent message) | other needs | amber word, semibold title | H B | **yes** | R9 |

## 5. Inventory: Home main (`screens/Home.tsx`, `screens/home-cards.tsx`, `HomeApps.tsx`, `HomeNeeds.tsx`, `HomeRunway.tsx`, `HomeRecent.tsx`, `SmootherCard.tsx`, `MorningSummary.tsx`)

| # | Element | State | Treatment today | Kind | Loud | Competes with |
|---|---|---|---|---|---|---|
| M1 | Title "n sessions need you" | needs | `text-title` ink | – | – (rightly the top) | – |
| M2 | Legend `?` | always | bordered circle | R | – | – |
| M3 | Counts "8 open · 1 inactive · 1 ended today" | always | ink-2 links | – | – | – |
| M4 | "Your apps" tiles | per App | border + `shadow-card` (running) or **dashed** border (not running), 210 px min | R | – | each other (4–6 boxes) |
| M5 | Tile: App icon | per App | vivid real icon, 32 px | H I | – | M7 |
| M6 | Tile: App colour dot + profile **semibold** | per App | 8-hue dot | H B | – | – |
| M7 | Tile: `⚠ 1 needs you` | App has needs | **amber-tinted pill**, amber semibold, alert icon | T H I B | **yes** | Needs card (same need) |
| M8 | Tile: kept-alive mark | keep alive on | accent-tinted chip + replay icon | T H I | semi | – |
| M9 | Tile: "Not running · …" | not running | muted text; **amber** when paused or lost | H | semi | – |
| M10 | Tile: Open / Start | per App | bordered button | R | – | – |
| M11 | Tile: runway bar (when shown) | per account | accent fill, amber when low | H | – | Runway card |
| M12 | Needs card frame | ≥ 1 need | border, radius-xl | R | – | – |
| M13 | Chips "All 3 · Asks you 2 · To review 1" | needs | inside a **bordered box**; selected chip accent-tinted | T R H | semi | M15 |
| M14 | First need: App icon + corner dot (App colour) | first need | icon + 8-hue dot | H I | – | – |
| M15 | First need: state word, top right | first need | **amber** (or **red** if severe) semibold | H B | **yes** | M7, R9 |
| M16 | First need: "Open its page ↵" | first need | **filled accent** (the page's primary) | T H | **yes** (rightly) | M27 |
| M17 | First need: ↗ | first need | icon button | I | – | – |
| M18 | First need: "Continue in another app…" | first need | bordered button | R | – | – |
| M19 | Other needs: rows | each | icon + App dot, medium title, muted line, **divider** between rows | H I R | – | – |
| M20 | Other needs: action | each | **bordered** "Open its page" + ↗ | R I | – | each other (2–5 boxes) |
| M21 | Severe need's icon tile (`NeedBox` initials) | sign-in, cap, severe | danger / warning / accent soft tile, bold | T H B | **yes** | – |
| M22 | Runway card | accounts | border; account rows divided by hairlines | R | – | – |
| M23 | Runway: Account colour disc | per account | 8-hue disc (allowed by D-69) | H | – | M24 |
| M24 | Runway: meter | per account | **accent fill** when healthy, amber when low | H | semi | M16 (accent again) |
| M25 | Runway: "58% left" / "0% left" / "Out" | per account | ink semibold; amber when low; red **pill** with icon when out | T H B I | **yes** when out | M15 |
| M26 | Runway: "Measure its limits ›" | not measured | accent link | H | – | – |
| M27 | Window-access card "Jump to a session's own window" | permission not granted | tinted card + **filled accent "Allow"** | T R H | **yes** | M16 (second primary) |
| M28 | Pick up where you left off | ended, unreviewed | card border, row dividers, app icons with a **grey state** corner dot, ✓ buttons | R I | – | – |
| M29 | Since you last looked (morning summary) | after a gap | card with sub-cards, each bordered, bold group headings with glyph (✕ ! ▶ ✓ ■) | R I B | semi | Needs card |
| M30 | Keyboard footer | always | muted sentence | – | – | – |
| M31 | Page background | always | violet + cyan **radial wash** (`body` background-image) | H | – | – |
| M32 | Card sheen | cards | 1 px white top highlight (`.sheen`) | R | – | – |

## 6. Count on the reference Home (94-places/home-typical-dark, 1280×800, 3 needs)

I counted the visible elements one by one from the capture, with the code above as the key.

| Area | Hue elements (H) | Tints / fills (T) | Bold non-headings (B) | Icons (I) | Borders / dividers (R) | Motion (M) | **Loud** |
|---|---|---|---|---|---|---|---|
| Header | 5 | 4 | 0 | 4 | 5 | 1 | **2** (clean-up pill, red bell) + 2 semi |
| Rail | 31 | 5 | 14 | 13 | 9 | 2 | **7** (Needs header, 3 amber words, 1 amber word in Open now, 2 `⚠1` chips) |
| Home main | 18 | 9 | 9 | 11 | 17 | 0 | **6** (3 tile chips, "Asks you", filled Open its page, filled Allow) + 1 semi |
| **Total** | **54** | **18** | **23** | **28** | **31** | **3** | **15 loud + 4 semi** |

**The facts that actually need the user: 2 questions waiting.** The third need, a review, is not urgent. There are 0 failures.

So there are about 7 loud elements per urgent fact. The ISA-style target in the proposal is 1 per fact per surface plus 1 primary action: **5 in total** on this screen, with every one of the other ~150 elements still shown, only quieter.

**In the light theme** the counts are the same. The ambers become orange-brown (`#9c4807`) on light tints, and the tints show more against white cards, so the light capture reads just as busy (loud-pixel share 1.54% against 1.56% in dark).

## 7. Things that are right and must survive

- The four text levels (ink, ink-2, muted, faint) are distinct and AA on every surface (contrast.spec.ts). Headings, labels and values already differ, as the "Readable and visual" ruling requires.
- Status is never colour alone. Every state has a word, which meets WCAG 1.4.1.
- The page title already states the one thing that matters ("3 sessions need you"). It is the right L2 summary.
- Snooze already exists (ISA's shelving). Notification mutes already exist (`NoticeMutes`).
- The All-clear state is genuinely calm (rail loud share 0.44%). The target is to look like that whenever nothing is wrong, and to break the calm only for real needs.
