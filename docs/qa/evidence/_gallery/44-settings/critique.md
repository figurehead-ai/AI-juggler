<!-- captures: 06f0f648de2a -->
# 44-settings: Settings › Advanced, the Figurehead & Cloud card from signed out to pairing code, linking, connected and unlink

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The cards stand off the page in both themes (white on the grey canvas in light, a step lighter in dark). In pairing-code the solid violet Connect account is the one clear primary next to the outlined Use a pairing code and the dim, disabled Link this Mac; in dark the violet is now the lighter accent, and the Cloud backup and Find my Macs switches in connected pick it up too. linking shows the QVM3-GABM code large in a mono box with Open the page again and Cancel, and connected lays out the three Cloud features with bold "Sends:" lines that make each one's cost easy to scan. unlink-confirm is a small, readable popover with a red-tinted Unlink beside Cancel. The design fixes show in the chrome: the rail's error rows now read "Stopped with an error" whole instead of "Stopped with an error · 7 …", and the top bar stays on one row at both sizes ("This Mac · Plenty of room" and "7 to clean up" at 1280, the pill alone at 1024). What the fixes did not touch: consent, paid-locked-offline, signed-out and unlinked still show the top of Advanced (the Cleanup card with its per-account checkboxes) and none of the Figurehead & Cloud card, so these four captures don't show the state they are named for. connected and unlink-confirm still open scrolled, with the card starting at "Cloud backup" under the top bar, and in light the off switch for Sync usage stats is a white track on a white card that almost disappears. Every capture was re-rendered.

## Weak

- consent, paid-locked-offline, signed-out, unlinked (all sizes, both themes): the capture stops at the Cleanup section and never reaches the Figurehead & Cloud card, so the named state isn't visible.
- connected-light, unlink-confirm-light: the off switch for "Sync usage stats to my dashboard" is nearly invisible on the white card.
- connected, unlink-confirm: the page is scrolled so the section heading is gone and the card starts cut under the top bar.

## 0.5.1

Re-captured on 0.5.1; only the shared chrome changed in these screens. The rail's head now starts past the traffic lights, its rows hold a fixed dot column (tools on hover), cards and panels separate by tone instead of a border, a chosen item is a fill, and the top bar's bottom line is calmer. Cloud and Advanced settings are unchanged; the settings list gained AI and Models.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: The lens switch's chosen segment is a solid fill; rail rows carry a model mark where the model changed (F77). Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.

## 0.5.2 (2026-11-25, AJ-E1558, Opus side-by-side, both themes)

Re-captured on the host from a clean export of HEAD (the shared tree held another agent's unbuilt Bot work), both themes, every changed capture diffed (changed-pixel count and box) and looked at old beside new in contact sheets; the ones named below also opened at full size. Mock-ups: `d126-rich-ui/kit-dark.png` and `notif-dark.png` (rich mode), `calm-ui/mock-r2-*.png` (the rail). Captures: Figurehead & Cloud and the Advanced pane behind it, every state, both themes.

**What changed.** The pane's sidebar tiles are the accent's soft step; Advanced's cleanup rows (seen in consent and linking states) are switch and check rows with chips; the Cloud card's switches are the kit's.

**Judged.** Dark and light side by side: in dark the chips and the accent's soft step read clearly against the cards; in light the soft step is paler but the lit chip's violet glyph, the switch's accent fill and the chosen fill still mark what is on (two cues each, never colour alone). No outline came back (PAT-SURF): separation is the card's tone and spacing. Text keeps the type roles (14 px titles, 13 px help), AA in both themes (the gate's contrast spec). Kit rows.

| # | Line | Score | Note |
|---|---|---|---|
| 4 | Mode | 4 | Rich, the kit's dials: icons on every row and choice, the accent for on and chosen. |
| 6 | Areas | 4 | Tone, not outlines; no box in a box. |
| 7 | Colour | 4 | The accent means on, chosen, recommended or navigation; status keeps its hue (amber words on neutral, never a fill). |
| 14 | Consistency | 4 | The kit's own pieces (RowChip, the switch, ChoiceOption cards, SELECTED_FILL, Modal's icon tile). |
| 19 | Rich-mode colour | 4 | Before (42aab7661): 2, monochrome rows, bare checkboxes, a dot or a ✓ alone for chosen. Now every row and choice its icon, on and chosen in the accent's soft step, chosen a fill. |

The other lines are as the 0.5.1 critique scored them (unchanged by this work; lowest 4).
