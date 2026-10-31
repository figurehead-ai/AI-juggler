<!-- captures: 28fe867cff5a -->
# 15-preview: a desktop session's live window preview: live, paused, screen-recording permission denied, and app exited

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

panel-live-* label the mirror with a teal "Live preview" chip, "frame age 0s" and "read-only mirror · click to focus the real window", which explains the preview well; the light capture keeps the raised white cards on the grey canvas, and at 1024 px its top bar stays one row with the cleanup pill shown as a broom and "7". panel-denied-1280x800 replaces the frame with an amber box, "Screen Recording permission needed — grant it in System Settings, then re-enable preview", but still offers no button to open the setting or re-enable the preview, and it still prints "frame age 0s" above a box that has no frame. panel-paused-1280x800 only adds a small grey "Preview paused — window not visible" above a frame that looks the same as a live one, so the paused state is easy to miss. All panels carry two solid violet buttons ("Go to it" and "Open / Focus window", in dark now the lighter accent), and panel-exited-1280x800 still offers "Open / Focus window" for an app that has exited. The rail's error rows read "Stopped with an error" with no cut age. All captures were re-rendered.

## Weak

- panel-denied-1280x800: no button to open the setting or re-enable the preview; "frame age 0s" shown with no frame.
- panel-paused-1280x800: the paused state is a small grey line over a frame that looks live.
- panel-live-*, panel-denied, panel-paused: two solid violet buttons; panel-exited offers "Open / Focus window" for an exited app.
