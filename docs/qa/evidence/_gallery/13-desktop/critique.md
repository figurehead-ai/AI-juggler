<!-- captures: 4e3e912fe0ef -->
# 13-desktop: a desktop app's session page (running with its window preview, exited, crashed), the single-account warning, and the "app" tag in the rail

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

panel-running-* lead with a solid violet "Go to it" in the header, and the app card adds a second solid violet "Open / Focus window", so two equal-weight primaries sit on one screen; the live preview below is a mirror of grey bars. panel-crashed-1280x800 and panel-exited-1280x800 explain the state in one line ("app crashed · exit details on the Processes tab", "app exited · Relaunch, above, starts it again") with an amber "isolation · Single-account app — Juggler tracks, cannot isolate" strip, but they still offer the solid violet "Open / Focus window" for an app that is not running, and the card leaves a large empty area under its text. The copy says "Juggler" where the rest of the app says "AI Juggler". panel-single-account-warning-1280x800 shows the running panel with no warning in view (the isolation strip is below the fold), so the capture does not show its subject. rail-app-chip-1280x800 is the default Home with the rail's "app" tags on the Fake Desk rows; they are legible but small. All captures were re-rendered.

## Weak

- panel-running-*, panel-single-account-warning-1280x800: two solid violet buttons ("Go to it" and "Open / Focus window").
- panel-crashed-1280x800, panel-exited-1280x800: the main violet action is "Open / Focus window" for an app that isn't running; large empty card area.
- panel-single-account-warning-1280x800: no single-account warning is visible.
- panel-crashed/exited: "Juggler" instead of "AI Juggler" in the copy.
