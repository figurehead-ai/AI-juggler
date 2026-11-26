<!-- captures: 56504cd46c0b -->
# 99-ai-models: Settings › AI › Models

Design critique (D-57) of Settings › AI › Models in every state of `ui/tests/visual/models-gallery.spec.ts` (`ui/src/preview/ai-models.ts` `STATES`), against the approved mock-up (`~/Dev/AIJuggler-design-refs/models-screen/`, AJ-E3812), the D-126 rich mode, PAT-SURF, PAT-COLOR and the critique checklist (VALIDATION §5). New family (AJ-E3826): the gallery's captures were written only to a scratch folder before.

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
