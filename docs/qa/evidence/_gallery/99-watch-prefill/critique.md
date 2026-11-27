<!-- captures: 47f02a3b69d7 -->
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
