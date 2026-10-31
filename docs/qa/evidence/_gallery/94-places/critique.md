<!-- captures: 3e405af410eb -->
# 94-places: Home and the rail with no, one, a typical and a busy set of sessions

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

Home at three loads (home-empty, home-typical with 8 open, home-busy with 40 open) and the rail alone at 1, 8 and 40 sessions, in both themes. The Your apps cards, the Needs you card and the Runway by account card stand a step off the canvas in dark and as white cards on grey in light, and the one primary per screen is solid violet (New session in home-empty, Open its page on the first Needs you row, Allow on the Accessibility prompt). The headline "5 sessions need you" with its "40 open · 1 inactive · 1 ended today" line gives a clear top level, and the amber "Asks you" / "Limit reached" states in the rail are readable in both themes. In the Your apps row the other cards put their Open button on one line and the amber "1 needs you" chip on the line below; the ChatGPT card has no chip, so Open ChatGPT drops to that bottom line and sits lower than Open Claude and Open Terminal beside it (home-typical, home-busy). In home-empty the Runway by account card sits alone in the right-hand column with nothing beside it, leaving a large empty block to its left. The rail captures (rail-1, rail-8, rail-40) are the 264 px rail only; there the account names in group headers are cut to a few letters ("Claude · Pers…", "Claude · Work…") to make room for the warning count.

## Weak

- home-typical, home-busy: Open ChatGPT sits lower than the other cards' Open buttons, because only its card has no "1 needs you" chip and its button takes the chip's line.
- home-empty: Runway by account sits alone in the right column, leaving a large empty area to the left under "Start your first session".
- rail-8, rail-40: group headers shorten the account to "Pers…" / "Work…" beside the ⚠ count, so two Claude groups are told apart only by their colour dot.
