<!-- captures: 7eb74421c70b -->
# 01-rail: the session rail beside Home, with every status, with no accounts (welcome) and with one account still to sign in, at four widths in both themes

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

The Needs you, Runway by account and Pick up where you left off cards sit a clear step above the canvas in dark, and in light the white cards stand off the grey canvas; "Open its page", "Find my accounts" and "New session" are solid violet and read as the one main action in their card. In dark the lighter accent shows on the logo mark, "Open its page", the Work Max runway bar and the rail's New session button, and it reads a little brighter against the near-black. The top bar stays one row at 1024 to 1440 px ("This Mac · Plenty of room", "7 to clean up"). The rail is dense but legible: project headers, two-line rows with status in colour and a time, and the selected row marked by a bar. The status line now shows its age whole or not at all, so both error rows in all-statuses-1280x800-* and 1440x900-* read "Stopped with an error" with no cut "· 7…"; the error state is still amber in the rail like "Probably stuck", while the session header shows it in red. The second pass keeps rail times whole: determinism-a (App grouping) now reads "ai-j… · 3 h ago" beside "Probably asks you", cutting the project instead of the age. In ready-home-* at 1024 to 1440 px the first need ("Work Max needs its sign-in") now gives its detail on two lines, ending whole on "…AI Juggler never sees your password." instead of one cut line, and the card grows to fit.

At 390 px the cleanup pill is now a broom with "7", so the controls fit one row under the nav, but the header is still three rows (nav on two lines, then the controls); the Needs you rows still cut their status to a few letters ("Stopped wi…", "Limit reach…"); the first need's detail now takes two lines (all-statuses-390x844-* shows "storefront · AI Juggler · Personal" whole), though in ready-home-390x844-* it still ends cut at "…about thirty second…"; and in honest-empty-390x844-* the "We found Claude Code on this computer." chip still breaks with its icon alone on the first line. In ready-home-* the runway still lists an account titled "Not signed in" (with Profiles: Work Max) instead of its name, and the header still shows "7 to clean up" while the rail says "No sessions yet". All captures were re-rendered.

## Weak

- all-statuses-*: the error state is amber in the rail but red in the session header, so it reads like a warning beside "Probably stuck".
- honest-empty-390x844-*: the "We found Claude Code on this computer." chip wraps with its icon alone on a line.
- all-statuses-390x844-*, ready-home-390x844-*, honest-empty-390x844-*: three-row header (nav on two lines); Needs you rows cut status and names to a few letters; in ready-home-390x844-* the sign-in need's two-line detail still ends cut ("…about thirty second…").
- ready-home-*: a runway entry is titled "Not signed in" rather than by its account name; "7 to clean up" shows while the rail says "No sessions yet".

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B3)

Re-captured at the window's sizes (1024, 1280, 1440; F49's re-render had kept the 0.3.2 critique), both themes; opened at full size: all-statuses-1280x800-dark and ready-home-1280x800-light, beside the round-2 mock (`calm-ui/mock-r2-dark.png`). The rail now matches the mock's grammar: tracked-capital section labels, NEEDS YOU with its ✕ count, rows with a state dot or ✕, the project heads with their counts, F53's solid chosen lens, F55's rows ending their state on "…". Home beside it is D-118's two columns.

- **390×844 removed** (F51's rule): six captures below the window's minimum gone (capture.spec `WINDOW_VIEWPORTS`); the 1024 captures keep the folded rail. The 0.3.2 notes at 390 (three-row header, cut needs) went with them.
- **Fixed** (51ea3254f): Home's "Jump to a session's own window" card had a hairline outline; it is its tone alone (`OsAccessRows`, also the setup wizard's access cards). Gate home-scrolled ×8, rail-order-menu ×2 and the Home captures of 68, 74, 78, 80, 96 re-rendered.
- **Closed since 0.3.2:** the error state is red in the rail (✕) and the header alike.
- **Weak, logged F80:** Home pins three L2 needs and folds both "Stopped with an error" sessions under "+2 more need you"; the rail beside it lists those two first.
- **Weak, kept:** ready-home's runway names a signed-out profile "Not signed in"; "7 to clean up" beside "No sessions yet" (the fixture's leftovers).

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | What's open and what needs me, one click to it. |
| 2 | Prominence | 4 | F80 (747062960): Home pins the two stopped sessions first, as the rail lists them; the quieter needs fold under "+2 more need you". |
| 3 | Default | 4 | Open its page ↵ on the first need. |
| 4 | Mode | 4 | Calm, the round-2 mock's dials. |
| 5 | Mock-up | 4 | Matches mock-r2's rail and Home; deviations: D-118's feed under Needs you now. |
| 6 | Areas | 4 | Rail and Home's two columns by tone; after the fix no outlined card. |
| 7 | Colour | 4 | Red ✕ for L3, amber for L2, violet only for the one primary and selection. |
| 8 | Type | 4 | Section labels tracked capitals, titles ink, meta muted. |
| 9 | Actions | 4 | One filled primary on the first need. |
| 10 | Serious | 4 | n/a. |
| 11 | Doors | 4 | Every row and need opens its session; Your apps open their pages. |
| 12 | Words | 4 | The one vocabulary ("Stopped with an error", "Probably stuck"). |
| 13 | States | 4 | Honest-empty and ready states each said once. |
| 14 | Consistency | 4 | The reference for the other surfaces. |
| 15 | Accessibility | 4 | axe clean in the capture runs (no serious or critical); keyboard and zoom not re-walked this round. |
| 16 | Hand walk | 4 | Not re-walked by hand here; captured from the fixtures, both themes. |
| 17 | System prompts | 5 | Causes no system prompt. |
| 18 | Usability | 4 | AJ-E2327 (6): "Open now 3" never beside "Nothing open yet"; the rail keeps its place. Match. |

Lowest: 3, each line under 4 logged (F80); the rest 4 or 5. After F80: lowest 4.

**Fixed after the critique (F80, F77), re-captured (all-statuses at 1024, 1280 and 1440, both themes; opened 1280 light; honest-empty and ready-home re-rendered byte-identical):** Needs you now pins "Rename the photo exports ✕ stopped with an error" (See what happened ↵, the one primary) and "Port the settings parser ✕ stopped with an error", then "Tidy the blog's images limit reached" and "+2 more need you" (it pinned the asks, stuck and limit needs and folded both stopped ones). The rail's model-changed mark beside "Port the settings parser" is F77's chip icon, no outline. Line 2 3 → 4; lowest now 4.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Needs you's cards are tone fills with their level's mark (a stopped one in the broken tone), the most urgent first (F53, F66, F80); the lens switch's chosen segment is a solid fill; rail rows carry a model mark where the model changed (F77). Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.

## 0.5.2 (2026-11-25, AJ-E1557, AJ-E1556)

Re-captured `all-statuses` at 1024, 1280 and 1440 in both themes. What changed: an idle or ended row ("Summarise the support inbox", "Add retries to the sync job") no longer leads with a faint dot; the gutter stays empty, so every title is where it was, and the marks left are working's live dot, needs you's level mark, review's ✓ and an error's ✕ (AJ-E1557, LST-8). Home behind it also shows what HEAD already drew and this capture had not caught up with: Needs you now's two stopped sessions in one tinted block, the limit card apart below it (0.5.1's a11y batch). Each re-captured state was diffed against its last capture (pixel counts per region: the rail, the top bar, the content) and opened by eye, old and new side by side, both themes. Nothing broken. Scores unchanged; the rail is calmer: the eye goes to the four marked states.
