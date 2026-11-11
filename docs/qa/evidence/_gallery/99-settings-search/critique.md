<!-- captures: 78bd2e00d618 -->
<!-- captures: pending -->
# 99-settings-search: Settings' search

Design critique (D-57) against the rail's search field and Settings' sidebar (65-settings-sections). New family (AJ-E2905).

## Checklist

| Check | Finding |
|---|---|
| Grid | The field heads the sidebar at the panes' width; results open under it, on the sidebar's column. |
| Hierarchy | Each result: its pane small and muted over the setting's name. |
| Spacing | Results on the menu rhythm; the panes below keep their places, dimmed. |
| Icons | The rail search field's ⌕, so both searches look alike. |
| Shared controls | The rail's search field styling; the listbox is the combobox pattern (↑↓ ↩). |
| Disabled reasons | n/a: panes without a match dim but stay clickable. |
| Plain words | "No setting matches “nothing like this”." |
| Type sizes | 13 px field and names, 12 px pane labels. |
| Both themes | Checked; the dimmed panes are still legible in light. |
| 1280×800 and 1024×700 | Both; at 1024 the "no match" line wraps inside the sidebar without cutting. |

## Good

- Picking a result opens the pane at the section and moves focus to its title (opened captures show the focus ring on "Alerts and the digest").
- The index finds settings by other words ("mute" finds Notifications › Muted).

## Weak

- One result shows as a single short card; with many results the list has no count.
- The focused section title's ring spans the whole row width, heavier than the other focus rings.

## Fixed

- Axe (aria-required-children, listitem): "No setting matches" was a plain item inside the listbox. It is now a status line under the field, and the combobox points at a list only while there are results.

## 0.4.1 re-score (2026-11-07, AJ-E3305)

The whole family was retaken on release/0.4.1 for the 14-line re-score (AJ-E3305, `docs/qa/0.4-walks/design.md`) and opened on contact sheets, both themes, 1280 and 1024. Shared with every screen since the last shoot: the header's New session is tinted (one filled button per view), the Floor button is a split control with its own menu, This Mac's line is its dot below 1120 px, and the rail's App headers put the profile first.

Unchanged in design: no match says so under the field and dims the panes.
