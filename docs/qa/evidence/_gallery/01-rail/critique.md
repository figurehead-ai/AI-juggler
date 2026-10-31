<!-- captures: bb660480c1d0 -->
# 01-rail: the session rail beside Home, with every status, with no accounts (welcome) and with one account still to sign in, at four widths in both themes

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The Needs you, Runway by account and Pick up where you left off cards now sit a clear step above the canvas in dark, and in light the white cards stand off the grey canvas; "Open its page", "Find my accounts" and "New session" are solid violet and read as the one main action in their card. The rail itself is dense but legible: project headers, two-line rows with status in colour and a time, and the selected row marked by a bar. Two rail rows lose information in all-statuses-1280x800-* and 1440x900-*: the first feat/read-model row reads "Stopped with an error · 7…" with its age cut, and the first Fake Desk row shows "Stopped with an error" with no age at all; the error state is now amber like "Probably stuck" in the rail, while the session header shows it in red. At 390 px the header wraps to three rows, the Needs you rows cut their status to a few letters ("Stopped wi…"), and in honest-empty-390x844-* the "We found Claude Code on this computer." chip breaks with its icon alone on the first line. In ready-home-* the runway lists an account titled "Not signed in" (with Profiles: Work Max) instead of its name, and the header still shows "7 to clean up" while the rail says "No sessions yet". All captures were re-rendered.

## Weak

- all-statuses-1280x800-*, all-statuses-1440x900-*, determinism-a: rail row "Stopped with an error · 7…" cuts its age; the first Fake Desk row shows no age.
- all-statuses-*: the error state is amber in the rail but red in the session header, so it reads like a warning beside "Probably stuck".
- honest-empty-390x844-*: the "We found Claude Code on this computer." chip wraps with its icon alone on a line.
- all-statuses-390x844-*, ready-home-390x844-*: three-row header; Needs you rows cut status and names to a few letters.
- ready-home-*: a runway entry is titled "Not signed in" rather than by its account name.
