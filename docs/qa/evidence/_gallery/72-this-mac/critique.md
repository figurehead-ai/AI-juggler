<!-- captures: 52172c2cad2a -->
# 72-this-mac: This Mac: memory, processor and swap now, what's using them, and the pressure states

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

On the re-rendered 1280 pages, the Memory, Processor, Swap and AI Juggler cards and the "What's using it" list lift off the canvas. Each big number (22 GB, 18%, 1.2 GB) leads its card, and the state line ("Plenty of room" green, "Getting tight" amber, "Under pressure" red) sits clearly above them. The small cards don't line up: Processor is shorter than Swap, so the AI Juggler card hangs under it with a gap, and on plenty its "Kept awake while 2 AI…" line is cut at the window's bottom. In pressure-from-other-apps the longer status pill "This Mac · Under memory pressure" pushes the header's actions onto a second row under the tabs. Ten captures were not re-rendered and show an older layout (a Home/Accounts/History/Settings/More header, small-caps labels, the This Mac panel inside Home): every 390x844 capture (nothing-running, plenty, pressure, tight-heavy-leftovers) and session-header-heavy, in both themes.

## Weak

- pressure-from-other-apps-1280x800-dark/light: the header wraps to two rows because of the long status pill.
- plenty-1280x800-*, nothing-running-1280x800-*: the Processor and Swap cards have different heights, leaving a gap above the AI Juggler card.
- plenty-*, pressure-*, tight-heavy-leftovers-1280x800-*: "Claude app · 1 window · not started by AI Jugg…" is cut mid-word.
- The 390x844 captures and session-header-heavy: not re-rendered; they show the older shell.
