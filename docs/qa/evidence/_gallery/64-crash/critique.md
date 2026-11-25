<!-- captures: 69916994daa5 -->
# 64-crash: the crash-report ask after a crash, and the crash-report setting in Privacy

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

In ask, the dialog puts the whole report in a dark code box under a short plain sentence, and the solid violet Send, with its focus ring, clearly leads Don't send and Always send; in dark, Send now uses the lighter violet. The report lines now wrap inside the box, so the long frame line ends whole on a second line ("app/src/commands.rs:4211:41") instead of losing ":41" at the right edge. It breaks after the hyphen in "juggler-", which is acceptable for a path. The report still stops mid-way, with the "app_version" line half cut at the bottom of the box and no fade or scroll cue. setting is re-framed on its subject: the "Crash reports · Ask me each time" row now sits near the top of the view with its scrub explanation and "1 kept on this computer" line, followed by the What it reads list. It still opens on the tail of the line above ("days, then only totals."), sliced at the top edge, with no Privacy section heading in view. The rail's ai-juggler row now reads "Stopped with an error" whole in both captures.

## Weak

- ask-1280x800-dark/light: the report box ends mid-report, with the "app_version" line half cut and no scroll hint.
- setting-1280x800-dark/light: the view opens on a sliced line ("days, then only totals.") above Crash reports, with no section heading in view.

## 0.5.1

Re-captured against 0.5.1: the next-launch question keeps its scrubbed report in a dark well with Send focused, and the Privacy rows are calm tonal cards; the setting reads Ask me each time with the kept-report note under it. Content did not change.

Nothing broken (checked both themes).

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Behind the dialog the rail and Needs you are in their 0.5.1 style; the dialog itself is unchanged. Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.

## 0.5.2 (2026-11-25, AJ-E1558, Opus side-by-side, both themes)

Re-captured on the host from a clean export of HEAD (the shared tree held another agent's unbuilt Bot work), both themes, every changed capture diffed (changed-pixel count and box) and looked at old beside new in contact sheets; the ones named below also opened at full size. Mock-ups: `d126-rich-ui/kit-dark.png` and `notif-dark.png` (rich mode), `calm-ui/mock-r2-*.png` (the rail). Captures: setting, both themes.

**What changed.** Crash reports and session search lead with their chips (a bug, a search), the switch is the kit's, the rows spaced instead of ruled.

**Judged.** Dark and light side by side: in dark the chips and the accent's soft step read clearly against the cards; in light the soft step is paler but the lit chip's violet glyph, the switch's accent fill and the chosen fill still mark what is on (two cues each, never colour alone). No outline came back (PAT-SURF): separation is the card's tone and spacing. Text keeps the type roles (14 px titles, 13 px help), AA in both themes (the gate's contrast spec). Kit rows.

| # | Line | Score | Note |
|---|---|---|---|
| 4 | Mode | 4 | Rich, the kit's dials: icons on every row and choice, the accent for on and chosen. |
| 6 | Areas | 4 | Tone, not outlines; no box in a box. |
| 7 | Colour | 4 | The accent means on, chosen, recommended or navigation; status keeps its hue (amber words on neutral, never a fill). |
| 14 | Consistency | 4 | The kit's own pieces (RowChip, the switch, ChoiceOption cards, SELECTED_FILL, Modal's icon tile). |
| 19 | Rich-mode colour | 4 | Before (42aab7661): 2, monochrome rows, bare checkboxes, a dot or a ✓ alone for chosen. Now every row and choice its icon, on and chosen in the accent's soft step, chosen a fill. |

The other lines are as the 0.5.1 critique scored them (unchanged by this work; lowest 4).
