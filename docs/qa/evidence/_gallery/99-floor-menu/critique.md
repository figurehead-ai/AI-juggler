<!-- captures: ba98a7b0d8b1 -->
<!-- captures: pending -->
# 99-floor-menu: the Floor button's menu

Design critique (D-57) against the approved wireframes (D-78) and the user menu (99-user-menu). New family (AJ-E3511).

## Checklist

| Check | Finding |
|---|---|
| Grid | A split button beside the user menu: the Floor icon opens the Floor, the small arrow opens the menu under it. |
| Hierarchy | Three rows, the first with its key at the end. |
| Spacing | RowMenu's row height and padding. |
| Icons | The Floor, a page, the gear — the same icon-label-hint row as the user menu (MenuLine, shared). |
| Shared controls | RowMenu, MenuItem, MenuLine. |
| Disabled reasons | None needed: each row always works; where the Floor can't run, the toast says why. |
| Plain words | "Open the Floor", "The Floor list", "Floor settings…". |
| Type sizes | 14 px rows, 12 px key. |
| Both themes | Checked. |
| 1280×800 and 1024×700 | Both. |

## Good

- The list and the settings are one click from the button that opens the Floor, so neither needs its own header entry.
- The key text comes from the shortcut registry, as every other control's does.

## Weak

- The arrow is a small glyph; its target is 24 × 36 px, but the arrow itself reads small beside the 18 px Floor icon.

## Fixed

- The arrow's target was 16 px wide, under the 24 px minimum (WCAG 2.5.8); now 24 px.

- Rows had no icons; they now use the user menu's MenuLine, moved into RowMenu so both menus share it.

## 0.4.1 re-score (2026-11-07, AJ-E3305)

The whole family was retaken on release/0.4.1 for the 14-line re-score (AJ-E3305, `docs/qa/0.4-walks/design.md`) and opened on contact sheets, both themes, 1280 and 1024. Shared with every screen since the last shoot: the header's New session is tinted (one filled button per view), the Floor button is a split control with its own menu, This Mac's line is its dot below 1120 px, and the rail's App headers put the profile first.

Unchanged in design: Open the Floor ⌘⇧M, The Floor list, Floor settings…, each with its icon.
