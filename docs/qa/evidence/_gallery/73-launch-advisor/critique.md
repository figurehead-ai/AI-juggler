<!-- captures: d9c5b49c810b -->
# 73-launch-advisor: the New session sheet advising whether this Mac has room for one more session

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The sheet reads in order: how it runs, "Where do you want to work?" as folder cards, then "Which account?" with Work Max in a tinted card and a violet "Suggested: the most room" chip, and a solid violet "Launch on Work Max" in the footer. After the design fixes the dark accent is lighter on the Suggested chip and the Work Max card's outline (no, ok, not-measured). tight (dark and light) is now re-rendered in this same sheet: an amber triangle with "Tight — this Mac can take it, just", the memory sentence under it, and the footer's solid Launch instead of the old inline "Launch →". The room verdict reads clearly in every state: a grey dot for "This Mac can take another session" (ok, not-measured), the amber triangle (tight) and a red square with "Not now — this Mac is short of room" (no). But in no the footer's solid violet Launch looks exactly as it does when all is well, so the red verdict and the primary button send mixed signals. The account line still ends in a dangling "·" after "58% left, from its latest session", now that the launch settings sit on their own line. None of the folder cards looks selected.

## Weak

- no-1280x800-*, ok-*, not-measured-*, tight-*: a stray "·" ends the account's first line.
- no-1280x800-*: the red "Not now" verdict sits above an unchanged solid primary Launch.
- All: none of the four folder cards looks selected, so the sheet doesn't say where the session will run.
