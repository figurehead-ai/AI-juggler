<!-- captures: 18a95f9b72c7 -->
# 69-rail-narrow: the narrow window

Design critique (D-57, AJ-E2814) against the reference screens: the row menus (77-menus), the rail's order menu, Usage and Processes. The baseline note says what changed on purpose; this says whether it is good.

## Checklist

| Check | Finding |
|---|---|
| Grid | The folded column centres every mark on one axis. |
| Hierarchy | The Group and View door first, then the needs count, then the places. |
| Spacing | Marks evenly spaced; the bottom actions grouped. |
| Icons | Every mark is an app's icon with its profile badge; the door is View's own icon. |
| Shared controls | Opened, the list has the shared switch and View. |
| Disabled reasons | Not applicable. |
| Plain words | Each icon names itself on hover and to screen readers. |
| Type sizes | The needs count 12 px; nothing smaller. |
| Both themes | Checked dark and light. |
| 1280×800 and 1024×700 | Below 1100 px the list folds; 860 and 390 captured. |

## Good

- The folded column keeps a Group and View door that opens the list where the switch and View are (⌘B too).

## Weak

- At 390 px the icons-only rail is the whole navigation; fine for a desktop minimum.
- At 390 px the header still takes two rows (the tabs wrap after Accounts; Usage and History share the second row with the actions), about 120 px before the page starts.
- opened-860x700 (both themes): with the list open, the header still wraps its actions (broom pill, New session, bell, avatar) onto a second row under the tabs.
- folded-390x844 (both themes): the Needs you rows cut their status to "Limit reach…" and "Stopped wi…".
- folded-390x844 (both themes): the first need's two-line detail breaks after a separator, so its first line ends in a dangling "·" ("storefront · AI Juggler ·").

## Fixed

- Re-render: below 1100 px the folded rail hid the grouping switch and View; the only way back was the logo or ⌘B. The door is at the top of the column now.
- Polish (0.3, this branch): the opened list's top is a shorter stack — search and View 28 px, the grouping switch 30 px (each segment still a 24 px target), less air before Needs you — so the first session sits 13 px higher, every control where it was.

- Second re-render: the opened list at 860 shows the shorter top (search and View 28 px, the switch 30 px) and Needs you starting higher; the folded column is unchanged. Nothing else to fix here.

## Captures

| Capture | What is on it | What changed in 0.3 |
|---|---|---|
| folded-860x700 | Home at 860: the folded column (Group and View door, needs count 5, six places) beside the 0.3 header. | The door (re-render fix). |
| folded-390x844 | The same at 390. | The door. |
| opened-860x700 | The list opened with ⌘B while narrow: search, View, the grouping switch, Needs you. | The 0.3 switch. This branch: the tighter top. |

## 0.3.2

The folded column is unchanged: the View door, the needs count and the app marks with their amber or brown "!" badges. The page beside it lifts its Needs you and Runway cards off the darker canvas (white on grey in light), and "Open its page" and New session are solid violet against the outlined row buttons. After the design fixes the cleanup pill is a broom with "7" in every capture here, the dark accent is lighter on the logo mark, the rail's New session outline and the Work Max runway bar, and at 390 the header drops from three rows to two, so "5 sessions need you" starts about 35 px higher. At 860 with the folded column the header is one row. With the list opened at 860 (opened-860), the header still wraps the broom, New session, bell and avatar onto a second row under the tabs, even with the shorter pill. At 390 the Needs you rows still cut their status to "Limit reach…" and "Stopped wi…", because each row's button takes about half the card's width. In the second pass the first need's detail may take two lines, so in folded-390 the Personal card reads "storefront · AI Juggler ·" / "Personal" instead of a cut "storefront · AI Juggler · …"; the card and everything under it moves down about 16 px, and the first line ends on a dangling "·".

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: opened-860x700 (light) and folded-390x844 (dark), beside the 0.3.2 notes. At 860 with the list open the rail is the calm App lens (Needs you, Open now) and Home stacks its columns with "Apps & runway ↓" in the head (the mock's narrow frame); at 390 the rail is icons with the "✕ 5" count and Home is one column.

- **Since 0.3.2:** the Needs you cards now say the whole state ("probably stuck", "limit reached") instead of "Limit reach…"; the dangling "storefront · AI Juggler ·" line is gone (the card's head is "Claude Code CLI · Personal  storefront").
- **Weak, kept:** the top bar still wraps at 860 (tabs on one row, 🧹 7 · >_ · Bot · bell · avatar on a second), about 45 px off every page, and at 390 takes three rows: logged **F57** with 46-fleet-board's 1024 case. At 390 the feed's chips wrap to three lines in their track.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Every screen stays usable in a narrow window; the rail folds to icons with its count. |
| 2 | Prominence | 4 | Needs you first at both widths; Apps & runway one link away. |
| 3 | Default | 4 | The rail folds by itself under 1,100 px; ⌘B opens it. |
| 4 | Mode | 4 | Calm. |
| 5 | Mock-up | 4 | Matches the mock's narrow frame (feed first, Apps & runway ↓). |
| 6 | Areas | 4 | F57 fixed: one row at 860 (Home · Projects · Pools · More ▾); 390, below the window's 1024 minimum, takes two. |
| 7 | Colour | 4 | As Home: amber and red words only on the needs. |
| 8 | Type | 4 | Card titles keep 13 px; nothing under 12 px. |
| 9 | Actions | 5 | Open its page is the one filled button. |
| 10 | Serious | n/a | Nothing destructive. |
| 11 | Doors | 4 | Every card and rail icon opens its thing. |
| 12 | Words | 4 | Whole states, no cut words in the cards. |
| 13 | States | 4 | The same states as the wide Home. |
| 14 | Consistency | 4 | The wide Home's cards and rail, folded. |
| 15 | Accessibility | 4 | Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | The PL1 walk (AJ-E2323) drove Home and the rail in Dev; not re-walked here. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | A narrow window keeps the job; the pages behind More are one click away. |

Lowest as first scored: 3 (line 6, F57 logged: the fold point should count the tabs; a top-bar layout change across every baseline).

**Re-captured after F57 (2026-11-24, Opus):** all six re-rendered and opened. opened-860x700 and folded-860x700: the top bar is one row (it took two): Home · Projects · Pools · More ▾ beside 🧹 7, >_, Bot, the bell, the Floor and the avatar, so the page starts about 45 px higher. folded-390x844, a width the window can't take (its minimum is 1024): Home · Projects · More over the cluster, two rows where it took three.

Lowest: 4 (line 6 was 3: F57 fixed).

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Needs you's cards are tone fills with their level's mark (a stopped one in the broken tone), the most urgent first (F53, F66, F80), at 390 px. Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.

## 0.5.2 (2026-11-25, AJ-E1557, AJ-E1556)

Re-captured the folded rail at 390 and 860 and the opened rail at 860, both themes. What changed: a folded icon for an idle session has no corner dot, a working one keeps its static live dot (AJ-E1557). Home behind it also shows what HEAD already drew and this capture had not caught up with: Needs you now's two stopped sessions in one tinted block, the limit card apart below it (0.5.1's a11y batch). Each re-captured state was diffed against its last capture (pixel counts per region: the rail, the top bar, the content) and opened by eye, old and new side by side, both themes. Nothing broken. Scores unchanged; the rail is calmer: the eye goes to the four marked states.
