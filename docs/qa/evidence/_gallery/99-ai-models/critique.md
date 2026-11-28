<!-- captures: 8671799d651a -->
# 99-ai-models: Settings › AI › Models

Design critique (D-57) of Settings › AI › Models in every state of `ui/tests/visual/models-gallery.spec.ts` (`ui/src/preview/ai-models.ts` `STATES`), against the approved mock-up (`~/Dev/AIJuggler-design-refs/models-screen/`, AJ-E3812), the D-126 rich mode, PAT-SURF, PAT-COLOR and the critique checklist (VALIDATION §5). New family (AJ-E3826): the gallery's captures were written only to a scratch folder before.

## 0.5.2 (2026-11-27, D-154, Opus, both themes)

Re-captured with `AIJ_MODELS_OUT` (52 frames, axe clean). The 22 changed 1280 frames were opened by eye in both themes; `downloading` and `nolist` are unchanged.
- **What changed:** the feed's AI summaries are an opt-in Beta on Apple's model (AJ-E3831's re-measure).
  - Each job uses gains "AI summaries in the feed" with its Beta switch, off. Its honest note reads "Off · Beta: preferred in 69 % of blind checks, but about 1 line in 5 got something wrong."; the covers line is the review's; Automatic is its picker.
  - Apple's recommended card lists it among its Betas, but **Use it does not turn it on** ("Turns on Turn reviews, Bot's AI and Digest as Beta"): only its own switch does.
  - The downloaded models' cards say "Not yet: AI summaries in the feed, digest and watches". An acronym keeps its capitals in that list; it read "ai summaries" before this fix.
- **Judged:** the new row has the same rhythm and controls as its neighbours, and the note's warning sits in the row's own line, never in a banner. Scores below are unchanged.

## 0.5.2 (2026-11-26, AJ-E3826, Opus, both themes)

Captured from the shared tree at HEAD (24c953b1b) with `AIJ_MODELS_OUT`, 13 states × dark and light × 1280 and 900 (52 frames, axe clean on every 1280 frame); every frame opened by eye, the 1280 ones kept here (`<state>-1280-<theme>.png`). States: `inuse` (Apple's model running, 12 answers today), `failed` (timed out), `installed` (three models, Nemotron picked, its Remove confirmation and the Digest picker open), `offline`, `approval`, `models` (nothing in use: Recommended with Use it), `noapple`, `downloading`, `nodisk`, `nolist`, and the engine-setup states `appleoff`, `noanswer`, `blocked`.

**What changed.** Once a model is in use the page opens on **Bot's AI**: each model that can write Bot's answers is a choice with its tile (Apple's chip, a downloaded model's letters, Automatic's sparkle when there are two or more), the current one in the accent's soft fill with its live state under it ("Running · answered 2 min ago" with the live dot; "Idle · starts when you ask Bot"; "Not running: … took too long to answer. Try again, or ask a shorter question." behind the warning's mark on neutral), and today's line ("Today · 12 answers · 8.1k tokens in · ≈ 2.3k est. out"). Recommended for this Mac shows only while no model is in use, so "In use" is gone from it. **Each job uses** lists all six jobs, each row led by the kit's 30 px chip (lit while the job is on) and one line: "Off · Not built yet" (turn reviews), "On · Beta with Apple's on-device model", "On · Built from the facts, so it needs no model", "Off · The plain line reads better for now.", "Off · Preferred over the plain roundup in 64 % of blind checks.", "Off · Drafts are being made exact before Bot fills the form."; no bare "Not yet", no rules between rows. The picker's options and its field carry each model's icon. The model cards, Use it's line and the second-model approval name only the jobs this version has (no Turn reviews · Beta beside a review that isn't built). Recommended is a tint (the accent's soft step over the surface) with no shadow, no colour-mixed edge; its checklist marks are the glyph in its hue on the neutral tone; cards and chips have no hairline ring; type is on the scale (meta, label, section).

**Judged.** Dark and light side by side at 1280 and 900. The chosen row's soft fill reads in both themes (paler in light, where the lit tile and the semibold name are its second cue). The live dot and the warning mark carry their state with words beside them. At 900 the job pickers drop under their row and the Bot's AI choices stay one column; nothing is clipped. The page's one filled primary stays (Use it, Download, or the setup guide's own). The jobs' lines read as answers, not labels; "Off · Not built yet" is the one terse line, honest for a job the version doesn't have.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Hierarchy | 4 | One answer first: Bot's AI once in use, Recommended (or the setup guide) before. |
| 4 | Mode | 4 | Rich: every choice and row its icon or tile; on and chosen in the accent's soft step. |
| 6 | Areas | 4 | Tone, no outlines, no rules between job rows, no shadow on a card that doesn't float. |
| 7 | Colour | 4 | Accent for chosen, on and Beta; live for running; amber only as the failure's mark and the disk refusal. |
| 11 | Plain words | 4 | State and why in one line per job; estimates labelled "≈ … est."; no internal terms. |
| 14 | Consistency | 4 | The kit's RowChip, Switch, Picker (option icons), Alert; the same status store as Bot's header. |
| 19 | Rich-mode colour | 4 | Before (c7243d086): the jobs' chips neutral while on, the picker's options without icons (allowlisted), amber and green fills behind the requirement marks. Now every choice its icon, on and chosen in the accent's soft step, chosen a fill, no bright edge. |

**Weak.**

- The failed line names the model twice (the row's title, then "Apple's on-device model took too long…"): AJ-E3825's failure words are shared with Bot's panel, where the name isn't above them.
- Other tested models still lists the remote list's watch and digest notes as they were signed (the nightly list), which can trail builtin.json's until the list is republished.

## 0.5.2 re-capture (2026-11-27, AJ-E3830 and AJ-E3820, Opus, both themes)

Captured from the worktree at HEAD (b74eab212, nothing uncommitted under `ui/`) with `AIJ_MODELS_OUT` to a scratch folder, 13 states × dark and light × 1280 and 900 (52 frames, axe clean on every 1280 frame); the 1280 ones kept here. Each kept frame was compared pixel by pixel with the one before (24 changed, `nolist` both themes unchanged and left as it was) and every changed frame opened by eye, light and dark side by side.

**What changed, by capture.**
- `inuse`, `failed`, `offline`, `installed` (Bot's AI on): **Each job uses**' Watches row was "Off · Drafts are being made exact before Bot fills the form." with no tag or switch. It now reads "On · Fills in what it understands from your words; you check the form and save." with its Beta tag, a switch (off) and the Beta line under it, "Beta: when Bot understands every part of a request, it asks you to confirm in its own card instead" (d3ccfce2e). Installed's Apple row (and Nemotron's in `installed`) now says "used by Bot's AI, Digest and Watches" ("used by Bot's AI and Watches" in `failed`, where Digest is off), since Watches rides Bot's model. In `installed` the open Digest picker now sits over the Watches row's switch (the menu floats there; the row is under it only in this frame).
- `models` (nothing in use, Bot's AI off): the Watches row reads "Off · Uses Bot's AI: turn it on above", with no tag and no switch: it can't be turned on alone (D-127, never an unusable control).
- Every state with a job list, and every model card (AJ-E3820 R4, 59ce09876): Turn reviews was "Off · Not built yet". It is now a Beta row like Digest, with its tag, switch (off), model picker (Automatic), "Off · Preferred over the plain review in 81 % of blind checks." and the line naming what it covers ("For Claude Code and Codex in the terminal, whose turn ends AI Juggler sees"). Each tested model's card lists Turn reviews · Beta above Bot's AI with its own figure (Qwen3 89 %, Granite 77 %, Nemotron 71 %, Gemma 74 %). Recommended's chips start with Turn reviews and Use it's line says "Turns on Turn reviews, Bot's AI and Digest as Beta" (`models`); in `noapple` and `nodisk` the Download line says "Then Turn reviews and Bot's AI start as Beta". This is why the frames grew by 104 to 230 px.
- `approval`: "Passed our tests for Turn reviews and Bot's AI." (was "for Bot's AI."). `downloading`: Recommended's chips add Turn reviews · Beta.
- `appleoff`, `noanswer`, `blocked`, `noapple`, `nodisk`: the model cards' Turn reviews lines only; the setup guides' words are as before.

**Judged.** The Watches row now reads like the other Beta rows (tag beside the name, one state line, the switch in the same column as Bot's AI and Digest, the Beta line in the muted step under it), so the list scans as one pattern. Where Bot's AI is off it says why it is off and where to start, rather than showing a switch that would do nothing. The Turn reviews row is the tallest (state plus its coverage line) but stays one row with one switch and one picker, as Digest. Accent only for the tags, the switches that are on and the chosen fill; no new hue. The model cards' two job lines keep their icons and the Beta in the accent text step; the cards grew evenly in each pair, so the grid stays aligned.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Hierarchy | 4 | Unchanged order: Bot's AI (or Recommended), the models, Installed, Each job uses. |
| 4 | Mode | 4 | Rich: every job row its 30 px chip, every Beta its tag; Watches' row now as complete as Digest's. |
| 6 | Areas | 4 | No new boxes: the Beta lines sit inside their rows; the job list stays one card with no rules. |
| 7 | Colour | 4 | Accent for Beta tags, on switches and the chosen row; the off switches neutral; no status hue added. |
| 11 | Plain words | 4 | "Fills in what it understands from your words; you check the form and save." says what the user does; "Uses Bot's AI: turn it on above" says why it is off. |
| 14 | Consistency | 4 | Watches and Turn reviews use the row, tag, Switch and Picker the other Beta jobs use. |
| 19 | Rich-mode colour | 4 | Every job and model line leads with its icon; on switches and Beta tags in the accent, the chips lit only while their job is on; no bright edge; both themes read the tags and the muted Beta lines (axe AA clean). |

**Weak.**

- `installed`: the Digest picker's open menu covers the Watches row's switch in this frame (a floating menu over the row below; closing it shows the row whole).
- The Turn reviews row's coverage line wraps to two lines at 1280 beside the switch and picker; it is the one three-line row in the list.
- Carried over: the failed line names the model twice; Other tested models' notes come from the signed remote list and can trail builtin.json until it is republished.

## 0.5.2 G72 (2026-11-27, Opus, both themes)

Captured from a clean checkout of c55c152da with `AIJ_MODELS_OUT`, 13 states × both themes × 1280 and 900; each 1280 frame compared byte by byte with the one before: `inuse`, `failed`, `installed` and `offline` changed (both themes, the states where the Watches row is on), the other nine are byte-equal and left as they were. Every changed frame opened.

**What changed.** The Watches row's second line says the Beta's state. It read "Beta: when Bot understands every part of a request, it asks you to confirm in its own card instead" whether the switch was on or off (RW1 F-S-8). Off, as in every frame here, it now reads "Beta off: Bot opens the Watches form for you to check. Turn it on and, when Bot understands every part of a request, it asks you to confirm in its own card instead". On (vitest; no gallery state has it on), the row reads "On · Beta with Bot’s own card", the other Beta rows' way ("On · Beta with …"), with "Beta on: … instead of the form".

**Judged.** The row is now readable without pressing the switch, in the words the other rows use; the line is the faint third-step grey, as Turn reviews' coverage line.

**Weak.** The line now wraps to three lines at 1280 beside the switch (the longest in the list); in `installed` the open Digest menu covers its end, as it covered the switch before.

| # | Line | Score | Note |
|---|---|---|---|
| 12 | Words | 4 | "Beta off: …", "Beta on: …", "On · Beta with Bot’s own card" (plain-words lint). |
| 13 | States | 4 | The row says on and off apart; the switch agrees with the words. |
| 14 | Consistency | 4 | "On · Beta with …" as Bot's AI and Digest say it. |
| 19 | Rich-mode colour | 4 | Unchanged: icon chip, the Beta pill in the accent's soft step, the switch the only accent fill. |

The other lines are as the sections above scored them.
