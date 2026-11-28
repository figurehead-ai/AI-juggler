<!-- captures: f2578e4b8f53 -->
# 99-watch-prefill: the Watches form as Bot pre-filled it, Bot's confirm card, the Watches row

Design critique (D-57, AJ-E2814), AJ-E3830 (0.5.2), Opus, 2026-11-27. Captured on the host from the working tree at the UI commit (`watch-prefill-gallery.spec.ts`: Bot docked at 1280, its turn opening the form; the card; Settings › AI › Models' jobs card), both themes; every capture opened. Mock-ups: the Watches form's approved mock and the d126 kit (`~/Dev/AIJuggler-design-refs/`), Bot's confirm card as the row's Watch… card (`bot` gallery's `watch`).

**What changed, by capture.**
- `form-left-1280-*`: Bot's turn "Tell me every time api finishes, until I remove it, yes even overnight" opened the form beside the docked panel. The form now folds by its own room (`FlowLayout fit="room"`, under 700 px): beside Bot the preview is the one-line strip at the top and the form has the full column (before this change the two columns squeezed the form to about 160 px). Under the head, a calm "Filled in by Bot" note (accent's 6 % tint, the Bot mark in its soft chip): "Bot filled in what it could read. How long and quiet hours are left for you. It couldn't read “…”." The target chip (api, Agent CLI) and Its turn ends are filled; Until shows no chosen segment and "Bot couldn't tell how long from your words. Pick one."; the quiet-hours row stays off with "Bot couldn't tell whether this should wake you. Check it."; the foot says "Pick how long first." with Watch off. The strip says the watch without a lifetime (none was chosen). Bot's line: "I couldn't read “…”. Left for you to pick: how long and quiet hours. The Watches form is open."
- `form-nearest-1280-*`: "Let me know when api wants something from me": the target filled, no condition card ticked (the form's own default is not applied to a Bot pre-fill), Asks you carries a "Nearest" tag in the accent's soft step, For what's line says "Bot couldn't tell what for". Until is the form's usual suggestion (Today, as with any new watch with no condition).
- `card-1280-*`: "Let me know when api finishes", read whole, Watches Beta on: Bot's line "Here is the watch. Check it, then click Watch." and the confirm card (the row card's own shell, eyebrow, echo, quiet line, Watch / Change… / Cancel, "acts only" line, now shared parts), with "How long wasn't said: this is the suggestion. Change… to pick another." The form does not open.
- `models-row-1280-*`: the Each job uses card: Watches now reads "On · Fills in what it understands from your words; you check the form and save." with its Beta tag, switch (off) and the line "Beta: when Bot understands every part of a request, it asks you to confirm in its own card instead". Before: "Off · Drafts are being made exact before Bot fills the form."

**Weak.** The quiet line on the card says 22:00–07:00 while the form says 10:00 PM–7:00 AM (the row card's existing format; one formatter for both is a follow-up). The nearest case's Until shows Today chosen as the form's suggestion with no "Suggested" label, because the label names the first condition and none is chosen yet (as on any empty form). The Models row's chip stays unlit beside a switch that is off, as the other Beta rows do.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | The request becomes a watch in one place; what Bot couldn't read is the user's pick, said there. |
| 2 | Prominence | 4 | The note sits above the sections, the left sections say so in their own lines; nothing new competes with Watch. |
| 3 | Default and folding | 4 | No default where Bot couldn't tell (the honest empty); the suggestion stays labelled where it was already the form's. |
| 4 | Mode | 4 | Rich create surface: icons, cards, preview strip; Bot's note is a calm tinted row, not an alert. |
| 5 | Mock-up | 4 | The form's sections and cards unchanged; the fold beside Bot is the mock's under-960 layout, by room. |
| 6 | Areas and spacing | 4 | One note card, then the four sections; no box in a box. |
| 7 | Colour | 4 | Accent only for the Bot chip, the chosen card, the Nearest tag and the primary; no status hue. |
| 8 | Type | 4 | Note title body-medium, line meta ink-2; section helps unchanged. |
| 9 | Actions | 4 | One primary (Watch), named; "Pick how long first." beside it when off. |
| 10 | Serious choices | 4 | Quiet hours keeps its consequence line; the check line under it says why it is off. |
| 11 | Doors and in-place | 4 | Bot's Open the form and the card's Change… open the same form filled in. |
| 12 | Words | 4 | "Left for you to pick: how long", "Bot couldn't tell…", "Nearest"; no internal word (plain-words lint). |
| 13 | States and speed | 4 | Filled, partly filled, nothing filled ("couldn't fill"), card; the form opens at once from the turn. |
| 14 | Consistency | 4 | The card is the row card's shell (shared parts); the tag is the kit's soft pill; Segmented's "none chosen" is opt-in. |
| 15 | Accessibility | 4 | axe clean on every capture; the note is a `role="note"`; Until's radiogroup keeps the first segment focusable with none chosen. |
| 16 | Hand walk | 3 | Walked in the preview by Playwright and vitest, not by hand in Dev yet (the coordinator's Sonnet walk). |
| 17 | System prompts | 4 | None new; Watch primes Notifications as before. |
| 18 | Usability | 4 | The task's six answers (AJ-E3830, 2026-11-27) hold on these screens. |
| 19 | Rich-mode colour | 4 | Every section and option leads with its icon; the chosen card and the Bot chip are the accent's soft step; Nearest a soft-step pill; no bright border; AA in both themes (axe). |

Lowest: 3 (line 16).

## 0.5.2 G51 (2026-11-27, Opus, both themes)

Re-captured from the worktree at HEAD (b74eab212) with `AIJ_PREFILL_OUT`; each frame compared pixel by pixel with the one before. Only `card-1280-*` changed; both opened by eye. (`form-left-1280-dark` came out once with the strip's echo still "…", a frame taken before the preview answered: not kept; the spec now waits for the echo, and for the card's quiet line.)

**What changed.** The first Weak point above is fixed. `card-1280-*`: the card's quiet line read "Between 22:00 and 07:00 it waits till morning." beside a form that writes "10:00 PM". It now reads "Between 10:00 PM and 7:00 AM it waits till morning.", the times from the form's own rule (the system's clock; G51, a2aeefa50), each kept on one line with its PM/AM. Nothing else on the card moved: the line is a little longer and still fits on one line in the docked panel at 1280.

**Judged.** The card and the form now say the same times in the same style; the moon icon and the muted step are unchanged; light and dark both read.

| # | Line | Score | Note |
|---|---|---|---|
| 12 | Words | 4 | One time style across the card, the form and the Watches page. |
| 14 | Consistency | 4 | The card takes the form's formatter instead of its own. |
| 19 | Rich-mode colour | 4 | Unchanged: the eyebrow and Watch in the accent, the quiet line muted with its icon; no bright edge. |

Lowest: 3 (line 16, unchanged).

## 0.5.2 G60 (2026-11-27, Opus, both themes)

Captured from a clean checkout of 28aae623b (no other session's uncommitted work in the frame), `AIJ_PREFILL_OUT`; every frame opened. `form-left-*` and `card-*` came out byte-equal and are unchanged.

**What changed, by capture.**
- `form-several-1280-*` (new): "Tell me when shop finishes", where "shop" names the project and sessions in it (the RW1 R31 shape: "web" the project and an open session named by its folder). The note says "More than one matches that name: pick which one."; Watch what says "Bot couldn't tell which one", and the picker is open under it with a new first group, "Your words match": api (Agent CLI · shop · Working), Fix the login redirect (Idle), then the project shop, open sessions first. Open now and Recent follow with what the matches don't hold (Open now's count is the rows it shows). Nothing is picked (0 picked); Until's step dot is not lit and Watch stays off. Before: the same form with an empty field and a closed picker, and Until showing Today chosen.
- `form-nearest-1280-*`: Until no longer shows Today chosen; no segment is lit and the line reads "How long is suggested once you pick what to watch for." (the form's suggestion follows the condition; it appears, labelled, once a card is ticked). This was the Weak point of the first section above.

**Judged.** The ambiguous name now ends in a choice the user makes in one click, the candidates named the way the rail names them, with their App and state, and nothing chosen for them; the matches group reuses the picker's own heading and rows (no new pattern). The unchosen Until reads as "not yet", not as Bot's fill.

**Weak.** The foot still says "Starts watching now · 7 of 50." beside a Watch that is off until a target is picked (the form's existing line for a new watch). The open picker covers sections 2 to 4 until it is closed or a target picked, as it does when opened by hand. The strip above says "Looks like this: …" while nothing is picked (as before).

| # | Line | Score | Note |
|---|---|---|---|
| 3 | Default and folding | 4 | No lifetime shown chosen while it depends on what Bot left; the suggestion comes labelled with the pick. |
| 12 | Words | 4 | "Your words match", "How long is suggested once you pick what to watch for." (plain-words lint). |
| 13 | States | 4 | Several matches: the picker open on them; nearest; left; card; unchanged ones byte-equal. |
| 14 | Consistency | 4 | The matches are the picker's own rows under its own heading style; open sessions first, as Open now orders. |
| 15 | Accessibility | 4 | axe clean on both new captures; the picker keeps its search focused and ↑↓ through the matches. |
| 16 | Hand walk | 3 | Walked by vitest and Playwright in the preview; the real-app re-walk of R31/R32 follows. |
| 19 | Rich-mode colour | 4 | The matches lead with each target's own mark and the project's folder icon; the open field keeps the accent ring; no bright border, AA in both themes (axe). |

The other lines are as the first section scored them (lowest 3: line 16).

## 0.5.2 G61: Bot's head in the docked panel (Opus, 2026-11-27)

Re-captured from a clean export of 195dc384e with `AIJ_PREFILL_OUT`, both themes; every frame opened. Eight captures changed (`card-*`, `form-left-*`, `form-nearest-*`, `form-several-*`), all for the docked Bot panel's head only; `models-row-*` came out byte-equal and is unchanged.

**What changed.** Bot's head is a grid now (G61, 5c4005c0c): "Bot" with Beta and the buttons on the first row, the model line ("Bot's AI" in the preview) under them across the head's whole width, so a chip among the buttons (the watch count) never cuts it. Here the line reads the same and sits a few pixels lower; the conversation starts that much lower. Nothing in the form, the card or the picker moved.

**Weak.** None new: the preview's engine is unnamed ("Bot's AI"), so these frames don't show the long line; `99-bot-breakdown/head-*-1755` does.

| # | Line | Score | Note |
|---|---|---|---|
| 6 | Areas | 4 | The panel's head two rows, the form and picker unchanged. |
| 14 | Consistency | 4 | The head as in every Bot capture since G61. |
| 15 | Accessibility | 4 | axe clean on all ten (the spec). |
| 19 | Rich-mode colour | 4 | Unchanged: the form's icons, the accent's soft step on the chosen card, Bot chip and Nearest; Beta's soft pill in the head; no bright border, AA in both themes (axe). |

The other lines are as the sections above scored them (lowest 3: line 16).

## 0.5.2 G72 (2026-11-27, Opus, both themes)

`models-row-1280-*` re-taken from a clean checkout of c55c152da (`AIJ_PREFILL_OUT`); the other frames came out byte-equal. Both opened. The Watches row's second line now says its Beta is off ("Beta off: Bot opens the Watches form for you to check. Turn it on and, when Bot understands every part of a request, it asks you to confirm in its own card instead"); before, the same words showed on and off (RW1 F-S-8). Judged: the state reads without pressing; three lines beside the switch, the longest row.

| # | Line | Score | Note |
|---|---|---|---|
| 12 | Words | 4 | The Beta's state said, as the other Beta rows say theirs. |
| 19 | Rich-mode colour | 4 | Unchanged: icon chip, Beta pill, the switch. |

## 0.5.2 G75: the reply says the form is open (Opus, 2026-11-27)

Re-captured from a clean export of 4fef54b27 with `AIJ_PREFILL_OUT`, both themes; every changed frame opened. Six changed (`form-left-*`, `form-nearest-*`, `form-several-*`), only in Bot's reply; `card-*` and `models-row-*` came out byte-equal and are unchanged.

**What changed.** RW1 Part T saw the reply still offer "Open the form" after the form had opened by itself. Now a reply whose form opened by itself offers no button: `form-left` and `form-nearest` already say "The Watches form is open." in their line, so the button is simply gone; `form-several`'s line ("More than one matches that name, so the Watches form asks which one.") gets a muted "The Watches form is open." under it, lined up with the reply's words. A turn read back later (its form not opened) still offers "Open the form" (vitest).

**Weak.** Once the user closes the form, this live turn has no button to open it again; asking again does (a turn read back offers it).

| # | Line | Score | Note |
|---|---|---|---|
| 9 | Actions | 4 | No action offered for what is already done; the way back is on a turn read back. |
| 12 | Words | 4 | "The Watches form is open." said once, never twice. |
| 13 | States | 4 | Opened by itself, or read back: each its own words (vitest). |
| 19 | Rich-mode colour | 4 | Unchanged: the reply's eye in the accent, the added line muted. |

The other lines are as the sections above scored them (lowest 3: line 16).

**2026-11-27 (D-154, Opus):** `models-row-1280-{dark,light}.png` re-captured and opened in both themes. The Each job uses card gains "AI summaries in the feed" (an opt-in Beta, off, with its honest note and the review's covers line), between Attention hints and Digest. The other rows are as before.
