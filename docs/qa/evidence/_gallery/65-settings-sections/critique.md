<!-- captures: 4863e5d5aea6 -->
# 65-settings-sections: every Settings section (General, Desktop apps, Accounts, Sessions, Switching, Agent access, Notifications, Privacy, Your Macs, Advanced)

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The re-rendered sections share one clear structure: a bold group heading over a card that stands off the canvas, a bright sentence level inside, and a quieter footnote under it. Selected tiles in general, the violet toggles in desktop-apps and the single solid "Yes, let them" in agent-access all read at a glance; in dark the selected tiles, radios, checkboxes and toggles now use the lighter violet. Accounts carries the Tools on this computer card ("Claude Code · 2 accounts · Version 2.1.296") between Accounts and profiles and Renewal dates. The rail's ai-juggler row now reads "Stopped with an error" whole in every 1280 capture. Some native controls still look foreign: the renewal date fields in accounts are raw mm/dd/yyyy inputs, and in dark their calendar icon nearly disappears. Your Macs (macs) still says its "Only this Mac can change it…" sentence twice, once in the card and again under it, and list-1024x680 actually shows Advanced, with the Settings nav taking a third of the width. Eight captures were not re-rendered and still show the older shell (a Home/Accounts/History/Settings/Fleet/More header, small-caps labels and native selects): cleanup, fleet, packs and renewal, in both themes.

## Weak

- accounts-1280x800-dark/light: renewal dates are unstyled native date inputs; in dark their calendar icon nearly disappears.
- macs-1280x800-dark/light: the card and its footnote repeat the same sentence.
- cleanup, fleet, packs, renewal (dark and light): not re-rendered; they show an older Settings layout that no longer matches the other sections.
