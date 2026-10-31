<!-- captures: 3de5ee075533 -->
# 83-keep-going: restarting crashed sessions and keeping the Mac awake

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

settings-keep-going groups the options under "Keep work going" on Settings › Sessions as checkboxes with a sentence each (restart sessions that crash, keep the Mac awake, on battery too, reopen AI Juggler, open at login); the settings card stands off the canvas in both themes and the explanations are a clear step below the labels. In dark the ticked checkboxes and the violet Desktop apps and Switching tiles in the settings list now use the lighter accent. launch-setting-restart shows the "If a session crashes: Restart it" choice inside a launch setting, with solid violet Launch buttons and Add an account (lighter in dark) and a pale, clearly disabled Save overrides. this-mac-awake is meant to show that the Mac is kept awake, but its "Kept awake while 1 AI Juggler…" line still sits at the very bottom of the AI Juggler card and is cut by the window edge, so the state it documents is half visible. The header stays one row in every capture. After the second pass the rail's "Refactor the settings screen" row keeps its time whole ("ai-ju… · viewed 2 min ago" where it read "ai-juggler · viewed 2 mi…"); nothing else changed.

## Weak

- this-mac-awake-1280x800 (both themes): the "Kept awake while 1 AI Juggler…" line is cut by the bottom of the window.
