<!-- captures: b71b57fe521a -->
# 65-settings-sections: every Settings section (General, Desktop apps, Accounts, Sessions, Switching, Agent access, Notifications, Privacy, Your Macs, Advanced)

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The re-rendered sections share one clear structure: a bold group heading over a card that stands off the canvas, a bright sentence level inside, and a quieter footnote under it. Selected tiles in general, the violet toggles in desktop-apps and the single solid "Yes, let them" in agent-access all read at a glance; in dark the selected tiles, radios, checkboxes and toggles now use the lighter violet. Accounts carries the Tools on this computer card ("Claude Code · 2 accounts · Version 2.1.296") between Accounts and profiles and Renewal dates. The rail's ai-juggler row now reads "Stopped with an error" whole in every 1280 capture. Some native controls still look foreign: the renewal date fields in accounts are raw mm/dd/yyyy inputs, and in dark their calendar icon nearly disappears. Your Macs (macs) still says its "Only this Mac can change it…" sentence twice, once in the card and again under it, and list-1024x680 actually shows Advanced, with the Settings nav taking a third of the width. Eight captures were not re-rendered and still show the older shell (a Home/Accounts/History/Settings/Fleet/More header, small-caps labels and native selects): cleanup, fleet, packs and renewal, in both themes.

## Weak

- accounts-1280x800-dark/light: renewal dates are unstyled native date inputs; in dark their calendar icon nearly disappears.
- macs-1280x800-dark/light: the card and its footnote repeat the same sentence.
- cleanup, fleet, packs, renewal (dark and light): not re-rendered; they show an older Settings layout that no longer matches the other sections.

## 0.5.1

Re-captured on 0.5.1; only the shared chrome changed in these screens. The rail's head now starts past the traffic lights, its rows hold a fixed dot column (tools on hover), cards and panels separate by tone instead of a border, a chosen item is a fill, and the top bar's bottom line is calmer. Settings sections keep their content; accounts and profiles now read apps, and the list gained AI and Models.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: The lens switch's chosen segment is a solid fill; rail rows carry a model mark where the model changed (F77). Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.

## 0.5.2 (2026-11-25, AJ-E1558, Opus side-by-side, both themes)

Re-captured on the host from a clean export of HEAD (the shared tree held another agent's unbuilt Bot work), both themes, every changed capture diffed (changed-pixel count and box) and looked at old beside new in contact sheets; the ones named below also opened at full size. Mock-ups: `d126-rich-ui/kit-dark.png` and `notif-dark.png` (rich mode), `calm-ui/mock-r2-*.png` (the rail). Captures: Every pane, 1280×800, both themes, and the list at 1024×680 (opened full size: general light, sessions dark, notifications dark, advanced dark, accounts light, switching both, privacy dark).

**What changed.** The sidebar's tiles are one hue now, the accent's soft step (navigation's colour, PAT-COLOR), with the pane shown in the accent's fill and a white glyph; before, seven saturated fills including amber (Agent access) and red (Notifications) meant nothing. Every section's card is its tone, with no outline, no rule between rows and no shadow (light's cards had a visible drop shadow). Every row leads with its 30 px icon chip, lit in the accent's soft step while it is on: Sessions' bare checkbox list is now the kit's switch rows (restart, awake, battery, reopen, login) and its numbers are rows with chips and filled fields, the group labels small capitals; Notifications' eleven kinds are switch rows with their icons, as the notif mock draws them; Advanced's cleanup pause is a switch row and each app's two tidying choices are check rows with chips on an inset tone; General's theme and session-list cards carry icons and the chosen card is a fill with its chip lit; Switching's key choices are the kit's cards with chips and a "Recommended" accent tag, its per-tool rows lead with the tool's logo; Accounts' renewal rows lead with the calendar chip; Agent access's pools, Your Macs' peers and Privacy's reads each have their chip; the switch is the kit's (the accent's fill when on, a quiet track when off).

**Judged.** Dark and light side by side: in dark the chips and the accent's soft step read clearly against the cards; in light the soft step is paler but the lit chip's violet glyph, the switch's accent fill and the chosen fill still mark what is on (two cues each, never colour alone). No outline came back (PAT-SURF): separation is the card's tone and spacing. Text keeps the type roles (14 px titles, 13 px help), AA in both themes (the gate's contrast spec). Matches the kit's CheckRow/ToggleRow and the notif mock's rows; deviation: the mock's tiles are several hues, these are one (PAT-COLOR, after the mock).

| # | Line | Score | Note |
|---|---|---|---|
| 4 | Mode | 4 | Rich, the kit's dials: icons on every row and choice, the accent for on and chosen. |
| 6 | Areas | 4 | Tone, not outlines; no box in a box. |
| 7 | Colour | 4 | The accent means on, chosen, recommended or navigation; status keeps its hue (amber words on neutral, never a fill). |
| 14 | Consistency | 4 | The kit's own pieces (RowChip, the switch, ChoiceOption cards, SELECTED_FILL, Modal's icon tile). |
| 19 | Rich-mode colour | 4 | Before (42aab7661): 2, monochrome rows, bare checkboxes, a dot or a ✓ alone for chosen. Now every row and choice its icon, on and chosen in the accent's soft step, chosen a fill. |

The other lines are as the 0.5.1 critique scored them (unchanged by this work; lowest 4).
