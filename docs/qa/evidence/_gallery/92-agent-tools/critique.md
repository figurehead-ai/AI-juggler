<!-- captures: e658d849545f -->
# 92-agent-tools: agent tools, the question on a pool's canvas and the setting with where they are off

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

agent-tools-question puts the question card ("Let agents in your projects see each other and work together?") above the Checkout redesign canvas with one solid violet "Yes, let them" and an outlined "Not now", a clear single primary; the dotted canvas holds the lead node (feat/read-model, Lead, Working) and the Activity panel on the right lists This pool's facts and an honest empty Live state, readable in both themes. "ADD A SESSION 0 ▾" in spaced capitals still looks like a section label rather than a control. settings-agent-tools shows "Agent tools in the sessions AI Juggler starts" ticked and a "Where they're off · 1 off" fold whose Projects and Profiles checklists tick the ones that do get the tools, so under a heading about where they're off a tick means on; the unticked ai-juggler is the one that is off. The top of the capture still shows a stray row of dots from the content above the edge. The design fixes show in dark as the lighter accent on the logo mark, New session, "Yes, let them", the "Change what agents may do…" link and the four ticked checkboxes, which now stand out more from the near-black; in light only the top bar's "This Mac · Plenty of room" shifted by a sub-pixel, and the bar keeps one row with the whole "7 to clean up" pill. In the second pass the rail's "Refactor the settings screen" row keeps its age whole ("ai-ju… · viewed 2 min ago") in all four captures, and in agent-tools-question-light the canvas re-centred: the lead node now sits lower and further right, in the middle of the dotted area, while the dark capture keeps it near the top left, so the two themes no longer frame the canvas the same way.

## Weak

- settings-agent-tools-1280x800-dark.png, settings-agent-tools-1280x800-light.png: ticks mean "gets the tools" under the heading "Where they're off".
- agent-tools-question-1280x800-dark.png, agent-tools-question-1280x800-light.png: "ADD A SESSION" in capitals with a count reads as a label, not a picker.
- settings-agent-tools-1280x800-dark.png, settings-agent-tools-1280x800-light.png: a stray row of dots at the top edge, above "Rules you made".
- agent-tools-question-1280x800-light vs -dark: the lead node sits in a different place on the canvas in each theme (centred in light, top left in dark).
