<!-- captures: 814f9dead166 -->
# 62-privacy: Settings › Privacy: usage statistics choices, what is collected, the exact weekly report, the dashboard link, and an upload error

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The privacy card stands off both canvases, the selected option tile is tinted violet, and the explanations read clearly. The What's collected tables (monospace keys, values right-aligned) and the exact JSON report in report-json are honest and readable; the focused View button on the 2026-W41 row shows where the report came from. The card still has no heading of its own, so it opens straight on its status line ("Sharing: one small report a week…", "This build doesn't include usage statistics.") under the general Settings title. In overridden, the disabled radios are still faint dashed rings that almost vanish in light. "Share weekly usage stats" stays highlighted even though the line above says it is off because DO_NOT_TRACK is set. The upload-error warning ("⚠ Couldn't reach Figurehead…") is still plain body text with no colour. In not-included, the "What it reads, on your Mac only" rows still use a larger type than the card text above them. report-json at 1024 still opens scrolled with "Features used · 2026-W42" cut at the top. The design fixes show only as the lighter dark accent (selected radio and tile, checkbox, the bar's New session) and, in the 1280 rail, the ai-juggler row now reading "Stopped with an error" whole.

## Weak

- local, off, overridden, share, synced, upload-error, not-included (all): the privacy card has no heading, so the section starts mid-thought under the generic Settings title.
- overridden (all four): the disabled radios are faint dashed rings (nearly invisible in light), and "Share weekly usage stats" stays highlighted while the stats are off.
- upload-error (all four): the failure line has no amber or red treatment and reads like an ordinary note.
- report-json-1024x680-dark/light: the "Features used · 2026-W42" heading is cut under the top edge.
- not-included (all four): the "What it reads" rows use a bigger type size than the card text above them.

## 0.5.1

Re-captured on 0.5.1; only the shared chrome changed in these screens. The rail's head now starts past the traffic lights, its rows hold a fixed dot column (tools on hover), cards and panels separate by tone instead of a border, a chosen item is a fill, and the top bar's bottom line is calmer. Privacy content is unchanged; the settings list gained AI and Models.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: The lens switch's chosen segment is a solid fill; rail rows carry a model mark where the model changed (F77). Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.

## 0.5.2 (2026-11-25, AJ-E1558, Opus side-by-side, both themes)

Re-captured on the host from a clean export of HEAD (the shared tree held another agent's unbuilt Bot work), both themes, every changed capture diffed (changed-pixel count and box) and looked at old beside new in contact sheets; the ones named below also opened at full size. Mock-ups: `d126-rich-ui/kit-dark.png` and `notif-dark.png` (rich mode), `calm-ui/mock-r2-*.png` (the rail). Captures: Every state at 1024×680 and 1280×800, both themes.

**What changed.** Usage statistics' three choices are the kit's cards with chips (sent, kept here, off) on an inset tone instead of an outlined fieldset with an accent outline on hover; the dashboard sync is a switch row; crash reports and session search are rows with chips, spaced, not divided by rules; the configuration row has its icon.

**Judged.** Dark and light side by side: in dark the chips and the accent's soft step read clearly against the cards; in light the soft step is paler but the lit chip's violet glyph, the switch's accent fill and the chosen fill still mark what is on (two cues each, never colour alone). No outline came back (PAT-SURF): separation is the card's tone and spacing. Text keeps the type roles (14 px titles, 13 px help), AA in both themes (the gate's contrast spec). Kit cards (RC-2) and rows.

| # | Line | Score | Note |
|---|---|---|---|
| 4 | Mode | 4 | Rich, the kit's dials: icons on every row and choice, the accent for on and chosen. |
| 6 | Areas | 4 | Tone, not outlines; no box in a box. |
| 7 | Colour | 4 | The accent means on, chosen, recommended or navigation; status keeps its hue (amber words on neutral, never a fill). |
| 14 | Consistency | 4 | The kit's own pieces (RowChip, the switch, ChoiceOption cards, SELECTED_FILL, Modal's icon tile). |
| 19 | Rich-mode colour | 4 | Before (42aab7661): 2, monochrome rows, bare checkboxes, a dot or a ✓ alone for chosen. Now every row and choice its icon, on and chosen in the accent's soft step, chosen a fill. |

The other lines are as the 0.5.1 critique scored them (unchanged by this work; lowest 4).
