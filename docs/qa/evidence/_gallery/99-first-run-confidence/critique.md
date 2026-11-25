<!-- captures: 66cd2de5f8d7 -->
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

## 0.4.1 re-score (2026-11-07, AJ-E3305)

The whole family was retaken on release/0.4.1 for the 14-line re-score (AJ-E3305, `docs/qa/0.4-walks/design.md`) and opened on contact sheets, both themes, 1280 and 1024. Shared with every screen since the last shoot: the header's New session is tinted (one filled button per view), the Floor button is a split control with its own menu, This Mac's line is its dot below 1120 px, and the rail's App headers put the profile first.

Unchanged in design: Found, Likely and Guessed per setup, Details folded, the way on at the window's foot at 1024.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

This is Add an app's Look again (the standalone first run, mode add), not the setup wizard. Opened at full size: details (1280 dark) and found (1024 light) from the 0.5.1 shoot, then details (1280 light) after the fix. What 0.5.1 changed: F29 dropped the Found / Likely / Guessed badges (the rows say "Signed in · Used yesterday", "Not signed in: you can sign in later", "Its sign-in may have expired: you can sign in later"), so how sure AI Juggler is lives in Details only; calm surfaces.

- **Before the fix:** Details read "from its sign-in file is there but empty" (and "from it has no conversations yet" for a folder with none): not sentences. **Fixed** (85e49a061): each source reads after "from": "from its sign-in file, there but empty", "from its folder, which has no conversations yet", "from the system's keychain, which keeps its sign-in". Re-rendered (gate details) and re-captured; checked by eye.
- **Words (F52), fixed** (b913d18e2): "You already have 4 Claude Code apps on this computer.", groups "The default app" and "Other Claude Code apps" (the adapter's own group names, so the real app says it too), and the wizard's links "Set up a new app instead" and "Add from another folder…". Re-rendered (gate found, details) and re-captured (both sizes, both themes); checked by eye. Details' note still says "A separate Claude configuration" (the scan's own note, beside the row; weak, kept).
- **Weak, kept:** the heading's 28 px and the 15 px semibold lead (the wizard now uses 22 px and regular); "Bring in 0 apps" waits disabled with its count as its reason; Look again is a plain link among the foot's links (0.4).

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Find the apps on this Mac and say how each word was worked out. |
| 2 | Prominence | 4 | Rows first, Details folded. |
| 3 | Default | 4 | Nothing ticked in Add (D-84). |
| 4 | Mode | 4 | Rich list. |
| 5 | Mock-up | 4 | The wizard's rows (the addapp mock not reopened this round). |
| 6 | Areas | 4 | Group headers by tone, rows, Details indented. |
| 7 | Colour | 4 | App dots only; no status colour for a guess now. |
| 8 | Type | 4 | Name 15 px, line 13 px muted, Details 12 px. |
| 9 | Actions | 4 | One primary (Bring in N), waiting at 0. |
| 10 | Serious | n/a | Nothing destructive. |
| 11 | Doors | 4 | Details per row, Where we looked. |
| 12 | Words | 4 | "apps" throughout, as the wizard (F52); Details' sources read as sentences; the scan's note "A separate Claude configuration" is the one leftover term. |
| 13 | States | 4 | Found, Details open, at 1024 and 1280. |
| 14 | Consistency | 4 | The wizard's rows. |
| 15 | Accessibility | 4 | Details is a button outside the row's label; not re-walked this round. |
| 16 | Hand walk | 4 | PL1 walk's Add an app; not re-walked here. |
| 17 | System prompts | 5 | Reading the tools' folders primed by AJ-E1547 in setup. |
| 18 | Usability | 4 | AJ-E1545 step 1 (1) "people think in apps, not setups": the heading, groups and links say apps (F52). Match. |

Re-scored after F52 (2026-11-24, Opus): found and details opened again at 1280 and 1024, both themes. Lowest: 4.
