<!-- captures: 7a8026c83b13 -->
# 21-tiers: the Processes page when automatic cleanup is paused, suspended after a crash, or about to tidy a process (5 s and 30 s), and the account's cleanup toggles off and on

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

paused-*, suspended-*, tier2-5s-* and tier2-appearance-* each put a one-line notice with one action above the process table ("Automatic cleanup is paused." · "Resume automatic cleanup"; "…waits for you to re-confirm it (up to 24 h)." · "Re-confirm"; "…will be tidied in 5 s unless you keep it." · "Undo: keep it running"), which is clear and calm. In light the notices are a beige fill on the grey canvas, close to it in tone; in dark they have an amber edge and stand out. The process table is well aligned, with coloured state words, small CPU sparklines and right-aligned actions. toggles-off-1440x900-* and toggles-on-1440x900-* are broken as evidence: off and on are pixel-for-pixel the same account overview, with no cleanup toggles at all (the toggles now live in the Processes page's Automatic cleanup card), while the previous render showed the ticked and unticked boxes. All captures were re-rendered.

## Weak

- toggles-off-1440x900-*, toggles-on-1440x900-*: identical captures; no cleanup toggles are visible.
- paused-*-light, suspended-*-light, tier2-*-light: the beige notice sits close to the grey canvas.
