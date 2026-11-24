<!-- captures: 44b025aa4752 -->
# 13-desktop: a desktop app's session page (running with its window preview, exited, crashed), the single-account warning, and the "app" tag in the rail

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

panel-running-* still lead with a solid violet "Go to it" in the header, and the app card adds a second solid violet "Open / Focus window", so two equal-weight primaries sit on one screen; the live preview below is a mirror of grey bars. In dark both buttons and the logo mark now use the lighter accent; panel-running-1024x680-light keeps the top bar on one row by showing the cleanup pill as a broom and "7". panel-crashed-1280x800 and panel-exited-1280x800 explain the state in one line ("app crashed · exit details on the Processes tab", "app exited · Relaunch, above, starts it again") with an amber "isolation · Single-account app — Juggler tracks, cannot isolate" strip, but they still offer the solid violet "Open / Focus window" for an app that is not running, and the card leaves a large empty area under its text. The copy says "Juggler" where the rest of the app says "AI Juggler". panel-single-account-warning-1280x800 still shows the running panel with no warning in view (the isolation strip is below the fold), so the capture does not show its subject. rail-app-chip-1280x800 is the default Home with the rail's "app" tags on the Fake Desk rows; they are legible but small, and the rail's error rows now read "Stopped with an error" with no cut age. All captures were re-rendered.

## Weak

- panel-running-*, panel-single-account-warning-1280x800: two solid violet buttons ("Go to it" and "Open / Focus window").
- panel-crashed-1280x800, panel-exited-1280x800: the main violet action is "Open / Focus window" for an app that isn't running; large empty card area.
- panel-single-account-warning-1280x800: no single-account warning is visible.
- panel-crashed/exited: "Juggler" instead of "AI Juggler" in the copy.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B3)

Re-captured after batch B2's F70 (059097f12), which rewrote this panel's words; opened at full size: panel-crashed (dark). The one-account note is now a plain line with an info mark ("One account at a time: AI Juggler can't open a second copy of this app, so one session at a time with it."; it was an amber "isolation" strip saying "Juggler tracks, cannot isolate"), and the body says "AI Juggler follows its process and never draws it here". The capture spec's two checks still asserted the old words; they now check F70's (79a98ea7c), and the rail app chip capture waits for a visible chip, since F55 folds it away in a narrow row (51ea3254f).

- **Closed since 0.3.2:** two filled violets on the running panel (Open / Focus window is a secondary; Go to it the one primary); "Juggler" in the copy (F70); the single-account capture now shows its note.
- **Weak, logged F81:** on a crashed or exited app the panel still offers Open / Focus window for an app that isn't running, lists "cpu — · ram — · children 0", repeats the header's "Stopped with an error" as a chip, leaves half the card empty, and the header's Resume is a disabled filled button with "Can't resume here".

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Watch and reach a desktop app's session; what to do once it stopped is F81. |
| 2 | Prominence | 4 | Window preview first for a desktop app; the facts under it. |
| 3 | Default | 4 | Go to it is the default while it runs. |
| 4 | Mode | 4 | Calm. |
| 5 | Mock-up | 4 | AJ-E1803's panel; F70's words. |
| 6 | Areas | 4 | Header, a facts card, the preview card; ended, the card as tall as what it says. |
| 7 | Colour | 4 | Red only for the stopped state; no amber since F70. |
| 8 | Type | 4 | Labels muted, the note in ink-2. |
| 9 | Actions | 4 | F81 (fdb76c5c4): ended, the header's Relaunch is the one primary; no greyed Resume (its tool can't resume, D-127) and no Open / Focus window for an app that isn't running. |
| 10 | Serious | 4 | n/a: nothing destructive. |
| 11 | Doors | 4 | The App line opens its page; exit details point to the Processes tab. |
| 12 | Words | 4 | Plain since F70; ended, one line says how it ended and why it can't resume (no "cpu — · ram —"). |
| 13 | States | 4 | Running, exited, crashed each said. |
| 14 | Consistency | 4 | The session page's header and tabs. |
| 15 | Accessibility | 4 | axe clean in the capture runs (no serious or critical); keyboard and zoom not re-walked this round. |
| 16 | Hand walk | 4 | Not re-walked by hand here; captured from the fixtures, both themes. |
| 17 | System prompts | 5 | Causes no system prompt. |
| 18 | Usability | 4 | AJ-E2327 (2): a row that can't show its work says why in words ("app crashed · exit details on the Processes tab"). Match; the ended actions are F81. |

Lowest: 3, each line under 4 logged (F81); the rest 4 or 5. After F81: lowest 4.

**Fixed after the critique (F81), re-captured (all six; opened panel-crashed, panel-exited and panel-running):** panel-crashed's header is Watch… and Relaunch (the one filled button), no "Resume / Can't resume here"; the Window preview card holds the app line (app · fake-desktop · fake-desktop · Fake Desk · sacrificial), the one-account note and "The app crashed; its exit details are on the Processes tab. fake-desktop can't resume a conversation from AI Juggler, so Relaunch, above, starts a fresh one.", and ends there (it repeated "Stopped with an error" as a chip, offered Open / Focus window, listed "cpu — · ram — · children 0 · No usage recorded yet" and left half the card empty). panel-exited the same with "The app exited." panel-running unchanged but for the rail. Line 9 3 → 4; lowest now 4.
