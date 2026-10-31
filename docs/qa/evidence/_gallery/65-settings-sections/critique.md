<!-- captures: 23a9c621d9f0 -->
# 65-settings-sections: every Settings section (General, Desktop apps, Accounts, Sessions, Switching, Agent access, Notifications, Privacy, Your Macs, Advanced)

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The re-rendered sections share one clear structure: a bold group heading over a card that now stands off the canvas, a brighter sentence level inside, and a quieter footnote under it. Selected tiles in general, the violet toggles in desktop-apps and the single solid "Yes, let them" in agent-access all read at a glance. Some native controls still look foreign: the renewal date fields in accounts are raw mm/dd/yyyy inputs, and in dark their calendar icon nearly disappears. Your Macs (macs) says its "Only this Mac can change it…" sentence twice, once in the card and again under it, and list-1024x680 actually shows Advanced, with the Settings nav taking a third of the width. Eight captures were not re-rendered and still show the older shell (a Home/Accounts/History/Settings/Fleet/More header, small-caps labels and native selects): cleanup, fleet, packs and renewal, in both themes.

## Weak

- accounts-1280x800-dark/light: renewal dates are unstyled native date inputs; in dark their calendar icon nearly disappears.
- macs-1280x800-dark/light: the card and its footnote repeat the same sentence.
- All re-rendered 1280 captures: rail row "Stopped with an error · 7…" loses its age.
- cleanup, fleet, packs, renewal (dark and light): not re-rendered; they show an older Settings layout that no longer matches the other sections.
