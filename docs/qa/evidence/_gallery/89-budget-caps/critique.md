<!-- captures: 48dd4e5e3dd2 -->
# 89-budget-caps: monthly caps, setting one, its alerts on Home, and the caps on Usage

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

cap-sheet is a compact dialog (Name, Amount a month, Counts as two radios with a project checklist) with Save as one small solid violet button beside a plain Cancel, and the fields sit a step lighter than the dialog in dark; there the chosen "A group" radio and the ticked ai-juggler box now use the lighter accent. On home-needs the Needs you card puts the sign-in ask first (Sign in is the page's one violet button) and lists the two cap alerts below it as their own bordered rows, an amber triangle for Acme at 81 % and a red square for Side project over, each with a plain "Got it" and an outlined "See usage" (the Side project row moved up slightly in this render), and cards stand off the canvas in both themes. The rail still cuts both alerts' titles at the word that matters ("Acme is at 81% of its monthl…", "Side project is over its month…"), and the Acme row leaves an empty line below "See the cap" that the other Needs you rows don't. On usage-caps the Monthly caps card fills only the left half of the page with nothing to its right, the three cap cards leave Work alone on a second row, and the top of the capture slices the row above; the bars read well (amber in dark but brown in light for Acme, red past 100 %, violet for Work).

## Weak

- home-needs-1280x800-dark.png, home-needs-1280x800-light.png: rail titles of the two cap alerts are cut before "cap".
- home-needs-1280x800, usage-caps-1280x800 (both themes): the rail's Acme alert row has an empty line under "See the cap", so the Needs you rows are unevenly spaced.
- usage-caps-1280x800-dark.png, usage-caps-1280x800-light.png: the Monthly caps card takes half the width and leaves the right half of the page empty.
- usage-caps-1280x800-dark.png, usage-caps-1280x800-light.png: the row above Monthly caps is sliced at the top edge.
