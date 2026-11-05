<!-- captures: 004df97472bf -->
<!-- captures: pending -->
# 99-first-run-confidence: first run says how sure it is

Design critique (D-57) against first run's found step (66-first-run) and Add an app (71-add-account). New family (AJ-E2906).

## Checklist

| Check | Finding |
|---|---|
| Grid | The found step's rows unchanged in their columns; the word sits after the name, the source after the path, Details at the row's end. |
| Hierarchy | Name, its word as a small chip; path and source muted on the second line; Details indented under the name. |
| Spacing | The rows' rhythm holds; Details opens below the row without moving the columns. |
| Icons | None added; ▸ turns to show Details is open. |
| Shared controls | The shared checkbox and the row's own label; Details is a real button outside the label (it never ticks the row). |
| Disabled reasons | n/a. |
| Plain words | "Found", "Likely", "Guessed · You can rename it after", "named by cc-switch", "May have expired", "from its sign-in file is there but empty". |
| Type sizes | Names 15 px, chips and sources 12 px. |
| Both themes | Checked; Guessed's amber chip reads in both. |
| 1280×800 and 1024×700 | Both; at 1024 the source line truncates with the full text in its tooltip. |

## Good

- A guess never reads as a fact: the weaker of name and sign-in decides the word.
- Details says how each part was worked out, in words, and keeps the scan's own notes.

## Weak

- "from the folder's name" reads oddly beside a name that is the folder's name; it says what it's from, not why it's a guess.
- Look again is a plain link among the footer's links; it's easy to miss after installing a tool.

## Fixed

- First render (seen by eye): the gallery's x7 row said Needs sign-in while its Details would have said Signed in (the fixture's own slip); the fixture now agrees with itself.
