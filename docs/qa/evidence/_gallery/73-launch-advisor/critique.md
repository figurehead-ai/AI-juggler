<!-- captures: 8ee48a19516c -->
# 73-launch-advisor: the New session sheet advising whether this Mac has room for one more session

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The sheet reads in order: how it runs, "Where do you want to work?" as folder cards, then "Which account?" with Work Max in a tinted card and a violet "Suggested: the most room" chip, and a solid violet "Launch on Work Max" in the footer. The room verdict reads clearly: a grey dot for "This Mac can take another session" (ok, not-measured) and a red square with "Not now — this Mac is short of room" (no). But in no the footer's solid violet Launch looks exactly as it does when all is well, so the red verdict and the primary button send mixed signals. The account line ends in a dangling "·" after "58% left, from its latest session", now that the launch settings moved to their own line. None of the folder cards looks selected in these captures. tight (dark and light) was not re-rendered and shows the older sheet, with small-caps headings and an inline "Launch →".

## Weak

- no-1280x800-*, ok-*, not-measured-*: a stray "·" ends the account's first line.
- no-1280x800-*: the red "Not now" verdict sits above an unchanged solid primary Launch.
- tight-1280x800-*: not re-rendered.
