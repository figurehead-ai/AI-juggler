<!-- captures: 8428de34cfea -->
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
