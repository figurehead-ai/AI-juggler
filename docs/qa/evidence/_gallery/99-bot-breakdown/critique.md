<!-- captures: ed2cd3250517 -->
<!-- captures: pending -->
# 99-bot-breakdown: Bot's context and memory breakdown, and a memory as a record

Design critique (D-57, Opus, 2026-11-26) of AJ-E3832's breakdown (AJ-E3825 R9 rebuilt on the world model) and G9's memory record, against the reference screens: Bot's panel (the bot gallery), the calm kit (`AIJuggler-design-refs/calm-ui`) and the menus' popover surface (77-menus). Captured by `ui/tests/visual/bot-breakdown-gallery.spec.ts` (`AIJ_BREAKDOWN_OUT`), in both themes at 1280×860 and 1024×860; axe has 0 critical or serious findings on each; each indicator opens with Enter and closes with Escape (the spec).

## Checklist

| Check | Finding |
|---|---|
| Grid | The breakdown is one column of label and value rows, then the linked lines, then "Used of the window" under a divider; the memory record is a Bot row like the others (mark, name, where-words, time on the right). |
| Hierarchy | Part labels muted, values in ink; each linked line its name in ink, then "· why" faint; the record's name in ink, "What Bot remembers · your name for api" muted. |
| Spacing | The breakdown's rhythm; the linked lines a little tighter than the part rows, as a sub-list. |
| Icons | The memory record carries the bookmark (as What Bot remembers and the alias pill do). |
| Shared controls | The usage bar's own indicator buttons; the popover surface the menus use (tone and soft shadow, no bright border). |
| Disabled reasons | None here. |
| Plain words | "Linked from your work", "named in your question", "your own name for it", "connected to api", "matches your words", "and 1 more", "What Bot remembers · your name for api", "you said, 2 d ago". No internal term (node, graph, seed, walk). |
| Type sizes | 12 px throughout (the bar's label size). |
| Both themes | Checked; the popover on the overlay tone in light, surface-2 in dark; the faint why-words readable in both (axe colour-contrast clean). |
| 1280 and 1024 | Both. Before this round the breakdown opened rightwards from its indicator and ran past the window in the docked panel (its right half cut off at 1280); it now opens leftwards when it would. |

## Good

- The breakdown answers "what did Bot look at?" in the one place that already listed what was sent: the world model's part has a size like every other part, and the lines under it say what and why in a few words.
- A memory Bot answered from is a record of its own: cited like a session, marked with the bookmark, and opening What Bot remembers on that memory.
- The linked list is bounded (six, then "and N more"), so a busy question doesn't grow the popover.

## Weak

- A memory's name in the linked list keeps its quotation marks (“the payments agent”), which the session names don't have; consistent with how Bot quotes the user's words in answers, but it reads a little heavier.
- The memory record's where-words wrap to two lines at 1024 ("What Bot remembers · your / name for api").

## Scores

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 5 | See what went to the model and why; open the memory an answer rested on. |
| 2 | Prominence | 4 | Hidden until asked (an indicator), as the bar's other numbers. |
| 3 | Default | 5 | Nothing to choose. |
| 4 | Mode | 5 | Calm: muted numbers, one popover surface. |
| 5 | Mock-up | 4 | No mock of its own; follows AJ-E3825's breakdown and the bot gallery. |
| 6 | Areas | 5 | Parts, linked lines, total; the record among the rows. |
| 7 | Colour | 4 | No accent but the focused indicator's ring and the Manage link; the memory record in the rows' tones. |
| 8 | Type | 4 | One size; ink, muted and faint for value, label and why. |
| 9 | Actions | 4 | Manage what Bot remembers and Settings › AI in the memory breakdown; the record opens its memory. |
| 10 | Serious | n/a | Nothing destructive here. |
| 11 | Doors | 5 | The record opens What Bot remembers on that memory; the links open their pages. |
| 12 | Words | 4 | Plain; "and 1 more" could name what. |
| 13 | States | 4 | Warming up or nothing linked: the part reads 0 and no list (not captured: an empty list is the AJ-E3825 capture). |
| 14 | Consistency | 4 | The bar's popover pattern; the memory row as Bot's rows; quotation marks on memory names (above). |
| 15 | Accessibility | 4 | axe clean on all 12; Enter opens and Escape closes each indicator; the record's link names the memory and where it is. |
| 16 | Hand walk | 4 | Driven by the spec in a built app with fixture data; not walked by hand in Dev. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | AJ-E3832's six answers hold (the task file): the part is named, the list says why, nothing unsent is listed, no internal terms. |
| 19 | Rich-mode colour | 4 | Not a create, edit or settings surface; its colour is the calm bar's: values in ink, why-words faint, the record's bookmark on the rows' tile tone, the accent only on focus and the Manage link. |

Lowest: 4 (lines 2, 5, 7, 8, 9, 12, 13, 14, 15, 16, 18, 19). Fixed in this round: the breakdown ran past the window's right edge in the docked panel (it opens leftwards now); the linked lines' names and reasons truncated each other (each truncates in its own room); the memory record showed no time (it says "you said, 2 d ago").

## 0.5.2 G35: the model's name under "Bot" (Opus, 2026-11-26)

All twelve re-captured from a clean export of HEAD (`bot-breakdown-gallery.spec.ts`), both themes, 1280 and 1024; every one opened. Nothing in the breakdowns or the record changed; only Bot's panel head did.

**What changed.** The head read "Bot · Bot’… · Beta" (the model and its state between "Bot" and the head's buttons, cut to a few letters; RW1 F-RW1-17 saw "Apple on-d…" at 1755 wide). Now "Bot" with its Beta pill on the first line and the model with its state on the line under it ("Bot’s AI" here: the preview has no engine; on a Mac "Apple on-device · Running"), the whole head's width up to the buttons, the whole words as its title when cut.

**Weak.** At the panel's narrowest (340 px, not captured) with Beta, the watch count and every button, a long model name with its state can still be cut; its title then holds it whole.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 5 | Which model answers, and how it stands, at a glance. |
| 2 | Prominence | 4 | Muted, under the title; the title stays the head's one strong word. |
| 3 | Default | 5 | Nothing to choose. |
| 4 | Mode | 5 | Calm: a muted second line, no new surface. |
| 5 | Mock-up | 4 | The panel head of D-136, its engine line moved under the title. |
| 6 | Areas | 5 | One head, two lines, the buttons where they were. |
| 7 | Colour | 4 | Ink title, the Beta pill's accent soft step, the engine line muted (a failure keeps its danger word). |
| 8 | Type | 4 | The section title over a label-size line. |
| 9 | Actions | 4 | Unchanged. |
| 10 | Serious | n/a | Nothing destructive here. |
| 11 | Doors | 5 | Unchanged. |
| 12 | Words | 4 | The engine's own short name and its state word. |
| 13 | States | 4 | Running, starting, idle, failed: one line each (vitest). |
| 14 | Consistency | 4 | Title over a muted line, as the Bot page's head. |
| 15 | Accessibility | 4 | axe clean on all 12; the title attribute carries the whole line. |
| 16 | Hand walk | 4 | Driven by the spec in a built app with fixture data; the re-walk reads it at 1755 in Dev. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | "Which model is answering me?" answered without hovering or widening the panel. |
| 19 | Rich-mode colour | 4 | Not a create, edit or settings surface; the head's colour is calm: ink, muted, and the accent only on Beta's soft pill. |

Lowest: 4.


## 0.5.2 G61: the model's line across the head (Opus, 2026-11-27)

All twelve re-captured, plus `head-{light,dark}-1755.png` (new): `bot-breakdown-gallery.spec.ts` on the working tree at 1755, 1280 and 1024, both themes; every one opened. The spec now gives the preview the engine as the walk saw it ("Apple on-device · Running"), so every capture shows the line at its real length, and a new test asserts the line isn't cut with the watch count up at all three widths.

**What changed.** RW1 R31 saw "Apple on-device · Runn…" at 1755 with the watch count among the head's buttons: the line under "Bot" only had the room left of the buttons. Now the head is a grid: "Bot" and Beta, then the buttons (watch count, New topic, Search, Expand, ⋯, Close) on the first row, and the model line spans the whole head under both, whole in all fourteen captures. The head is a few pixels taller (about 52 px against 48); the ⋯ menu opens under it.

**Weak.** The line now runs under the buttons, so at a glance it reads as the head's subtitle more than "Bot"'s; its left edge still lines up with "Bot", which keeps the pairing.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 5 | Which model answers and how it stands, whole, whatever chips are up. |
| 2 | Prominence | 4 | Muted, under the title, as before. |
| 3 | Default | 5 | Nothing to choose. |
| 4 | Mode | 5 | Calm: no new surface. |
| 5 | Mock-up | 4 | D-136's panel head; the engine line under the title (G35), now the head's width. |
| 6 | Areas | 5 | Title and buttons on one row, the model line under them. |
| 7 | Colour | 4 | Unchanged: ink title, Beta's soft pill, muted line. |
| 8 | Type | 4 | Unchanged. |
| 9 | Actions | 4 | Unchanged; the ⋯ menu placed under the taller head. |
| 10 | Serious | n/a | Nothing destructive here. |
| 11 | Doors | 5 | Unchanged. |
| 12 | Words | 4 | "Apple on-device · Running", whole. |
| 13 | States | 4 | Running here; starting, idle and failed as before (vitest). |
| 14 | Consistency | 4 | Title over a muted line, as the Bot page. |
| 15 | Accessibility | 4 | axe clean on all fourteen; the line keeps its title attribute. |
| 16 | Hand walk | 4 | Driven by the spec in a built app with fixture data at the walk's 1755. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 5 | The walk's cut line can't happen at the panel's widths with the count up (asserted). |
| 19 | Rich-mode colour | 4 | Not a create, edit or settings surface; calm colour, the accent only on Beta. |

Lowest: 4.

## 0.5.2 G74: a long title truncates before its state (Opus, 2026-11-27)

Captured from a clean export of e5f44de9c (`bot-breakdown-gallery.spec.ts`): four new frames, `long-title-{light,dark}-{1755,1300}.png`, the first answer's rows given the walk's long titles ("Split the cart total into its own module and test it", working; "Add a retry helper to the API client", ended); the other fourteen came out byte-equal and are unchanged. Every new frame opened.

**What changed.** RW1 Part T (T4) saw the title run under the row's state ("…module and tes[Working]", "…API client ● Ended") and the panel scroll sideways: the title and place lines sized to their content inside the row's button. The title now takes the column's width and truncates ("Split the cart total into its own m…", "Add a retry helper to the …") with the state whole at the right; the whole title stays the row's tooltip. The spec asserts the title ends before the state, and nothing in the panel scrolls sideways, at both widths and both themes (it failed 4 of 4 on the old layout).

**Weak.** At the docked panel's width the second line wraps a long place ("Task · Release pool · builder-2" over two lines), as before; the title gives way first, as it should.

| # | Line | Score | Note |
|---|---|---|---|
| 6 | Areas | 5 | Name, place, state: each in its own column, none drawn over another. |
| 8 | Type | 4 | Unchanged sizes; the cut is the ellipsis, the tooltip has the rest. |
| 13 | States | 4 | Working and Ended whole beside a long title. |
| 15 | Accessibility | 4 | axe clean at 1755; the row's accessible name is unchanged (the title whole). |
| 19 | Rich-mode colour | 4 | Unchanged: the rows' calm tones, the state's dot and word. |

The other lines are as the sections above scored them (lowest 4).
