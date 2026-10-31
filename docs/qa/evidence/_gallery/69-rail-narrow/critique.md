<!-- captures: cbf86497c13b -->
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

The folded column is unchanged: the View door, the needs count and the app marks with their amber or brown "!" badges. The page beside it lifts its Needs you and Runway cards off the darker canvas (white on grey in light), and "Open its page" and New session are solid violet against the outlined row buttons. After the design fixes the cleanup pill is a broom with "7" in every capture here, the dark accent is lighter on the logo mark, the rail's New session outline and the Work Max runway bar, and at 390 the header drops from three rows to two, so "5 sessions need you" starts about 35 px higher. At 860 with the folded column the header is one row. With the list opened at 860 (opened-860), the header still wraps the broom, New session, bell and avatar onto a second row under the tabs, even with the shorter pill. At 390 the Needs you rows still cut their status to "Limit reach…" and "Stopped wi…", because each row's button takes about half the card's width.
