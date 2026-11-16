<!-- captures: 45f1f59ec58e -->
# 92-agent-tools: agent tools, the question on a pool's canvas and the setting with where they are off

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

agent-tools-question puts the question card ("Let agents in your projects see each other and work together?") above the Checkout redesign canvas with one solid violet "Yes, let them" and an outlined "Not now", a clear single primary; the dotted canvas holds the lead node (feat/read-model, Lead, Working) and the Activity panel on the right lists This pool's facts and an honest empty Live state, readable in both themes. "ADD A SESSION 0 ▾" in spaced capitals still looks like a section label rather than a control. settings-agent-tools shows "Agent tools in the sessions AI Juggler starts" ticked and a "Where they're off · 1 off" fold whose Projects and Profiles checklists tick the ones that do get the tools, so under a heading about where they're off a tick means on; the unticked ai-juggler is the one that is off. The top of the capture still shows a stray row of dots from the content above the edge. The design fixes show in dark as the lighter accent on the logo mark, New session, "Yes, let them", the "Change what agents may do…" link and the four ticked checkboxes, which now stand out more from the near-black; in light only the top bar's "This Mac · Plenty of room" shifted by a sub-pixel, and the bar keeps one row with the whole "7 to clean up" pill. In the second pass the rail's "Refactor the settings screen" row keeps its age whole ("ai-ju… · viewed 2 min ago") in all four captures, and in agent-tools-question-light the canvas re-centred: the lead node now sits lower and further right, in the middle of the dotted area, while the dark capture keeps it near the top left, so the two themes no longer frame the canvas the same way.

## Weak

- settings-agent-tools-1280x800-dark.png, settings-agent-tools-1280x800-light.png: ticks mean "gets the tools" under the heading "Where they're off".
- agent-tools-question-1280x800-dark.png, agent-tools-question-1280x800-light.png: "ADD A SESSION" in capitals with a count reads as a label, not a picker.
- settings-agent-tools-1280x800-dark.png, settings-agent-tools-1280x800-light.png: a stray row of dots at the top edge, above "Rules you made".
- agent-tools-question-1280x800-light vs -dark: the lead node sits in a different place on the canvas in each theme (centred in light, top left in dark).

## 0.4.6 (AJ-E3011 §7): the question where it applies

agent-tools-question (both themes) no longer has a card over the canvas: the newcomer walks read "Let agents … see each other and work together? … Asked once, for every pool" as a global switch and left it unanswered. It is now the compact card at the top of the pool's setup checklist in the inspector: "Let AI Juggler give these sessions the pool's tools: messages, shared notes and tasks.", its example ("Claude can see that Codex is reviewing and send it a note."), that it is one switch for every pool in Settings › Agent access, then a tinted Turn on and a ghost Not now (the page's one filled button stays elsewhere). After Not now it says what won't work until it's on. The canvas is left whole; the lead node sits where the fit puts it in both themes.

- Weak: the card sits inside the checklist, so the checklist's steps start lower while the question is open.

## 0.4.6 round 2 (AJ-E3011 #1, D-30): team tools, asked once, off per project or app

agent-tools-question (both themes): run 2 left the card unanswered over "one switch for every pool". It is now "Let your assistants work as a team": what it gives (messages, shared notes and tasks) with its example, who it covers (sessions AI Juggler starts or connects in your pools, never one outside a pool), that it costs nothing extra, that without it the pool is set up but nobody can message (in ink, the one line that matters most), then "Asked once. After that you can switch it off for a project or an app in Settings › Agent access" with that place as a link, and a tinted "Turn on team tools" beside a ghost Not now. It sits above the checklist now (not inside it), so it stays once every step is done, and says "Off for web: they can't message there." when the pool's project or App has it off. Review in the setup says the same as a line. Checked dark and light: the facts are ink-2 at 13 px, the link underlined in ink.

- Weak: seven short lines in the 340 px column; it pushes the checklist below the fold at 800 px while unanswered.

## 0.4.6 round 4 (AJ-E3011, main's ruling on D-30): one question, at the top of the pool's page

agent-tools-question (both themes): the question is no longer in the inspector. It is one card across the top of the pool's page while unanswered, the same component as New pool's Team tools step and Settings › Agent access: "Let the sessions in your pools message each other and share notes and tasks?", what they get with its example, "It costs nothing extra: a session uses its own turns only when it acts.", "You can switch it off per project or app in Settings." (Settings a link), then a filled "Yes, let them work together" and a secondary "Not now". Answered, it's gone; Not now leaves one quiet line with Turn on. When the tools can't run from AI Juggler's folder (AJ-E3013), the same place says why and to move it to Applications, with no Yes or Turn on (unit-tested; no capture). Checked dark and light: one primary on the page, the body ink-2 at 13 px.

- Weak: the card takes about 190 px above the diagram at 1280×800 until it's answered.
- Weak: while unanswered, the inspector's summary repeats it as "Team tools: Not answered yet…" with Turn on.
- settings-agent-tools is unchanged by this round (left as committed).

## 0.4.6 round 4 follow-up (Sam's walk)

agent-tools-question (both themes): the card's scope line reads "This is for every pool you make; you can switch it off for a project or an app in Settings." ("your pools" alone was vague). Pass.

## 0.4.6 round 5

agent-tools-question (both themes): the card's cost line reads "It costs nothing extra. Your assistants use their own turns only when they act." and a new line says "This is a setting in AI Juggler; it changes nothing in your apps." (Casey wondered if it changed their Claude app). The inspector beside it is 380 px. Pass.

## Section titles fixed (2026-11-16)

Section titles re-rendered (6db085536): a colour token named like a type size (`--color-section` beside `--text-section`) had Tailwind build `.text-section` as a colour only, so every section title in the captures from 56f98a514 on was about 13 px regular grey. Re-captured and checked by eye against the earlier captures:

- agent-tools-question (both themes): the inspector's title is 15 px semibold ink again, its sections a couple of pixels higher. Nothing else changed. Pass.
