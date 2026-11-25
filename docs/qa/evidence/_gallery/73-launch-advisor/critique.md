<!-- captures: 15ebc23e7b61 -->
# 73-launch-advisor: the New session sheet advising whether this Mac has room for one more session

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The sheet reads in order: how it runs, "Where do you want to work?" as folder cards, then "Which account?" with Work Max in a tinted card and a violet "Suggested: the most room" chip, and a solid violet "Launch on Work Max" in the footer. After the design fixes the dark accent is lighter on the Suggested chip and the Work Max card's outline (no, ok, not-measured). tight (dark and light) is now re-rendered in this same sheet: an amber triangle with "Tight — this Mac can take it, just", the memory sentence under it, and the footer's solid Launch instead of the old inline "Launch →". The room verdict reads clearly in every state: a grey dot for "This Mac can take another session" (ok, not-measured), the amber triangle (tight) and a red square with "Not now — this Mac is short of room" (no). But in no the footer's solid violet Launch looks exactly as it does when all is well, so the red verdict and the primary button send mixed signals. The account line still ends in a dangling "·" after "58% left, from its latest session", now that the launch settings sit on their own line. None of the folder cards looks selected.

## Weak

- no-1280x800-*, ok-*, not-measured-*, tight-*: a stray "·" ends the account's first line.
- no-1280x800-*: the red "Not now" verdict sits above an unchanged solid primary Launch.
- All: none of the four folder cards looks selected, so the sheet doesn't say where the session will run.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B2)

Re-captured after F53 (only the rail's lens behind changed). All four are New session's stated choice with the Mac's verdict as one line inside its block: ok and not-measured say nothing extra, tight "Tight — …", no a red square with "Not now — this Mac is short of room", why ("macOS reports critical memory pressure; swap is growing — 620 MB a minute. Closing Fix the flaky login test frees about 3.0 GB.") and "You can still start it." The 0.3.2 notes (the account line's "·", unselected folder cards) no longer apply.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 5 | |
| 2 | Prominence | 4 | The verdict sits under the App's lines. |
| 3 | Default | 5 | |
| 4 | Mode | 4 | |
| 5 | Mock-up | 4 | |
| 6 | Areas | 5 | |
| 7 | Colour | 4 | Red only for "Not now"; the primary unchanged by design (D-31, the user's call). |
| 8 | Type | 4 | |
| 9 | Actions | 5 | |
| 10 | Serious | 4 | Says what to close and how much it frees. |
| 11 | Doors | 4 | The session to close is named, not a link. |
| 12 | Words | 4 | "swap" is a technical word. |
| 13 | States | 5 | |
| 14 | Consistency | 5 | |
| 15 | Accessibility | 4 | |
| 16 | Hand walk | 4 | Preview. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | |

Lowest: 4.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Behind the dialog the rail and Needs you are in their 0.5.1 style; the dialog itself is unchanged. Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
