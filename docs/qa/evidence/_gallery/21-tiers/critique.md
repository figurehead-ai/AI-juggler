<!-- captures: 04458eac446c -->
# 21-tiers: the Processes page when automatic cleanup is paused, suspended after a crash, or about to tidy a process (5 s and 30 s), and the account's cleanup toggles off and on

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

paused-*, suspended-*, tier2-5s-* and tier2-appearance-* each put a one-line notice with one action above the process table ("Automatic cleanup is paused." · "Resume automatic cleanup"; "…waits for you to re-confirm it (up to 24 h)." · "Re-confirm"; "…will be tidied in 5 s unless you keep it." · "Undo: keep it running"), which is clear and calm. In light the notices are a beige fill with a tan edge on the grey canvas, close to it in tone; in dark they have an amber edge and stand out. The process table has coloured state words, small CPU sparklines and right-aligned actions; the second pass re-spaced its columns at 1440, moving State, CPU and Memory right and narrowing the gap before the actions, and the State column is now too narrow for its longest word: "Left behind while closed" runs into the python3 row's "0.2 %" ("…while clos0.2 %") in paused-*, suspended-*, tier2-5s-* and tier2-appearance-*, in both themes; in dark the logo mark and rail's New session button take the lighter accent, and the rail's error rows read "Stopped with an error" with no cut age. toggles-off-1440x900-* and toggles-on-1440x900-* are framed on their subject again: the account's Settings card shows Tidying up with "Automatically tidy left-behind dev servers" and "Also tidy after a crash (with notification)" unticked in off and ticked in violet in on, so the pair now proves the toggles. In toggles-on the paragraph under them still begins "Both off by default", which reads oddly beside two ticked boxes, and the top of both captures is a cut Sessions list over an empty card bottom. All captures were re-rendered.

## Weak

- paused-*-light, suspended-*-light, tier2-*-light: the beige notice sits close to the grey canvas.
- paused-*, suspended-*, tier2-5s-*, tier2-appearance-* (1440x900, both themes): in the python3 (http.server) row, the state "Left behind while closed" overlaps the CPU value "0.2 %".
- toggles-off-1440x900-*, toggles-on-1440x900-*: a cut Sessions list and an empty card bottom fill the top of the panel above the Settings card.
