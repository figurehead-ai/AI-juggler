<!-- captures: 22ac17804eea -->
# 72-this-mac: This Mac: memory, processor and swap now, what's using them, and the pressure states

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

On the 1280 pages, the Memory, Processor, Swap and AI Juggler cards and the "What's using it" list lift off the canvas. Each big number (22 GB, 18%, 1.2 GB) leads its card, and the state line ("Plenty of room" green, "Getting tight" amber, "Under memory pressure" red) sits clearly above them. After the design fixes the dark accent is lighter on the logo mark and the "AI tools" segment of the memory bar, and the top bar stays one row: in pressure-from-other-apps the status now reads "This Mac · Under pressure", so the actions no longer drop to a second row. The 390x844 captures (nothing-running, plenty, pressure, tight-heavy-leftovers, and the new pressure-from-other-apps) and session-header-heavy are now re-rendered in the 0.3.2 shell. At 390 the This Mac page stacks one column: title, state line, then the Memory card with its bar and legend; the header takes two rows with the broom-and-count cleanup pill. session-header-heavy shows the session page with the heavy line in amber under the title ("CPU 96% · 3.0 GB · 5 more processes — working your Mac hard"); after the second pass its terminal's black fills the card to the edges with the text inset, and the rail's Open now row keeps its time whole ("ai-j… · 3 h ago" where it read "ai-jugg… · 3 …"). Still off: the small cards don't line up (Processor is shorter than Swap, so the AI Juggler card hangs under it with a gap and its "Kept awake while 2 AI…" line is cut at the window's bottom on plenty), and "Claude app · 1 window · not started by AI Jugg…" is cut mid-word.

## Weak

- plenty-1280x800-*, nothing-running-1280x800-*: the Processor and Swap cards have different heights, leaving a gap above the AI Juggler card.
- plenty-*, pressure-*, tight-heavy-leftovers-1280x800-*: "Claude app · 1 window · not started by AI Jugg…" is cut mid-word.
- session-header-heavy-1280x800 (both themes): the rail is captured scrolled; its top row shows only "storefront · AI Juggler" with the name cut off, and the search and switch are out of view. The rail's pressure row reads "Working your Mac hard — CP…".

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B3)

Re-captured at the window's sizes: 1280×800 and, in place of 390×844 (below the window's minimum, F51's rule), 1024×680 (this-mac-gallery's VIEWPORTS); session-header-heavy unchanged. Opened at full size: tight-heavy-leftovers-1024x680-light. At 1024 the folded rail's This Mac dot opens the page with its tip ("This Mac · Getting tight"); the page keeps its title, the state line with its mark, the Memory card with its bar and legend, then Processor and Swap.

- **390×844 → 1024×680:** ten 390×844 captures removed, ten 1024×680 added; their 0.3.2 notes (two-row header, broom pill) went with that size.
- **Fixed** (51ea3254f): a fast swap read "3004 MB a minute"; past a gigabyte it reads "2.9 GB a minute" (`machine_alerts::swap_rate_words`, test a_large_swap_rate_reads_in_gigabytes; the preview's words follow). Shows on 81-this-mac's pressure page.
- **Weak, kept:** "Claude app · 6 windows · not started by AI Jugg…" still cuts in What's using it at 1280; the Memory card's explanation is two paragraphs of help under the legend.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Why the Mac is slow, and what's using it. |
| 2 | Prominence | 4 | The state line, then Memory, then the list. |
| 3 | Default | 4 | "What it means" folded. |
| 4 | Mode | 4 | Calm dashboard. |
| 5 | Mock-up | 4 | AJ-E2105's page. |
| 6 | Areas | 4 | Now and What's using it, cards by tone. |
| 7 | Colour | 4 | Green, amber, red only for the state; violet for AI tools' share. |
| 8 | Type | 4 | Big figures lead; legend muted. |
| 9 | Actions | 4 | No filled primary; the page informs. |
| 10 | Serious | 4 | n/a: quitting is the user's, in their apps. |
| 11 | Doors | 4 | Each App in What's using it opens its page. |
| 12 | Words | 4 | Plain; after the fix rates read in GB past a gigabyte. |
| 13 | States | 4 | Nothing running, plenty, tight, pressure each said. |
| 14 | Consistency | 4 | 81-this-mac's page, the same parts. |
| 15 | Accessibility | 4 | axe clean in the capture runs (no serious or critical); keyboard and zoom not re-walked this round. |
| 16 | Hand walk | 4 | Not re-walked by hand here; captured from the fixtures, both themes. |
| 17 | System prompts | 5 | Causes no system prompt. |
| 18 | Usability | 4 | Re-read against PAT-USE: memory is measured where it runs, nothing sent, said on the page. Match. |

Lowest: 4 (every line 4 or 5).
