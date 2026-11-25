<!-- captures: 2a3629435650 -->
# 84-notifications: Settings › Notifications, which alerts reach macOS, quiet hours and the digest's accounts

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

The alerts card stands a clear step off the page in both themes (a lighter card on the near-black canvas, a white card on the grey one), and the nine choices read as two levels: a bright label over a dimmer one-line hint, with the intro and footnote sentences a level brighter than the hints. The two captures differ only in their ticks (settings-chosen has "A session waits for you" and "This Mac is getting tight"; settings-off has none), and the violet tick is easy to spot in both themes; in dark the ticks, the digest's checkboxes, the Desktop apps and Switching tiles and New session now use the lighter accent. The only violet fill on the page is New session in the header, which is right for settings that apply at once. The page opens scrolled so the card starts at the top edge with no "Notifications" heading above it, and every capture ends half-way through the second row of "In the digest and notifications" ("Claude Code · Personal" is cut by the window edge), so the digest list is never seen whole. After the second pass the rail's "Refactor the settings screen" row keeps its time whole ("ai-ju… · viewed 2 min ago" where it read "ai-juggler · viewed 2 mi…"); nothing else changed.

## Weak

- settings-chosen-1280x800, settings-off-1280x800 (both themes): the capture cuts the digest list's "Claude Code · Personal" row in half at the bottom edge, so the end of the page is never shown.
- settings-chosen-1280x800, settings-off-1280x800 (both themes): the page's heading is scrolled out of view; the card starts at the top edge with nothing naming it.

## 0.5.1

Re-captured against 0.5.1: checked boxes read as accent fills against empty ones and the section is one tonal card with no outline. The copy and order of the choices did not change.

Nothing broken (checked both themes).

## 0.5.2 (2026-11-25, AJ-E1558, Opus side-by-side, both themes)

Re-captured on the host from a clean export of HEAD (the shared tree held another agent's unbuilt Bot work), both themes, every changed capture diffed (changed-pixel count and box) and looked at old beside new in contact sheets; the ones named below also opened at full size. Mock-ups: `d126-rich-ui/kit-dark.png` and `notif-dark.png` (rich mode), `calm-ui/mock-r2-*.png` (the rail). Captures: settings-chosen and settings-off at 1280×800, both themes (old beside new in the contact sheet).

**What changed.** The eleven kinds were bare checkboxes with words; each is now the kit's switch row with its icon (asks you, looks stuck, limit, finished, left running, low, crash, room again, This Mac, watches, digest), the chosen ones' switches in the accent and their chips lit; When to tell me and quiet hours are rows with chips; the digest's accounts are check rows. The tile column beside it is the accent's soft step.

**Judged.** Dark and light side by side: in dark the chips and the accent's soft step read clearly against the cards; in light the soft step is paler but the lit chip's violet glyph, the switch's accent fill and the chosen fill still mark what is on (two cues each, never colour alone). No outline came back (PAT-SURF): separation is the card's tone and spacing. Text keeps the type roles (14 px titles, 13 px help), AA in both themes (the gate's contrast spec). Matches `d126-rich-ui/notif-*.png` closely (icon chip, title, one line, switch).

| # | Line | Score | Note |
|---|---|---|---|
| 4 | Mode | 4 | Rich, the kit's dials: icons on every row and choice, the accent for on and chosen. |
| 6 | Areas | 4 | Tone, not outlines; no box in a box. |
| 7 | Colour | 4 | The accent means on, chosen, recommended or navigation; status keeps its hue (amber words on neutral, never a fill). |
| 14 | Consistency | 4 | The kit's own pieces (RowChip, the switch, ChoiceOption cards, SELECTED_FILL, Modal's icon tile). |
| 19 | Rich-mode colour | 4 | Before (42aab7661): 2, monochrome rows, bare checkboxes, a dot or a ✓ alone for chosen. Now every row and choice its icon, on and chosen in the accent's soft step, chosen a fill. |

The other lines are as the 0.5.1 critique scored them (unchanged by this work; lowest 4).
