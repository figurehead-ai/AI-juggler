<!-- captures: 210b1facad9e -->
<!-- captures: pending -->
# 99-home-chips: Home's chips

Design critique (D-57) against Home (67-home). New family (AJ-E2910).

## Checklist

| Check | Finding |
|---|---|
| Grid | A segmented row above Needs you's items, with "n more under All" beside it when narrowed. |
| Hierarchy | All first with the total, then each kind with its count. |
| Spacing | The card's rhythm. |
| Icons | None. |
| Shared controls | The shared Segmented control. |
| Disabled reasons | n/a: a kind with nothing isn't offered. |
| Plain words | "Asks you 1", "Probably stuck 1", "4 more under All". |
| Type sizes | 12 px chips. |
| Both themes | Checked dark and light. |
| 1280×800, 1024×700 and 1440×900 | All three. At 1440 (dark) also Mark all and its toast ("Marked 2 reviewed", with Undo); no chip changes the rail (checked in the run). |

## Good

- Narrowing never hides that more waits: "4 more under All" is a link back.

## Weak

- The heading's count ("5 sessions need you") and All's count can differ when agents' asks are counted separately.

## Fixed

- Nothing on first render.

## 0.4.1 re-score (2026-11-07, AJ-E3305)

The whole family was retaken on release/0.4.1 for the 14-line re-score (AJ-E3305, `docs/qa/0.4-walks/design.md`) and opened on contact sheets, both themes, 1280 and 1024. Shared with every screen since the last shoot: the header's New session is tinted (one filled button per view), the Floor button is a split control with its own menu, This Mac's line is its dot below 1120 px, and the rail's App headers put the profile first.

Unchanged in design: the chips ("All 5 · Asks you 1 · Probably stuck 1 · Limit reached 1"), a chosen chip saying what it hides ("4 more under All"), Runway by account with Out, Pick up where you left off.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: narrowed-1280x800 light and marked-all-1440x900 dark (before and after the fix), beside the 0.4.1 re-score. Home is D-118's two columns in every capture: Needs you now as one card per need (the first card's primary the one filled button), the feed with its chips and Showing:, then Your apps and Runway by account on the right.

- **narrowed:** the Needs you chip chosen; "Filtered: Needs you × · Needs you now shows every need" says what's narrowed and how to undo it. In these captures (before F53, 1432a68d9) the chosen chip in light is only its bold word; F53 makes it a solid fill.
- **marked-all:** Finished chosen, "Mark all 3 reviewed" from the feed's ⋯, the toast "Marked 3 reviewed. Undo (5s)". **Before the fix** the ⋯ menu stayed open over its own toast, still offering "Mark all 3 reviewed". **Fixed** (e2544e734): a Mark all closes the menu. Re-captured (local) and checked by eye; home-review-all.test checks the menu is gone.
- **Weak:** the feed's ⋯ menu lines carry no icon (N8; `IconRead` exists for Mark all read): logged **F60**.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Narrow the feed by kind; mark a whole kind at once with Undo. |
| 2 | Prominence | 4 | All first; the chips in one row with Showing:. |
| 3 | Default | 4 | All by default; the filtered line says how to clear it. |
| 4 | Mode | 4 | Calm. |
| 5 | Mock-up | 4 | The mock's chip row; "Everything ▾" for its Project and App filters (accepted in 67-home). |
| 6 | Areas | 4 | One row of chips, the filtered line under it. |
| 7 | Colour | 4 | Accent only on the toast's Undo and the one primary. |
| 8 | Type | 4 | Chips 13 px, the filtered line 12 px muted. |
| 9 | Actions | 5 | Open its page the one filled button. |
| 10 | Serious | 4 | Mark all has Undo (PAT-DES-1). |
| 11 | Doors | 4 | Each chip narrows in place; × clears. |
| 12 | Words | 4 | "Marks these finished sessions as looked at": says what it does. |
| 13 | States | 4 | After the fix: the menu closes once done. |
| 14 | Consistency | 3 | F60: the feed's menu lines have no icons. |
| 15 | Accessibility | 4 | Chips are a radio group. Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | The PL1 walk (AJ-E2323) drove Home and the rail in Dev; not re-walked here. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | No chip changes the rail (the spec checks). |

Lowest: 3 (line 14, F60 logged).
