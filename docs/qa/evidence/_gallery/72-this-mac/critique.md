<!-- captures: 5a1f0f95b004 -->
# 72-this-mac: This Mac: memory, processor and swap now, what's using them, and the pressure states

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

On the 1280 pages, the Memory, Processor, Swap and AI Juggler cards and the "What's using it" list lift off the canvas. Each big number (22 GB, 18%, 1.2 GB) leads its card, and the state line ("Plenty of room" green, "Getting tight" amber, "Under memory pressure" red) sits clearly above them. After the design fixes the dark accent is lighter on the logo mark and the "AI tools" segment of the memory bar, and the top bar stays one row: in pressure-from-other-apps the status now reads "This Mac · Under pressure", so the actions no longer drop to a second row. The 390x844 captures (nothing-running, plenty, pressure, tight-heavy-leftovers, and the new pressure-from-other-apps) and session-header-heavy are now re-rendered in the 0.3.2 shell. At 390 the This Mac page stacks one column: title, state line, then the Memory card with its bar and legend; the header takes two rows with the broom-and-count cleanup pill. session-header-heavy shows the session page with the heavy line in amber under the title ("CPU 96% · 3.0 GB · 5 more processes — working your Mac hard"). Still off: the small cards don't line up (Processor is shorter than Swap, so the AI Juggler card hangs under it with a gap and its "Kept awake while 2 AI…" line is cut at the window's bottom on plenty), and "Claude app · 1 window · not started by AI Jugg…" is cut mid-word.

## Weak

- plenty-1280x800-*, nothing-running-1280x800-*: the Processor and Swap cards have different heights, leaving a gap above the AI Juggler card.
- plenty-*, pressure-*, tight-heavy-leftovers-1280x800-*: "Claude app · 1 window · not started by AI Jugg…" is cut mid-word.
- session-header-heavy-1280x800 (both themes): the rail is captured scrolled; its top row shows only "storefront · AI Juggler" with the name cut off, and the search and switch are out of view. The rail's pressure row reads "Working your Mac hard — CP…".
